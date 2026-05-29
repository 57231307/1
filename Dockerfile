# 多阶段构建 - 后端
FROM rust:1.94.1-slim as backend-builder

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --bin server --bin bingxi
COPY backend/src ./src
RUN cargo build --release --bin server --bin bingxi

# 多阶段构建 - 前端
FROM node:22-alpine as frontend-builder

WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npx vite build

# 生产镜像
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    nginx \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 创建应用目录
RUN mkdir -p /opt/bingxi-erp/backend \
    /opt/bingxi/frontend/dist \
    /etc/bingxi \
    /var/log/bingxi-erp

# 创建非root用户
RUN groupadd -r appuser && useradd -r -g appuser -d /opt/bingxi-erp -s /sbin/nologin appuser

# 复制后端二进制文件
COPY --from=backend-builder /app/target/release/server /opt/bingxi-erp/backend/
COPY --from=backend-builder /app/target/release/bingxi /opt/bingxi-erp/backend/
RUN chmod +x /opt/bingxi-erp/backend/server /opt/bingxi-erp/backend/bingxi

# 复制前端文件
COPY --from=frontend-builder /app/dist/ /opt/bingxi/frontend/dist/

# 复制配置文件
COPY backend/.env.example /etc/bingxi/.env
COPY deploy/nginx.conf /etc/nginx/sites-available/bingxi-erp
RUN ln -sf /etc/nginx/sites-available/bingxi-erp /etc/nginx/sites-enabled/ \
    && rm -f /etc/nginx/sites-enabled/default

# 复制启动脚本
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# 设置权限
RUN chown -R appuser:appuser /opt/bingxi-erp \
    && chown -R appuser:appuser /opt/bingxi/frontend/dist \
    && chown -R appuser:appuser /var/log/bingxi-erp \
    && chown -R appuser:appuser /etc/bingxi

EXPOSE 80 8082

HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8082/api/v1/erp/health || exit 1

USER appuser
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]