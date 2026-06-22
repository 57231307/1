#!/bin/bash
# ==============================================================================
# scripts/ci/clippy-check.sh
#
# 用途：本地复现 CI 的 clippy 严格检查（带 baseline 对比）
# 用法：bash scripts/ci/clippy-check.sh
# 返回：
#   0 = 无新增警告
#   1 = 有新增警告
#   2 = 工具链缺失
# 前置：本地已安装 rust 1.94+ 工具链
# ==============================================================================
set -euo pipefail

# 切换到项目根目录
cd "$(dirname "$0")/../.."

BASELINE_FILE="backend/.clippy-baseline.txt"
REPORTS_DIR="backend/reports"

# 检查工具链
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误：cargo 未安装"
    exit 2
fi

if ! command -v jq &> /dev/null; then
    echo "❌ 错误：jq 未安装（apt install jq / brew install jq）"
    exit 2
fi

if ! cargo clippy --version &> /dev/null; then
    echo "❌ 错误：clippy 未安装（rustup component add clippy）"
    exit 2
fi

mkdir -p "$REPORTS_DIR"

# 跑 clippy 并收集警告
echo "🔍 运行 cargo clippy --all-targets --locked..."
cd backend
cargo clippy --all-targets --locked --message-format=json 2>reports/clippy-stderr.txt \
    | tee reports/clippy-output.json > /dev/null || true
cat reports/clippy-stderr.txt >> reports/clippy-output.json 2>/dev/null || true

# 解析警告
cat reports/clippy-output.json | \
    jq -r 'select(.reason == "compiler-message") |
           .message |
           select(.level == "warning" or .level == "error") |
           .rendered' 2>/dev/null | \
    sort -u > reports/clippy-current.txt

CURRENT_COUNT=$(wc -l < reports/clippy-current.txt)
echo ""
echo "📊 当前 clippy 警告/错误数: $CURRENT_COUNT"

# 对比基线
if [ ! -f "$BASELINE_FILE" ]; then
    echo "⚠️  未找到 $BASELINE_FILE"
    echo ""
    echo "💡 建议："
    echo "  1. 跑 'bash scripts/ci/setup-clippy-baseline.sh' 建立基线"
    echo "  2. 或直接查看 reports/clippy-current.txt 处理所有警告"
    echo ""
    exit 1
fi

BASELINE_COUNT=$(wc -l < "$BASELINE_FILE")
echo "📊 基线警告数: $BASELINE_COUNT"
echo ""

# 新警告
comm -23 reports/clippy-current.txt "$BASELINE_FILE" > reports/clippy-new.txt
NEW_COUNT=$(wc -l < reports/clippy-new.txt)

# 已修复警告
comm -13 reports/clippy-current.txt "$BASELINE_FILE" > reports/clippy-fixed.txt
FIXED_COUNT=$(wc -l < reports/clippy-fixed.txt)

echo "🆕 新增警告: $NEW_COUNT"
echo "🔧 已修复警告: $FIXED_COUNT"
echo ""

if [ "$NEW_COUNT" -eq "0" ]; then
    echo "✅ 无新增 clippy 警告，CI 会通过"
    if [ "$FIXED_COUNT" -gt "0" ]; then
        echo ""
        echo "🎉 本次运行修复了 $FIXED_COUNT 个历史警告！"
        echo "   建议刷新基线：bash scripts/ci/setup-clippy-baseline.sh"
    fi
    cd ..
    exit 0
fi

echo "❌ 新增 $NEW_COUNT 个警告，CI 会失败"
echo ""
echo "📋 新警告列表："
echo ""
head -30 reports/clippy-new.txt
echo ""

cd ..

echo "💡 处理方式："
echo "  1. 打开 reports/clippy-new.txt 查看完整警告"
echo "  2. 修复每个警告（修改代码或加 #[allow(clippy::lint_name)] 抑制）"
echo "  3. 重新跑本脚本验证"
echo ""

exit 1
