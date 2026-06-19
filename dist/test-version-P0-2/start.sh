#!/bin/bash
# 冰溪 ERP P0-2 主备隔离 TEST 测试版本启动脚本

set -e

echo "==================================="
echo "  冰溪 ERP P0-2 主备隔离"
echo "  TEST 测试版本"
echo "==================================="
echo ""

# 检查 Docker
if ! command -v docker &> /dev/null; then
    echo "错误: 未安装 Docker"
    echo "请访问 https://docs.docker.com/get-docker/ 安装"
    exit 1
fi

# 检查 Docker Compose
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "错误: 未安装 Docker Compose"
    echo "请访问 https://docs.docker.com/compose/install/ 安装"
    exit 1
fi

# 选择 docker compose 命令
if command -v docker-compose &> /dev/null; then
    DC="docker-compose"
else
    DC="docker compose"
fi

# 复制配置（如果不存在）
if [ ! -f "config/failover.toml" ]; then
    echo "[1/5] 复制配置模板..."
    cp config/failover.toml.example config/failover.toml
    echo "已生成 config/failover.toml"
else
    echo "[1/5] config/failover.toml 已存在，跳过"
fi

# 检查环境变量
if [ -z "$POSTGRES_PRIMARY_PASSWORD" ] || [ -z "$POSTGRES_BACKUP_PASSWORD" ] || [ -z "$JWT_SECRET" ]; then
    echo ""
    echo "警告: 部分环境变量未设置"
    echo "请设置以下环境变量（或创建 .env 文件）："
    echo "  POSTGRES_PRIMARY_PASSWORD"
    echo "  POSTGRES_BACKUP_PASSWORD"
    echo "  JWT_SECRET"
    echo ""
    if [ -f ".env.example" ]; then
        echo "示例："
        cat .env.example
    fi
    echo ""
    read -p "是否继续？(y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# 构建镜像
echo "[2/5] 构建应用镜像..."
$DC build --no-cache

# 启动服务
echo "[3/5] 启动服务..."
$DC up -d

# 等待启动
echo "[4/5] 等待服务启动..."
sleep 15

# 健康检查
echo "[5/5] 健康检查..."
HEALTH_URL="http://localhost:8080/api/v1/erp/admin/failover/health"
if curl -s -f "$HEALTH_URL" > /dev/null 2>&1; then
    echo "✅ 服务启动成功"
else
    echo "⚠️  健康检查失败，请查看日志："
    echo "   $DC logs app"
    exit 1
fi

echo ""
echo "==================================="
echo "  启动成功！"
echo "==================================="
echo ""
echo "访问地址："
echo "  API:        http://localhost:8080"
echo "  监控页面:   http://localhost:8080/admin/failover"
echo "  健康检查:   http://localhost:8080/api/v1/erp/admin/failover/health"
echo "  状态:       http://localhost:8080/api/v1/erp/admin/failover/status"
echo "  指标:       http://localhost:8080/api/v1/erp/admin/failover/metrics"
echo ""
echo "下一步："
echo "  1. 查看故障注入测试：cat chaos-test-scenarios.md"
echo "  2. 停止服务：./stop.sh 或 $DC down"
echo ""
