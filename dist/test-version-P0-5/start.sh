#!/bin/bash
# P0-5 面料多色号定价扩展 - 一键启动脚本

set -e

echo "🚀 启动 P0-5 TEST 测试版本..."

# 1. 检查 Docker
if ! command -v docker &> /dev/null; then
  echo "❌ 未安装 Docker，请先安装 Docker 20.10+"
  exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
  echo "❌ 未安装 Docker Compose，请先安装 Docker Compose 2.0+"
  exit 1
fi

# 2. 检查环境变量
if [ ! -f .env ]; then
  echo "⚠️  未找到 .env 文件，从 .env.example 复制..."
  cp .env.example .env
fi

# 3. 启动服务
echo "📦 启动 Docker Compose..."
docker-compose up -d

# 4. 等待服务就绪
echo "⏳ 等待服务启动（约 30-60 秒）..."
sleep 10

# 5. 检查后端健康状态
for i in {1..30}; do
  if curl -s http://localhost:8081/health > /dev/null 2>&1; then
    echo "✅ 后端服务已就绪"
    break
  fi
  echo "  等待后端启动... ($i/30)"
  sleep 2
done

# 6. 检查前端
if curl -s http://localhost:8080 > /dev/null 2>&1; then
  echo "✅ 前端服务已就绪"
fi

# 7. 输出访问信息
echo ""
echo "=========================================="
echo "🎉 P0-5 TEST 测试版本启动成功！"
echo "=========================================="
echo "前端地址: http://localhost:8080"
echo "后端 API: http://localhost:8081"
echo "PostgreSQL: localhost:5432 (user: bingxi / pass: bingxi_p0_5_test)"
echo ""
echo "默认账号："
echo "  管理员: admin / admin123"
echo "  销售员: sales / sales123"
echo ""
echo "面料多色号定价扩展核心功能："
echo "  - 16 个 API 端点"
echo "  - 3 个前端页面（列表 / 详情 / 批量调价）"
echo "  - 2 个组件（价格历史图表 / 批量调价对话框）"
echo "  - 5 张数据库表（1 扩展 + 4 新建）"
echo "  - 13 个 HTTP handler"
echo "  - 5 个业务 service"
echo "  - 1 个价格计算引擎（4 档阶梯 + VIP 95 折 + 季节 + 客户专属）"
echo "=========================================="
echo "查看日志: docker-compose logs -f"
echo "停止服务: docker-compose down"
echo "=========================================="
