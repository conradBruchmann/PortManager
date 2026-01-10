# PortManager

**The Localhost Control Plane for Modern Developers.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built_with-Rust-orange)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-alpha-yellow)]()

PortManager is a lightweight daemon that manages local TCP ports. It acts as a central authority for port allocation, preventing conflicts when running multiple services locally.

**One binary. One port. API + Dashboard included.**

![Dashboard](docs/dashboard.png)

---

## Quick Start

```bash
# Install via Homebrew (macOS)
brew install bruchmann-tec/tap/portmanager

# Run any command with automatic port allocation
portctl run my-api -- npm start
# → Allocated port 8000, sets $PORT, releases on exit

# See what's running
portctl list

# Open Dashboard
open http://localhost:3030
```

---

## The Problem

You're running 5 microservices, 2 AI agents, a database, and a frontend locally. Each needs a port. You edit `.env` files, restart services, get "Address already in use" errors, and waste 20 minutes figuring out who's using port 3000.

**PortManager fixes this.**

| Approach | When to Use | Limitation |
|----------|-------------|------------|
| **Hardcoded ports** | Solo monolith | Conflicts with multiple projects |
| **Docker Compose** | Containerized apps | Host port mapping still manual |
| **Kubernetes** | Production | Overkill for local dev |
| **PortManager** | **Local development** | **Made for this** |

---

## Features

- **Zero Integration**: `portctl run` injects `$PORT` automatically - no code changes
- **Service Discovery**: Find any service by name via API or CLI
- **Persistent Leases**: Survives daemon restarts (SQLite backend)
- **Auto-Cleanup**: Crashed processes release ports automatically (TTL-based)
- **Built-in Dashboard**: Visual overview at `localhost:3030`
- **REST API**: Language-agnostic integration
- **Single Binary**: ~4MB, no runtime dependencies

---

## Installation

### macOS (Homebrew)

```bash
brew install bruchmann-tec/tap/portmanager

# Starts automatically as background service
# Dashboard: http://localhost:3030
```

### Manual Installation

```bash
# Clone and build
git clone https://github.com/bruchmann-tec/portmanager.git
cd portmanager/port_manager
cargo build --release

# Install binaries
cp target/release/daemon ~/.local/bin/portmanager-daemon
cp target/release/client ~/.local/bin/portctl

# Start daemon
portmanager-daemon
```

### Run as Background Service (macOS)

```bash
# Create LaunchAgent for auto-start
cat > ~/Library/LaunchAgents/com.bruchmann-tec.portmanager.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.bruchmann-tec.portmanager</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/YOUR_USERNAME/.local/bin/portmanager-daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

# Load and start
launchctl load ~/Library/LaunchAgents/com.bruchmann-tec.portmanager.plist
```

---

## Usage

### The Recommended Way: `portctl run`

```bash
# Basic usage - injects PORT environment variable
portctl run my-api -- npm start

# Custom environment variable
portctl run my-db --env-name DATABASE_PORT -- ./start-postgres.sh

# Custom TTL (10 minutes instead of default 5)
portctl run my-service --ttl 600 -- python server.py
```

Your app just needs to read `process.env.PORT` (Node), `os.environ['PORT']` (Python), or `std::env::var("PORT")` (Rust). Most frameworks do this by default.

### Other Commands

```bash
# Manual allocation
portctl alloc my-service
# → Allocated port: 8000

# List all active leases
portctl list
# → Port: 8000, Service: my-service, TTL: 300s

# Find a service
portctl lookup my-service
# → 8000

# Release manually
portctl release 8000
```

### Dashboard

Open **http://localhost:3030** in your browser.

The dashboard shows all active port allocations in real-time.

---

## API Reference

All endpoints are available at `http://localhost:3030`.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/alloc` | Allocate a port |
| `POST` | `/release` | Release a port |
| `POST` | `/heartbeat` | Renew lease TTL |
| `GET` | `/list` | List all leases |
| `GET` | `/lookup?service=<name>` | Find port by service name |
| `GET` | `/` | Dashboard UI |

### Example: Allocate via curl

```bash
curl -X POST http://localhost:3030/alloc \
  -H "Content-Type: application/json" \
  -d '{"service_name": "my-api", "ttl_seconds": 300}'

# {"port":8000,"lease":{"port":8000,"service_name":"my-api",...}}
```

### Example: Service Discovery

```bash
# Start backend
portctl run backend -- node server.js &

# Frontend finds backend dynamically
BACKEND=$(portctl lookup backend)
curl http://localhost:$BACKEND/api/health
```

---

## Integration Examples

### Docker

```bash
# PortManager assigns the host port
portctl run my-redis -- docker run -p $PORT:6379 redis
```

### Python (Flask/FastAPI)

```python
import os
port = int(os.environ.get('PORT', 8080))
app.run(port=port)
```

```bash
portctl run my-flask -- python app.py
```

### Node.js (Express)

```javascript
const port = process.env.PORT || 3000;
app.listen(port);
```

```bash
portctl run my-express -- node server.js
```

### Rust (Axum/Actix)

```rust
let port: u16 = std::env::var("PORT")
    .unwrap_or("8080".into())
    .parse()
    .unwrap();
```

```bash
portctl run my-rust -- cargo run
```

---

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│                    PortManager Daemon                   │
│                   localhost:3030                        │
├─────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌───────────┐  │
│  │  /alloc │  │ /release│  │  /list  │  │ Dashboard │  │
│  └─────────┘  └─────────┘  └─────────┘  └───────────┘  │
├─────────────────────────────────────────────────────────┤
│                   SQLite Storage                        │
│              ~/.portmanager/leases.db                   │
└─────────────────────────────────────────────────────────┘

Port Range: 8000-9000 (1000 ports available)
Default TTL: 300 seconds (5 minutes)
Cleanup: Every 10 seconds, expired leases are removed
```

---

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| Port Range | 8000-9000 | Available ports for allocation |
| Default TTL | 300s | Lease duration if not specified |
| Cleanup Interval | 10s | How often expired leases are removed |
| Database | `~/.portmanager/leases.db` | SQLite storage location |
| Listen Address | `127.0.0.1:3030` | Daemon bind address |
| `PM_PORT_MIN` | `8000` | Start of port range (Environment Variable) |
| `PM_PORT_MAX` | `9000` | End of port range (Environment Variable) |

---

## Troubleshooting

### "Connection refused" when running portctl

The daemon isn't running. Start it:
```bash
portmanager-daemon
# or via launchctl
launchctl start com.bruchmann-tec.portmanager
```

### "Address already in use" on port 3030

Another process is using the daemon port:
```bash
lsof -i :3030
kill <PID>
```

### Ports not being released

Check if the process crashed without cleanup. List and manually release:
```bash
portctl list
portctl release <port>
```

---

## Roadmap

- [x] SQLite persistence
- [x] Service discovery (`/lookup`)
- [x] Zero-integration wrapper (`portctl run`)
- [x] Embedded dashboard
- [x] LaunchAgent support (macOS)
- [ ] Homebrew formula
- [ ] Linux systemd service
- [ ] WebSocket for real-time dashboard
- [x] Port range configuration
- [ ] Multi-user namespaces

---

## Contributing

Contributions welcome! Please open an issue first to discuss what you'd like to change.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## About

**PortManager** is developed by [BRUCHMANN\[TEC\] INNOVATION GMBH](https://bruchmann-tec.com).

- Web: [bruchmann-tec.com](https://bruchmann-tec.com)
- Email: conrad@bruchmann-tec.com

---

*Built with Rust. No JavaScript required to run.*
