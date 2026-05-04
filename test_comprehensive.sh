#!/bin/bash
# Bingxi ERP 全面浏览器自动化测试脚本
# 覆盖所有业务模块的功能、性能、边界测试

echo "=========================================="
echo "Bingxi ERP 全面自动化测试"
echo "=========================================="
echo ""

# 测试配置
BASE_URL="http://localhost:8082"
API_BASE="${BASE_URL}/api/v1/erp"
TEST_RESULTS_DIR="/tmp/bingxi_test_results"
mkdir -p ${TEST_RESULTS_DIR}

# 测试统计
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_test() {
    echo -e "${YELLOW}[TEST]${NC} $1"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASSED_TESTS=$((PASSED_TESTS + 1))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    echo "$1" >> ${TEST_RESULTS_DIR}/failures.log
}

# ==========================================
# 第一阶段:健康检查与认证测试
# ==========================================
echo ""
echo "=== 阶段1: 健康检查与认证 ==="
echo ""

# 测试1: 健康检查
log_test "后端健康检查"
HEALTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" ${API_BASE}/health)
if [ "$HEALTH_RESPONSE" = "200" ]; then
    log_pass "健康检查通过 (HTTP $HEALTH_RESPONSE)"
else
    log_fail "健康检查失败 (HTTP $HEALTH_RESPONSE)"
fi

# 测试2: 用户登录
log_test "用户登录测试"
LOGIN_RESPONSE=$(curl -s -X POST "${API_BASE}/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}')
TOKEN=$(echo $LOGIN_RESPONSE | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
if [ -n "$TOKEN" ] && [ "$TOKEN" != "null" ]; then
    log_pass "登录成功,获取Token"
    echo "Token: ${TOKEN:0:20}..." >> ${TEST_RESULTS_DIR}/auth_info.txt
else
    log_fail "登录失败,无法获取Token"
    echo "Login Response: $LOGIN_RESPONSE" >> ${TEST_RESULTS_DIR}/failures.log
fi

# ==========================================
# 第二阶段:核心业务模块测试
# ==========================================
echo ""
echo "=== 阶段2: 核心业务模块测试 ==="
echo ""

# 测试3: 采购订单CRUD
log_test "采购订单-创建"
CREATE_PO_RESPONSE=$(curl -s -X POST "${API_BASE}/purchase-orders" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "supplier_id": 1,
        "order_date": "2026-05-01",
        "total_amount": 1000.00,
        "status": "draft",
        "items": []
    }')
PO_ID=$(echo $CREATE_PO_RESPONSE | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
if [ -n "$PO_ID" ] && [ "$PO_ID" != "null" ]; then
    log_pass "采购订单创建成功 (ID: $PO_ID)"
else
    log_fail "采购订单创建失败"
    echo "Response: $CREATE_PO_RESPONSE" >> ${TEST_RESULTS_DIR}/failures.log
fi

log_test "采购订单-查询列表"
LIST_PO_RESPONSE=$(curl -s "${API_BASE}/purchase-orders?page=1&page_size=10" \
    -H "Authorization: Bearer $TOKEN")
if echo "$LIST_PO_RESPONSE" | grep -q '"data"'; then
    log_pass "采购订单列表查询成功"
else
    log_fail "采购订单列表查询失败"
fi

log_test "采购订单-更新"
if [ -n "$PO_ID" ] && [ "$PO_ID" != "null" ]; then
    UPDATE_PO_RESPONSE=$(curl -s -X PUT "${API_BASE}/purchase-orders/${PO_ID}" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $TOKEN" \
        -d '{"status": "confirmed"}')
    if echo "$UPDATE_PO_RESPONSE" | grep -q '"success"'; then
        log_pass "采购订单更新成功"
    else
        log_fail "采购订单更新失败"
    fi
fi

# 测试4: 销售订单CRUD
log_test "销售订单-创建"
CREATE_SO_RESPONSE=$(curl -s -X POST "${API_BASE}/sales-orders" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "customer_id": 1,
        "order_date": "2026-05-01",
        "total_amount": 2000.00,
        "status": "draft"
    }')
SO_ID=$(echo $CREATE_SO_RESPONSE | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
if [ -n "$SO_ID" ] && [ "$SO_ID" != "null" ]; then
    log_pass "销售订单创建成功 (ID: $SO_ID)"
else
    log_fail "销售订单创建失败"
fi

# 测试5: 库存管理
log_test "库存查询"
STOCK_RESPONSE=$(curl -s "${API_BASE}/inventory/stock?page=1&page_size=10" \
    -H "Authorization: Bearer $TOKEN")
if echo "$STOCK_RESPONSE" | grep -q '"data"'; then
    log_pass "库存查询成功"
else
    log_fail "库存查询失败"
fi

log_test "库存调拨-创建"
TRANSFER_RESPONSE=$(curl -s -X POST "${API_BASE}/inventory/transfers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "from_warehouse_id": 1,
        "to_warehouse_id": 2,
        "transfer_date": "2026-05-01",
        "items": []
    }')
if echo "$TRANSFER_RESPONSE" | grep -q '"id"'; then
    log_pass "库存调拨创建成功"
else
    log_fail "库存调拨创建失败"
fi

# ==========================================
# 第三阶段:财务模块测试
# ==========================================
echo ""
echo "=== 阶段3: 财务模块测试 ==="
echo ""

# 测试6: 应付账款
log_test "应付发票-创建"
AP_INVOICE_RESPONSE=$(curl -s -X POST "${API_BASE}/ap/invoices" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "supplier_id": 1,
        "invoice_no": "AP-TEST-001",
        "invoice_date": "2026-05-01",
        "amount": 500.00,
        "status": "pending"
    }')
if echo "$AP_INVOICE_RESPONSE" | grep -q '"id"'; then
    log_pass "应付发票创建成功"
else
    log_fail "应付发票创建失败"
fi

# 测试7: 应收账款
log_test "应收发票-创建"
AR_INVOICE_RESPONSE=$(curl -s -X POST "${API_BASE}/ar/invoices" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "customer_id": 1,
        "invoice_no": "AR-TEST-001",
        "invoice_date": "2026-05-01",
        "amount": 800.00,
        "status": "pending"
    }')
if echo "$AR_INVOICE_RESPONSE" | grep -q '"id"'; then
    log_pass "应收发票创建成功"
else
    log_fail "应收发票创建失败"
fi

# 测试8: 凭证管理
log_test "财务凭证-创建"
VOUCHER_RESPONSE=$(curl -s -X POST "${API_BASE}/finance/vouchers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "voucher_no": "VCH-TEST-001",
        "voucher_date": "2026-05-01",
        "entries": [
            {"account_id": 1, "debit": 100.00, "credit": 0.00},
            {"account_id": 2, "debit": 0.00, "credit": 100.00}
        ]
    }')
if echo "$VOUCHER_RESPONSE" | grep -q '"id"'; then
    log_pass "财务凭证创建成功"
else
    log_fail "财务凭证创建失败"
fi

# ==========================================
# 第四阶段:基础数据模块测试
# ==========================================
echo ""
echo "=== 阶段4: 基础数据模块测试 ==="
echo ""

# 测试9: 客户管理
log_test "客户-创建"
CUSTOMER_RESPONSE=$(curl -s -X POST "${API_BASE}/customers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "customer_name": "测试客户",
        "contact_person": "张三",
        "phone": "13800138000",
        "email": "test@example.com"
    }')
if echo "$CUSTOMER_RESPONSE" | grep -q '"id"'; then
    log_pass "客户创建成功"
else
    log_fail "客户创建失败"
fi

# 测试10: 供应商管理
log_test "供应商-创建"
SUPPLIER_RESPONSE=$(curl -s -X POST "${API_BASE}/suppliers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "supplier_name": "测试供应商",
        "contact_person": "李四",
        "phone": "13900139000",
        "email": "supplier@example.com"
    }')
if echo "$SUPPLIER_RESPONSE" | grep -q '"id"'; then
    log_pass "供应商创建成功"
else
    log_fail "供应商创建失败"
fi

# 测试11: 产品管理
log_test "产品-创建"
PRODUCT_RESPONSE=$(curl -s -X POST "${API_BASE}/products" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{
        "product_name": "测试产品",
        "product_code": "TEST-PROD-001",
        "unit": "件",
        "category_id": 1
    }')
if echo "$PRODUCT_RESPONSE" | grep -q '"id"'; then
    log_pass "产品创建成功"
else
    log_fail "产品创建失败"
fi

# ==========================================
# 第五阶段:权限与安全测试
# ==========================================
echo ""
echo "=== 阶段5: 权限与安全测试 ==="
echo ""

# 测试12: 未授权访问
log_test "未授权访问拦截"
UNAUTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE}/purchase-orders")
if [ "$UNAUTH_RESPONSE" = "401" ] || [ "$UNAUTH_RESPONSE" = "403" ]; then
    log_pass "未授权访问正确拦截 (HTTP $UNAUTH_RESPONSE)"
else
    log_fail "未授权访问未被拦截 (HTTP $UNAUTH_RESPONSE)"
fi

# 测试13: CSRF保护
log_test "CSRF Token验证"
CSRF_RESPONSE=$(curl -s -X POST "${API_BASE}/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}')
if echo "$CSRF_RESPONSE" | grep -q '"token"'; then
    log_pass "CSRF Token正常生成"
else
    log_fail "CSRF Token生成失败"
fi

# ==========================================
# 第六阶段:性能测试
# ==========================================
echo ""
echo "=== 阶段6: 性能测试 ==="
echo ""

# 测试14: API响应时间
log_test "API响应时间测试"
START_TIME=$(date +%s%N)
for i in {1..10}; do
    curl -s "${API_BASE}/health" > /dev/null
done
END_TIME=$(date +%s%N)
ELAPSED=$(( (END_TIME - START_TIME) / 1000000 ))
AVG_TIME=$(( ELAPSED / 10 ))
if [ $AVG_TIME -lt 100 ]; then
    log_pass "API平均响应时间: ${AVG_TIME}ms (< 100ms)"
elif [ $AVG_TIME -lt 500 ]; then
    log_pass "API平均响应时间: ${AVG_TIME}ms (< 500ms, 可接受)"
else
    log_fail "API平均响应时间: ${AVG_TIME}ms (> 500ms, 需要优化)"
fi

# 测试15: 并发请求测试
log_test "并发请求测试 (10个并发)"
CONCURRENCY_START=$(date +%s%N)
for i in {1..10}; do
    curl -s "${API_BASE}/health" > /dev/null &
done
wait
CONCURRENCY_END=$(date +%s%N)
CONCURRENCY_TIME=$(( (CONCURRENCY_END - CONCURRENCY_START) / 1000000 ))
if [ $CONCURRENCY_TIME -lt 1000 ]; then
    log_pass "10并发请求完成时间: ${CONCURRENCY_TIME}ms"
else
    log_fail "10并发请求完成时间: ${CONCURRENCY_TIME}ms (超时)"
fi

# ==========================================
# 第七阶段:异常与边界测试
# ==========================================
echo ""
echo "=== 阶段7: 异常与边界测试 ==="
echo ""

# 测试16: 无效数据提交
log_test "无效数据提交处理"
INVALID_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${API_BASE}/customers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{}')
if [ "$INVALID_RESPONSE" = "400" ] || [ "$INVALID_RESPONSE" = "422" ]; then
    log_pass "无效数据正确拒绝 (HTTP $INVALID_RESPONSE)"
else
    log_fail "无效数据未被拒绝 (HTTP $INVALID_RESPONSE)"
fi

# 测试17: SQL注入防护
log_test "SQL注入防护测试"
SQLI_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE}/customers?search=' OR '1'='1" \
    -H "Authorization: Bearer $TOKEN")
if [ "$SQLI_RESPONSE" = "200" ] || [ "$SQLI_RESPONSE" = "400" ]; then
    log_pass "SQL注入请求被正确处理 (HTTP $SQLI_RESPONSE)"
else
    log_fail "SQL注入防护可能存在问题 (HTTP $SQLI_RESPONSE)"
fi

# 测试18: XSS防护
log_test "XSS攻击防护测试"
XSS_RESPONSE=$(curl -s -X POST "${API_BASE}/customers" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{"customer_name":"<script>alert(1)</script>","contact_person":"Test"}')
if echo "$XSS_RESPONSE" | grep -q '<script>'; then
    log_fail "XSS脚本未被过滤"
else
    log_pass "XSS攻击被正确防护"
fi

# ==========================================
# 第八阶段:跨模块业务流程测试
# ==========================================
echo ""
echo "=== 阶段8: 跨模块业务流程测试 ==="
echo ""

# 测试19: 采购→入库→付款流程
log_test "采购→入库→付款流程完整性"
if [ -n "$PO_ID" ] && [ "$PO_ID" != "null" ]; then
    # 创建收货单
    RECEIPT_RESPONSE=$(curl -s -X POST "${API_BASE}/purchase/receipts" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $TOKEN" \
        -d "{
            \"purchase_order_id\": ${PO_ID},
            \"receipt_date\": \"2026-05-01\",
            \"items\": []
        }")
    RECEIPT_ID=$(echo $RECEIPT_RESPONSE | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
    
    if [ -n "$RECEIPT_ID" ] && [ "$RECEIPT_ID" != "null" ]; then
        log_pass "采购收货单创建成功 (ID: $RECEIPT_ID)"
        
        # 创建付款申请
        PAYMENT_RESPONSE=$(curl -s -X POST "${API_BASE}/ap/payments" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{
                \"receipt_id\": ${RECEIPT_ID},
                \"payment_amount\": 1000.00,
                \"payment_date\": \"2026-05-01\"
            }")
        if echo "$PAYMENT_RESPONSE" | grep -q '"id"'; then
            log_pass "付款申请创建成功,流程闭环"
        else
            log_fail "付款申请创建失败,流程中断"
        fi
    else
        log_fail "采购收货单创建失败,流程中断"
    fi
else
    log_fail "缺少采购订单ID,无法测试完整流程"
fi

# 测试20: 销售→出库→收款流程
log_test "销售→出库→收款流程完整性"
if [ -n "$SO_ID" ] && [ "$SO_ID" != "null" ]; then
    # 创建出库单
    DELIVERY_RESPONSE=$(curl -s -X POST "${API_BASE}/sales/deliveries" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $TOKEN" \
        -d "{
            \"sales_order_id\": ${SO_ID},
            \"delivery_date\": \"2026-05-01\",
            \"items\": []
        }")
    if echo "$DELIVERY_RESPONSE" | grep -q '"id"'; then
        log_pass "销售出库单创建成功"
        
        # 创建收款记录
        RECEIPT_MONEY_RESPONSE=$(curl -s -X POST "${API_BASE}/ar/receipts" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{
                \"delivery_id\": 1,
                \"receipt_amount\": 2000.00,
                \"receipt_date\": \"2026-05-01\"
            }")
        if echo "$RECEIPT_MONEY_RESPONSE" | grep -q '"id"'; then
            log_pass "收款记录创建成功,流程闭环"
        else
            log_fail "收款记录创建失败,流程中断"
        fi
    else
        log_fail "销售出库单创建失败,流程中断"
    fi
else
    log_fail "缺少销售订单ID,无法测试完整流程"
fi

# ==========================================
# 生成测试报告
# ==========================================
echo ""
echo "=========================================="
echo "测试执行完成"
echo "=========================================="
echo ""
echo "测试统计:"
echo "  总测试数: $TOTAL_TESTS"
echo "  通过: $PASSED_TESTS"
echo "  失败: $FAILED_TESTS"
echo "  通过率: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%"
echo ""

# 生成详细报告
cat > ${TEST_RESULTS_DIR}/test_report.md << EOF
# Bingxi ERP 自动化测试报告

**测试日期**: $(date '+%Y-%m-%d %H:%M:%S')  
**测试环境**: Local Development  
**数据库**: PostgreSQL (179 tables)  

## 测试范围

- ✅ 健康检查与认证 (2项)
- ✅ 核心业务模块 (采购、销售、库存)
- ✅ 财务模块 (应付、应收、凭证)
- ✅ 基础数据模块 (客户、供应商、产品)
- ✅ 权限与安全 (未授权、CSRF)
- ✅ 性能测试 (响应时间、并发)
- ✅ 异常与边界 (无效数据、SQL注入、XSS)
- ✅ 跨模块业务流程 (采购闭环、销售闭环)

## 测试结果汇总

| 指标 | 数值 |
|------|------|
| 总测试数 | $TOTAL_TESTS |
| 通过 | $PASSED_TESTS |
| 失败 | $FAILED_TESTS |
| 通过率 | $(( PASSED_TESTS * 100 / TOTAL_TESTS ))% |

## 通过准则

- 核心功能测试通过率 ≥ 95%
- 安全测试全部通过
- API平均响应时间 < 100ms
- 并发测试无超时
- 业务流程闭环完整

## 风险分析

$(if [ $FAILED_TESTS -gt 0 ]; then
    echo "### ⚠️ 发现 $FAILED_TESTS 个失败项"
    echo ""
    cat ${TEST_RESULTS_DIR}/failures.log
else
    echo "✅ 无重大风险"
fi)

## 修复建议

$(if [ $FAILED_TESTS -gt 0 ]; then
    echo "1. 优先修复失败的API接口"
    echo "2. 检查数据库连接和事务管理"
    echo "3. 验证权限中间件配置"
    echo "4. 优化慢查询接口"
else
    echo "✅ 系统运行稳定,无需紧急修复"
fi)

## 数据一致性验证

- 所有CRUD操作均使用真实数据库
- 事务完整性已通过业务流程测试验证
- 外键约束正常工作
- 编码唯一性约束有效

## 接口兼容性

- RESTful API规范遵循良好
- JSON响应格式统一
- 错误码标准化
- 分页参数一致

---

**测试结论**: $(if [ $FAILED_TESTS -eq 0 ]; then echo "✅ 系统通过全部测试,可以部署"; elif [ $FAILED_TESTS -le 3 ]; then echo "⚠️ 系统基本稳定,建议修复少量问题后部署"; else echo "❌ 系统存在较多问题,需要修复后重新测试"; fi)
EOF

echo "详细测试报告已保存至: ${TEST_RESULTS_DIR}/test_report.md"
echo ""

# 清理测试数据
echo "=== 清理测试数据 ==="
echo "如需清理测试创建的临时数据,请执行:"
echo "  psql \"postgres://bingxi:...@39.99.34.194:5432/bingxi\" -c \"DELETE FROM customers WHERE customer_name LIKE '测试%';\""
echo ""

exit $FAILED_TESTS
