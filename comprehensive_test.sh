#!/bin/bash
# ERP 系统全面功能测试脚本

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

# 检查健康状态
test_health() {
    log_info "测试健康检查..."
    RESPONSE=$(curl -s "${BASE_URL}/health")
    if echo "$RESPONSE" | jq -e '.status' > /dev/null; then
        log_pass "GET /health"
    else
        log_fail "GET /health - 响应格式错误：$RESPONSE"
    fi
}

# 登录获取 Token
get_token() {
    log_info "获取认证 Token..."
    TOKEN_RESPONSE=$(curl -s -X POST "${BASE_URL}/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"${USERNAME}\",\"password\":\"${PASSWORD}\"}")
    
    TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.data.token // empty')
    
    if [ -z "$TOKEN" ]; then
        log_fail "登录失败：$TOKEN_RESPONSE"
        exit 1
    fi
    
    log_pass "POST /auth/login"
    export AUTH_TOKEN="$TOKEN"
}

# 测试用户相关
test_user_endpoints() {
    log_info "测试用户管理..."
    
    # 获取当前用户信息
    RESPONSE=$(curl -s "${BASE_URL}/users/me" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /users/me"
    else
        log_fail "GET /users/me - $RESPONSE"
    fi
}

# 测试财务发票
test_finance_invoices() {
    log_info "测试财务发票..."
    
    # 获取发票列表
    RESPONSE=$(curl -s "${BASE_URL}/finance/invoices" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /finance/invoices"
    else
        log_fail "GET /finance/invoices - $RESPONSE"
    fi
}

# 测试财务收款
test_finance_payments() {
    log_info "测试财务收款..."
    
    # 获取收款列表
    RESPONSE=$(curl -s "${BASE_URL}/finance/payments" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /finance/payments"
    else
        log_fail "GET /finance/payments - $RESPONSE"
    fi
}

# 测试财务报表
test_finance_reports() {
    log_info "测试财务报表..."
    
    # 资产负债表
    RESPONSE=$(curl -s "${BASE_URL}/finance/reports/balance-sheet" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /finance/reports/balance-sheet"
    else
        log_fail "GET /finance/reports/balance-sheet - $RESPONSE"
    fi
    
    # 利润表
    RESPONSE=$(curl -s "${BASE_URL}/finance/reports/income-statement" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /finance/reports/income-statement"
    else
        log_fail "GET /finance/reports/income-statement - $RESPONSE"
    fi
}

# 测试销售订单
test_sales_orders() {
    log_info "测试销售订单..."
    
    # 获取销售订单列表
    RESPONSE=$(curl -s "${BASE_URL}/sales/orders" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /sales/orders"
    else
        log_fail "GET /sales/orders - $RESPONSE"
    fi
}

# 测试采购订单
test_purchase_orders() {
    log_info "测试采购管理..."
    
    # 获取采购订单列表
    RESPONSE=$(curl -s "${BASE_URL}/purchase/orders" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /purchase/orders"
    else
        log_fail "GET /purchase/orders - $RESPONSE"
    fi
    
    # 获取入库单列表
    RESPONSE=$(curl -s "${BASE_URL}/purchase/receipts" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /purchase/receipts"
    else
        log_fail "GET /purchase/receipts - $RESPONSE"
    fi
    
    # 获取质检单列表
    RESPONSE=$(curl -s "${BASE_URL}/purchases/inspections" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /purchases/inspections"
    else
        log_fail "GET /purchases/inspections - $RESPONSE"
    fi
    
    # 获取退货单列表
    RESPONSE=$(curl -s "${BASE_URL}/purchases/returns" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /purchases/returns"
    else
        log_fail "GET /purchases/returns - $RESPONSE"
    fi
}

# 测试库存管理
test_inventory() {
    log_info "测试库存管理..."
    
    # 获取库存列表
    RESPONSE=$(curl -s "${BASE_URL}/inventory/stock" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /inventory/stock"
    else
        log_fail "GET /inventory/stock - $RESPONSE"
    fi
    
    # 获取仓库列表
    RESPONSE=$(curl -s "${BASE_URL}/inventory/warehouses" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /inventory/warehouses"
    else
        log_fail "GET /inventory/warehouses - $RESPONSE"
    fi
}

# 测试 CRM
test_crm() {
    log_info "测试 CRM..."
    
    # 获取客户列表
    RESPONSE=$(curl -s "${BASE_URL}/customers" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /customers"
    else
        log_fail "GET /customers - $RESPONSE"
    fi
    
    # 获取线索列表
    RESPONSE=$(curl -s "${BASE_URL}/crm/leads" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /crm/leads"
    else
        log_fail "GET /crm/leads - $RESPONSE"
    fi
    
    # 获取商机列表
    RESPONSE=$(curl -s "${BASE_URL}/crm/opportunities" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /crm/opportunities"
    else
        log_fail "GET /crm/opportunities - $RESPONSE"
    fi
}

# 测试产品管理
test_products() {
    log_info "测试产品管理..."
    
    # 获取产品列表
    RESPONSE=$(curl -s "${BASE_URL}/products" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /products"
    else
        log_fail "GET /products - $RESPONSE"
    fi
    
    # 获取产品分类
    RESPONSE=$(curl -s "${BASE_URL}/product-categories" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /product-categories"
    else
        log_fail "GET /product-categories - $RESPONSE"
    fi
}

# 测试供应商
test_suppliers() {
    log_info "测试供应商管理..."
    
    # 获取供应商列表
    RESPONSE=$(curl -s "${BASE_URL}/suppliers" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /suppliers"
    else
        log_fail "GET /suppliers - $RESPONSE"
    fi
    
    # 获取供应商评估
    RESPONSE=$(curl -s "${BASE_URL}/supplier-evaluation/evaluations" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /supplier-evaluation/evaluations"
    else
        log_fail "GET /supplier-evaluation/evaluations - $RESPONSE"
    fi
}

# 测试应收管理
test_ar() {
    log_info "测试应收管理..."
    
    # 获取对账单列表
    RESPONSE=$(curl -s "${BASE_URL}/ar-reconciliations" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /ar-reconciliations"
    else
        log_fail "GET /ar-reconciliations - $RESPONSE"
    fi
    
    # 获取账龄分析
    RESPONSE=$(curl -s "${BASE_URL}/ap/invoices/aging" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /ap/invoices/aging"
    else
        log_fail "GET /ap/invoices/aging - $RESPONSE"
    fi
}

# 测试币种汇率
test_currency() {
    log_info "测试币种汇率..."
    
    # 获取币种列表
    RESPONSE=$(curl -s "${BASE_URL}/currencies" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /currencies"
    else
        log_fail "GET /currencies - $RESPONSE"
    fi
    
    # 获取汇率列表
    RESPONSE=$(curl -s "${BASE_URL}/exchange-rates/query" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /exchange-rates/query"
    else
        log_fail "GET /exchange-rates/query - $RESPONSE"
    fi
}

# 测试系统管理
test_system() {
    log_info "测试系统管理..."
    
    # 获取用户列表
    RESPONSE=$(curl -s "${BASE_URL}/users" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /users"
    else
        log_fail "GET /users - $RESPONSE"
    fi
    
    # 获取角色列表
    RESPONSE=$(curl -s "${BASE_URL}/roles" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /roles"
    else
        log_fail "GET /roles - $RESPONSE"
    fi
    
    # 获取会计期间
    RESPONSE=$(curl -s "${BASE_URL}/accounting-periods/current" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /accounting-periods/current"
    else
        log_fail "GET /accounting-periods/current - $RESPONSE"
    fi
}

# 测试仪表板
test_dashboard() {
    log_info "测试仪表板..."
    
    # 获取总览
    RESPONSE=$(curl -s "${BASE_URL}/dashboard/overview" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /dashboard/overview"
    else
        log_fail "GET /dashboard/overview - $RESPONSE"
    fi
}

# 测试成本归集
test_cost_collections() {
    log_info "测试成本管理..."
    
    # 获取成本归集列表
    RESPONSE=$(curl -s "${BASE_URL}/cost-collections" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /cost-collections"
    else
        log_fail "GET /cost-collections - $RESPONSE"
    fi
}

# 测试高级功能
test_advanced() {
    log_info "测试高级功能..."
    
    # 获取批次管理
    RESPONSE=$(curl -s "${BASE_URL}/batches" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /batches"
    else
        log_fail "GET /batches - $RESPONSE"
    fi
    
    # 获取染色配方
    RESPONSE=$(curl -s "${BASE_URL}/dye-recipes" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /dye-recipes"
    else
        log_fail "GET /dye-recipes - $RESPONSE"
    fi
    
    # 获取坯布管理
    RESPONSE=$(curl -s "${BASE_URL}/greige-fabrics" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /greige-fabrics"
    else
        log_fail "GET /greige-fabrics - $RESPONSE"
    fi
}

# 测试销售分析
test_sales_analysis() {
    log_info "测试销售分析..."
    
    # 获取销售趋势
    RESPONSE=$(curl -s "${BASE_URL}/sales-analysis/trends" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    # 允许 400 错误（无数据）
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/sales-analysis/trends" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "400" ]; then
        log_pass "GET /sales-analysis/trends (HTTP $HTTP_CODE)"
    else
        log_fail "GET /sales-analysis/trends - HTTP $HTTP_CODE"
    fi
}

# 测试初始化状态
test_init_status() {
    log_info "测试初始化状态..."
    
    RESPONSE=$(curl -s "${BASE_URL}/init/status" \
        -H "Authorization: Bearer ${AUTH_TOKEN}")
    if echo "$RESPONSE" | jq -e '.data' > /dev/null; then
        log_pass "GET /init/status"
    else
        log_fail "GET /init/status - $RESPONSE"
    fi
}

# 打印测试报告
print_report() {
    echo ""
    echo "=========================================="
    echo "           测试报告汇总"
    echo "=========================================="
    echo "总测试数：$TOTAL"
    echo -e "${GREEN}通过：$PASSED${NC}"
    echo -e "${RED}失败：$FAILED${NC}"
    
    if [ $TOTAL -gt 0 ]; then
        PASS_RATE=$((PASSED * 100 / TOTAL))
        echo "通过率：${PASS_RATE}%"
    fi
    
    echo "=========================================="
}

# 主函数
main() {
    echo "=========================================="
    echo "   ERP 系统全面功能测试"
    echo "   服务器：${SERVER_IP}"
    echo "   时间：$(date '+%Y-%m-%d %H:%M:%S')"
    echo "=========================================="
    echo ""
    
    # 测试健康
    test_health
    
    # 获取 Token
    get_token
    
    # 执行所有测试
    test_user_endpoints
    test_finance_invoices
    test_finance_payments
    test_finance_reports
    test_sales_orders
    test_purchase_orders
    test_inventory
    test_crm
    test_products
    test_suppliers
    test_ar
    test_currency
    test_system
    test_dashboard
    test_cost_collections
    test_advanced
    test_sales_analysis
    test_init_status
    
    # 打印报告
    print_report
    
    # 返回退出码
    if [ $FAILED -gt 0 ]; then
        exit 1
    else
        exit 0
    fi
}

main
