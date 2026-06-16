#!/bin/bash
# 冰溪 ERP P0-3 测试版本启动脚本
# 创建时间: 2026-06-17

set -e

echo "========================================="
echo "  冰溪 ERP 定制订单全流程跟踪 P0-3 测试版"
echo "  启动中..."
echo "========================================="

# 1. 等待数据库就绪
echo "[1/4] 等待 PostgreSQL 就绪..."
for i in {1..30}; do
  if pg_isready -h postgres -p 5432 -U bingxi 2>/dev/null; then
    echo "  PostgreSQL 已就绪"
    break
  fi
  if [ $i -eq 30 ]; then
    echo "  ✗ PostgreSQL 启动超时"
    exit 1
  fi
  sleep 2
done

# 2. 执行数据库 migration
echo "[2/4] 执行数据库 migration..."
MIGRATION_DIR="/app/backend/migrations"
for dir in $(ls -d $MIGRATION_DIR/20260617* 2>/dev/null | sort); do
  echo "  执行 $(basename $dir)..."
  PGPASSWORD=${DB_PASSWORD:-bingxi123} psql -h postgres -U bingxi -d bingxi_erp -f "$dir/up.sql" 2>&1 | tail -3
done

# 3. 启动后端服务
echo "[3/4] 启动后端服务..."
cd /app
./bingxi-erp --config /app/dist/config/custom-order.toml &
BACKEND_PID=$!
echo "  后端 PID: $BACKEND_PID"

# 4. 等待后端健康
echo "[4/4] 等待后端健康检查..."
for i in {1..30}; do
  if curl -f http://localhost:8080/health 2>/dev/null; then
    echo "  ✓ 后端服务正常"
    break
  fi
  if [ $i -eq 30 ]; then
    echo "  ✗ 后端健康检查超时"
    exit 1
  fi
  sleep 2
done

echo ""
echo "========================================="
echo "  启动完成"
echo "  后端 API: http://localhost:8080/api/v1/erp/custom-orders"
echo "  健康检查: http://localhost:8080/health"
echo "========================================="

# 保持进程运行
wait $BACKEND_PID
