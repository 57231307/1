#!/bin/bash
# ==============================================================================
# scripts/ci/nextest-summary.sh
#
# 用途：从 nextest 的输出文件里提取**权威**计数，供 CI 报告页与门禁判定共用。
# 用法：read -r PASSED FAILED SKIPPED TOTAL HAS_SUMMARY < <(bash scripts/ci/nextest-summary.sh reports/nextest-output.txt)
# 输出：单行 5 个整数 `PASSED FAILED SKIPPED TOTAL HAS_SUMMARY`
#   HAS_SUMMARY=1 → 数字来自 nextest 自己的 `Summary … N tests run: A passed, B failed, C skipped`
#   HAS_SUMMARY=0 → 输出里没有 Summary 行（编译失败 / 进程中途崩溃 / 文件为空）。
#                   此时前 4 个数只是按状态行的粗估，**调用方不得据此判"全通过"**，
#                   必须按"未验证"处理并让门禁保持红（fail-closed）。
#
# 为什么需要这个脚本（CI #4669 实证）：分片报告用 `awk '/^ FAIL /{c++}'` 统计，
# 而 nextest 的状态行带 8 空格缩进（`        FAIL [ … ]`），该模式**永不匹配** →
# 215 例真实失败时报告页仍写"通过 0 / 失败 0 / 总计 0"，并同时落"进程异常退出"文案，
# 把真实失败数完全掩盖（判责时只能去翻 nextest-output.txt 的 Summary 行）。
# 另：重定向到文件时 nextest 通常自动关色码，但环境里存在 FORCE_COLOR 时不会，
# 故先剥 ANSI 色码再匹配，避免 `^ *Summary` 因前缀转义序列而失配。
# ==============================================================================
set -uo pipefail

OUT_FILE="${1:-}"
if [ -z "$OUT_FILE" ] || [ ! -f "$OUT_FILE" ]; then
  echo "0 0 0 0 0"
  exit 0
fi

CLEAN="$(sed -e 's/\x1b\[[0-9;]*[A-Za-z]//g' "$OUT_FILE")"

# Summary 行取最后一条（`--no-fail-fast` 下不会出现第二条，取尾是防御多段输出）
SUMMARY_LINE="$(printf '%s\n' "$CLEAN" | grep -E '^[[:space:]]*Summary' | tail -1)"

# 逗号后紧跟数字（`105 passed,`）会把字段粘上逗号，统一换成空格再按词取前一个数
field_before() {
  printf '%s' "$1" | tr ',' ' ' | awk -v k="$2" '{for (i = 1; i <= NF; i++) if ($i == k) {print $(i - 1) + 0; exit}}'
}

if [ -n "$SUMMARY_LINE" ]; then
  PASSED="$(field_before "$SUMMARY_LINE" passed)"
  FAILED="$(field_before "$SUMMARY_LINE" failed)"
  SKIPPED="$(field_before "$SUMMARY_LINE" skipped)"
  TOTAL="$(field_before "$SUMMARY_LINE" tests)"
  echo "${PASSED:-0} ${FAILED:-0} ${SKIPPED:-0} ${TOTAL:-0} 1"
  exit 0
fi

# 无 Summary：按状态行粗估（`FAIL` / `EXECUTION FAIL`），并把 HAS_SUMMARY 置 0
EST_FAIL="$(printf '%s\n' "$CLEAN" | grep -cE '^[[:space:]]+(EXECUTION )?FAIL' || true)"
EST_PASS="$(printf '%s\n' "$CLEAN" | grep -cE '^[[:space:]]+PASS' || true)"
echo "${EST_PASS:-0} ${EST_FAIL:-0} 0 $(( ${EST_PASS:-0} + ${EST_FAIL:-0} )) 0"
exit 0
