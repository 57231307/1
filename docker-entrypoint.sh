#!/bin/bash
set -e

# 启动后端服务
echo "启动后端服务..."
/opt/bingxi-erp/backend/server &
BACKEND_PID=$!

# 等待后端启动
echo "等待后端服务启动..."
MAX_RETRIES=30
RETRY_INTERVAL=2
for i in $(seq 1 $MAX_RETRIES); do
    if curl -f http://localhost:8082/api/v1/erp/health >/dev/null 2>&1; then
        echo "后端服务启动成功 (PID: $BACKEND_PID)"
        break
    fi
    if [ $i -eq $MAX_RETRIES ]; then
        echo "错误：后端服务启动超时"
        exit 1
    fi
    sleep $RETRY_INTERVAL
done

# 执行传入的命令（默认是 nginx）
exec "$@"