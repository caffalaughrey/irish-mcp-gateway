# Admin Processes

This document describes the administrative processes available for the Irish MCP Gateway.

## 🚀 **Available Admin Commands**

### 1. **Health Check**
```bash
# Check service health
./irish-mcp-gateway health

# Check specific service URL
./irish-mcp-gateway health --url http://localhost:8080
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-01-06T19:00:00Z",
  "version": "0.1.0",
  "services": {
    "grammar": {
      "status": "healthy",
      "url": "http://grammar-service:8080"
    }
  }
}
```

### 2. **Configuration Validation**
```bash
# Validate current configuration
./irish-mcp-gateway config --validate
```

**Validates:**
- `MODE` is either "server" or "stdio"
- `PORT` is valid and not 0 (for server mode)
- All required environment variables are present

### 3. **Service Status**
```bash
# Show comprehensive service status
./irish-mcp-gateway status

# Check specific service URL
./irish-mcp-gateway status --url http://localhost:8080
```

**Output:**
```
🏥 Health Status: ✅ Healthy
🔧 Tools: ✅ Available

📋 Configuration:
  Mode: server
  Port: 8080
  Log Level: info
  Grammar Service: http://grammar-service:8080
```

### 4. **Grammar Service Testing**
```bash
# Test grammar service with default text
./irish-mcp-gateway test-grammar

# Test with specific URL and text
./irish-mcp-gateway test-grammar --url http://grammar:8080 --text "Tá an peann ar an mbord"
```

**Output:**
```
📝 Grammar check for: "Tá an peann ar an mbord"
🔍 Found 2 issues:
  1. Spelling error (SPELL:0:2)
  2. Grammar suggestion (GRAMMAR:5:8)
```

## 🔧 **Health Check Endpoint**

### GET `/healthz`

**Response Format:**
```json
{
  "status": "healthy" | "degraded" | "unhealthy",
  "timestamp": "2025-01-06T19:00:00Z",
  "version": "0.1.0",
  "services": {
    "grammar": {
      "status": "healthy" | "unhealthy",
      "url": "http://grammar-service:8080"
    }
  }
}
```

**Status Codes:**
- `200 OK` - Service is healthy
- `503 Service Unavailable` - Service is degraded or unhealthy

## 🐳 **Docker Health Check**

The Dockerfile includes a built-in health check:

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS http://127.0.0.1:${PORT}/healthz || exit 1
```

## 📊 **Monitoring Integration**

### Prometheus Metrics (Future)
- Service uptime
- Request count and latency
- Grammar service availability
- Error rates

### Log Aggregation
- Structured JSON logs via `tracing`
- Configurable log levels via `RUST_LOG`
- Request/response logging

## 🔍 **Troubleshooting**

### Common Issues

1. **Service won't start**
   ```bash
   ./irish-mcp-gateway config --validate
   ```

2. **Grammar service unreachable**
   ```bash
   ./irish-mcp-gateway test-grammar --url http://grammar:8080
   ```

3. **Health check failing**
   ```bash
   ./irish-mcp-gateway health --url http://localhost:8080
   ```

### Debug Mode
```bash
RUST_LOG=debug ./irish-mcp-gateway
```

## 🚀 **Production Deployment**

### Environment Variables
```bash
# Required
MODE=server
PORT=8080

# Optional
RUST_LOG=info
GRAMADOIR_BASE_URL=http://grammar-service:8080
DEPRECATE_REST=false
```

### Health Check in Kubernetes
```yaml
livenessProbe:
  httpGet:
    path: /healthz
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /healthz
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

## 📈 **Future Enhancements**

1. **Metrics Endpoint** - `/metrics` for Prometheus
2. **Admin Dashboard** - Web UI for service management
3. **Configuration Hot Reload** - Update config without restart
4. **Service Discovery** - Automatic backend service detection
5. **Circuit Breaker** - Graceful degradation patterns
