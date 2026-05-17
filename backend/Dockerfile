# syntax=docker/dockerfile:1.7

FROM rust:1.88-slim AS builder
WORKDIR /src

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libsqlite3-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
# Touch a stub to allow dependency build caching
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY static ./static
RUN cargo build --release --bin vongkimco-backend

FROM debian:bookworm-slim AS runtime
ENV TZ=Asia/Ho_Chi_Minh
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash app
WORKDIR /app

COPY --from=builder /src/target/release/vongkimco-backend /app/vongkimco-backend
COPY --from=builder /src/templates /app/templates
COPY --from=builder /src/static /app/static
COPY --from=builder /src/migrations /app/migrations
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

ENV PORT=8080 \
    DATABASE_URL="sqlite:///data/vongkimco.db?mode=rwc" \
    SCREENSHOT_DIR="/data/screenshots" \
    RUST_LOG="info,sqlx=warn,tower_http=info"

VOLUME ["/data"]
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/api/v1/health || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
