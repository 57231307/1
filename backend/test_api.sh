#!/bin/bash
# 后端 API 端点测试脚本

BASE_URL="http://localhost:8082/api/v1/erp"
PASS=0
FAIL=0
WARN=0

echo "=========================================="
echo "  后端 API 端点全面测试"
echo "=========================================="
echo "基准 URL: $BASE_URL"
echo ""

# 测试函数
test_endpoint() {
    local method=$1
    local path=$2
    local name=$3
    local expected_status=${4:-200}
    
    local url="$BASE_URL/$path"
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -o /dev/null -w "%{http_code}" "$url")
    elif [ "$method" = "POST" ]; then
        response=$(curl -s -o /dev/null -w "%{http_code}" -X POST -H "Content-Type: application/json" -d '{}' "$url")
    fi
    
    if [ "$response" = "$expected_status" ] || [ "$response" = "201" ] || [ "$response" = "404" ] || [ "$response" = "405" ]; then
        if [ "$response" = "200" ] || [ "$response" = "201" ]; then
            echo "✓ $name: $method /$path -> $response"
            PASS=$((PASS + 1))
        elif [ "$response" = "401" ] || [ "$response" = "403" ]; then
            echo "⚠ $name: $method /$path -> $response (需要认证)"
            WARN=$((WARN + 1))
        else
            echo "✗ $name: $method /$path -> $response"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "✗ $name: $method /$path -> $response (期望: $expected_status)"
        FAIL=$((FAIL + 1))
    fi
}

echo "--- 系统与健康检查 ---"
test_endpoint "GET" "health" "健康检查"
test_endpoint "GET" "init/status" "初始化状态"

echo ""
echo "--- 认证与用户 ---"
test_endpoint "POST" "auth/login" "用户登录"
test_endpoint "GET" "users" "用户列表"

echo ""
echo "--- 系统管理 ---"
test_endpoint "GET" "tenants" "租户列表"
test_endpoint "GET" "roles" "角色列表"
test_endpoint "GET" "departments" "部门列表"

echo ""
echo "--- 客户与供应商 (CRM) ---"
test_endpoint "GET" "customers" "客户列表"
test_endpoint "GET" "suppliers" "供应商列表"
test_endpoint "GET" "crm/pool" "客户池"

echo ""
echo "--- 产品与库存 ---"
test_endpoint "GET" "products" "产品列表"
test_endpoint "GET" "product-categories" "产品分类"
test_endpoint "GET" "inventory/stock" "库存查询"
test_endpoint "GET" "warehouses" "仓库列表"

echo ""
echo "--- 采购管理 ---"
test_endpoint "GET" "purchase-orders" "采购订单"
test_endpoint "GET" "purchase-receipts" "采购入库"
test_endpoint "GET" "purchase-returns" "采购退货"

echo ""
echo "--- 销售管理 ---"
test_endpoint "GET" "sales-orders" "销售订单"
test_endpoint "GET" "sales/contracts" "销售合同"
test_endpoint "GET" "sales/returns" "销售退货"

echo ""
echo "--- 财务管理 ---"
test_endpoint "GET" "finance/account-subjects" "会计科目"
test_endpoint "GET" "finance/vouchers" "凭证列表"
test_endpoint "GET" "finance/reports" "财务报表"

echo ""
echo "--- 应收管理 (AR) ---"
test_endpoint "GET" "ar/invoices" "应收发票"
test_endpoint "GET" "ar/reconciliation" "应收对账"

echo ""
echo "--- 应付管理 (AP) ---"
test_endpoint "GET" "ap/invoices" "应付发票"
test_endpoint "GET" "ap/reconciliation" "应付对账"

echo ""
echo "--- 面料管理 ---"
test_endpoint "GET" "fabrics" "面料列表"
test_endpoint "GET" "dye-recipes" "染色配方"

echo ""
echo "--- 生产与 BOM ---"
test_endpoint "GET" "bom" "BOM列表"
test_endpoint "GET" "production-orders" "生产订单"
test_endpoint "GET" "mrp/requirements" "MRP需求"

echo ""
echo "--- 报表与分析 ---"
test_endpoint "GET" "reports/dashboard" "仪表盘报表"
test_endpoint "GET" "reports/sales" "销售报表"
test_endpoint "GET" "reports/purchase" "采购报表"

echo ""
echo "--- 高级功能 ---"
test_endpoint "GET" "notifications" "通知列表"
test_endpoint "GET" "audit/logs" "审计日志"

echo ""
echo "=========================================="
echo "  测试汇总"
echo "=========================================="
echo "通过: $PASS"
echo "警告: $WARN"
echo "失败: $FAIL"
echo "总计: $((PASS + WARN + FAIL))"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "存在失败的 API 端点，需要进一步检查"
    exit 1
else
    echo "所有 API 端点测试完成"
    exit 0
fi
