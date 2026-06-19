# notifications-service Dockerfile
# 多阶段构建：rust 编译 + debian slim 运行

FROM rust:1.94-slim-bookworm AS builder
WORKDIR /app

# 安装 protobuf-compiler
RUN apt-get update && \
    apt-get install -y protobuf-compiler pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# 先复制 Cargo 文件以利用 Docker 缓存
COPY Cargo.toml ./
COPY build.rs ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/notifications_service*

# 复制源码
COPY src ./src
COPY proto ./proto
COPY tests ./tests

# 真实编译
RUN cargo build --release

# 运行镜像（精简）
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/notifications-service /usr/local/bin/notifications-service
COPY migrations ./migrations

EXPOSE 50056

ENV DATABASE_URL=postgres://erp:erp@postgres:5432/notifications_db
ENV GRPC_PORT=50056

CMD ["notifications-service"]
