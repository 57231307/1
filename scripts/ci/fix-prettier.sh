#!/bin/bash
# ==============================================================================
# scripts/ci/fix-prettier.sh
#
# 用途：自动修复所有前端格式问题
# 用法：bash scripts/ci/fix-prettier.sh
# 前置：本地已安装 Node.js 22+ 和 npm
# ==============================================================================
set -euo pipefail

# 切换到项目根目录
cd "$(dirname "$0")/../.."

echo "🔧 修复前端格式..."
echo ""

# 检查工具链
if ! command -v node &> /dev/null; then
    echo "❌ 错误：node 未安装"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "❌ 错误：npm 未安装"
    exit 1
fi

# 修复 frontend 格式
cd frontend

if [ ! -d "node_modules" ]; then
    echo "📦 安装前端依赖..."
    npm ci --prefer-offline --no-audit
    echo ""
fi

echo "📁 frontend/src/"
npm run format
echo "✅ frontend/src/ 格式已修复"
echo ""

# 验证修复
echo "🔍 验证修复..."
if npx prettier --check 'src/**/*.{vue,ts,tsx,js,jsx,css,scss,json,md}' &> /dev/null; then
    echo "✅ 所有代码格式正确"
else
    echo "⚠️  仍有格式差异（详见上面输出）"
fi

cd ..
echo ""
echo "💡 建议下一步："
echo "  1. 运行 'git diff frontend/src/' 检查修改"
echo "  2. 运行 'npm run build' 验证构建"
echo "  3. 提交修改"
