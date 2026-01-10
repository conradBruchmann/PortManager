use clap::{Parser, Subcommand};
use common::{AllocateRequest, AllocateResponse, HeartbeatRequest, ReleaseRequest, Lease, LookupResponse};
use reqwest::Client;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Allocate a new port
    Alloc {
        service_name: String,
        #[arg(long)]
        ttl: Option<u64>,
    },
    /// Release an allocated port
    Release {
        port: u16,
    },
    /// List all active leases
    List,
    /// Allocate a port and send heartbeats in a loop
    Loop {
        service_name: String,
        #[arg(long)]
        ttl: Option<u64>,
    },
    /// Lookup a service by name
    Lookup {
        service_name: String,
    },
    /// Run a command with an allocated port
    Run {
        /// Service name for the allocation
        service_name: String,

        /// TTL in seconds (default: 300)
        #[arg(long)]
        ttl: Option<u64>,

        /// Environment variable name for the port (default: PORT)
        #[arg(long, default_value = "PORT")]
        env_name: String,

        /// Command and arguments to execute
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}

const BASE_URL: &str = "http://localhost:3030";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = Client::new();

    match cli.command {
        Commands::Alloc { service_name, ttl } => {
            let req = AllocateRequest {
                service_name,
                ttl_seconds: ttl,
                tags: None,
            };
            let resp = client.post(format!("{}/alloc", BASE_URL))
                .json(&req)
                .send()
                .await?;

            if resp.status().is_success() {
                let alloc_resp: AllocateResponse = resp.json().await?;
                println!("Allocated port: {}", alloc_resp.port);
                println!("Lease: {:?}", alloc_resp.lease);
            } else {
                eprintln!("Failed to allocate port: {}", resp.status());
            }
        }
        Commands::Release { port } => {
            let req = ReleaseRequest { port };
            let resp = client.post(format!("{}/release", BASE_URL))
                .json(&req)
                .send()
                .await?;

            if resp.status().is_success() {
                println!("Released port: {}", port);
            } else {
                eprintln!("Failed to release port: {}", resp.status());
            }
        }
        Commands::List => {
            let resp = client.get(format!("{}/list", BASE_URL))
                .send()
                .await?;

            if resp.status().is_success() {
                let leases: Vec<Lease> = resp.json().await?;
                println!("Active Leases:");
                for lease in leases {
                    println!("Port: {}, Service: {}, TTL: {}s", lease.port, lease.service_name, lease.ttl_seconds);
                }
            } else {
                eprintln!("Failed to list leases: {}", resp.status());
            }
        }
        Commands::Loop { service_name, ttl } => {
            let req = AllocateRequest {
                service_name: service_name.clone(),
                ttl_seconds: ttl,
                tags: None,
            };
            let resp = client.post(format!("{}/alloc", BASE_URL))
                .json(&req)
                .send()
                .await?;

            if resp.status().is_success() {
                let alloc_resp: AllocateResponse = resp.json().await?;
                let port = alloc_resp.port;
                println!("Allocated port: {}. Starting heartbeat loop...", port);

                let mut interval = time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    let hb_req = HeartbeatRequest { port };
                    match client.post(format!("{}/heartbeat", BASE_URL)).json(&hb_req).send().await {
                        Ok(r) if r.status().is_success() => println!("Heartbeat sent for {}", port),
                        Ok(r) => {
                            eprintln!("Heartbeat failed: {}", r.status());
                            break;
                        }
                        Err(e) => {
                            eprintln!("Heartbeat error: {}", e);
                            break;
                        }
                    }
                }
            } else {
                eprintln!("Failed to allocate port: {}", resp.status());
            }
        }
        Commands::Lookup { service_name } => {
            let resp = client.get(format!("{}/lookup?service={}", BASE_URL, service_name))
                .send()
                .await?;

            if resp.status().is_success() {
                let lookup: LookupResponse = resp.json().await?;
                if let Some(port) = lookup.port {
                    println!("{}", port);
                } else {
                    eprintln!("No port found for service: {}", service_name);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to lookup service: {}", resp.status());
                std::process::exit(1);
            }
        }
        Commands::Run { service_name, ttl, env_name, command } => {
            if command.is_empty() {
                eprintln!("No command specified");
                std::process::exit(1);
            }

            // Allocate port
            let req = AllocateRequest {
                service_name: service_name.clone(),
                ttl_seconds: ttl,
                tags: None,
            };
            let resp = client.post(format!("{}/alloc", BASE_URL))
                .json(&req)
                .send()
                .await?;

            if !resp.status().is_success() {
                eprintln!("Failed to allocate port: {}", resp.status());
                std::process::exit(1);
            }

            let alloc_resp: AllocateResponse = resp.json().await?;
            let port = alloc_resp.port;
            println!("Allocated port {} for service '{}'", port, service_name);

            // Flag to signal heartbeat thread to stop
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            // Spawn heartbeat task
            let heartbeat_client = client.clone();
            let heartbeat_handle = tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(5));
                while running_clone.load(Ordering::SeqCst) {
                    interval.tick().await;
                    if !running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let hb_req = HeartbeatRequest { port };
                    match heartbeat_client.post(format!("{}/heartbeat", BASE_URL))
                        .json(&hb_req)
                        .send()
                        .await
                    {
                        Ok(r) if r.status().is_success() => {}
                        Ok(r) => {
                            eprintln!("Heartbeat failed: {}", r.status());
                            break;
                        }
                        Err(e) => {
                            eprintln!("Heartbeat error: {}", e);
                            break;
                        }
                    }
                }
            });

            // Run the command with PORT environment variable
            let cmd = &command[0];
            let args = &command[1..];

            println!("Running: {} {:?} with {}={}", cmd, args, env_name, port);

            let status = Command::new(cmd)
                .args(args)
                .env(&env_name, port.to_string())
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();

            // Stop heartbeat
            running.store(false, Ordering::SeqCst);
            heartbeat_handle.abort();

            // Release port
            let rel_req = ReleaseRequest { port };
            let _ = client.post(format!("{}/release", BASE_URL))
                .json(&rel_req)
                .send()
                .await;
            println!("Released port {}", port);

            // Exit with the command's exit code
            match status {
                Ok(s) => {
                    if !s.success() {
                        std::process::exit(s.code().unwrap_or(1));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run command: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
