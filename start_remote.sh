#!/bin/bash
# 秉羲ERP系统启动脚本

echo "========================================"
echo "秉羲ERP系统启动脚本"
echo "========================================"

# 创建日志目录
mkdir -p /var/log/bingxi

# 停止旧进程
pkill -f server || true
sleep 2

# 切换到部署目录
cd /tmp/bingxi

# 解压前端文件
if [ -f "frontend.zip" ]; then
    unzip -o frontend.zip -d /tmp/bingxi/
    echo "前端文件解压完成"
fi

# 设置执行权限
chmod +x server

# 创建前端目录
mkdir -p /var/www/bingxi

# 复制前端文件到Nginx目录
cp -r /tmp/bingxi/dist/* /var/www/bingxi/
echo "前端文件部署完成"

# 启动后端服务
nohup ./server --addr 0.0.0.0:8080 > /var/log/bingxi/backend.log 2>&1 &
BACKEND_PID=$!
echo "后端服务已启动 (PID: $BACKEND_PID)"

# 等待后端启动
sleep 5

# 检查后端是否运行
if pgrep -f server > /dev/null; then
    echo "✓ 后端服务运行正常 (PID: $(pgrep -f server))"
else
    echo "✗ 后端服务启动失败，请查看日志"
    cat /var/log/bingxi/backend.log
    exit 1
fi

# 检查端口
if netstat -tuln | grep -q ":8080 "; then
    echo "✓ 后端端口 8080 监听正常"
else
    echo "✗ 后端端口 8080 未监听"
fi

echo ""
echo "========================================"
echo "部署完成！"
echo "后端地址: http://129.204.17.232:8080"
echo "前端地址: http://129.204.17.232"
echo "日志位置: /var/log/bingxi/"
echo "========================================"
