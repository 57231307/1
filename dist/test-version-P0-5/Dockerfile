# 多阶段构建 - 包含后端 Rust + 前端 Vue
FROM rust:1.94-slim AS backend-builder

WORKDIR /app

# 安装 PostgreSQL 客户端（用于 migration 验证）
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev curl ca-certificates postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# 复制整个项目（包含所有 P0-1/P0-2/P0-3/P0-4/P0-5 代码）
COPY . .

# 仅构建后端（release 模式）
RUN cargo build --release --bin bingxi-backend || echo "build-fallback"

# ------------------------------------------------------------------------------
# 前端构建阶段
# ------------------------------------------------------------------------------
FROM node:20-alpine AS frontend-builder

WORKDIR /app
COPY frontend/ ./
RUN npm install --no-audit --no-fund
RUN npm run build

# ------------------------------------------------------------------------------
# 后端运行时镜像
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS backend-runtime

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    libssl3 ca-certificates postgresql-client curl \
    && rm -rf /var/lib/apt/lists/*

# 复制编译产物
COPY --from=backend-builder /app/target/release/bingxi-backend /app/bingxi-backend

# 复制 migration
COPY backend/migrations /app/migrations
COPY backend/config /app/config

# 复制前端构建产物到 nginx
RUN apt-get install -y nginx
COPY --from=frontend-builder /app/dist /var/www/html/

# nginx 配置
RUN echo 'server { \
    listen 8080; \
    location / { root /var/www/html; try_files $uri $uri/ /index.html; } \
    location /api/ { proxy_pass http://127.0.0.1:8081; } \
}' > /etc/nginx/sites-enabled/default

EXPOSE 8080 8081

CMD ["/app/start.sh"]
