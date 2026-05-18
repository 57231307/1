# 全面功能测试脚本

## 测试环境信息
- **服务器 IP**: 111.230.99.236
- **数据库**: 39.99.34.194:5432/bingxi
- **API 基础路径**: `/api/v1/erp/`
- **健康检查**: `/api/v1/erp/health`

## 测试分类

### P0 - 核心功能（必须测试）
1. **用户认证**
   - POST /api/v1/erp/auth/login - 用户登录
   - POST /api/v1/erp/auth/logout - 用户登出
   - GET /api/v1/erp/users/me - 获取当前用户信息

2. **财务管理**
   - GET /api/v1/erp/finance/invoices - 发票列表
   - POST /api/v1/erp/finance/invoices - 创建发票
   - GET /api/v1/erp/finance/payments - 收款列表
   - POST /api/v1/erp/finance/payments - 创建收款
   - GET /api/v1/erp/finance/reports/balance-sheet - 资产负债表
   - GET /api/v1/erp/finance/reports/income-statement - 利润表

3. **销售管理**
   - GET /api/v1/erp/sales/orders - 销售订单列表
   - POST /api/v1/erp/sales/orders - 创建销售订单
   - PUT /api/v1/erp/sales/orders/{id}/approve - 审批订单
   - PUT /api/v1/erp/sales/orders/{id}/complete - 完成订单

4. **采购管理**
   - GET /api/v1/erp/purchase/orders - 采购订单列表
   - POST /api/v1/erp/purchase/orders - 创建采购订单
   - GET /api/v1/erp/purchase/receipts - 入库单列表
   - POST /api/v1/erp/purchase/inspections - 创建质检单
   - GET /api/v1/erp/purchase/returns - 退货单列表

5. **库存管理**
   - GET /api/v1/erp/inventory/stock - 库存列表
   - POST /api/v1/erp/inventory/adjustments - 库存调整
   - POST /api/v1/erp/inventory/transfers - 新建调拨单
   - GET /api/v1/erp/inventory/warehouses - 仓库列表

6. **CRM 客户管理**
   - GET /api/v1/erp/crm/leads - 线索列表
   - POST /api/v1/erp/crm/leads - 创建线索
   - GET /api/v1/erp/crm/opportunities - 商机列表
   - POST /api/v1/erp/crm/opportunities - 创建商机
   - GET /api/v1/erp/customers - 客户列表

### P1 - 重要功能
7. **产品管理**
   - GET /api/v1/erp/products - 产品列表
   - POST /api/v1/erp/products - 创建产品
   - GET /api/v1/erp/product-categories - 产品分类
   - POST /api/v1/erp/products/import - 产品导入

8. **供应商管理**
   - GET /api/v1/erp/suppliers - 供应商列表
   - POST /api/v1/erp/suppliers - 创建供应商
   - GET /api/v1/erp/supplier-evaluations - 供应商评估

9. **应收管理**
   - GET /api/v1/erp/ar-reconciliations - 对账单列表
   - POST /api/v1/erp/ar-reconciliations - 创建对账单
   - GET /api/v1/erp/ar-aging - 账龄分析

10. **应付管理**
    - GET /api/v1/erp/ap/invoices - 应付发票列表
    - GET /api/v1/erp/ap/payments - 应付付款列表

11. **成本管理**
    - GET /api/v1/erp/cost/collections - 成本归集列表
    - POST /api/v1/erp/cost/collections - 创建成本归集

12. **币种与汇率**
    - GET /api/v1/erp/currencies - 币种列表
    - GET /api/v1/erp/exchange-rates - 汇率列表
    - POST /api/v1/erp/exchange-rates - 创建汇率

### P2 - 辅助功能
13. **系统管理**
    - GET /api/v1/erp/system/users - 用户列表
    - GET /api/v1/erp/system/roles - 角色列表
    - GET /api/v1/erp/system/departments - 部门列表
    - GET /api/v1/erp/system/accounting-periods - 会计期间

14. **报表分析**
    - GET /api/v1/erp/dashboard/overview - 总览仪表板
    - GET /api/v1/erp/sales-analysis/trends - 销售趋势分析
    - GET /api/v1/erp/report-engine/templates - 报表模板

15. **高级功能**
    - GET /api/v1/erp/advanced/batches - 批次管理
    - GET /api/v1/erp/advanced/dye-recipes - 染色配方
    - GET /api/v1/erp/advanced/greige-fabrics - 坯布管理

16. **初始化状态**
    - GET /api/v1/erp/init/status - 系统初始化状态

### P3 - AI 功能
17. **AI 预测**
    - POST /api/v1/erp/ai/forecast-sales - 销售预测

## 测试步骤

### 1. 健康检查
```bash
curl -s http://111.230.99.236/api/v1/erp/health | jq .
```

### 2. 登录获取 Token
```bash
curl -s -X POST http://111.230.99.236/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq .
```

### 3. 保存 Token
```bash
TOKEN=$(curl -s -X POST http://111.230.99.236/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')
echo "Token: $TOKEN"
```

### 4. 测试所有端点
```bash
# 设置 headers
AUTH_HEADER="Authorization: Bearer $TOKEN"

# 测试示例 - 获取用户信息
curl -s http://111.230.99.236/api/v1/erp/users/me \
  -H "$AUTH_HEADER" | jq .

# 测试示例 - 获取发票列表
curl -s http://111.230.99.236/api/v1/erp/finance/invoices \
  -H "$AUTH_HEADER" | jq .

# 测试示例 - 获取客户列表
curl -s http://111.230.99.236/api/v1/erp/customers \
  -H "$AUTH_HEADER" | jq .
```

## 预期结果
- P0 功能：100% 可用
- P1 功能：95% 可用
- P2 功能：90% 可用
- P3 功能：85% 可用

## 问题记录模板

### 问题描述
- 端点：
- 预期行为：
- 实际行为：
- 错误信息：
- 优先级：P0/P1/P2/P3

### 修复方案
- 根因分析：
- 修复步骤：
- 验证方法：

## 测试报告输出格式
```markdown
# ERP 系统全面功能测试报告

## 测试时间
[日期时间]

## 测试环境
- 服务器：111.230.99.236
- 版本：[从 /api/v1/erp/health 获取]

## 测试结果汇总
- P0 功能：X/Y 通过 (Z%)
- P1 功能：X/Y 通过 (Z%)
- P2 功能：X/Y 通过 (Z%)
- P3 功能：X/Y 通过 (Z%)
- 总计：X/Y 通过 (Z%)

## 详细结果
[按模块列出每个端点的测试结果]

## 问题清单
[列出所有发现的问题]

## 修复规划
[按优先级列出修复计划]
```
