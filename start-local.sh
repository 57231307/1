#!/bin/bash
# 秉羲管理系统 - 本地快速启动脚本
# 使用方法: ./start-local.sh

set -e

echo "========================================"
echo "  秉羲管理系统 - 本地启动"
echo "========================================"

# 项目根目录
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$PROJECT_DIR/backend"
FRONTEND_DIR="$PROJECT_DIR/frontend"

echo "项目目录: $PROJECT_DIR"
echo ""

# 1. 检查环境
echo "[1/5] 检查环境..."

# 检查 Rust
if ! command -v rustc &> /dev/null; then
    echo "❌ 错误: Rust 未安装"
    exit 1
fi
echo "✅ Rust: $(rustc --version)"

# 检查 Cargo
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误: Cargo 未安装"
    exit 1
fi
echo "✅ Cargo: $(cargo --version)"

# 检查 Trunk
if ! command -v trunk &> /dev/null; then
    echo "❌ 错误: Trunk 未安装"
    echo "请运行: cargo install trunk"
    exit 1
fi
echo "✅ Trunk: $(trunk --version)"

# 检查 PostgreSQL 客户端
if ! command -v psql &> /dev/null; then
    echo "⚠️  警告: PostgreSQL 客户端未安装"
else
    echo "✅ PostgreSQL 客户端: $(psql --version)"
fi

echo ""

# 2. 检查 .env 文件
echo "[2/5] 检查配置文件..."
if [ ! -f "$BACKEND_DIR/.env" ]; then
    echo "⚠️  .env 文件不存在，正在从 .env.example 创建..."
    cp "$BACKEND_DIR/.env.example" "$BACKEND_DIR/.env"
    echo "✅ 已创建 .env 文件，请根据需要修改配置"
else
    echo "✅ .env 文件已存在"
fi

echo ""

# 3. 创建日志目录
echo "[3/5] 创建日志目录..."
mkdir -p "$BACKEND_DIR/logs"
echo "✅ 日志目录已创建"

echo ""

# 4. 编译后端
echo "[4/5] 编译后端..."
cd "$BACKEND_DIR"
echo "正在编译后端 (开发模式)..."
cargo build
echo "✅ 后端编译完成"

echo ""

# 5. 启动说明
echo "[5/5] 启动说明"
echo ""
echo "========================================"
echo "  启动服务"
echo "========================================"
echo ""
echo "请在两个独立的终端中分别运行以下命令："
echo ""
echo "终端 1 - 启动后端:"
echo "  cd $BACKEND_DIR"
echo "  cargo run"
echo ""
echo "终端 2 - 启动前端:"
echo "  cd $FRONTEND_DIR"
echo "  trunk serve --port 3000"
echo ""
echo "访问地址:"
echo "  - 前端: http://127.0.0.1:3000"
echo "  - 后端: http://127.0.0.1:8080"
echo "  - 健康检查: http://127.0.0.1:8080/api/v1/health"
echo ""
echo "详细部署指南请查看: ../../本地部署指南.md"
echo ""
echo "========================================"
