#!/bin/bash
# 冰溪 ERP P0-4 色卡仓储管理测试版本启动脚本
# 创建时间: 2026-06-17

set -e

echo "=========================================="
echo "  冰溪 ERP P0-4 色卡仓储管理测试版本"
echo "  启动中..."
echo "=========================================="

# 检查 docker-compose 是否可用
if ! command -v docker-compose &> /dev/null; then
    echo "错误: docker-compose 未安装"
    exit 1
fi

# 启动服务
echo "[1/4] 启动 PostgreSQL + Redis..."
docker-compose up -d postgres redis

echo "[2/4] 等待数据库就绪..."
sleep 10

echo "[3/4] 启动后端服务..."
docker-compose up -d backend

echo "[4/4] 启动前端服务..."
docker-compose up -d frontend

echo ""
echo "=========================================="
echo "  启动完成！"
echo "=========================================="
echo ""
echo "前端地址: http://localhost:3000"
echo "后端 API: http://localhost:8080"
echo "色卡模块路径: /color-cards/list"
echo ""
echo "默认登录: 请使用管理员账号登录后访问色卡模块"
echo ""
echo "停止服务: docker-compose down"
echo "查看日志: docker-compose logs -f"
echo "=========================================="
