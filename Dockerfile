FROM rust:1.80-slim-bookworm AS chef
# 使用 cargo-chef 优化依赖构建缓存
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY backend/ ./backend/
WORKDIR /app/backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS backend-builder
# 安装编译所需工具和库
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend
COPY --from=planner /app/backend/recipe.json recipe.json
# 仅构建依赖层（能被充分缓存）
RUN cargo chef cook --release --recipe-path recipe.json
# 复制源码构建真正的应用
COPY backend/ .
RUN cargo build --release

# 第二阶段：前端构建
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ .
RUN cp .env.production.example .env.production || true
RUN npm run build

# 第三阶段：合并部署环境
FROM debian:bookworm-slim
# 最小权限原则：创建普通用户 appuser
RUN groupadd -r appuser && useradd -r -g appuser -m appuser

# 安装运行环境，包括 nginx, supervisor，并清理 apt 缓存
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置运行目录并转移权限给 appuser
WORKDIR /app
RUN chown -R appuser:appuser /app \
    && chown -R appuser:appuser /var/log/nginx \
    && chown -R appuser:appuser /var/lib/nginx \
    && chown -R appuser:appuser /run/nginx

# 复制后端产物
COPY --from=backend-builder --chown=appuser:appuser /app/backend/target/release/server /app/server
COPY --from=backend-builder --chown=appuser:appuser /app/backend/migration /app/migration
COPY --from=backend-builder --chown=appuser:appuser /app/backend/config.yaml.example /app/config.yaml.example

# 复制前端产物
COPY --from=frontend-builder --chown=appuser:appuser /app/frontend/dist /usr/share/nginx/html
# 复制 nginx 与 supervisor 配置
COPY frontend/nginx.conf /etc/nginx/nginx.conf
COPY deploy/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# 修正部分系统目录权限，确保 nginx/supervisor 可以用非 root 运行
RUN touch /var/run/nginx.pid && chown appuser:appuser /var/run/nginx.pid \
    && mkdir -p /var/log/supervisor && chown -R appuser:appuser /var/log/supervisor \
    && mkdir -p /var/run/supervisor && chown -R appuser:appuser /var/run/supervisor

# 切换到非 root 用户
USER appuser

EXPOSE 8080
# 使用 supervisor 守护 nginx 与后端的双进程
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
