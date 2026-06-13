#!/usr/bin/env bash
# ==============================================================================
# 清理冗余的文件级 #![allow(dead_code)] 抑制（v2：动态行号定位）
# ==============================================================================
set -uo pipefail

cd /workspace

echo "=== Step 1: 列出待清理文件（排除 models/） ==="
FILES=$(grep -rln '#!\[allow(dead_code)\]' backend/src 2>/dev/null | grep -v 'backend/src/models/' || true)
COUNT=$(echo "$FILES" | grep -c .)
echo "待处理文件数: $COUNT"

echo ""
echo "=== Step 2: 动态定位后批量删除两行 ==="
# 模式：
#   找到 ^#![allow(dead_code)]$ 所在行 N
#   验证 N+1 行是 ^// TODO(tech-debt): 业务接入或重评估后逐项移除.*$
#   删除 N 与 N+1 两行
PROCESSED=0
SKIPPED=0
SKIP_FILES=()
while IFS= read -r f; do
  [ -z "$f" ] && continue
  # 找 #![allow(dead_code)] 所在行号
  allow_line=$(grep -n '^#!\[allow(dead_code)\]$' "$f" | head -1 | cut -d: -f1)
  if [ -z "$allow_line" ]; then
    SKIP_FILES+=("$f (无匹配行)")
    SKIPPED=$((SKIPPED+1))
    continue
  fi
  # 验证下一行是 TODO 注释
  next_line=$((allow_line + 1))
  todo_content=$(sed -n "${next_line}p" "$f")
  if echo "$todo_content" | grep -q '^// TODO(tech-debt): 业务接入或重评估后逐项移除'; then
    # 用 perl 删除指定两行（更安全）
    perl -i -ne "print unless \$. == $allow_line || \$. == $next_line" "$f"
    PROCESSED=$((PROCESSED+1))
  else
    SKIP_FILES+=("$f (第 $next_line 行不是 TODO 注释: $todo_content)")
    SKIPPED=$((SKIPPED+1))
  fi
done <<< "$FILES"

echo "已处理: $PROCESSED 文件"
echo "跳过:   $SKIPPED 文件"
if [ $SKIPPED -gt 0 ]; then
  echo "--- 跳过文件详情 ---"
  for s in "${SKIP_FILES[@]}"; do
    echo "  $s"
  done
fi

echo ""
echo "=== Step 3: 残留校验 ==="
REMAINING_ALLOW=$(grep -rln '#!\[allow(dead_code)\]' backend/src 2>/dev/null | grep -v 'backend/src/models/' | wc -l)
REMAINING_TODO=$(grep -rln 'TODO(tech-debt): 业务接入或重评估后逐项移除' backend/src 2>/dev/null | grep -v 'backend/src/models/' | wc -l)
MODELS_ALLOW=$(grep -rln '#!\[allow(dead_code)\]' backend/src/models 2>/dev/null | wc -l)
echo "非 models/ 残留 #![allow(dead_code)] 文件: $REMAINING_ALLOW (应为 0)"
echo "非 models/ 残留 TODO(tech-debt) 注释文件: $REMAINING_TODO (应为 0)"
echo "models/ 保留 #![allow(dead_code)] 文件: $MODELS_ALLOW (SeaORM 例外)"

if [ "$REMAINING_ALLOW" -eq 0 ] && [ "$REMAINING_TODO" -eq 0 ]; then
  echo ""
  echo "清理成功！"
  echo ""
  echo "=== Step 4: 修改统计 ==="
  git diff --stat | tail -10
else
  echo ""
  echo "残留校验未通过"
  exit 1
fi
