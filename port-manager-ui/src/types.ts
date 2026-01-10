export interface Lease {
    port: number;
    service_name: string;
    allocated_at: string;
    last_heartbeat: string;
    ttl_seconds: number;
    tags: string[];
}

export interface AllocateRequest {
    service_name: string;
    ttl_seconds?: number;
    tags?: string[];
}

export interface AllocateResponse {
    port: number;
    lease: Lease;
}
