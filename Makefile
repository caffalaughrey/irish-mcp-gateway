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
	cargo test --all-features --all-targets --no-fail-fast

test-coverage:
	cargo llvm-cov --workspace --lcov --output-path lcov.info --exclude-files "main.rs"
	cargo llvm-cov report --exclude-files "main.rs"

test-coverage-html:
	cargo llvm-cov --workspace --html --output-path coverage/html --exclude-files "main.rs"

# push:
# 	docker push $(REGISTRY)/$(APP):dev
