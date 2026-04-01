# Deployment

This guide covers deploying **noti-server** in production. The server is a single statically-linked binary with zero runtime dependencies beyond an optional SQLite database file.

## Quick Start with Docker

The fastest way to run noti-server:

```bash
# Build and start
docker compose up -d

# Verify
curl http://localhost:3000/health
```

## Docker

### Build the image

```bash
docker build -t noti-server .
```

The multi-stage Dockerfile produces a minimal Debian-slim image (~30 MB compressed) containing both `noti-server` and the `noti` CLI.

### Run with Docker

```bash
# Minimal (in-memory queue, no auth)
docker run -d --name noti -p 3000:3000 noti-server

# Production (SQLite queue, API keys, JSON logs)
docker run -d --name noti \
  -p 3000:3000 \
  -v noti-data:/data \
  -e NOTI_QUEUE_BACKEND=sqlite \
  -e NOTI_QUEUE_DB_PATH=/data/noti-queue.db \
  -e NOTI_API_KEYS="key-1,key-2" \
  -e NOTI_LOG_FORMAT=json \
  -e NOTI_WORKER_COUNT=8 \
  noti-server
```

### Docker Compose

The included `docker-compose.yml` provides a production-ready configuration:

```bash
# Start with default settings
docker compose up -d

# Start with custom API keys
NOTI_API_KEYS="my-secret-key" docker compose up -d

# View logs
docker compose logs -f noti-server

# Stop
docker compose down
```

Override any setting via environment variables or a `.env` file:

```ini
# .env
NOTI_API_KEYS=production-key-1,production-key-2
NOTI_LOG_LEVEL=debug
NOTI_WORKER_COUNT=8
NOTI_RATE_LIMIT_MAX=200
NOTI_PORT=8080
```

## Bare Metal / VM

### Download the binary

Pre-built binaries are available on the [Releases](https://github.com/loonghao/noti/releases) page. The `noti-server` binary is included alongside the CLI binary.

### Build from source

```bash
# Requires Rust 1.85+
cargo build --release --bin noti-server

# Binary at: target/release/noti-server
```

### Run directly

```bash
# Set environment and start
export NOTI_PORT=3000
export NOTI_QUEUE_BACKEND=sqlite
export NOTI_QUEUE_DB_PATH=/var/lib/noti/queue.db
export NOTI_API_KEYS="your-secret-key"
export NOTI_LOG_FORMAT=json

noti-server
```

## systemd Service

Create `/etc/systemd/system/noti-server.service`:

```ini
[Unit]
Description=noti notification server
Documentation=https://github.com/loonghao/noti
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=noti
Group=noti
ExecStart=/usr/local/bin/noti-server
Restart=always
RestartSec=5

# Environment
Environment=NOTI_PORT=3000
Environment=NOTI_QUEUE_BACKEND=sqlite
Environment=NOTI_QUEUE_DB_PATH=/var/lib/noti/queue.db
Environment=NOTI_LOG_FORMAT=json
Environment=NOTI_LOG_LEVEL=info
Environment=NOTI_WORKER_COUNT=4

# Or load from file
# EnvironmentFile=/etc/noti/noti-server.env

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/noti
PrivateTmp=yes
PrivateDevices=yes

[Install]
WantedBy=multi-user.target
```

Set up and start:

```bash
# Create user and data directory
sudo useradd --system --no-create-home noti
sudo mkdir -p /var/lib/noti
sudo chown noti:noti /var/lib/noti

# Install binary
sudo cp target/release/noti-server /usr/local/bin/

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now noti-server

# Check status
sudo systemctl status noti-server
sudo journalctl -u noti-server -f
```

## Environment Variables Reference

All configuration is done via environment variables. See the full list in [Environment Variables](/reference/environment-variables).

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_HOST` | `0.0.0.0` | Bind address |
| `NOTI_PORT` | `3000` | Listen port |
| `NOTI_API_KEYS` | *(empty)* | Comma-separated API keys; empty = no auth |
| `NOTI_QUEUE_BACKEND` | `memory` | `memory` or `sqlite` |
| `NOTI_QUEUE_DB_PATH` | `noti-queue.db` | SQLite database path |
| `NOTI_WORKER_COUNT` | `4` | Background queue workers |
| `NOTI_LOG_FORMAT` | `text` | `text` or `json` |
| `NOTI_LOG_LEVEL` | `info` | Log level filter |
| `NOTI_RATE_LIMIT_MAX` | `100` | Max requests per window |
| `NOTI_RATE_LIMIT_WINDOW_SECS` | `60` | Rate-limit window (seconds) |
| `NOTI_CORS_ALLOWED_ORIGINS` | `*` | Comma-separated allowed origins |
| `NOTI_MAX_BODY_SIZE` | `2097152` | Max request body (bytes) |

## Production Checklist

Before going to production, make sure you've addressed these items:

### Security

- [ ] Set `NOTI_API_KEYS` with strong, unique keys
- [ ] Restrict `NOTI_CORS_ALLOWED_ORIGINS` to your domains
- [ ] Place behind a reverse proxy (nginx, Caddy, Traefik) for TLS
- [ ] Don't expose port 3000 directly to the internet

### Reliability

- [ ] Use `NOTI_QUEUE_BACKEND=sqlite` for persistent task queue
- [ ] Mount the SQLite database on a persistent volume
- [ ] Set `NOTI_WORKER_COUNT` based on your load (1 worker per ~50 msg/min)
- [ ] Configure `NOTI_RATE_LIMIT_MAX` appropriate to your use case

### Observability

- [ ] Set `NOTI_LOG_FORMAT=json` for structured log aggregation
- [ ] Monitor the `/health` endpoint for liveness/readiness probes
- [ ] Monitor the `/api/v1/metrics` endpoint for operational metrics
- [ ] Set up alerting on health check failures

### Reverse Proxy Example (nginx)

```nginx
upstream noti {
    server 127.0.0.1:3000;
}

server {
    listen 443 ssl http2;
    server_name noti.example.com;

    ssl_certificate     /etc/ssl/certs/noti.pem;
    ssl_certificate_key /etc/ssl/private/noti.key;

    location / {
        proxy_pass http://noti;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Pass through request ID
        proxy_set_header X-Request-ID $request_id;
    }

    location /health {
        proxy_pass http://noti;
        access_log off;
    }
}
```

## Cloud Deployment

### Fly.io

```bash
# Install flyctl, then:
fly launch --image ghcr.io/loonghao/noti-server:latest
fly secrets set NOTI_API_KEYS="your-key"
fly volumes create noti_data --size 1
```

### Railway / Render

These platforms detect the `Dockerfile` automatically:

1. Connect your repository
2. Set environment variables in the dashboard
3. Deploy — the platform builds and runs the container

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: noti-server
spec:
  replicas: 2
  selector:
    matchLabels:
      app: noti-server
  template:
    metadata:
      labels:
        app: noti-server
    spec:
      containers:
        - name: noti-server
          image: ghcr.io/loonghao/noti-server:latest
          ports:
            - containerPort: 3000
          env:
            - name: NOTI_QUEUE_BACKEND
              value: "sqlite"
            - name: NOTI_QUEUE_DB_PATH
              value: "/data/noti-queue.db"
            - name: NOTI_LOG_FORMAT
              value: "json"
            - name: NOTI_API_KEYS
              valueFrom:
                secretKeyRef:
                  name: noti-secrets
                  key: api-keys
          volumeMounts:
            - name: data
              mountPath: /data
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests:
              cpu: 100m
              memory: 64Mi
            limits:
              cpu: 500m
              memory: 256Mi
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: noti-data
---
apiVersion: v1
kind: Service
metadata:
  name: noti-server
spec:
  selector:
    app: noti-server
  ports:
    - port: 80
      targetPort: 3000
  type: ClusterIP
```

::: tip Queue backend in Kubernetes
When running multiple replicas, each pod gets its own SQLite database. For shared queue state across replicas, use an external message broker or a shared database. The current SQLite backend is best suited for single-replica deployments.
:::
