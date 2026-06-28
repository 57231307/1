# ==============================================================================
# 阶段 1: Cargo Chef 依赖规划 (Recipe)
# ==============================================================================
FROM rust:1.94-slim-bookworm AS chef
# 使用 --locked 确保使用 Cargo.lock 中锁定的依赖版本，避免因上游 crate 更新导致构建不可重复
RUN cargo install cargo-chef --locked
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
# 清理 build 缓存，减小 builder 镜像层（不影响 runtime 镜像，但加快推送）
# 批次 21 修复：builder 阶段继承 cargo-chef + 全套 build-essential + cargo registry 缓存，
# 导致镜像臃肿。cargo clean 移除 target/ 下的中间产物，保留 release 二进制由后续 COPY 提取。
# 注意：cargo clean 会删除整个 target/ 目录，需先备份 release 二进制再恢复，
# 避免 runtime 阶段 COPY --from=builder /app/target/release/server 失败。
RUN cp target/release/server /tmp/server-backup \
    && cargo clean \
    && mkdir -p target/release \
    && cp /tmp/server-backup target/release/server \
    && rm -f /tmp/server-backup

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
