APP=irish-mcp-gateway
REGISTRY?=caffalaughrey

.PHONY: build run docker
build:
	cargo build -p tools-gateway

run:
	GRAMADOIR_BASE_URL?=http://localhost:5000 \
	RUST_LOG=info \
	cargo run -p tools-gateway

mcp-stdio:
	GRAMADOIR_BASE_URL?=http://localhost:5000 \
	RUST_LOG=info \
	cargo run -p tools-gateway -- --stdio

docker:
	docker build -t $(REGISTRY)/$(APP) .

docker-run:
	docker run --rm -p 8080:8080 -e MODE=server -e PORT=8080 $(REGISTRY)/$(APP)

test:
	cargo test --all

# push:
# 	docker push $(REGISTRY)/$(APP):dev
