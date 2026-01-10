use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub port: u16,
    pub service_name: String,
    pub allocated_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub ttl_seconds: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocateRequest {
    pub service_name: String,
    pub ttl_seconds: Option<u64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocateResponse {
    pub port: u16,
    pub lease: Lease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRequest {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResponse {
    pub service_name: String,
    pub port: Option<u16>,
    pub all_ports: Vec<u16>,
    pub lease: Option<Lease>,
}
