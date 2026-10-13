# The API image: a cargo-chef staged build so dependency compilation is
# cached across code changes. Ships the server, the migrate tool and the
# sample seed (which carries the three fixture logs).

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
COPY src src
COPY migrations migrations
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
COPY .cargo .cargo
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY migrations migrations
COPY tests/fixtures/parser/testlog.txt tests/fixtures/parser/testlog2.txt tests/fixtures/parser/testlog3.txt tests/fixtures/parser/
RUN cargo build --release --bin loganalyzer --bin migrate --bin seed_samples

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/loganalyzer /app/target/release/migrate \
    /app/target/release/seed_samples /usr/local/bin/
RUN useradd --system --create-home --uid 10001 app \
    && mkdir -p /data/combat-logs \
    && chown -R app:app /app /data/combat-logs
USER app
ENV BIND_ADDR=0.0.0.0:3000 \
    LOG_STORAGE_DIR=/data/combat-logs
EXPOSE 3000
CMD ["loganalyzer"]
