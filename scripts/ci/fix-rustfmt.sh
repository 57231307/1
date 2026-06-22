#!/bin/bash
# ==============================================================================
# scripts/ci/fix-rustfmt.sh
#
# 用途：自动修复所有 Rust 格式问题
# 用法：bash scripts/ci/fix-rustfmt.sh
# 前置：本地已安装 rust 1.94+ 工具链
# ==============================================================================
set -euo pipefail

# 切换到项目根目录
cd "$(dirname "$0")/../.."

echo "🔧 修复 Rust 格式..."
echo ""

# 检查工具链
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误：cargo 未安装"
    exit 1
fi

if ! cargo fmt --version &> /dev/null; then
    echo "❌ 错误：rustfmt 未安装（rustup component add rustfmt）"
    exit 1
fi

# 修复 backend 格式
cd backend
echo "📁 backend/"
cargo fmt --all
echo "✅ backend/ 格式已修复"
echo ""

# 验证修复
echo "🔍 验证修复..."
if cargo fmt --all -- --check &> /dev/null; then
    echo "✅ 所有代码格式正确"
else
    echo "⚠️  仍有格式差异（可能 rustfmt 与代码风格存在根本差异）"
    cargo fmt --all -- --check || true
fi

cd ..
echo ""
echo "💡 建议下一步："
echo "  1. 运行 'git diff backend/' 检查修改"
echo "  2. 运行 'cargo build --release --locked' 验证编译"
echo "  3. 提交修改（建议单独 commit 以便 review）"
