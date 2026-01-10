# PortManager
**The Localhost Control Plane for Modern Developers.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/built_with-Rust-orange)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-alpha-yellow)]()

PortManager is a lightweight, system-wide daemon that manages local TCP ports. It acts as a central authority for port allocation, preventing conflicts in complex microservice architectures and keeping your localhost development environment sane.

---

## Why PortManager?

In the era of microservices, AI agents, and local dev-stacks, assigning static ports (e.g., `8080`, `3000`) leads to "Address already in use" errors and configuration hell.

### The "Port Binding" Spectrum

| Solution | Scope | Best For | The Problem |
|----------|-------|----------|-------------|
| **Manual / Static** | Hardcoded configs | Simple Monoliths | Conflicts when running >1 project. "Who is using Port 3000?" |
| **PortManager** | **Localhost OS** | **Local Dev & Scripts** | **It fills this gap!** Centralized coordination for *all* local processes. |
| **Docker** | Container Network | Isolated Apps | Great internally, but you still map to Host Ports. PortManager can assign those host ports! |
| **Kubernetes** | Cluster | Production / Cloud | Overkill for running a simple local script or test suite. |

### When to use what?
- **Use Kubernetes**: When you are orchestrating containers across multiple servers or need production-grade self-healing.
- **Use PortManager**: When you are a developer running 5 scripts, 3 Docker containers, and a frontend locally, and you just want them to *work* without editing `.env` files every time.
- **Co-existence**: PortManager can feed Docker!
  ```bash
  # Example: Ask PortManager for a port, then tell Docker to use it
  PORT=$(portctl alloc my-db)
  docker run -p $PORT:5432 postgres
  ```

---

## Features

- **Centralized Registry**: One source of truth for "Who runs where?".
- **Lease Management**: Ports are leased with a TTL (Time-To-Live). If a script crashes, the port is freed automatically.
- **Persistent Storage**: Leases survive daemon restarts (SQLite backend).
- **Service Discovery**: Find services by name with the lookup endpoint.
- **Zero-Integration Wrapper**: Run any command with automatic port injection - no code changes required.
- **REST API**: Simple JSON API standardizes how tools request ports.
- **Dashboard**: A clean React UI to visualize your local port usage.

---

## Installation & Usage

### 1. Start the Daemon
```bash
cd port_manager
cargo run -p daemon
# Using database: ~/.portmanager/leases.db
# Listening on localhost:3030
```

### 2. Client CLI

#### Allocate a port
```bash
cargo run -p client -- alloc my-fast-api
# Allocated port: 8001
```

#### List all leases
```bash
cargo run -p client -- list
# Port: 8001, Service: my-fast-api, TTL: 300s
```

#### Release a port
```bash
cargo run -p client -- release 8001
# Released port: 8001
```

#### Lookup a service (Service Discovery)
```bash
cargo run -p client -- lookup my-backend
# 8001
```

#### Run a command with automatic port injection (recommended)
```bash
# Standard usage - injects PORT environment variable
cargo run -p client -- run my-server -- npm start
# Allocated port 8001 for service 'my-server'
# Running: npm ["start"] with PORT=8001
# ... your server runs with PORT=8001 ...
# Released port 8001

# Custom environment variable name
cargo run -p client -- run my-db --env-name DB_PORT -- ./start-db.sh

# With custom TTL (10 minutes)
cargo run -p client -- run my-service --ttl 600 -- python server.py
```

The `run` command is the **recommended way** to use PortManager - it requires zero changes to your application code. As long as your app reads the `PORT` environment variable (which most frameworks do by default), it just works.

### 3. Dashboard
```bash
cd port-manager-ui
npm install
npm run dev
# Open http://localhost:5173
```

---

## API Reference

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/alloc` | Allocate a new port |
| `POST` | `/release` | Release an allocated port |
| `POST` | `/heartbeat` | Renew a lease (prevent expiration) |
| `GET` | `/list` | List all active leases |
| `GET` | `/lookup?service=<name>` | Find port(s) by service name |

### Examples

#### Allocate a port
```bash
curl -X POST http://localhost:3030/alloc \
  -H "Content-Type: application/json" \
  -d '{"service_name": "my-api", "ttl_seconds": 300}'

# Response:
# {"port": 8000, "lease": {"port": 8000, "service_name": "my-api", ...}}
```

#### Lookup a service
```bash
curl "http://localhost:3030/lookup?service=my-api"

# Response:
# {"service_name": "my-api", "port": 8000, "all_ports": [8000], "lease": {...}}
```

#### Service Discovery in scripts
```bash
# Start your backend
cargo run -p client -- run backend -- node server.js &

# In another script, find the backend port dynamically
BACKEND_PORT=$(cargo run -p client -- lookup backend)
curl "http://localhost:$BACKEND_PORT/api/health"
```

---

## Integration Examples

### With Docker
```bash
PORT=$(cargo run -p client -- alloc my-db | grep "Allocated" | awk '{print $3}')
docker run -p $PORT:5432 postgres
```

### In Python
```python
import os
port = os.environ.get('PORT', 8080)
# Run with: cargo run -p client -- run my-python-app -- python app.py
```

### In Node.js
```javascript
const port = process.env.PORT || 3000;
// Run with: cargo run -p client -- run my-node-app -- node server.js
```

---

## Architecture

```
~/.portmanager/
  leases.db          # SQLite database (persistent storage)

localhost:3030       # Daemon REST API
  /alloc             # Allocate ports
  /release           # Release ports
  /heartbeat         # Keep leases alive
  /list              # List all leases
  /lookup            # Service discovery
```

---

## Contact & Support

**BRUCHMANN[TEC] INNOVATION GMBH**
*Innovation in every line of code.*

- **Web**: [bruchmann-tec.com](https://bruchmann-tec.com)
- **Email**: conrad@bruchmann-tec.com

---
*Built with Rust and React.*
