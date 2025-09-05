#!/usr/bin/env bash
set -euo pipefail

BASE="${BASE:-http://localhost:8080}"

echo "[1/4] Initializing session at $BASE/mcp"
SESSION_ID=$(curl -s -S -D /tmp/mcp_init_headers -o /tmp/mcp_init_body \
  -X POST "$BASE/mcp" \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 0.5' \
  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"roots":{"listChanged":true},"sampling":{}},"clientInfo":{"name":"curl_acceptance","version":"0.1.0"}}}' >/dev/null; \
  grep -i 'MCP-Session-Id' /tmp/mcp_init_headers | awk -F': ' '{print $2}' | tr -d '\r')
if [[ -z "${SESSION_ID}" ]]; then
  echo "Failed to obtain MCP-Session-Id" >&2
  exit 1
fi
echo "Session: $SESSION_ID"

echo "[2/4] Posting notifications/initialized"
curl -s -S -i -X POST "$BASE/mcp" \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -H "MCP-Session-Id: $SESSION_ID" \
  --data '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' | head -n 1

echo "[3/4] Listing tools"
TOOLS_JSON=$(curl -s -S -N -X POST "$BASE/mcp" \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -H "MCP-Session-Id: $SESSION_ID" \
  --data '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | awk -F'data: ' '/^data: /{print $2; exit}')
echo "$TOOLS_JSON" | jq '.result.tools | map(.name)'

echo "[4/4] Calling gael.grammar_check"
CALL_JSON=$(curl -s -S -N -X POST "$BASE/mcp" \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -H "MCP-Session-Id: $SESSION_ID" \
  --data '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"gael.grammar_check","arguments":{"text":"Ta an peann ar an mbord"}}}' | awk -F'data: ' '/^data: /{print $2; exit}')
echo "$CALL_JSON" | jq '.result.structuredContent'

echo "Done."


