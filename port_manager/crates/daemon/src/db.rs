use common::Lease;
use rusqlite::{Connection, Result, params};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS leases (
    port INTEGER PRIMARY KEY,
    service_name TEXT NOT NULL,
    allocated_at TEXT NOT NULL,
    last_heartbeat TEXT NOT NULL,
    ttl_seconds INTEGER NOT NULL,
    tags TEXT NOT NULL
);
"#;

/// Initialize the database at the given path, creating the directory if needed.
pub fn init_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

/// Get the default database path (~/.portmanager/leases.db)
pub fn default_db_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".portmanager")
        .join("leases.db")
}

/// Load all leases from the database into a HashMap.
pub fn load_leases(conn: &Connection) -> Result<HashMap<u16, Lease>> {
    let mut stmt = conn.prepare("SELECT port, service_name, allocated_at, last_heartbeat, ttl_seconds, tags FROM leases")?;

    let lease_iter = stmt.query_map([], |row| {
        let port: u16 = row.get(0)?;
        let service_name: String = row.get(1)?;
        let allocated_at_str: String = row.get(2)?;
        let last_heartbeat_str: String = row.get(3)?;
        let ttl_seconds: u64 = row.get(4)?;
        let tags_json: String = row.get(5)?;

        let allocated_at = DateTime::parse_from_rfc3339(&allocated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let last_heartbeat = DateTime::parse_from_rfc3339(&last_heartbeat_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(Lease {
            port,
            service_name,
            allocated_at,
            last_heartbeat,
            ttl_seconds,
            tags,
        })
    })?;

    let mut map = HashMap::new();
    for lease_result in lease_iter {
        if let Ok(lease) = lease_result {
            map.insert(lease.port, lease);
        }
    }
    Ok(map)
}

/// Save a lease to the database.
pub fn save_lease(conn: &Connection, lease: &Lease) -> Result<()> {
    let tags_json = serde_json::to_string(&lease.tags).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO leases (port, service_name, allocated_at, last_heartbeat, ttl_seconds, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            lease.port,
            lease.service_name,
            lease.allocated_at.to_rfc3339(),
            lease.last_heartbeat.to_rfc3339(),
            lease.ttl_seconds,
            tags_json,
        ],
    )?;
    Ok(())
}

/// Delete a lease from the database.
pub fn delete_lease(conn: &Connection, port: u16) -> Result<bool> {
    let rows = conn.execute("DELETE FROM leases WHERE port = ?1", params![port])?;
    Ok(rows > 0)
}

/// Update the heartbeat timestamp for a lease.
pub fn update_heartbeat(conn: &Connection, port: u16, timestamp: DateTime<Utc>) -> Result<bool> {
    let rows = conn.execute(
        "UPDATE leases SET last_heartbeat = ?1 WHERE port = ?2",
        params![timestamp.to_rfc3339(), port],
    )?;
    Ok(rows > 0)
}

/// Delete all expired leases from the database.
pub fn delete_expired(conn: &Connection, now: DateTime<Utc>) -> Result<Vec<u16>> {
    // First, get the expired ports
    let mut stmt = conn.prepare(
        "SELECT port, last_heartbeat, ttl_seconds FROM leases"
    )?;

    let expired: Vec<u16> = stmt.query_map([], |row| {
        let port: u16 = row.get(0)?;
        let last_heartbeat_str: String = row.get(1)?;
        let ttl_seconds: i64 = row.get(2)?;

        let last_heartbeat = DateTime::parse_from_rfc3339(&last_heartbeat_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| now);

        let expires_at = last_heartbeat + chrono::Duration::seconds(ttl_seconds);

        Ok((port, now > expires_at))
    })?
    .filter_map(|r| r.ok())
    .filter(|(_, expired)| *expired)
    .map(|(port, _)| port)
    .collect();

    // Delete expired leases
    for port in &expired {
        conn.execute("DELETE FROM leases WHERE port = ?1", params![port])?;
    }

    Ok(expired)
}
