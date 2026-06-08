# ==============================================================================
# 阶段 1: 构建后端 (Rust)
# ==============================================================================
FROM rust:1.80-slim-bookworm as backend-builder

RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/ ./backend/
COPY database/ ./database/
WORKDIR /app/backend

# 构建后端应用
RUN cargo build --release --bin server

# ==============================================================================
# 阶段 2: 构建前端 (Vue/Vite)
# ==============================================================================
FROM node:20-slim as frontend-builder
WORKDIR /app

# 安装前端依赖
COPY frontend/package*.json ./
RUN npm install

# 复制前端源码并构建
COPY frontend/ .
RUN cp .env.production.example .env.production || true
RUN npm run build

# ==============================================================================
# 阶段 3: 最终运行镜像 (All-in-one)
# ==============================================================================
FROM debian:bookworm-slim

# 安装运行环境依赖 (Nginx, Supervisor, OpenSSL 等)
RUN apt-get update && apt-get install -y libssl3 ca-certificates nginx supervisor && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. 拷贝后端产物
COPY --from=backend-builder /app/backend/target/release/server /app/server
COPY --from=backend-builder /app/backend/config.yaml.example /app/config.yaml
COPY --from=backend-builder /app/database /app/database
RUN mkdir -p /app/logs

# 2. 拷贝前端产物
COPY --from=frontend-builder /app/dist /usr/share/nginx/html

# 3. 配置 Nginx
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
# 修改 Nginx 配置中的后端地址为 127.0.0.1，并将监听端口修改为 8080
RUN sed -i 's/http:\/\/backend:8082/http:\/\/127.0.0.1:8082/g' /etc/nginx/conf.d/default.conf && \
    sed -i 's/listen 80;/listen 8080;/g' /etc/nginx/conf.d/default.conf

# 4. 配置 Supervisor (用于同时管理 Nginx 和 Rust 后端)
RUN echo '[supervisord]\n\
nodaemon=true\n\
logfile=/var/log/supervisor/supervisord.log\n\
pidfile=/var/run/supervisord.pid\n\
\n\
[program:backend]\n\
command=/app/server\n\
directory=/app\n\
autostart=true\n\
autorestart=true\n\
stdout_logfile=/dev/stdout\n\
stdout_logfile_maxbytes=0\n\
stderr_logfile=/dev/stderr\n\
stderr_logfile_maxbytes=0\n\
\n\
[program:nginx]\n\
command=nginx -g "daemon off;"\n\
autostart=true\n\
autorestart=true\n\
stdout_logfile=/dev/stdout\n\
stdout_logfile_maxbytes=0\n\
stderr_logfile=/dev/stderr\n\
stderr_logfile_maxbytes=0\n\
' > /etc/supervisor/conf.d/supervisord.conf

# 暴露端口 (只需暴露 Nginx 的 8080 端口，外部通过 8080 访问前后端)
EXPOSE 8080 50051

# 启动 Supervisor
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
