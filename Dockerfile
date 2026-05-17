# 后端构建阶段
FROM rust:1.75-slim as backend-builder

WORKDIR /app/backend
COPY backend/ .

# 安装依赖
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# 构建后端
RUN cargo build --release

# 前端构建阶段
FROM node:20-slim as frontend-builder

WORKDIR /app/frontend
COPY frontend/ .

# 安装依赖并构建
RUN npm ci && npm run build

# 运行阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    nginx \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# 创建应用目录
WORKDIR /app

# 复制后端二进制文件
COPY --from=backend-builder /app/backend/target/release/server /app/backend/server

# 复制前端静态文件
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# 复制配置文件
COPY backend/config.yaml /app/backend/config.yaml
COPY deploy/nginx.conf /etc/nginx/sites-available/default

# 创建启动脚本
COPY <<'STARTUP' /app/start.sh
#!/bin/bash
set -e

# 等待数据库就绪
echo "等待数据库就绪..."
until pg_isready -h ${DB_HOST:-localhost} -p ${DB_PORT:-5432} -U ${DB_USER:-bingxi}; do
    sleep 1
done

# 启动后端
echo "启动后端服务..."
cd /app/backend
./server --config config.yaml &

# 启动 Nginx
echo "启动 Nginx..."
nginx -g "daemon off;" &
wait
STARTUP

RUN chmod +x /app/start.sh

# 暴露端口
EXPOSE 80

# 启动服务
CMD ["/app/start.sh"]
