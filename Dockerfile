# # ---- Build stage
# FROM rust:1.89.0-slim AS builder
# WORKDIR /app
# # Faster deps caching
# COPY Cargo.toml ./
# COPY crates/nlp-common/Cargo.toml crates/nlp-common/Cargo.toml
# COPY crates/gramadoir-remote/Cargo.toml crates/gramadoir-remote/Cargo.toml
# COPY crates/tools-gateway/Cargo.toml crates/tools-gateway/Cargo.toml
# RUN mkdir -p crates/nlp-common/src crates/gramadoir-remote/src crates/tools-gateway/src && \
#      echo 'fn main(){}' > crates/tools-gateway/src/main.rs && \
#      echo 'pub fn f(){}' > crates/nlp-common/src/lib.rs && \
#      echo 'pub fn f(){}' > crates/gramadoir-remote/src/lib.rs
# RUN cargo build --release -p tools-gateway

# # Real source
# COPY . .
# RUN cargo build --release -p tools-gateway

# ---- Runtime stage
# FROM debian:bookworm-slim AS runtime
# RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
# ENV RUST_LOG=info
# ENV PORT=8080
# WORKDIR /srv
# COPY --from=builder /app/target/release/tools-gateway /usr/local/bin/irish-mcp-gateway
# EXPOSE 8080
# CMD ["irish-mcp-gateway"]

# ---- build
FROM rust:1.89.0-slim AS builder
WORKDIR /app
COPY Cargo.toml .
COPY src ./src
RUN cargo build --release

# ---- run
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
ENV PORT=8080
ENV MODE=server
WORKDIR /srv
COPY --from=builder /app/target/release/hello-mcp-server /usr/local/bin/hello-mcp
EXPOSE 8080
CMD ["/usr/local/bin/hello-mcp"]
