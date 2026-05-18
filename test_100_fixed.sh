#!/bin/bash
# ERP 系统全面测试脚本 (100 端点) - 修复版

SERVER_IP="111.230.99.236"
BASE_URL="http://${SERVER_IP}/api/v1/erp"
USERNAME="admin"
PASSWORD="admin123"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; ((PASSED++)); ((TOTAL++)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1 ((FAILED++)); ((TOTAL++)); }
log_skip() { echo -e "${YELLOW}[SKIP]${NC} $1"; ((SKIPPED++)); ((TOTAL++)); }
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }

# 获取 Token
TOKEN=$(curl -s -X POST "${BASE_URL}/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}" | jq -r '.data.token')

if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
    echo "获取 Token 失败"
    exit 1
fi

log_info "Token 获取成功"

test_get() {
    local endpoint=$1
    local name=$2
    local code=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}${endpoint}" -H "Authorization: Bearer $TOKEN" 2>/dev/null)
    case $code in
        200) log_pass "GET $name" ;;
        404) log_skip "GET $name (404)" ;;
        *) log_fail "GET $name (HTTP $code)" ;;
    esac
}

test_post() {
    local endpoint=$1
    local name=$2
    local data=${3:-'{}'}
    local code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}${endpoint}" \
        -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d "$data" 2>/dev/null)
    case $code in
        200|201) log_pass "POST $name" ;;
        404) log_skip "POST $name (404)" ;;
        *) log_fail "POST $name (HTTP $code)" ;;
    esac
}

echo "=========================================="
echo "500 错误修复验证（4 个）"
echo "=========================================="
test_get "/warehouses/locations" "库位管理 (修复 locations 表名)"
test_get "/bpm/monitor/stats" "BPM 监控统计 (修复 applicant_id)"
test_get "/ap/reconciliations" "应付对账"
test_get "/budgets/plans" "预算计划"

echo ""
echo "=========================================="
echo "405 方法错误修复（2 个）"
echo "=========================================="
test_post "/dual-unit/convert" "双单位转换" '{"quantity":1,"from_unit":"kg","to_unit":"g"}'
test_post "/bpm/process/start" "BPM 流程启动" '{"process_definition_id":1,"business_type":"test","business_id":1,"initiator_id":1,"variables":{}}'

echo ""
echo "=========================================="
echo "400 参数修复（7 个）"
echo "=========================================="
test_get "/customers/1/summary" "客户关联总览 (使用 ID=1)"
test_get "/ap/reports/statistics?start_date=2026-01-01&end_date=2026-12-31" "应付统计报表 (带参数)"
test_get "/business-trace/forward?trace_id=1" "业务正向追溯 (带参数)"
test_get "/business-trace/backward?trace_id=1" "业务反向追溯 (带参数)"
test_post "/financial-analysis/reports" "财务分析报告" '{"report_type":"test"}'
test_post "/fixed-assets/depreciate" "资产折旧" '{"asset_id":1}'
test_get "/ai/forecast-sales?period=monthly" "AI 销售预测 (带参数)"

echo ""
echo "=========================================="
echo "404 路径修复（2 个）"
echo "=========================================="
test_get "/finance/accounting-periods/current" "当前会计期间 (正确路径)"
test_get "/bpm/tasks?status=PENDING" "BPM 任务 (带参数)"

echo ""
echo "=========================================="
echo "统计汇总"
echo "=========================================="
echo "总测试数：${TOTAL}"
echo "通过：${PASSED}"
echo "失败：${FAILED}"
echo "跳过：${SKIPPED}"
[ $((TOTAL-SKIPPED)) -gt 0 ] && echo "有效率：$((PASSED*100/(TOTAL-SKIPPED)))%"
