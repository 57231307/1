#!/bin/bash
# P2-2 后端 N+1 扫描脚本
# 用法：bash scripts/p2-2-n1-scan.sh
# 范围：backend/src/services/**/*.rs（约 449 个 API 函数）
# 输出：markdown 表格，列出 N+1 风险候选清单

set -e
cd "$(dirname "$0")/.."

echo "# P2-2 后端 N+1 风险扫描"
echo ""

echo "## 1. find_with_related / find_related 模式"
echo ""
echo "| 文件 | 行号 | 模式 |"
echo "|------|------|------|"
grep -rn "find_with_related\|find_related" src/services/ 2>/dev/null | \
  head -30 | \
  awk -F: '{ printf "| %s | %s | %s |\n", $1, $2, $3 }'

echo ""
echo "## 2. 循环中调用其他 Service 的模式（潜在 N+1）"
echo ""
echo "注：以下模式需人工 review 确认是否 N+1"
echo ""
grep -rn "for .* in .*{" src/services/ -A 3 2>/dev/null | \
  grep -B 1 "service::" | \
  head -30 | \
  awk -F: '{ printf "| %s | %s | 候选 |\n", $1, $2 }'

echo ""
echo "## 3. 总结"
TOTAL=$(grep -rn "find_with_related\|find_related" src/services/ 2>/dev/null | wc -l)
echo "- find_with_related 总数：$TOTAL"
