#!/bin/bash
# ==============================================================================
# scripts/ci/setup-test-baseline.sh
#
# 用途：建立或更新 Rust 单元测试基线
# 背景：CI 新架构采用 baseline 机制，新失败 0 容忍，历史失败渐进清理
# 用法：
#   bash scripts/ci/setup-test-baseline.sh            # 建立基线
#   bash scripts/ci/setup-test-baseline.sh --refresh   # 重建基线（用于清理后）
# 前置：本地已安装 rust 1.94+ 工具链
# ==============================================================================
set -euo pipefail

# 切换到项目根目录
cd "$(dirname "$0")/../.."

BASELINE_FILE="backend/.test-baseline.txt"

# 检查工具链
if ! command -v cargo &> /dev/null; then
    echo "❌ 错误：cargo 未安装"
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
echo "🔧 生成测试基线..."
echo ""
cd backend

# 跑测试并提取失败列表
cargo test --lib --jobs 1 -- --test-threads=1 2>/dev/null | \
    grep -E "^test .* FAILED$" | awk '{print $2}' | sort -u > "../$BASELINE_FILE" || true

cd ..

NEW_COUNT=$(wc -l < "$BASELINE_FILE" 2>/dev/null || echo 0)
BASELINE_SIZE=$(du -h "$BASELINE_FILE" 2>/dev/null | cut -f1 || echo "N/A")

echo "✅ 基线已生成：$BASELINE_FILE"
echo "   失败测试数: $NEW_COUNT"
echo "   文件大小: $BASELINE_SIZE"
echo ""

if [ "$NEW_COUNT" -gt "0" ]; then
    echo "📋 失败测试列表："
    echo ""
    head -30 "$BASELINE_FILE" | sed 's/^/  /'
    echo ""
fi

echo "💡 下一步："
echo "  1. 检查 git diff $BASELINE_FILE"
echo "  2. 提交 baseline 文件"
echo "  3. 后续 PR 严格化：新失败 = 失败"
