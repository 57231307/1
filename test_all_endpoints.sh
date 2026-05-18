#!/bin/bash
# ERP 系统全部端点测试脚本（40+ 端点）

set -e

SERVER_IP="111.230.99.236"
BASE_URL="http://${SERVER_IP}/api/v1/erp"
USERNAME="admin"
PASSWORD="admin123"

# 颜色输出
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 计数器
TOTAL=0
PASSED=0
FAILED=0

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASSED++))
    ((TOTAL++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAILED++))
    ((TOTAL++))
}

log_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

# 获取 Token
get_token() {
    RESPONSE=$(curl -s -X POST "${BASE_URL}/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}")
    
    TOKEN=$(echo "$RESPONSE" | jq -r '.data.token')
    if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
        echo "获取 Token 失败：$RESPONSE"
        exit 1
    fi
    echo "$TOKEN"
}

# 测试端点
test_endpoint() {
    local endpoint=$1
    local name=$2
    local method=${3:-GET}
    local data=${4:-}
    
    if [ "$method" = "GET" ]; then
        RESPONSE=$(curl -s "${BASE_URL}${endpoint}" -H "Authorization: Bearer $TOKEN")
    else
        RESPONSE=$(curl -s -X "$method" "${BASE_URL}${endpoint}" \
            -H "Authorization: Bearer $TOKEN" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi
    
    # 检查响应是否为有效 JSON 且不是空字符串
    if [ -n "$RESPONSE" ] && echo "$RESPONSE" | jq . > /dev/null 2>&1; then
        log_pass "$method $name"
    else
        log_fail "$method $name - 响应无效或为空"
    fi
}

echo "=========================================="
echo "   ERP 系统全面功能测试（40+ 端点）"
echo "   服务器：$SERVER_IP"
echo "   时间：$(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

# 获取 Token
log_info "获取认证 Token..."
TOKEN=$(get_token)
log_info "Token 获取成功"
echo ""

# P0 核心功能测试
echo "=========================================="
echo "P0 核心功能测试 (15 个)"
echo "=========================================="
test_endpoint "/health" "健康检查"
test_endpoint "/finance/invoices" "财务发票列表"
test_endpoint "/finance/payments" "财务收款列表"
test_endpoint "/sales/orders" "销售订单列表"
test_endpoint "/purchases/orders" "采购订单列表"
test_endpoint "/inventory/stock" "库存列表"
test_endpoint "/crm/leads" "CRM 线索列表"
test_endpoint "/crm/opportunities" "CRM 商机列表"
test_endpoint "/products" "产品列表"
test_endpoint "/suppliers" "供应商列表"
test_endpoint "/currencies" "币种列表"
test_endpoint "/customers" "客户列表"
test_endpoint "/batches" "批次管理列表"
test_endpoint "/dye-recipes" "染色配方列表"
test_endpoint "/greige-fabrics" "坯布管理列表"

echo ""
# P1 重要功能测试
echo "=========================================="
echo "P1 重要功能测试 (12 个)"
echo "=========================================="
test_endpoint "/ar-reconciliations" "应收对账单列表"
test_endpoint "/ap/invoices" "应付发票列表"
test_endpoint "/ap/payments" "应付付款列表"
test_endpoint "/ap/payment-requests" "付款申请列表"
test_endpoint "/ap/verifications" "应付验证列表"
test_endpoint "/ap/reconciliations" "应付对账列表"
test_endpoint "/ap/invoices/aging" "应付账龄分析"
test_endpoint "/cost-collections" "成本归集列表"
test_endpoint "/product-categories" "产品分类列表"
test_endpoint "/purchase-receipts" "采购入库列表"
test_endpoint "/purchase-returns" "采购退货列表"
test_endpoint "/dashboard/overview" "仪表板总览"

echo ""
# P2 辅助功能测试
echo "=========================================="
echo "P2 辅助功能测试 (10 个)"
echo "=========================================="
test_endpoint "/users" "系统用户列表"
test_endpoint "/roles" "系统角色列表"
test_endpoint "/departments" "部门列表"
test_endpoint "/warehouses" "仓库列表"
test_endpoint "/supplier-evaluation/evaluations" "供应商评估列表"
test_endpoint "/inventory/counts" "库存盘点列表"
test_endpoint "/inventory/transfers" "库存调拨列表"
test_endpoint "/inventory/adjustments" "库存调整列表"
test_endpoint "/sales/fabric-orders" "面料销售订单列表"
test_endpoint "/init/status" "系统初始化状态"

echo ""
# 高级功能测试
echo "=========================================="
echo "高级功能测试 (8 个)"
echo "=========================================="
test_endpoint "/sales-analysis/statistics" "销售统计分析"
test_endpoint "/sales-analysis/rankings" "销售排行"
test_endpoint "/financial-analysis/trends" "财务分析趋势"
test_endpoint "/fund-management/accounts" "资金管理账户"
test_endpoint "/quality-standards" "质量标准列表"
test_endpoint "/quality-inspection/standards" "质量检验标准"
test_endpoint "/budgets" "预算管理列表"
test_endpoint "/fixed-assets" "固定资产列表"

echo ""
# 总账与财务测试
echo "=========================================="
echo "总账与财务测试 (5 个)"
echo "=========================================="
test_endpoint "/gl/subjects" "会计科目列表"
test_endpoint "/gl/vouchers" "会计凭证列表"
test_endpoint "/accounting-periods/current" "当前会计期间"
test_endpoint "/dual-unit/convert" "双单位转换" POST '{"quantity":1,"from_unit":"kg","to_unit":"g"}'
test_endpoint "/five-dimension/stats" "五维统计"

echo ""
# 打印统计
echo "=========================================="
echo "测试统计"
echo "=========================================="
echo -e "总测试数：${YELLOW}${TOTAL}${NC}"
echo -e "通过：${GREEN}${PASSED}${NC}"
echo -e "失败：${RED}${FAILED}${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}所有测试通过！通过率：100%${NC}"
else
    PASS_RATE=$((PASSED * 100 / TOTAL))
    echo -e "通过率：${YELLOW}${PASS_RATE}%${NC}"
fi

echo ""
echo "=========================================="
