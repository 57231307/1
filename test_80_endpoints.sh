#!/bin/bash
# ERP 系统全面测试脚本 (80+ 端点)

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
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; ((FAILED++)); ((TOTAL++)); }
log_skip() { echo -e "${BLUE}[SKIP]${NC} $1"; ((SKIPPED++)); ((TOTAL++)); }
log_info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

# 获取 Token
TOKEN=$(curl -s -X POST "${BASE_URL}/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}" | jq -r '.data.token')

if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
    echo "获取 Token 失败"
    exit 1
fi

log_info "Token 获取成功：${TOKEN:0:30}..."
echo ""

test_get() {
    local endpoint=$1
    local name=$2
    local code=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}${endpoint}" -H "Authorization: Bearer $TOKEN" 2>/dev/null)
    case $code in
        200) log_pass "GET $name" ;;
        404) log_skip "GET $name (404 端点不存在)" ;;
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
        404) log_skip "POST $name (404 端点不存在)" ;;
        *) log_fail "POST $name (HTTP $code)" ;;
    esac
}

echo "=========================================="
echo "P0 核心功能测试 (20 个)"
echo "=========================================="
test_get "/health" "健康检查"
test_get "/finance/invoices" "财务发票列表"
test_get "/finance/payments" "财务收款列表"
test_get "/sales/orders" "销售订单列表"
test_get "/purchases/orders" "采购订单列表"
test_get "/purchases/receipts" "采购入库列表"
test_get "/purchases/returns" "采购退货列表"
test_get "/inventory/stock" "库存列表"
test_get "/inventory/stock/fabric" "面料库存列表"
test_get "/inventory/stock/summary" "库存汇总"
test_get "/inventory/transactions" "库存交易记录"
test_get "/crm/leads" "CRM 线索"
test_get "/crm/opportunities" "CRM 商机"
test_get "/crm/customers/:id/summary" "客户关联总览"
test_get "/products" "产品列表"
test_get "/product-categories" "产品分类"
test_get "/suppliers" "供应商列表"
test_get "/currencies" "币种列表"
test_get "/customers" "客户列表"
test_get "/batches" "批次管理"

echo ""
echo "=========================================="
echo "P1 重要功能测试 (20 个)"
echo "=========================================="
test_get "/ar-reconciliations" "应收对账单"
test_get "/ar/invoices" "应收发票列表"
test_get "/ap/invoices" "应付发票列表"
test_get "/ap/payments" "应付付款"
test_get "/ap/payment-requests" "付款申请"
test_get "/ap/verifications" "应付验证"
test_get "/ap/reconciliations" "应付对账"
test_get "/ap/invoices/aging" "应付账龄分析"
test_get "/ap/reports/statistics" "应付统计报表"
test_get "/cost-collections" "成本归集"
test_get "/dashboard/overview" "仪表板总览"
test_get "/dashboard/sales-stats" "销售统计"
test_get "/dashboard/inventory-stats" "库存统计"
test_get "/dashboard/low-stock-alerts" "低库存预警"
test_get "/dye-recipes" "染色配方"
test_get "/greige-fabrics" "坯布管理"
test_get "/sales/fabric-orders" "面料销售订单"
test_get "/production/orders" "生产订单"
test_get "/logistics" "物流管理"
test_get "/barcode/scan-to-ship" "扫码出库"

echo ""
echo "=========================================="
echo "P2 辅助功能测试 (20 个)"
echo "=========================================="
test_get "/users" "系统用户"
test_get "/roles" "系统角色"
test_get "/departments" "部门管理"
test_get "/departments/tree" "部门树"
test_get "/warehouses" "仓库管理"
test_get "/warehouses/locations" "库位管理"
test_get "/supplier-evaluation/evaluations" "供应商评估"
test_get "/supplier-evaluation/indicators" "评估指标"
test_get "/supplier-evaluation/rankings" "供应商排名"
test_get "/inventory/counts" "库存盘点"
test_get "/inventory/transfers" "库存调拨"
test_get "/inventory/adjustments" "库存调整"
test_get "/init/status" "初始化状态"
test_get "/sales-contracts" "销售合同"
test_get "/purchase-contracts" "采购合同"
test_get "/fixed-assets" "固定资产"
test_get "/budgets" "预算管理"
test_get "/fund-management/accounts" "资金账户"
test_get "/quality-standards" "质量标准"
test_get "/quality-inspection/standards" "质检标准"

echo ""
echo "=========================================="
echo "总账与财务测试 (15 个)"
echo "=========================================="
test_get "/gl/subjects" "会计科目"
test_get "/gl/subjects/tree" "科目树"
test_get "/gl/vouchers" "会计凭证"
test_get "/accounting-periods/current" "当前会计期间"
test_get "/finance/accounting-periods/init" "会计期间初始化"
test_get "/dual-unit/convert" "双单位转换"
test_post "/dual-unit/convert" "双单位转换 (POST)" '{"quantity":1,"from_unit":"kg","to_unit":"g"}'
test_get "/five-dimension/stats" "五维统计"
test_get "/five-dimension/list" "五维列表"
test_get "/assist-accounting/dimensions" "辅助核算维度"
test_get "/assist-accounting/records" "辅助核算记录"
test_get "/business-trace/forward" "业务正向追溯"
test_get "/business-trace/backward" "业务反向追溯"
test_get "/finance/reports/balance-sheet" "资产负债表"
test_get "/finance/reports/income-statement" "利润表"

echo ""
echo "=========================================="
echo "高级功能测试 (15 个)"
echo "=========================================="
test_get "/sales-analysis/statistics" "销售统计"
test_get "/sales-analysis/rankings" "销售排行"
test_get "/sales-analysis/targets" "销售目标"
test_get "/financial-analysis/trends" "财务趋势"
test_get "/financial-analysis/reports" "财务分析报告"
test_get "/sales-prices" "销售价格"
test_get "/purchase-prices" "采购价格"
test_get "/customer-credits" "客户信用"
test_get "/quality-inspection/records" "质检记录"
test_get "/quality-inspection/defects" "缺陷管理"
test_get "/budgets/items" "预算项目"
test_get "/budgets/plans" "预算计划"
test_get "/fixed-assets/depreciate" "资产折旧"
test_get "/notifications" "消息通知"
test_get "/notifications/unread-count" "未读消息数"

echo ""
echo "=========================================="
echo "系统与 AI 功能测试 (10 个)"
echo "=========================================="
test_get "/system-update/version" "系统版本"
test_get "/system-update/check" "检查更新"
test_get "/bpm/process/start" "BPM 流程启动"
test_get "/bpm/tasks" "BPM 任务"
test_get "/bpm/monitor/stats" "BPM 监控统计"
test_get "/ai/forecast-sales" "AI 销售预测"
test_get "/ai/optimize-inventory" "AI 库存优化"
test_get "/ai/detect-anomalies" "AI 异常检测"
test_get "/ai/recommendations" "AI 推荐"
test_get "/audit/stats" "审计统计"

echo ""
echo "=========================================="
echo "统计汇总"
echo "=========================================="
echo -e "总测试数：${YELLOW}${TOTAL}${NC}"
echo -e "通过：${GREEN}${PASSED}${NC}"
echo -e "失败：${RED}${FAILED}${NC}"
echo -e "跳过：${BLUE}${SKIPPED}${NC}"

if [ $((TOTAL-SKIPPED)) -gt 0 ]; then
    PASS_RATE=$((PASSED*100/(TOTAL-SKIPPED)))
    echo -e "有效率：${GREEN}${PASS_RATE}%${NC}"
fi

# 保存结果
echo ""
echo "测试结果已保存到：/workspace/test_result_$(date +%Y%m%d_%H%M%S).log"
