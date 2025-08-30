
# irish-mcp-gateway
## Get started
### Build and run
```bash
docker build -t irish-mcp-gateway . && docker run --network irish-mcp-net --rm -p 8080:8080 -e GRAMADOIR_BASE_URL=http://gramadoir-server:5000 irish-mcp-gateway
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

### Test `gael.grammar_check` tool
```bash
$ curl -s -X POST localhost:8080/mcp -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"tools.call","params":{"name":"gael.grammar_check","arguments":{"text":"Tá an peann ar an bord"}}}' 

{"jsonrpc":"2.0","id":1,"result":{"issues":[{"code":"Lingua::GA::Gramadoir/CLAOCHLU","end":21,"message":"Initial mutation missing","start":12,"suggestions":[]}]}}
```
