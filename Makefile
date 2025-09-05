APP=irish-mcp-gateway
REGISTRY?=caffalaughrey

.PHONY: build run docker
build:
	cargo build --release

run:
	GRAMADOIR_BASE_URL?=http://localhost:5000 \
	RUST_LOG=info \
	MODE=server \
	cargo run --quiet

mcp-stdio:
	GRAMADOIR_BASE_URL?=http://localhost:5000 \
	RUST_LOG=info \
	MODE=stdio \
	cargo run --quiet

docker:
	docker build -t $(REGISTRY)/$(APP) .

docker-run:
	docker run --rm -p 8080:8080 -e MODE=server -e PORT=8080 $(REGISTRY)/$(APP)

test:
	cargo test --all

# push:
# 	docker push $(REGISTRY)/$(APP):dev
