# Multi-stage build: compile then run
# Use latest Rust so all dependencies are supported
FROM rust:bookworm AS builder

# Pass git SHA for Sentry release (fly deploy --build-arg GIT_REV_SHORT=$(git rev-parse --short=7 HEAD))
ARG GIT_REV_SHORT=unknown
ENV GIT_REV_SHORT=$GIT_REV_SHORT

WORKDIR /app
COPY . .

RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/cgm-insights-api /app/cgm-insights-api
COPY migrations /app/migrations

ENV PORT=3000
EXPOSE 3000

CMD ["/app/cgm-insights-api"]
