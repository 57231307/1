#!/bin/bash
# 冰溪 ERP P0-2 主备隔离 TEST 测试版本停止脚本

# 选择 docker compose 命令
if command -v docker-compose &> /dev/null; then
    DC="docker-compose"
else
    DC="docker compose"
fi

echo "正在停止服务..."
$DC down

echo "✅ 停止成功"
echo ""
echo "完全清理（包括卷）："
echo "  $DC down -v"
