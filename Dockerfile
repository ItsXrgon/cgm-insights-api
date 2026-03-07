# Multi-stage build: compile then run
FROM rust:1.82-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/cgm-insights-api /app/cgm-insights-api

ENV PORT=3000
EXPOSE 3000

CMD ["/app/cgm-insights-api"]
