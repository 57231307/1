#!/bin/bash
set -e

# 启动后端服务
echo "启动后端服务..."
/opt/bingxi-erp/backend/server &
BACKEND_PID=$!

# 等待后端启动
echo "等待后端服务启动..."
sleep 5

# 检查后端是否启动成功
if ! kill -0 $BACKEND_PID 2>/dev/null; then
    echo "错误：后端服务启动失败"
    exit 1
fi

echo "后端服务启动成功 (PID: $BACKEND_PID)"

# 执行传入的命令（默认是 nginx）
exec "$@"