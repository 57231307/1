# ==============================================================================
# 阶段 1: Cargo Chef 依赖规划 (Recipe)
# ==============================================================================
FROM rust:1.94-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# 阶段 2: 构建 (Builder)
# ==============================================================================
FROM chef AS builder
# 安装必要的编译依赖 (protobuf, jemalloc, ssl等)
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# 仅编译依赖层并缓存
RUN cargo chef cook --release --recipe-path recipe.json

# 复制源码进行实际编译
COPY . .
# 强制使用 jemalloc (防止碎片化)
ENV JEMALLOC_SYS_WITH_MALLOC_CONF="background_thread:true,narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,abort_conf:true"
RUN cargo build --release --bin server

# ==============================================================================
# 阶段 3: 运行环境 (Runtime - Distroless)
# ==============================================================================
# 使用 Google 的 distroless 镜像，完全移除 shell，提供极致的安全隔离
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app

# 从 builder 拷贝编译好的二进制
COPY --from=builder --chown=nonroot:nonroot /app/target/release/server /app/server
# 拷贝配置文件
COPY --from=builder --chown=nonroot:nonroot /app/config.yaml.example /app/config.yaml
# 拷贝迁移文件
COPY --from=builder --chown=nonroot:nonroot /app/database /app/database

# 设置环境变量
ENV JEMALLOC_SYS_WITH_MALLOC_CONF="background_thread:true,narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,abort_conf:true"
ENV RUST_LOG="info"

EXPOSE 8082 50051
USER nonroot

CMD ["/app/server"]
