#!/bin/bash
# 后端 API 端点测试脚本（带认证）

BASE_URL="http://localhost:8082/api/v1/erp"
PASS=0
FAIL=0
WARN=0

echo "=========================================="
echo "  后端 API 端点全面测试（带认证）"
echo "=========================================="
echo "基准 URL: $BASE_URL"
echo ""

# 获取认证 token
echo "--- 获取认证 Token ---"
TOKEN=$(/usr/bin/curl -s -X POST "$BASE_URL/auth/login" -H "Content-Type: application/json" -d '{"username":"admin","password":"admin123"}' | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['token'])")
if [ -z "$TOKEN" ]; then
    echo "✗ 获取 Token 失败"
    exit 1
fi
echo "✓ Token 获取成功"
echo ""

# 测试函数
test_endpoint() {
    local method=$1
    local path=$2
    local name=$3
    
    local url="$BASE_URL/$path"
    
    if [ "$method" = "GET" ]; then
        response=$(/usr/bin/curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN" "$url")
    elif [ "$method" = "POST" ]; then
        response=$(/usr/bin/curl -s -o /dev/null -w "%{http_code}" -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{}' "$url")
    fi
    
    if [ "$response" = "200" ] || [ "$response" = "201" ]; then
        echo "✓ $name: $method /$path -> $response"
        PASS=$((PASS + 1))
    elif [ "$response" = "404" ]; then
        echo "⚠ $name: $method /$path -> $response (路由不存在)"
        WARN=$((WARN + 1))
    elif [ "$response" = "500" ]; then
        echo "✗ $name: $method /$path -> $response (服务器错误)"
        FAIL=$((FAIL + 1))
    else
        echo "⚠ $name: $method /$path -> $response"
        WARN=$((WARN + 1))
    fi
}

echo "--- 系统与健康检查 ---"
test_endpoint "GET" "health" "健康检查"
test_endpoint "GET" "init/status" "初始化状态"

echo ""
echo "--- 认证与用户 ---"
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

echo ""
echo "--- 产品与库存 ---"
test_endpoint "GET" "products" "产品列表"
test_endpoint "GET" "product-categories" "产品分类"
test_endpoint "GET" "inventory/stock" "库存查询"
test_endpoint "GET" "warehouses" "仓库列表"

echo ""
echo "--- 财务管理 ---"
test_endpoint "GET" "gl/subjects" "会计科目"
test_endpoint "GET" "gl/vouchers" "凭证列表"

echo ""
echo "--- 应收管理 (AR) ---"
test_endpoint "GET" "ar/invoices" "应收发票"

echo ""
echo "--- 应付管理 (AP) ---"
test_endpoint "GET" "ap/invoices" "应付发票"

echo ""
echo "--- 面料管理 ---"
test_endpoint "GET" "greige-fabrics" "面料列表"

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
