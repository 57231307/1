# 冰溪 ERP 后端镜像 - P0-2 主备隔离
# 多阶段构建：builder + runtime

# ==================== 阶段 1: builder ====================
FROM rust:1.94-slim AS builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制依赖清单
COPY backend/Cargo.toml backend/Cargo.lock /app/backend/
COPY backend/migration/Cargo.toml /app/backend/migration/ 2>/dev/null || true

# 预编译依赖（利用 Docker 缓存层）
RUN mkdir -p /app/backend/src && \
    echo "fn main() {}" > /app/backend/src/main.rs && \
    cd /app/backend && \
    cargo build --release --bin server || true

# 复制源代码
COPY backend/ /app/backend/

# 重新编译（只编译修改的部分）
RUN cd /app/backend && \
    cargo build --release --bin server

# ==================== 阶段 2: runtime ====================
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    curl \
    iptables \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户
RUN useradd -m -u 1000 -s /bin/bash bingxi

# 复制可执行文件
COPY --from=builder /app/backend/target/release/server /usr/local/bin/server

# 复制配置
COPY dist/test-version-P0-2/config /app/config

# 设置工作目录
WORKDIR /app

# 切换到非 root 用户
USER bingxi

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动命令
CMD ["server"]
