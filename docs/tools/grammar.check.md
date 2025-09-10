# grammar.check (An Gramadóir)

- TOOL_NAME: `grammar.check`
- Remote endpoint: POST `/api/gramadoir/1.0` on `gramadoir-server` (container name: `gramadoir`)
- Request body: `{ "teacs": string }`
- Response body (current): array of issues with fields including `msg`, `ruleId`, and positional strings; gateway maps to `{ code, message, start, end, suggestions }[]`

Config (ToolConfig):
- `base_url`: from `GRAMADOIR_BASE_URL` or TOML `grammar.base_url`
- `request_timeout_ms`, `retries`, `concurrency_limit`
