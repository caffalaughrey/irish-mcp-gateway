
# irish-mcp-gateway
## Get started
### Build and run
```bash
docker build -t irish-mcp-gateway . && docker run --rm -p 8080:8080 irish-mcp-gateway
```

### List tools
```bash
$ curl -s -X POST localhost:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools.list"}'

{"jsonrpc":"2.0","id":1,"result":{"tools":[{"description":"Return a friendly greeting","inputSchema":{"properties":{"name":{"type":"string"}},"required":[],"type":"object"},"name":"hello.echo"}]}}
```

### Test `hello.echo` tool
```bash
$ curl -s -X POST localhost:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools.call","params":{"name":"hello.echo","arguments":{"name":"Caffalaughrey"}}}'

{"jsonrpc":"2.0","id":2,"result":{"message":"Dia dhuit, Caffalaughrey!"}}
```