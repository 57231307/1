#!/bin/bash

# 批次 28 v7 P0-2 修复：移除硬编码生产 IP，改为 fail-secure 模式。
# 原值直接暴露生产服务器地址，攻击者扫描 GitHub 即可定位生产环境进行攻击。
# 必须设置 BINGXI_API_BASE 环境变量（如 http://localhost:8082 或 https://erp.example.com）。
SERVER="${BINGXI_API_BASE:?必须设置 BINGXI_API_BASE 环境变量（被测系统基础地址）}"
BASE_API="/api/v1/erp"

# 登录获取 token
echo "=== 登录获取 token ==="
TOKEN=$(curl -s -X POST "${SERVER}${BASE_API}/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${BINGXI_ADMIN_USERNAME:-admin}\",\"password\":\"${BINGXI_ADMIN_PASSWORD:?必须设置 BINGXI_ADMIN_PASSWORD 环境变量}\"}" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

echo "Token: ${TOKEN:0:50}..."
echo ""

AUTH_HEADER="Authorization: Bearer $TOKEN"

# 测试所有主要模块的 CRUD 接口
echo "=== 测试各模块 API ==="

test_api() {
  local name="$1"
  local endpoint="$2"
  
  echo -n "测试 $name... "
  RESPONSE=$(curl -s -H "$AUTH_HEADER" "${SERVER}${BASE_API}${endpoint}")
  
  if echo "$RESPONSE" | grep -q '"code":200\|"code":400'; then
    echo "✓ OK"
  elif echo "$RESPONSE" | grep -q '"code":404'; then
    echo "❌ 404 Not Found"
    echo "   URL: ${SERVER}${BASE_API}${endpoint}"
  elif echo "$RESPONSE" | grep -q '"code":500'; then
    echo "❌ 500 Server Error"
    echo "   Response: $RESPONSE"
  else
    echo "⚠️ Unknown: $RESPONSE"
  fi
}

# 基础数据模块
test_api "用户列表" "/users?page=1&page_size=20"
test_api "角色列表" "/roles?page=1&page_size=20"
test_api "部门列表" "/departments?page=1&page_size=20"
test_api "客户列表" "/crm/customers?page=1&page_size=20"
test_api "供应商列表" "/suppliers?page=1&page_size=20"
test_api "产品列表" "/products?page=1&page_size=20"
test_api "仓库列表" "/warehouses?page=1&page_size=20"

# 销售模块
test_api "销售订单" "/sales/orders?page=1&page_size=20"
test_api "销售合同" "/sales/contracts?page=1&page_size=20"
test_api "销售退货" "/sales/returns?page=1&page_size=20"

# 采购模块
test_api "采购订单" "/purchase/orders?page=1&page_size=20"
test_api "采购合同" "/purchase/contracts?page=1&page_size=20"
test_api "采购入库" "/purchase/receipts?page=1&page_size=20"

# 库存模块
test_api "库存查询" "/inventory?page=1&page_size=20"
test_api "库存盘点" "/inventory/counts?page=1&page_size=20"
test_api "库存调拨" "/inventory/transfers?page=1&page_size=20"

# 财务模块
test_api "会计科目" "/finance/subjects?page=1&page_size=20"
test_api "凭证" "/finance/vouchers?page=1&page_size=20"
test_api "应收" "/ar/invoices?page=1&page_size=20"
test_api "应付" "/ap/invoices?page=1&page_size=20"

# 打印接口测试
echo ""
echo "=== 测试打印相关接口 ==="
test_api "销售订单打印" "/sales/orders/1/print"
test_api "采购订单打印" "/purchase/orders/1/print"
test_api "入库单打印" "/purchase/receipts/1/print"

echo ""
echo "=== 测试完成 ==="
