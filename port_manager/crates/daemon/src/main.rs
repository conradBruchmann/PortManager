mod db;

use axum::{
    body::Body,
    extract::{Path, Query, State, Json},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use common::{AllocateRequest, AllocateResponse, ReleaseRequest, HeartbeatRequest, Lease, LookupResponse};
use rust_embed::Embed;
use rusqlite::Connection;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock, Mutex},
    time::Duration,
};
use tokio::time;
use tower_http::cors::CorsLayer;
use chrono::Utc;

const DEFAULT_TTL: u64 = 300; // 5 minutes

#[derive(Embed)]
#[folder = "dashboard/"]
struct DashboardAssets;

#[derive(Clone)]
struct AppState {
    leases: Arc<RwLock<HashMap<u16, Lease>>>,
    db: Arc<Mutex<Connection>>,
    min_port: u16,
    max_port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize database
    let db_path = db::default_db_path();
    println!("Using database: {}", db_path.display());

    let conn = db::init_db(&db_path).expect("Failed to initialize database");

    // Load existing leases from database
    let existing_leases = db::load_leases(&conn).unwrap_or_default();
    let lease_count = existing_leases.len();
    if lease_count > 0 {
        println!("Loaded {} existing lease(s) from database", lease_count);
    }

    // Clean up expired leases immediately
    match db::delete_expired(&conn, Utc::now()) {
        Ok(expired) => {
            if !expired.is_empty() {
                println!("Cleaned up {} expired lease(s) on startup", expired.len());
            }
        },
        Err(e) => eprintln!("Failed to cleanup expired leases on startup: {}", e),
    }

    // Read configuration from environment
    let min_port: u16 = std::env::var("PM_PORT_MIN")
        .unwrap_or_else(|_| "8000".to_string())
        .parse()
        .expect("PM_PORT_MIN must be a valid port number");

    let max_port: u16 = std::env::var("PM_PORT_MAX")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .expect("PM_PORT_MAX must be a valid port number");

    println!("Port Range Configuration: {}-{}", min_port, max_port);

    let state = AppState {
        leases: Arc::new(RwLock::new(existing_leases)),
        db: Arc::new(Mutex::new(conn)),
        min_port,
        max_port,
    };

    // Background cleaner
    let cleaner_state = state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let now = Utc::now();

            // Get expired ports from memory
            let expired: Vec<u16> = {
                let leases = cleaner_state.leases.read().unwrap();
                leases
                    .iter()
                    .filter(|(_, lease)| {
                        let expires_at = lease.last_heartbeat + chrono::Duration::seconds(lease.ttl_seconds as i64);
                        now > expires_at
                    })
                    .map(|(port, _)| *port)
                    .collect()
            };

            // Remove from both memory and database
            if !expired.is_empty() {
                let mut leases = cleaner_state.leases.write().unwrap();
                let db = cleaner_state.db.lock().unwrap();

                for port in expired {
                    println!("Releasing expired port: {}", port);
                    leases.remove(&port);
                    let _ = db::delete_lease(&db, port);
                }
            }
        }
    });

    // API routes
    let api_routes = Router::new()
        .route("/alloc", post(allocate_port))
        .route("/release", post(release_port))
        .route("/heartbeat", post(heartbeat))
        .route("/list", get(list_leases))
        .route("/lookup", get(lookup_service))
        .with_state(state);

    // Main app: API + Dashboard
    let app = Router::new()
        .merge(api_routes)
        .route("/", get(index_handler))
        .route("/assets/{*path}", get(static_handler))
        .fallback(get(index_handler))  // SPA fallback
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3030));
    println!("Listening on http://{}", addr);
    println!("Dashboard available at http://{}/", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Serve index.html
async fn index_handler() -> impl IntoResponse {
    match DashboardAssets::get("index.html") {
        Some(content) => Html(content.data.into_owned()).into_response(),
        None => (StatusCode::NOT_FOUND, "Dashboard not found").into_response(),
    }
}

// Serve static assets (JS, CSS, images)
async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = format!("assets/{}", path);

    match DashboardAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}

async fn allocate_port(
    State(state): State<AppState>,
    Json(payload): Json<AllocateRequest>,
) -> Result<Json<AllocateResponse>, StatusCode> {
    let mut leases = state.leases.write().unwrap();

    // Find free port
    let mut selected_port = None;
    for port in state.min_port..=state.max_port {
        if !leases.contains_key(&port) {
            selected_port = Some(port);
            break;
        }
    }

    match selected_port {
        Some(port) => {
            let now = Utc::now();
            let lease = Lease {
                port,
                service_name: payload.service_name,
                allocated_at: now,
                last_heartbeat: now,
                ttl_seconds: payload.ttl_seconds.unwrap_or(DEFAULT_TTL),
                tags: payload.tags.unwrap_or_default(),
            };

            // Save to database first
            {
                let db = state.db.lock().unwrap();
                if let Err(e) = db::save_lease(&db, &lease) {
                    eprintln!("Failed to save lease to database: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }

            // Then update memory
            leases.insert(port, lease.clone());
            Ok(Json(AllocateResponse { port, lease }))
        }
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn release_port(
    State(state): State<AppState>,
    Json(payload): Json<ReleaseRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut leases = state.leases.write().unwrap();

    if leases.remove(&payload.port).is_some() {
        // Also delete from database
        let db = state.db.lock().unwrap();
        let _ = db::delete_lease(&db, payload.port);
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut leases = state.leases.write().unwrap();

    if let Some(lease) = leases.get_mut(&payload.port) {
        let now = Utc::now();
        lease.last_heartbeat = now;

        // Also update database
        let db = state.db.lock().unwrap();
        let _ = db::update_heartbeat(&db, payload.port, now);

        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn list_leases(
    State(state): State<AppState>,
) -> Json<Vec<Lease>> {
    let leases = state.leases.read().unwrap();
    Json(leases.values().cloned().collect())
}

async fn lookup_service(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<LookupResponse>, StatusCode> {
    let service_name = params.get("service").ok_or(StatusCode::BAD_REQUEST)?;

    let leases = state.leases.read().unwrap();
    let matching: Vec<&Lease> = leases
        .values()
        .filter(|l| l.service_name == *service_name)
        .collect();

    if matching.is_empty() {
        Ok(Json(LookupResponse {
            service_name: service_name.clone(),
            port: None,
            all_ports: vec![],
            lease: None,
        }))
    } else {
        let all_ports: Vec<u16> = matching.iter().map(|l| l.port).collect();
        let first_lease = matching.first().cloned().cloned();

        Ok(Json(LookupResponse {
            service_name: service_name.clone(),
            port: first_lease.as_ref().map(|l| l.port),
            all_ports,
            lease: first_lease,
        }))
    }
}
