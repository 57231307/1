#!/bin/bash
# ERP 系统最终修复测试脚本

SERVER_IP="111.230.99.236"
BASE_URL="http://${SERVER_IP}/api/v1/erp"
USERNAME="admin"
PASSWORD="admin123"

TOKEN=$(curl -s -X POST "${BASE_URL}/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}" | jq -r '.data.token')

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0
TOTAL=0

test_get() {
    local endpoint=$1
    local name=$2
    local code=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}${endpoint}" -H "Authorization: Bearer $TOKEN")
    ((TOTAL++))
    if [ "$code" = "200" ]; then
        echo -e "${GREEN}[PASS]${NC} $name"
        ((PASS++))
    else
        echo -e "${RED}[FAIL ${code}]${NC} $name"
        ((FAIL++))
    fi
}

test_post() {
    local endpoint=$1
    local name=$2
    local data=${3:-'{}'}
    local code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${BASE_URL}${endpoint}" \
        -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d "$data")
    ((TOTAL++))
    if [ "$code" = "200" ] || [ "$code" = "201" ]; then
        echo -e "${GREEN}[PASS]${NC} $name"
        ((PASS++))
    else
        echo -e "${RED}[FAIL ${code}]${NC} $name"
        ((FAIL++))
    fi
}

echo "=========================================="
echo "修复验证测试（15 个失败端点）"
echo "=========================================="
echo ""

echo "=== 500 错误修复（4 个） ==="
test_get "/warehouses/locations" "库位管理（表名修复）"
test_get "/bpm/monitor/stats" "BPM 监控（列名修复）"
test_get "/budgets/plans" "预算计划（模型修复）"
test_get "/ap/reconciliations" "应付对账（字段修复）"

echo ""
echo "=== 405 方法修复（2 个） ==="
test_post "/dual-unit/convert" "双单位转换" '{"quantity":1,"from_unit":"kg","to_unit":"g"}'
test_post "/scanner/scan-to-ship" "扫码出库" '{"order_id":1,"barcode":"test"}'

echo ""
echo "=== 400 参数修复（7 个） ==="
test_get "/customers/1/summary" "客户关联总览"
test_get "/ap/reports/statistics?start_date=2026-01-01&end_date=2026-12-31" "应付统计报表"
test_get "/business-trace/forward?trace_id=1" "业务正向追溯"
test_get "/business-trace/backward?trace_id=1" "业务反向追溯"
test_post "/financial-analysis/reports" "财务分析报告" '{"report_type":"test"}'
test_post "/fixed-assets/depreciate" "资产折旧" '{"asset_id":1}'
test_get "/ai/forecast-sales?period=monthly" "AI 销售预测"

echo ""
echo "=== 404 路径修复（2 个） ==="
test_get "/inventory/stock/transactions" "库存交易（正确路径）"
test_get "/supplier-evaluation/evaluations/indicators" "评估指标（正确路径）"

echo ""
echo "=========================================="
echo "统计汇总"
echo "=========================================="
echo "总测试：${TOTAL}"
echo -e "通过：${GREEN}${PASS}${NC}"
echo -e "失败：${RED}${FAIL}${NC}"
[ $TOTAL -gt 0 ] && echo "通过率：$((PASS*100/TOTAL))%"
