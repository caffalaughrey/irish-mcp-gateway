# spell.check (GaelSpell)

- TOOL_NAME: `spell.check`
- Remote endpoint: POST `/api/gaelspell/1.0` on `gaelspell-server` (container name: `gaelspell`)
- Request body: `{ "teacs": string }`
- Response body (current): `[[token: string, suggestions: string[]], ...]`

Gateway mapping:
- Corrections array: `{ token, start, end, suggestions }[]` (positions are 0,0 in current MVP until upstream exposes offsets)

Config (ToolConfig):
- `base_url`: from `SPELLCHECK_BASE_URL` or TOML `spell.base_url`
- `request_timeout_ms`, `retries`, `concurrency_limit`
