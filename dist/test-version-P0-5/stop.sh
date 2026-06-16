#!/bin/bash
# P0-5 面料多色号定价扩展 - 停止脚本

echo "🛑 停止 P0-5 TEST 测试版本..."

cd "$(dirname "$0")"

docker-compose down

echo "✅ 已停止"
echo ""
echo "完全清理（包括数据卷）："
echo "  docker-compose down -v"
