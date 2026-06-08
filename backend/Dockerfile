# Builder stage
FROM rust:1.80-slim-bookworm as builder

# Install protobuf compiler and pkg-config for gRPC
RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/ ./backend/
WORKDIR /app/backend

# Build the application
RUN cargo build --release --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/backend/target/release/server /app/server

# Copy the config example as the default config.yaml
COPY --from=builder /app/backend/config.yaml.example /app/config.yaml

# Copy database migrations
COPY database /app/database

# Create logs directory
RUN mkdir -p /app/logs

EXPOSE 8082 50051

CMD ["/app/server"]
