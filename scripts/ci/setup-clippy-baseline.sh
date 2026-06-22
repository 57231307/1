#!/bin/bash
# ==============================================================================
# scripts/ci/setup-clippy-baseline.sh
#
# 用途：建立或更新 clippy 警告基线
# 背景：CI 新架构采用 baseline 机制，新警告 0 容忍，历史警告不阻塞
# 用法：
#   bash scripts/ci/setup-clippy-baseline.sh            # 建立基线
#   bash scripts/ci/setup-clippy-baseline.sh --refresh   # 重建基线（用于清理后）
# 前置：本地已安装 rust 1.94+ 工具链
# ==============================================================================
set -euo pipefail

# 切换到项目根目录
cd "$(dirname "$0")/../.."

BASELINE_FILE="backend/.clippy-baseline.txt"

# 检查工具链
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误：cargo 未安装"
    exit 1
fi

if ! cargo clippy --version &> /dev/null; then
    echo "❌ 错误：clippy 未安装（rustup component add clippy）"
    exit 1
fi

# 备份现有基线
if [ -f "$BASELINE_FILE" ]; then
    BACKUP_FILE="${BASELINE_FILE}.bak-$(date +'%Y%m%d-%H%M%S')"
    cp "$BASELINE_FILE" "$BACKUP_FILE"
    echo "📦 旧基线已备份到 $BACKUP_FILE"
    echo ""
fi

# 生成新基线
echo "🔧 生成 clippy 警告基线..."
echo ""
cd backend

# 跑 clippy 并收集所有警告
cargo clippy --all-targets --locked --message-format=json 2>/dev/null | \
    jq -r 'select(.reason == "compiler-message") |
           .message |
           select(.level == "warning" or .level == "error") |
           .rendered' 2>/dev/null | \
    sort -u > "../$BASELINE_FILE"

cd ..

NEW_COUNT=$(wc -l < "$BASELINE_FILE")
BASELINE_SIZE=$(du -h "$BASELINE_FILE" | cut -f1)

echo "✅ 基线已生成：$BASELINE_FILE"
echo "   警告数: $NEW_COUNT"
echo "   文件大小: $BASELINE_SIZE"
echo ""

# 统计 TOP 10 文件
if [ "$NEW_COUNT" -gt "0" ]; then
    echo "📊 TOP 10 受影响文件："
    echo ""
    grep -oE 'src/[^:]+\.rs' "$BASELINE_FILE" 2>/dev/null | sort | uniq -c | sort -rn | head -10 | \
        awk '{printf "  %4d  %s\n", $1, $2}'
    echo ""
    echo "💡 建议清理策略："
    echo "  1. 优先修复高频文件（TOP 10）"
    echo "  2. 按文件提交（避免单 PR 修改过多）"
    echo "  3. 每修复一个文件，重新跑本脚本刷新基线"
    echo ""
fi

echo "💡 下一步："
echo "  1. 检查 git diff $BASELINE_FILE"
echo "  2. 提交 baseline 文件"
echo "  3. 后续 PR 严格化：cargo clippy --all-targets 失败 = 新警告"
