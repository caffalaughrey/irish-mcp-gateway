
# irish-mcp-gateway

[![CI](https://github.com/caffalaughrey/irish-mcp-gateway/actions/workflows/ci.yml/badge.svg)](https://github.com/caffalaughrey/irish-mcp-gateway/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/caffalaughrey/irish-mcp-gateway/branch/main/graph/badge.svg)](https://codecov.io/gh/caffalaughrey/irish-mcp-gateway)
## Get started
### Build and run
```bash
docker build -t irish-mcp-gateway . && docker run --network irish-mcp-net --rm -p 8080:8080 -e GRAMADOIR_BASE_URL=http://gramadoir-server:5000 irish-mcp-gateway
```

### Acceptance (spec-aligned curl flow)
Use the acceptance script to validate the rmcp 0.5 stateful HTTP + SSE flow:
```bash
./scripts/acceptance.sh
```

The script performs:
1) POST initialize → capture `MCP-Session-Id`
2) POST `notifications/initialized` (expect 202)
3) POST→SSE tools/list → print tool names
4) POST→SSE tools/call `gael.grammar_check` → print structuredContent

### Configuration
- The gateway reads environment variables and an optional TOML file to configure tools.
- Precedence: environment variables override TOML values if both are set.
- To use TOML, set `TOOLING_CONFIG=/path/to/tooling-config.example.toml` and adjust values.
- Common vars:
  - `GRAMADOIR_BASE_URL`, `SPELLCHECK_BASE_URL`
  - `GRAMMAR_TIMEOUT_MS`, `SPELL_TIMEOUT_MS`
  - `GRAMMAR_RETRIES`, `SPELL_RETRIES`
