FROM rust:1.92-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake pkg-config libopus-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY tests ./tests
COPY README.md LICENSE ./
RUN cargo build --locked --release --bin omo-gateway

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libopus0 fonts-dejavu-core fonts-noto-cjk \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 omon \
    && useradd --system --uid 10001 --gid omon --home-dir /app --shell /usr/sbin/nologin omon \
    && mkdir -p /app/data /app/workspace \
    && chown -R omon:omon /app

WORKDIR /app
COPY --from=builder /build/target/release/omo-gateway /usr/local/bin/omo-gateway

ENV DATABASE_URL=sqlite:///app/data/omon_gateway.db \
    OMON_WORKSPACE_ROOT=/app/workspace \
    RUST_LOG=info

USER omon

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD test -r /proc/1/cmdline && grep -aq "omo-gateway" /proc/1/cmdline || exit 1

ENTRYPOINT ["/usr/local/bin/omo-gateway"]
