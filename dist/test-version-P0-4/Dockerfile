# 冰溪 ERP 色卡仓储管理 P0-4 测试版本
# 创建时间: 2026-06-17
# 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md

FROM rust:1.94-slim AS backend-builder

WORKDIR /app/backend

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# 复制后端代码
COPY backend/ ./

# 编译
RUN cargo build --release --bin bingxi-erp

# 前端构建
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/ ./
RUN npm install && npm run build

# 运行镜像
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制后端二进制
COPY --from=backend-builder /app/backend/target/release/bingxi-erp /app/bingxi-erp
# 复制前端 dist
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
# 复制配置和脚本
COPY dist/test-version-P0-4/ /app/dist/

EXPOSE 8080 3000

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动脚本
COPY dist/test-version-P0-4/start.sh /app/start.sh
RUN chmod +x /app/start.sh

CMD ["/app/start.sh"]
