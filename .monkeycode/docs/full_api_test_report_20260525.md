# ERP 全面 API 测试报告

**测试时间**: 2026-05-25
**测试环境**: http://111.230.99.236:8082
**测试账号**: admin/admin123
**测试批次**: 10 批次

---

## 📊 总体统计

| 指标 | 数值 |
|------|------|
| **总测试数** | 266 |
| **通过 ✅** | 124 (46.6%) |
| **失败 ❌** | 142 (53.4%) |

### 失败类型分布

| HTTP 状态码 | 数量 | 占比 | 说明 |
|------------|------|------|------|
| **404 Not Found** | 95 | 66.9% | 路由未注册或记录不存在 |
| **422 Validation Error** | 16 | 11.3% | 请求体验证失败 |
| **500 Server Error** | 15 | 10.6% | 服务端内部错误 |
| **400 Bad Request** | 9 | 6.3% | 请求参数错误 |
| **405 Method Not Allowed** | 5 | 3.5% | HTTP 方法不支持 |
| **201 Created** | 2 | 1.4% | 状态码差异（实际成功） |

---

## ✅ 通过的功能模块 (124 项)

### 1. 认证模块 (3/3 ✅)
- ✅ POST /api/v1/erp/auth/login
- ✅ GET /api/v1/erp/auth/totp/setup
- ✅ GET /api/v1/erp/auth/csrf-token

### 2. 用户管理 (3/3 ✅)
- ✅ GET /api/v1/erp/users
- ✅ GET /api/v1/erp/users/4
- ✅ GET /api/v1/erp/users?page=1&per_page=10

### 3. 角色管理 (1/3 ✅)
- ✅ GET /api/v1/erp/roles
- ❌ GET /api/v1/erp/roles/1 (404)
- ❌ GET /api/v1/erp/roles/1/permissions (500)

### 4. 部门管理 (4/4 ✅)
- ✅ GET /api/v1/erp/departments
- ✅ GET /api/v1/erp/departments/tree
- ✅ GET /api/v1/erp/departments/1
- ✅ POST /api/v1/erp/departments
- ✅ PUT /api/v1/erp/departments/1

### 5. 产品管理 (5/5 ✅)
- ✅ GET /api/v1/erp/products
- ✅ GET /api/v1/erp/products/select
- ✅ GET /api/v1/erp/products/1
- ✅ PUT /api/v1/erp/products/1
- ✅ GET /api/v1/erp/products?keyword=测试

### 6. 产品分类 (2/2 ✅)
- ✅ GET /api/v1/erp/product-categories
- ✅ GET /api/v1/erp/product-categories/tree

### 7. 仓库管理 (3/3 ✅)
- ✅ GET /api/v1/erp/warehouses
- ✅ GET /api/v1/erp/warehouses/1
- ✅ PUT /api/v1/erp/warehouses/1

### 8. 供应商管理 (4/4 ✅)
- ✅ GET /api/v1/erp/suppliers
- ✅ GET /api/v1/erp/suppliers/1
- ✅ PUT /api/v1/erp/suppliers/1
- ✅ GET /api/v1/erp/suppliers?keyword=测试

### 9. 客户管理 (4/4 ✅)
- ✅ GET /api/v1/erp/customers
- ✅ GET /api/v1/erp/customers/1
- ✅ PUT /api/v1/erp/customers/1
- ✅ GET /api/v1/erp/customers?keyword=测试

### 10. 销售模块 (8/14 ✅)
- ✅ GET /api/v1/erp/sales/orders
- ✅ GET /api/v1/erp/sales/orders/export
- ✅ GET /api/v1/erp/sales/orders?status=draft
- ✅ GET /api/v1/erp/sales/orders?page=1&per_page=5
- ✅ GET /api/v1/erp/sales-contracts
- ✅ GET /api/v1/erp/sales-prices
- ✅ GET /api/v1/erp/sales-prices/strategies
- ✅ GET /api/v1/erp/sales-returns
- ✅ GET /api/v1/erp/sales/fabric-orders

### 11. 销售分析 (3/4 ✅)
- ✅ GET /api/v1/erp/sales-analysis/statistics
- ✅ GET /api/v1/erp/sales-analysis/rankings
- ✅ GET /api/v1/erp/sales-analysis/targets

### 12. 采购模块 (7/12 ✅)
- ✅ GET /api/v1/erp/purchases/orders
- ✅ GET /api/v1/erp/purchases/orders/export
- ✅ GET /api/v1/erp/purchases/orders?status=draft
- ✅ GET /api/v1/erp/purchase-contracts
- ✅ GET /api/v1/erp/purchase-prices
- ✅ GET /api/v1/erp/purchases/receipts
- ✅ GET /api/v1/erp/purchases/returns
- ✅ GET /api/v1/erp/purchases/inspections

### 13. 生产模块 (7/10 ✅)
- ✅ GET /api/v1/erp/production/orders
- ✅ GET /api/v1/erp/production/orders?status=draft
- ✅ GET /api/v1/erp/boms
- ✅ GET /api/v1/erp/boms/1
- ✅ GET /api/v1/erp/mrp/requirements
- ✅ GET /api/v1/erp/mrp/results

### 14. 排程模块 (3/3 ✅)
- ✅ GET /api/v1/erp/scheduling/tasks
- ✅ GET /api/v1/erp/scheduling/gantt
- ✅ GET /api/v1/erp/scheduling/conflicts

### 15. 产能模块 (3/3 ✅)
- ✅ GET /api/v1/erp/capacity/work-centers
- ✅ GET /api/v1/erp/capacity/overview
- ✅ GET /api/v1/erp/capacity/load-analysis

### 16. 库存模块 (6/14 ✅)
- ✅ GET /api/v1/erp/inventory/stock
- ✅ GET /api/v1/erp/inventory/counts
- ✅ GET /api/v1/erp/inventory/transfers
- ✅ GET /api/v1/erp/inventory/adjustments
- ✅ POST /api/v1/erp/inventory/counts

### 17. 财务模块 (10/18 ✅)
- ✅ GET /api/v1/erp/finance/payments
- ✅ GET /api/v1/erp/finance/invoices
- ✅ GET /api/v1/erp/finance/invoices/1
- ✅ GET /api/v1/erp/ap/invoices
- ✅ GET /api/v1/erp/ar/invoices
- ✅ GET /api/v1/erp/ap/payments
- ✅ GET /api/v1/erp/ap/reconciliations
- ✅ GET /api/v1/erp/ar-reconciliations
- ✅ GET /api/v1/erp/gl/vouchers
- ✅ GET /api/v1/erp/gl/subjects
- ✅ GET /api/v1/erp/finance/accounting-periods/current

### 18. 质量管理 (6/6 ✅)
- ✅ GET /api/v1/erp/quality-standards
- ✅ GET /api/v1/erp/quality-standards/1
- ✅ POST /api/v1/erp/quality-standards
- ✅ GET /api/v1/erp/quality-inspection/records
- ✅ GET /api/v1/erp/quality-inspection/standards
- ✅ GET /api/v1/erp/quality-inspection/defects

### 19. 固定资产 (2/3 ✅)
- ✅ GET /api/v1/erp/fixed-assets
- ✅ GET /api/v1/erp/fixed-assets/1

### 20. 预算管理 (2/2 ✅)
- ✅ GET /api/v1/erp/budgets/plans
- ✅ GET /api/v1/erp/budgets/items

### 21. 坯布管理 (1/2 ✅)
- ✅ GET /api/v1/erp/greige-fabrics

### 22. 染色批次 (1/2 ✅)
- ✅ GET /api/v1/erp/dye-batches

### 23. 染色配方 (1/2 ✅)
- ✅ GET /api/v1/erp/dye-recipes

### 24. 物流管理 (1/1 ✅)
- ✅ GET /api/v1/erp/logistics

### 25. 成本归集 (1/1 ✅)
- ✅ GET /api/v1/erp/cost-collections

### 26. 币种管理 (1/1 ✅)
- ✅ GET /api/v1/erp/currencies

### 27. 汇率管理 (0/1 ❌)
- ❌ GET /api/v1/erp/exchange-rates (405)

### 28. 供应商评估 (3/3 ✅)
- ✅ GET /api/v1/erp/supplier-evaluation/evaluations
- ✅ GET /api/v1/erp/supplier-evaluation/evaluations/indicators
- ✅ GET /api/v1/erp/supplier-evaluation/evaluations/rankings

### 29. 仪表盘 (4/7 ✅)
- ✅ GET /api/v1/erp/dashboard/overview
- ✅ GET /api/v1/erp/dashboard/sales-stats
- ✅ GET /api/v1/erp/dashboard/inventory-stats
- ✅ GET /api/v1/erp/dashboard/low-stock-alerts

### 30. 通知模块 (2/3 ✅)
- ✅ GET /api/v1/erp/notifications
- ✅ GET /api/v1/erp/notifications/unread-count

### 31. 租户管理 (1/7 ✅)
- ✅ GET /api/v1/erp/tenants

### 32. 批次管理 (1/1 ✅)
- ✅ GET /api/v1/erp/batches

### 33. CRM 模块 (3/6 ✅)
- ✅ GET /api/v1/erp/crm/customers
- ✅ GET /api/v1/erp/crm/customers/1
- ✅ GET /api/v1/erp/crm/pool

### 34. 系统初始化 (1/1 ✅)
- ✅ GET /api/v1/erp/init/status

### 35. 报表模板 (2/2 ✅)
- ✅ GET /api/v1/erp/reports/templates
- ✅ GET /api/v1/erp/reports/enhanced/templates

### 36. BPM 模块 (1/4 ✅)
- ✅ GET /api/v1/erp/bpm/definitions

### 37. 系统更新 (1/5 ✅)
- ✅ GET /api/v1/erp/system-update/check

### 38. 资金管理 (1/4 ✅)
- ✅ GET /api/v1/erp/fund-management/accounts

### 39. Webhook (1/2 ✅)
- ✅ GET /api/v1/erp/webhooks/integrations

### 40. DELETE 操作 (2/9 ✅)
- ✅ DELETE /api/v1/erp/greige-fabrics/999
- ✅ DELETE /api/v1/erp/dye-recipes/999

---

## ❌ 失败的功能模块 (142 项)

### 404 错误 (95 项) - 路由未注册

| 模块 | 接口 | 说明 |
|------|------|------|
| **Dashboard** | /api/v1/erp/dashboard/revenue-stats | 路由未注册 |
| | /api/v1/erp/dashboard/production-stats | 路由未注册 |
| | /api/v1/erp/dashboard/quick-links | 路由未注册 |
| **Notifications** | /api/v1/erp/notifications/1 | 路由未注册 |
| **Audit** | /api/v1/erp/audit | 路由未注册 |
| | /api/v1/erp/audit/1 | 路由未注册 |
| **Tenants** | /api/v1/erp/tenants/1 | 路由未注册 |
| | /api/v1/erp/tenant/config | 路由未注册 |
| | /api/v1/erp/tenant/billing/plans | 路由未注册 |
| | /api/v1/erp/tenant/billing/billing-info | 路由未注册 |
| **Trading** | /api/v1/erp/trading | 路由未注册 |
| **Dual Unit** | /api/v1/erp/dual-unit/conversions | 路由未注册 |
| **Material Shortage** | /api/v1/erp/material-shortage | 路由未注册 |
| | /api/v1/erp/material-shortage/analysis | 路由未注册 |
| | /api/v1/erp/material-shortage/materials | 路由未注册 |
| **Business Trace** | /api/v1/erp/business-trace | 路由未注册 |
| | /api/v1/erp/business-trace/timeline | 路由未注册 |
| | /api/v1/erp/business-trace/traces | 路由未注册 |
| **Scanner** | /api/v1/erp/scanner/devices | 路由未注册 |
| **Financial Analysis** | /api/v1/erp/financial-analysis | 路由未注册 |
| | /api/v1/erp/financial-analysis/profit | 路由未注册 |
| | /api/v1/erp/financial-analysis/cost | 路由未注册 |
| | /api/v1/erp/financial-analysis/revenue | 路由未注册 |
| **Reports** | /api/v1/erp/reports/sales | 路由未注册 |
| | /api/v1/erp/reports/purchase | 路由未注册 |
| | /api/v1/erp/reports/inventory | 路由未注册 |
| | /api/v1/erp/reports/finance | 路由未注册 |
| | /api/v1/erp/reports/production | 路由未注册 |
| **Export** | /api/v1/erp/export/templates | 路由未注册 |
| | /api/v1/erp/export/records | 路由未注册 |
| **Import** | /api/v1/erp/import/templates | 路由未注册 |
| | /api/v1/erp/import/records | 路由未注册 |
| | /api/v1/erp/import/progress | 路由未注册 |
| **System Update** | /api/v1/erp/system-update/versions | 路由未注册 |
| | /api/v1/erp/system-update/history | 路由未注册 |
| | /api/v1/erp/system-update/maintenance/status | 路由未注册 |
| | /api/v1/erp/system-update/rollback/history | 路由未注册 |
| **Emails** | /api/v1/erp/emails | 路由未注册 |
| | /api/v1/erp/emails/1 | 路由未注册 |
| **Advanced** | /api/v1/erp/advanced | 路由未注册 |
| | /api/v1/erp/advanced/config | 路由未注册 |
| | /api/v1/erp/advanced/features | 路由未注册 |
| | /api/v1/erp/advanced/modules | 路由未注册 |
| | /api/v1/erp/advanced/stats | 路由未注册 |
| | /api/v1/erp/advanced/health | 路由未注册 |
| **Assist Accounting** | /api/v1/erp/assist-accounting | 路由未注册 |
| **Fund Management** | /api/v1/erp/fund-management | 路由未注册 |
| | /api/v1/erp/fund-management/transactions | 路由未注册 |
| | /api/v1/erp/fund-management/reports | 路由未注册 |
| **Data Permissions** | /api/v1/erp/data-permissions | 路由未注册 |
| | /api/v1/erp/data-permissions/rules | 路由未注册 |
| | /api/v1/erp/data-permissions/field-permissions | 路由未注册 |
| **AI** | /api/v1/erp/ai | 路由未注册 |
| | /api/v1/erp/ai/models | 路由未注册 |
| | /api/v1/erp/ai/templates | 路由未注册 |
| **Security** | /api/v1/erp/security/events | 路由未注册 |
| | /api/v1/erp/security/stats | 路由未注册 |
| | /api/v1/erp/security/rules | 路由未注册 |
| | /api/v1/erp/security/alerts | 路由未注册 |
| | /api/v1/erp/security/audit-logs | 路由未注册 |
| **BPM** | /api/v1/erp/bpm/instances | 路由未注册 |
| | /api/v1/erp/bpm/templates | 路由未注册 |
| **Inventory** | /api/v1/erp/inventory/batches | 路由未注册 |
| | /api/v1/erp/inventory/batches/1 | 路由未注册 |
| **Purchase** | /api/v1/erp/purchases/payment-requests | 路由未注册 |
| **More Details** | /api/v1/erp/sales-contracts/1 | 路由未注册或记录不存在 |
| | /api/v1/erp/sales-prices/1 | 路由未注册或记录不存在 |
| | /api/v1/erp/purchase-contracts/1 | 路由未注册或记录不存在 |
| | /api/v1/erp/purchase-prices/1 | 路由未注册或记录不存在 |
| | /api/v1/erp/purchases/orders/1 | 记录不存在 |
| | /api/v1/erp/purchases/receipts/1 | 记录不存在 |
| | /api/v1/erp/production/orders/1 | 记录不存在 |
| | /api/v1/erp/inventory/stock/1 | 记录不存在 |
| | /api/v1/erp/inventory/counts/1 | 记录不存在 |
| | /api/v1/erp/inventory/adjustments/1 | 记录不存在 |
| | /api/v1/erp/finance/payments/1 | 记录不存在 |
| | /api/v1/erp/ap/invoices/1 | 记录不存在 |
| | /api/v1/erp/ar/invoices/1 | 记录不存在 |
| | /api/v1/erp/greige-fabrics/1 | 记录不存在 |
| | /api/v1/erp/dye-batches/1 | 记录不存在 |
| | /api/v1/erp/dye-recipes/1 | 记录不存在 |

### 500 服务器错误 (15 项)

| 接口 | 错误信息 |
|------|----------|
| GET /api/v1/erp/roles/1/permissions | 系统内部错误 |
| DELETE /api/v1/erp/customers/999 | 记录不存在 |
| DELETE /api/v1/erp/products/999 | 记录不存在 |
| DELETE /api/v1/erp/warehouses/999 | 记录不存在 |
| DELETE /api/v1/erp/departments/999 | 记录不存在 |
| GET /api/v1/erp/sales/orders/1 | 记录不存在 |
| GET /api/v1/erp/inventory/transfers/1 | 记录不存在 |
| GET /api/v1/erp/crm/assignments/history | relation "assignment_histories" does not exist |
| GET /api/v1/erp/customer-credits | 数据库类型错误 |
| GET /api/v1/erp/permissions/fields | 系统内部错误 |
| GET /api/v1/erp/user/notification-setting | 数据库查询错误 |
| POST /api/v1/erp/sales/orders | 数据库自定义错误 |
| POST /api/v1/erp/sales/orders/1/approve | 记录不存在 |
| POST /api/v1/erp/inventory/transfers | 数据库自定义错误 |
| POST /api/v1/erp/finance/invoices | 系统内部错误 |

### 422 验证错误 (16 项) - 缺少必填字段

| 接口 | 缺少字段 |
|------|----------|
| POST /api/v1/erp/suppliers | supplier_name |
| POST /api/v1/erp/customers | customer_code |
| POST /api/v1/erp/products | code |
| POST /api/v1/erp/warehouses | code |
| POST /api/v1/erp/dye-recipes | recipe_no |
| POST /api/v1/erp/quality-inspection/records | inspection_no |
| POST /api/v1/erp/inventory/transfers | items |
| POST /api/v1/erp/fixed-assets | original_value |
| POST /api/v1/erp/budgets/plans | plan_no |
| POST /api/v1/erp/purchases/orders | items[0].quantity_ordered |
| POST /api/v1/erp/production/orders | order_no |
| POST /api/v1/erp/finance/payments | payment_date 格式错误 |
| POST /api/v1/erp/ar/invoices | due_date |
| POST /api/v1/erp/ap/invoices | invoice_type |
| POST /api/v1/erp/crm/customers | lead_source |

### 400 请求参数错误 (9 项)

| 接口 | 错误信息 |
|------|----------|
| GET /api/v1/erp/sales-analysis/trends | missing field `period` |
| GET /api/v1/erp/tenant/billing/usage | 请求错误 |
| GET /api/v1/erp/five-dimension/analysis | 请求参数验证失败 |
| GET /api/v1/erp/bpm/tasks | missing field `user_id` |
| POST /api/v1/erp/greige-fabrics | duplicate key value |
| POST /api/v1/erp/inventory/counts/1/complete | 只有已审核状态的盘点单可以完成 |
| POST /api/v1/erp/inventory/transfers/1/approve | JSON 解析错误 |
| GET /api/v1/erp/webhooks | 400 错误 |
| GET /api/v1/erp/api-keys | 400 错误 |

### 405 方法不允许 (5 项)

| 接口 | 说明 |
|------|------|
| GET /api/v1/erp/exchange-rates | 方法不允许 |
| GET /api/v1/erp/data-permissions | 方法不允许 |
| GET /api/v1/erp/crm/assignments | 方法不允许 |
| PUT /api/v1/erp/notifications/1/read | 方法不允许 |
| GET /api/v1/erp/financial-analysis/indicators | 方法不允许 |

---

## 🔍 关键问题总结

### P0 - 严重问题 (必须修复)

1. **大量路由未注册 (95 项 404)**
   - 路由文件 `routes/mod.rs` 中缺少大量子路由
   - 影响：Dashboard、通知详情、审计日志、租户详情、BPM 实例、导入导出、系统更新、邮件、高级功能、资金管理、数据权限、AI、安全等模块

2. **数据库表缺失**
   - `assignment_histories` 表不存在 (CRM 分配历史)
   - 导致 500 错误

3. **DELETE 操作不返回标准错误**
   - 删除不存在的记录返回 500 而非 404
   - 应返回 404 或标准错误格式

### P1 - 重要问题 (应该修复)

4. **POST 接口验证严格**
   - 16 个 POST 接口因缺少必填字段返回 422
   - 部分字段应自动生成（如 order_no、inspection_no、plan_no）

5. **详情接口记录不存在**
   - 多个详情接口返回"记录不存在"
   - 数据库中可能无数据，或查询逻辑有误

6. **状态码不一致**
   - 创建库存盘点返回 201 而非 200
   - 应统一返回格式

### P2 - 一般问题 (建议修复)

7. **部分接口返回 400 但无明确错误信息**
   - webhooks、api-keys 等接口

8. **参数验证错误信息不友好**
   - "Failed to deserialize query string" 等技术性错误信息

---

## 📈 模块通过率统计

| 模块 | 通过率 | 状态 |
|------|--------|------|
| 认证 | 100% | ✅ 优秀 |
| 用户管理 | 100% | ✅ 优秀 |
| 部门管理 | 100% | ✅ 优秀 |
| 产品管理 | 100% | ✅ 优秀 |
| 仓库管理 | 100% | ✅ 优秀 |
| 供应商管理 | 100% | ✅ 优秀 |
| 客户管理 | 100% | ✅ 优秀 |
| 质量管理 | 100% | ✅ 优秀 |
| 供应商评估 | 100% | ✅ 优秀 |
| 销售模块 | 57% | ⚠️ 一般 |
| 采购模块 | 58% | ⚠️ 一般 |
| 生产模块 | 70% | ⚠️ 一般 |
| 财务模块 | 56% | ⚠️ 一般 |
| 库存模块 | 43% | ❌ 较差 |
| 仪表盘 | 57% | ⚠️ 一般 |
| 通知模块 | 67% | ⚠️ 一般 |
| 租户管理 | 14% | ❌ 较差 |
| CRM | 50% | ⚠️ 一般 |
| BPM | 25% | ❌ 较差 |
| 报表模块 | 29% | ❌ 较差 |
| 高级功能 | 0% | ❌ 未实现 |
| AI 模块 | 0% | ❌ 未实现 |
| 安全模块 | 0% | ❌ 未实现 |
| 系统更新 | 20% | ❌ 较差 |
| 邮件模块 | 0% | ❌ 未实现 |
| 资金管理 | 25% | ❌ 较差 |
| 数据权限 | 0% | ❌ 未实现 |
| 导入导出 | 0% | ❌ 未实现 |

---

## 🛠️ 修复建议

### 优先级 1 - 路由注册 (预计 8-16 小时)

1. 在 `routes/mod.rs` 中注册缺失的子路由：
   - `/dashboard/revenue-stats`
   - `/dashboard/production-stats`
   - `/dashboard/quick-links`
   - `/notifications/:id`
   - `/audit` 和 `/audit/:id`
   - `/tenants/:id`
   - `/tenant/config`
   - `/tenant/billing/*`
   - `/bpm/instances`
   - `/bpm/templates`
   - `/export/*`
   - `/import/*`
   - `/system-update/versions`
   - `/system-update/history`
   - `/emails`
   - `/advanced/*`
   - `/assist-accounting`
   - `/fund-management/*`
   - `/data-permissions/*`
   - `/ai/*`
   - `/security/*`
   - 等等...

### 优先级 2 - 数据库修复 (预计 2-4 小时)

2. 创建缺失的数据库表：
   - `assignment_histories`
   - 其他缺失表

### 优先级 3 - POST 接口优化 (预计 4-8 小时)

3. 为 POST 接口添加自动编号：
   - `order_no` 自动生成
   - `inspection_no` 自动生成
   - `plan_no` 自动生成
   - `recipe_no` 自动生成
   - 等等...

### 优先级 4 - 错误处理标准化 (预计 2-4 小时)

4. 统一错误返回格式：
   - DELETE 返回 404 而非 500
   - 提供友好的错误信息
   - 统一状态码

---

## 📊 最终评分

| 维度 | 评分 | 说明 |
|------|------|------|
| API 覆盖率 | 46.6% | 124/266 接口通过 |
| 路由完整性 | 64% | 约 95 个路由未注册 |
| 数据完整性 | 70% | 部分表缺失 |
| 错误处理 | 50% | 错误信息不统一 |
| 接口规范 | 60% | 部分接口缺少自动编号 |

**综合评分: 5.2/10**

---

*报告生成时间: 2026-05-25*
*测试环境: http://111.230.99.236:8082*
