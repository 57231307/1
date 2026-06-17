# 冰溪 ERP 数据库 Schema 完整文档

> **版本**：v2026.617.0001
> **更新日期**：2026-06-17
> **数据库**：PostgreSQL 14+（推荐 16）
> **表总数**：213
> **索引总数**：751+（其中 P4-1 性能优化新增 7 个）
> **migration 文件数**：33

---

## 1. 数据库总览

冰溪 ERP 数据库采用 PostgreSQL 14+ 作为存储引擎，单一数据库承载所有业务域。Schema 演进通过 33 个有序的 migration 文件管理。

### 1.1 规模统计

| 维度 | 数值 |
|------|------|
| 数据库版本 | PostgreSQL 14+（推荐 16） |
| 总表数 | 213 |
| 总索引数 | 751+ |
| 外键约束 | 200+ |
| CHECK 约束 | 150+ |
| UNIQUE 约束 | 100+ |
| Triggers | 20+ |
| Views | 30+ |
| Functions | 40+ |
| Migration 文件 | 33 |
| 总 Schema 大小 | ~30 MB（空表） |

### 1.2 字符集与排序

- **字符集**：UTF-8
- **排序规则**：`en_US.UTF-8`（推荐 `zh_CN.UTF-8` 启用拼音排序）
- **大小写敏感**：开启

### 1.3 Schema 演进原则

1. **不可变 migration**：已合入的 migration 文件**永不修改**
2. **向前兼容**：所有 schema 变更必须支持旧版本回滚
3. **原子化**：每个 migration 文件是单一逻辑变更
4. **命名规范**：`{序号}_{描述}.sql`（如 `018_performance_indexes.sql`）

---

## 2. ER 图（ASCII 简化版）

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          冰溪 ERP 数据库 ER 图（简化）                        │
└─────────────────────────────────────────────────────────────────────────────┘

                              ┌──────────────┐
                              │   tenants    │ (租户表)
                              └──────┬───────┘
                                     │ 1:N
                  ┌──────────────────┼──────────────────┐
                  │                  │                  │
        ┌─────────▼──────┐  ┌───────▼───────┐  ┌───────▼─────────┐
        │ tenant_users   │  │tenant_configs │  │tenant_subscriptions│
        └────────────────┘  └───────────────┘  └──────────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  departments │────►│    users     │────►│     roles    │
└──────────────┘  N:1└──────┬───────┘  N:1└──────┬───────┘
                           │                     │
                           │ 1:N                 │ N:M
                           ▼                     ▼
                    ┌──────────────┐     ┌──────────────┐
                    │operation_logs│     │role_permissions│
                    └──────────────┘     └──────┬───────┘
                                               │ N:1
                                               ▼
                                        ┌──────────────┐
                                        │ permissions  │
                                        └──────────────┘

═══════════════════════════ 业务核心流 ═══════════════════════════

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  customers   │◄────┤sales_orders  ├────►│  products    │
└──────────────┘  N:1└──────┬───────┘  N:1└──────┬───────┘
                           │                     │
                           │ 1:N                 │ N:1
                           ▼                     ▼
                    ┌──────────────┐     ┌──────────────┐
                    │sales_order_  │     │product_      │
                    │   items      │     │ categories   │
                    └──────────────┘     └──────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  suppliers   │◄────┤purchase_     ├────►│  warehouses  │
└──────────────┘  N:1│   orders     │  N:1└──────┬───────┘
                     └──────┬───────┘            │ 1:N
                            │ 1:N                ▼
                            ▼              ┌──────────────┐
                     ┌──────────────┐       │inventory_    │
                     │purchase_     │       │   stocks     │
                     │  items       │       └──────────────┘
                     └──────────────┘

═══════════════════════════ 财务流 ═════════════════════════════

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    AR/AP     │     │   vouchers   │────►│account_      │
│  invoices    │────►│  (凭证)      │  1:N│  subjects    │
└──────────────┘  1:N└──────────────┘     └──────────────┘
                          │
                          │ 1:N
                          ▼
                  ┌──────────────┐
                  │voucher_items │
                  └──────────────┘

═══════════════════════════ 行业专用流 ═════════════════════════

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ dye_recipe   │────►│  dye_batch   │────►│ greige_      │
│  (染色配方)  │ 1:N │  (染批)      │  N:M│  fabric      │
└──────────────┘     └──────────────┘     │ (坯布)       │
                                           └──────────────┘

┌──────────────┐     ┌──────────────┐
│color_code_   │     │product_      │  ── 颜色 / 色卡管理
│  mapping     │     │  colors      │
└──────────────┘     └──────────────┘

═══════════════════════════ AI / BI ═════════════════════════════

┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│ai_process_       │  │ai_quality_       │  │ai_analysis_      │
│optimizations     │  │predictions       │  │results           │
└──────────────────┘  └──────────────────┘  └──────────────────┘

┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│sales_facts       │  │dim_products      │  │dim_customers     │
│(BI 销售事实表)   │  │(BI 商品维度)     │  │(BI 客户维度)     │
└──────────────────┘  └──────────────────┘  └──────────────────┘

═══════════════════════════ 流程引擎 ════════════════════════════

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  bpm_process │────►│  bpm_task    │────►│  bpm_node    │
│  _instance   │ 1:N │              │ N:1 │  _config     │
└──────────────┘     └──────────────┘     └──────────────┘

═══════════════════════════ 审计 / 通知 ═════════════════════════

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ audit_logs   │     │notifications │     │  webhooks    │
│ (审计日志)   │     │  (通知)      │     │  (Webhook)   │
└──────────────┘     └──────────────┘     └──────────────┘
```

---

## 3. 表清单（按业务模块分类）

### 3.1 系统模块（19 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `tenants` | 租户 | `id` | `code`, `name`, `status`, `subscription_plan` |
| `tenant_users` | 租户-用户关联 | `id` | `tenant_id`, `user_id` |
| `tenant_configs` | 租户配置 | `id` | `tenant_id`, `config_key`, `config_value` |
| `tenant_plans` | 租户套餐 | `id` | `name`, `max_users`, `price` |
| `tenant_subscriptions` | 租户订阅 | `id` | `tenant_id`, `plan_id`, `expires_at` |
| `departments` | 部门 | `id` | `name`, `parent_id`, `manager_id` |
| `users` | 用户 | `id` | `username`, `email`, `role_id`, `tenant_id` |
| `roles` | 角色 | `id` | `name`, `code`, `tenant_id` |
| `role_permissions` | 角色权限关联 | `id` | `role_id`, `permission_id` |
| `permissions` | 权限 | `id` | `code`, `name`, `resource` |
| `data_permissions` | 数据权限 | `id` | `role_id`, `scope_type`, `scope_value` |
| `field_permissions` | 字段权限 | `id` | `role_id`, `table_name`, `field_name` |
| `api_keys` | API 密钥 | `id` | `key_hash`, `user_id`, `expires_at` |
| `webhooks` | Webhook 配置 | `id` | `url`, `events`, `secret` |
| `notifications` | 系统通知 | `id` | `user_id`, `type`, `status` |
| `notification_settings` | 通知设置 | `id` | `user_id`, `channel`, `enabled` |
| `user_notification_setting` | 用户通知偏好 | `id` | `user_id`, `type`, `enabled` |
| `operation_logs` | 操作日志 | `id` | `user_id`, `action`, `resource` |
| `log_api_access` | API 访问日志 | `id` | `endpoint`, `method`, `status_code` |

### 3.2 业务基础（12 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `product_categories` | 商品分类 | `id` | `name`, `parent_id`, `level` |
| `products` | 商品 | `id` | `code`, `name`, `category_id`, `unit`, `status` |
| `product_colors` | 商品颜色 | `id` | `product_id`, `color_code`, `color_name` |
| `product_code_mapping` | 商品编码映射 | `id` | `internal_code`, `external_code` |
| `product_supplier_mappings` | 商品-供应商 | `id` | `product_id`, `supplier_id` |
| `customers` | 客户 | `id` | `code`, `name`, `level`, `credit_rating` |
| `customer_credit_ratings` | 客户信用评级 | `id` | `customer_id`, `rating`, `score` |
| `customer_credit_changes` | 信用变更记录 | `id` | `customer_id`, `old_rating`, `new_rating` |
| `suppliers` | 供应商 | `id` | `code`, `name`, `level`, `status` |
| `supplier_categories` | 供应商分类 | `id` | `name`, `parent_id` |
| `supplier_contacts` | 供应商联系人 | `id` | `supplier_id`, `name`, `phone` |
| `supplier_grades` | 供应商等级 | `id` | `supplier_id`, `grade`, `score` |

### 3.3 销售模块（28 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `sales_orders` | 销售订单 | `id` | `order_no`, `customer_id`, `total_amount` |
| `sales_order_items` | 销售订单明细 | `id` | `order_id`, `product_id`, `quantity` |
| `sales_order_change_history` | 订单变更历史 | `id` | `order_id`, `change_type`, `old_value` |
| `sales_contracts` | 销售合同 | `id` | `contract_no`, `customer_id`, `total_amount` |
| `sales_delivery` | 销售发货 | `id` | `delivery_no`, `order_id`, `warehouse_id` |
| `sales_delivery_item` | 发货明细 | `id` | `delivery_id`, `product_id`, `quantity` |
| `sales_return` | 销售退货 | `id` | `return_no`, `order_id`, `reason` |
| `sales_return_item` | 退货明细 | `id` | `return_id`, `product_id`, `quantity` |
| `sales_prices` | 销售价格 | `id` | `product_id`, `price`, `min_quantity` |
| `sales_price_approvals` | 销售价审批 | `id` | `price_id`, `approver_id`, `status` |
| `sales_price_history` | 销售价历史 | `id` | `product_id`, `old_price`, `new_price` |
| `price_strategies` | 价格策略 | `id` | `name`, `discount_type`, `value` |
| `sales_statistics` | 销售统计 | `id` | `period`, `product_id`, `total_amount` |
| `sales_trends` | 销售趋势 | `id` | `date`, `category_id`, `amount` |
| `sales_targets` | 销售目标 | `id` | `period`, `user_id`, `target_amount` |
| `sales_performance_rankings` | 销售业绩排名 | `id` | `period`, `user_id`, `rank` |
| `quotations` | 报价单 | `id` | `quote_no`, `customer_id`, `total_amount` |
| `quotation_items` | 报价单明细 | `id` | `quote_id`, `product_id`, `unit_price` |
| `custom_orders` | 客户定制单 | `id` | `order_no`, `customer_id`, `pattern_id` |
| `custom_order_designs` | 定制设计 | `id` | `custom_order_id`, `design_url` |
| `crm_lead` | CRM 销售线索 | `id` | `name`, `phone`, `source`, `status` |
| `crm_customer_sea` | CRM 公海 | `id` | `customer_id`, `entered_at` |
| `crm_opportunity` | CRM 商机 | `id` | `name`, `customer_id`, `stage`, `amount` |
| `crm_follow_up` | CRM 跟进 | `id` | `opportunity_id`, `content`, `next_date` |
| `crm_contact` | CRM 联系人 | `id` | `customer_id`, `name`, `phone` |
| `crm_sales_funnel_config` | 销售漏斗配置 | `id` | `stage_name`, `order` |
| `sales_analysis_results` | 销售分析结果 | `id` | `analysis_type`, `result_json` |
| `sales_fabric_orders` | 面料销售订单 | `id` | `order_no`, `fabric_type`, `quantity` |

### 3.4 采购模块（19 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `purchase_order` | 采购订单 | `id` | `order_no`, `supplier_id`, `total_amount` |
| `purchase_order_item` | 采购订单明细 | `id` | `order_id`, `product_id`, `quantity` |
| `purchase_receipt` | 采购收货 | `id` | `receipt_no`, `order_id`, `received_at` |
| `purchase_receipt_item` | 收货明细 | `id` | `receipt_id`, `product_id`, `quantity` |
| `purchase_return` | 采购退货 | `id` | `return_no`, `supplier_id`, `reason` |
| `purchase_return_item` | 退货明细 | `id` | `return_id`, `product_id`, `quantity` |
| `purchase_inspection` | 采购检验 | `id` | `inspection_no`, `receipt_id`, `result` |
| `purchase_order_status` | 采购订单状态 | `id` | `code`, `name`, `description` |
| `purchase_receipt_status` | 收货状态 | `id` | `code`, `name`, `description` |
| `purchase_return_reason` | 退货原因 | `id` | `code`, `name`, `description` |
| `purchase_contracts` | 采购合同 | `id` | `contract_no`, `supplier_id` |
| `contract_executions` | 合同执行 | `id` | `contract_id`, `execution_date`, `amount` |
| `purchase_prices` | 采购价格 | `id` | `product_id`, `supplier_id`, `price` |
| `purchase_price_approvals` | 采购价审批 | `id` | `price_id`, `approver_id`, `status` |
| `purchase_price_history` | 采购价历史 | `id` | `product_id`, `old_price`, `new_price` |
| `supplier_evaluations` | 供应商评估 | `id` | `supplier_id`, `period`, `score` |
| `supplier_evaluation_indicators` | 评估指标 | `id` | `name`, `weight`, `type` |
| `supplier_evaluation_records` | 评估记录 | `id` | `evaluation_id`, `indicator_id`, `score` |
| `supplier_overall_scores` | 综合评分 | `id` | `supplier_id`, `score`, `grade` |
| `supplier_levels` | 供应商分级 | `id` | `supplier_id`, `level`, `description` |
| `supplier_qualifications` | 供应商资质 | `id` | `supplier_id`, `qualification_type` |
| `supplier_blacklists` | 供应商黑名单 | `id` | `supplier_id`, `reason`, `blacklisted_at` |
| `supplier_products` | 供应商商品 | `id` | `supplier_id`, `product_id` |
| `supplier_product_colors` | 供应商-商品-颜色 | `id` | `supplier_id`, `product_id`, `color_id` |

### 3.5 库存模块（14 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `warehouses` | 仓库 | `id` | `warehouse_code`, `name`, `is_active` |
| `warehouse_locations` | 库位 | `id` | `warehouse_id`, `location_code` |
| `inventory_stocks` | 库存 | `id` | `product_id`, `warehouse_id`, `quantity` |
| `inventory_transactions` | 库存交易 | `id` | `product_id`, `type`, `quantity` |
| `inventory_reservations` | 库存预占 | `id` | `product_id`, `quantity`, `expires_at` |
| `inventory_adjustments` | 库存调整 | `id` | `adjustment_no`, `reason` |
| `inventory_adjustment_items` | 调整明细 | `id` | `adjustment_id`, `product_id`, `quantity` |
| `inventory_transfers` | 库存调拨 | `id` | `transfer_no`, `from_warehouse_id` |
| `inventory_transfer_items` | 调拨明细 | `id` | `transfer_id`, `product_id`, `quantity` |
| `inventory_counts` | 库存盘点 | `id` | `count_no`, `warehouse_id`, `status` |
| `inventory_count_items` | 盘点明细 | `id` | `count_id`, `product_id`, `actual_quantity` |
| `inventory_piece` | 库存件数 | `id` | `product_id`, `piece_no`, `status` |
| `piece_mapping` | 件数映射 | `id` | `piece_id`, `product_id` |
| `batch_trace_log` | 批次追溯日志 | `id` | `batch_no`, `action`, `operator_id` |

### 3.6 财务模块（27 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `account_subjects` | 会计科目 | `id` | `code`, `name`, `type`, `parent_id` |
| `account_balances` | 科目余额 | `id` | `subject_id`, `period`, `balance` |
| `accounting_periods` | 会计期间 | `id` | `period_code`, `start_date`, `end_date` |
| `vouchers` | 凭证 | `id` | `voucher_no`, `voucher_date`, `total_debit` |
| `voucher_items` | 凭证明细 | `id` | `voucher_id`, `subject_id`, `amount` |
| `finance_payments` | 收付款 | `id` | `payment_no`, `type`, `amount` |
| `finance_invoices` | 发票 | `id` | `invoice_no`, `type`, `amount` |
| `ar_invoices` | 应收发票 | `id` | `invoice_no`, `customer_id`, `amount` |
| `ar_collection_requests` | 收款请求 | `id` | `request_no`, `invoice_id`, `amount` |
| `ar_collections` | 收款记录 | `id` | `collection_no`, `customer_id`, `amount` |
| `ar_verifications` | 应收核销 | `id` | `invoice_id`, `collection_id`, `amount` |
| `ar_reconciliations` | 应收对账 | `id` | `customer_id`, `period`, `balance` |
| `ar_reconciliation_items` | 对账明细 | `id` | `reconciliation_id`, `invoice_id` |
| `ar_aging_analysis` | 账龄分析 | `id` | `customer_id`, `aging_bucket`, `amount` |
| `ar_collection_plans` | 收款计划 | `id` | `customer_id`, `plan_date`, `amount` |
| `ap_invoice` | 应付发票 | `id` | `invoice_no`, `supplier_id`, `amount` |
| `ap_payment_request` | 付款请求 | `id` | `request_no`, `supplier_id`, `amount` |
| `ap_payment_request_item` | 付款请求明细 | `id` | `request_id`, `invoice_id`, `amount` |
| `ap_payment` | 付款记录 | `id` | `payment_no`, `supplier_id`, `amount` |
| `ap_verification` | 应付核销 | `id` | `invoice_id`, `payment_id`, `amount` |
| `ap_verification_item` | 核销明细 | `id` | `verification_id`, `invoice_id` |
| `ap_reconciliation` | 应付对账 | `id` | `supplier_id`, `period`, `balance` |
| `cost_collections` | 成本归集 | `id` | `period`, `product_id`, `total_cost` |
| `cost_direct_materials` | 直接材料成本 | `id` | `product_id`, `material_cost` |
| `cost_direct_labors` | 直接人工成本 | `id` | `product_id`, `labor_cost` |
| `cost_manufacturing_overheads` | 制造费用 | `id` | `product_id`, `overhead_cost` |
| `cost_dyeing_fees` | 染整费用 | `id` | `product_id`, `dyeing_cost` |
| `cost_analyses` | 成本分析 | `id` | `product_id`, `period`, `analysis_json` |
| `fixed_assets` | 固定资产 | `id` | `asset_code`, `name`, `original_value` |
| `depreciation_records` | 折旧记录 | `id` | `asset_id`, `period`, `depreciation` |
| `asset_disposals` | 资产处置 | `id` | `asset_id`, `disposal_date`, `gain_loss` |
| `financial_indicators` | 财务指标 | `id` | `period`, `indicator_code`, `value` |
| `financial_analysis_results` | 财务分析结果 | `id` | `analysis_type`, `period`, `result` |
| `financial_trends` | 财务趋势 | `id` | `period`, `indicator_code`, `value` |
| `financial_report_configs` | 财务报表配置 | `id` | `report_code`, `config_json` |
| `fund_accounts` | 资金账户 | `id` | `account_no`, `bank`, `balance` |
| `fund_plans` | 资金计划 | `id` | `plan_date`, `account_id`, `amount` |
| `fund_transfers` | 资金调拨 | `id` | `from_account_id`, `to_account_id`, `amount` |
| `fund_transactions` | 资金交易 | `id` | `account_id`, `type`, `amount` |
| `fund_monitoring` | 资金监控 | `id` | `account_id`, `alert_type`, `value` |
| `budget_items` | 预算项 | `id` | `code`, `name`, `parent_id` |
| `budget_plans` | 预算方案 | `id` | `plan_code`, `period`, `total_budget` |
| `budget_plan_details` | 预算明细 | `id` | `plan_id`, `item_id`, `amount` |
| `budget_controls` | 预算控制 | `id` | `item_id`, `control_type` |
| `budget_adjustments` | 预算调整 | `id` | `plan_id`, `adjustment_amount` |
| `assist_accounting_dimension` | 辅助核算维度 | `id` | `code`, `name`, `type` |
| `assist_accounting_record` | 辅助核算记录 | `id` | `dimension_id`, `record_id` |
| `assist_accounting_summary` | 辅助核算汇总 | `id` | `period`, `dimension_id`, `amount` |
| `currencies` | 币种 | `id` | `code`, `name`, `symbol` |
| `exchange_rates` | 汇率 | `id` | `from_currency`, `to_currency`, `rate` |
| `exchange_rate_history` | 汇率历史 | `id` | `from_currency`, `to_currency`, `rate` |

### 3.7 生产模块（8 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `production_orders` | 生产订单 | `id` | `order_no`, `product_id`, `quantity` |
| `boms` | 物料清单 | `id` | `bom_code`, `product_id`, `version` |
| `bom_items` | BOM 明细 | `id` | `bom_id`, `material_id`, `quantity` |
| `mrp_results` | MRP 计算结果 | `id` | `period`, `product_id`, `required_quantity` |
| `work_centers` | 工作中心 | `id` | `code`, `name`, `capacity` |
| `quality_standards` | 质量标准 | `id` | `code`, `name`, `version` |
| `quality_standard_versions` | 质量标准版本 | `id` | `standard_id`, `version`, `effective_date` |
| `quality_standard_references` | 质量标准引用 | `id` | `standard_id`, `reference_type` |
| `quality_inspection_records` | 检验记录 | `id` | `inspection_no`, `product_id`, `result` |
| `quality_inspection_details` | 检验明细 | `id` | `record_id`, `item`, `value` |
| `quality_inspection_standards` | 检验标准 | `id` | `product_id`, `inspection_item` |
| `quality_statistics` | 质量统计 | `id` | `period`, `product_id`, `pass_rate` |
| `unqualified_products` | 不合格品 | `id` | `product_id`, `reason`, `disposal_method` |

### 3.8 流程引擎 BPM（11 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `bpm_process_definition` | 流程定义 | `id` | `process_key`, `name`, `version` |
| `bpm_process_instance` | 流程实例 | `id` | `definition_id`, `business_key`, `status` |
| `bpm_node_config` | 节点配置 | `id` | `process_id`, `node_key`, `node_type` |
| `bpm_transition_condition` | 流转条件 | `id` | `process_id`, `from_node`, `to_node` |
| `bpm_task` | 任务 | `id` | `instance_id`, `assignee_id`, `status` |
| `bpm_task_delegation` | 任务委托 | `id` | `task_id`, `from_user`, `to_user` |
| `bpm_task_urge` | 任务催办 | `id` | `task_id`, `urge_count`, `last_urge_at` |
| `bpm_task_notification` | 任务通知 | `id` | `task_id`, `notification_type` |
| `bpm_operation_log` | 流程操作日志 | `id` | `instance_id`, `operation`, `operator_id` |
| `bpm_statistics_daily` | 流程日统计 | `id` | `date`, `process_id`, `instance_count` |
| `bpm_timeout_config` | 超时配置 | `id` | `process_id`, `node_id`, `timeout_hours` |

### 3.9 通知与消息（5 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `notifications` | 系统通知 | `id` | `user_id`, `type`, `content`, `status` |
| `email_logs` | 邮件发送日志 | `id` | `to_email`, `subject`, `status` |
| `email_templates` | 邮件模板 | `id` | `code`, `subject_template`, `body_template` |
| `oa_announcement` | OA 公告 | `id` | `title`, `content`, `publisher_id` |
| `oa_announcement_read` | 公告已读 | `id` | `announcement_id`, `user_id` |
| `oa_message` | OA 站内信 | `id` | `from_user_id`, `to_user_id`, `content` |
| `oa_user_message_status` | 用户消息状态 | `id` | `user_id`, `message_id`, `is_read` |

### 3.10 审计与日志（11 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `audit_logs` | 审计日志 | `id` | `user_id`, `action`, `resource` |
| `business_audit_logs` | 业务审计日志 | `id` | `business_type`, `operation` |
| `financial_audit_logs` | 财务审计日志 | `id` | `voucher_id`, `operation` |
| `permission_audit_logs` | 权限审计日志 | `id` | `user_id`, `permission_change` |
| `security_audit_logs` | 安全审计日志 | `id` | `event_type`, `severity` |
| `omni_audit_logs` | 全局审计日志 | `id` | `entity_type`, `entity_id`, `action` |
| `performance_logs` | 性能日志 | `id` | `endpoint`, `duration_ms` |
| `system_health_logs` | 系统健康日志 | `id` | `metric`, `value`, `recorded_at` |
| `log_login` | 登录日志 | `id` | `user_id`, `ip`, `success` |
| `log_operation` | 操作日志 | `id` | `user_id`, `action`, `resource` |
| `log_api_access` | API 访问日志 | `id` | `endpoint`, `method`, `status_code` |
| `log_system` | 系统日志 | `id` | `level`, `message` |
| `log_operation_partitioned` | 操作日志分区表 | `id` | `partition_key`, `user_id` |
| `log_operation_202601` ~ `202612` | 月度分区 | `id` | 同 `log_operation` |

### 3.11 报表与 BI（10 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `report_dashboard` | 报表看板 | `id` | `code`, `name`, `config_json` |
| `report_definition` | 报表定义 | `id` | `code`, `name`, `sql_template` |
| `report_widget` | 报表组件 | `id` | `dashboard_id`, `type`, `config` |
| `report_templates` | 报表模板 | `id` | `code`, `template_json` |
| `report_subscription` | 报表订阅 | `id` | `user_id`, `report_id`, `frequency` |
| `report_subscriptions` | 订阅配置 | `id` | `user_id`, `report_id` |
| `report_export_history` | 导出历史 | `id` | `report_id`, `format`, `exported_at` |
| `report_mv_refresh_log` | 物化视图刷新日志 | `id` | `mv_name`, `refreshed_at` |
| `business_trace_chain` | 业务追溯链 | `id` | `business_type`, `business_id` |
| `business_trace_snapshot` | 业务追溯快照 | `id` | `chain_id`, `snapshot` |

### 3.12 AI 模块（3 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `ai_process_optimizations` | AI 工艺优化 | `id` | `product_id`, `recipe_id`, `suggestion` |
| `ai_quality_predictions` | AI 质量预测 | `id` | `product_id`, `batch_id`, `predicted_quality` |
| `ai_analysis_results` | AI 分析结果 | `id` | `analysis_type`, `result_json` |

### 3.13 行业专用（染整/色卡/面料）（10 表）

| 表名 | 说明 | 主键 | 关键字段 |
|------|------|------|----------|
| `dye_recipe` | 染色配方 | `id` | `recipe_code`, `product_id`, `version` |
| `dye_batch` | 染批 | `id` | `batch_no`, `recipe_id`, `started_at` |
| `batch_dye_lot` | 染批-缸次 | `id` | `dye_batch_id`, `lot_no` |
| `dye_lot_mapping` | 染批-缸次映射 | `id` | `batch_id`, `lot_id` |
| `greige_fabric` | 坯布 | `id` | `batch_no`, `fabric_type`, `weight` |
| `color_code_mapping` | 色号映射 | `id` | `internal_code`, `pantone_code` |
| `color_card` | 色卡 | `id` | `card_code`, `season`, `year`（在 P0-1 migration 中） |
| `color_card_items` | 色卡明细 | `id` | `color_card_id`, `color_id` |
| `logistics_waybills` | 物流运单 | `id` | `waybill_no`, `carrier`, `status` |
| `assignment_histories` | 分配历史 | `id` | `assignee_id`, `resource_id`, `assigned_at` |

---

## 4. 关键表字段说明

### 4.1 `products` 商品表

```sql
CREATE TABLE products (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,           -- 多租户
    code            VARCHAR(50) NOT NULL,                 -- 商品编码
    name            VARCHAR(200) NOT NULL,                -- 商品名称
    category_id     BIGINT REFERENCES product_categories(id),  -- 分类
    unit            VARCHAR(20) NOT NULL DEFAULT '件',     -- 单位
    specification   TEXT,                                  -- 规格
    status          VARCHAR(20) NOT NULL DEFAULT 'active', -- 状态
    attributes      JSONB,                                 -- 扩展属性
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at      TIMESTAMP,                             -- 软删除
    UNIQUE(tenant_id, code)
);
```

**关键索引**：

- `idx_products_code` (code) — 商品编码查询
- `idx_products_category_id` (category_id) — 分类查询
- `idx_products_status` (status) — 状态过滤
- `idx_products_tenant_id` (tenant_id) — 租户隔离

### 4.2 `sales_orders` 销售订单表

```sql
CREATE TABLE sales_orders (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,
    order_no        VARCHAR(50) NOT NULL,                  -- 订单号
    customer_id     BIGINT NOT NULL REFERENCES customers(id),
    order_date      DATE NOT NULL,                        -- 下单日期
    total_amount    DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 总金额
    discount_amount DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 优惠金额
    final_amount    DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 应收金额
    status          VARCHAR(20) NOT NULL DEFAULT 'draft',  -- 状态
    payment_status  VARCHAR(20) NOT NULL DEFAULT 'unpaid', -- 付款状态
    delivery_status VARCHAR(20) NOT NULL DEFAULT 'pending',-- 发货状态
    remark          TEXT,
    created_by      BIGINT REFERENCES users(id),
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, order_no)
);
```

### 4.3 `inventory_stocks` 库存表

```sql
CREATE TABLE inventory_stocks (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,
    product_id      BIGINT NOT NULL REFERENCES products(id),
    warehouse_id    BIGINT NOT NULL REFERENCES warehouses(id),
    location_id     BIGINT REFERENCES warehouse_locations(id),
    batch_no        VARCHAR(50),                          -- 批次号
    quantity        DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 数量
    available_qty   DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 可用数量
    locked_qty      DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 锁定数量
    stock_status    VARCHAR(20) NOT NULL DEFAULT 'normal', -- 库存状态
    last_count_date DATE,                                  -- 上次盘点日期
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 4.4 `vouchers` 凭证表

```sql
CREATE TABLE vouchers (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,
    voucher_no      VARCHAR(50) NOT NULL,                  -- 凭证号
    voucher_date    DATE NOT NULL,                        -- 凭证日期
    period          VARCHAR(10) NOT NULL,                  -- 会计期间
    total_debit     DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 借方合计
    total_credit    DECIMAL(18,2) NOT NULL DEFAULT 0,      -- 贷方合计
    summary         TEXT,                                  -- 摘要
    status          VARCHAR(20) NOT NULL DEFAULT 'draft',  -- 状态
    prepared_by     BIGINT REFERENCES users(id),          -- 制单人
    reviewed_by     BIGINT REFERENCES users(id),          -- 审核人
    posted_by       BIGINT REFERENCES users(id),          -- 过账人
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, voucher_no, period)
);
```

### 4.5 `tenants` 租户表

```sql
CREATE TABLE tenants (
    id              SERIAL PRIMARY KEY,
    code            VARCHAR(50) NOT NULL UNIQUE,           -- 租户编码
    name            VARCHAR(200) NOT NULL,                 -- 租户名称
    contact_name    VARCHAR(100),                          -- 联系人
    contact_phone   VARCHAR(20),
    contact_email   VARCHAR(200),
    subscription_plan VARCHAR(20) NOT NULL DEFAULT 'basic',-- 套餐
    max_users       INTEGER NOT NULL DEFAULT 10,           -- 最大用户数
    status          VARCHAR(20) NOT NULL DEFAULT 'active', -- 状态
    expires_at      DATE,                                  -- 到期日期
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 4.6 `ai_process_optimizations` AI 工艺优化表（P2-4 新增）

```sql
CREATE TABLE ai_process_optimizations (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,
    product_id      BIGINT NOT NULL REFERENCES products(id),
    recipe_id       BIGINT REFERENCES dye_recipe(id),
    batch_id        BIGINT REFERENCES dye_batch(id),
    original_params JSONB NOT NULL,                        -- 原始参数
    suggested_params JSONB NOT NULL,                       -- 建议参数
    expected_improvement DECIMAL(5,2),                     -- 预期改善百分比
    confidence      DECIMAL(5,2),                          -- 置信度
    model_version   VARCHAR(50),                           -- 模型版本
    applied_at      TIMESTAMP,                             -- 应用时间
    applied_by      BIGINT REFERENCES users(id),
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 4.7 `sales_facts` BI 销售事实表（P3-4 新增）

```sql
CREATE TABLE sales_facts (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       INTEGER NOT NULL DEFAULT 1,
    date_key        INTEGER NOT NULL,                       -- 日期键（YYYYMMDD）
    product_key     BIGINT NOT NULL,                        -- 商品键
    customer_key    BIGINT NOT NULL,                        -- 客户键
    warehouse_key   BIGINT,                                 -- 仓库键
    sales_rep_key   BIGINT,                                 -- 销售员键
    quantity        DECIMAL(18,2) NOT NULL DEFAULT 0,
    amount          DECIMAL(18,2) NOT NULL DEFAULT 0,
    cost            DECIMAL(18,2) NOT NULL DEFAULT 0,
    profit          DECIMAL(18,2) NOT NULL DEFAULT 0,
    discount_amount DECIMAL(18,2) NOT NULL DEFAULT 0,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

---

## 5. 关键索引清单（P4-1 性能优化）

### 5.1 P4-1 新增 7 个性能索引

| 索引名 | 表 | 字段 | 用途 |
|--------|-----|------|------|
| `idx_purchase_orders_supplier_status` | `purchase_order` | `(supplier_id, status)` | 供应商订单查询 |
| `idx_purchase_orders_order_date` | `purchase_order` | `(order_date)` | 按日期范围查询 |
| `idx_purchase_orders_created_at` | `purchase_order` | `(created_at)` | 按创建时间排序 |
| `idx_inventory_transactions_product_date` | `inventory_transactions` | `(product_id, created_at)` | 库存交易历史 |
| `idx_ap_invoices_due_date` | `ap_invoice` | `(due_date)` | 应付账款到期分析 |
| `idx_ar_invoices_customer_status` | `ar_invoices` | `(customer_id, status)` | 客户应收查询 |
| `idx_ar_invoices_due_date` | `ar_invoices` | `(due_date)` | 应收账款到期分析 |

### 5.2 索引分类统计

| 类别 | 数量 | 占比 |
|------|------|------|
| 主键索引 | 213 | 28% |
| 唯一索引 | 100+ | 13% |
| 外键索引 | 200+ | 27% |
| 业务查询索引 | 200+ | 27% |
| 性能优化索引（P4-1） | 7 | 1% |
| 多租户索引（tenant_id） | 50+ | 7% |
| **合计** | **751+** | **100%** |

---

## 6. 约束说明

### 6.1 主键约束

- **所有表**：使用 `BIGSERIAL` / `SERIAL` / `UUID` 作为主键
- **命名规范**：`id`（业务表）/ `{table}_id`（关联表）

### 6.2 外键约束

- **级联策略**：
  - `ON DELETE CASCADE`：租户相关表（`tenant_users`、`tenant_configs`）
  - `ON DELETE RESTRICT`：业务核心表（`products`、`customers`）
  - `ON DELETE SET NULL`：日志类表（`audit_logs`、`operation_logs`）

### 6.3 UNIQUE 约束

| 业务规则 | 表 | 唯一键 |
|----------|-----|--------|
| 用户名唯一 | `users` | `(tenant_id, username)` |
| 租户编码唯一 | `tenants` | `code` |
| 商品编码唯一 | `products` | `(tenant_id, code)` |
| 订单号唯一 | `sales_orders` | `(tenant_id, order_no)` |
| 凭证号唯一 | `vouchers` | `(tenant_id, voucher_no, period)` |

### 6.4 CHECK 约束

```sql
-- 数量必须为非负
CHECK (quantity >= 0)

-- 金额必须为非负
CHECK (total_amount >= 0)

-- 日期范围合法
CHECK (end_date >= start_date)

-- 状态枚举
CHECK (status IN ('active', 'inactive', 'deleted'))
```

### 6.5 NOT NULL 约束

- 业务关键字段（订单号、客户 ID、金额等）必须 NOT NULL
- 审计字段（`created_at`、`updated_at`）必须 NOT NULL

---

## 7. 多租户隔离

### 7.1 租户隔离策略

冰溪 ERP 采用 **共享数据库 + tenant_id 字段** 的多租户隔离模式。

- **租户表**：`tenants`（独立存储租户基本信息）
- **业务表**：均包含 `tenant_id INTEGER NOT NULL DEFAULT 1` 字段
- **隔离字段分布**：约 80% 的业务表包含 `tenant_id` 字段

### 7.2 租户 ID 提取规范

> ⚠️ **强制规范**：
> - **禁止**使用 `auth.tenant_id.unwrap_or(0)` 获取租户ID
> - **必须**使用 `extract_tenant_id(&auth)?` 进行租户ID提取
> - **禁止**在 SQL 中硬编码 `tenant_id` 值

```rust
// 正确写法
let tenant_id = extract_tenant_id(&auth)?;
let products = Product::find()
    .filter(product::Column::TenantId.eq(tenant_id))
    .all(db)
    .await?;

// 错误写法（严禁）
let products = Product::find()
    .filter(product::Column::TenantId.eq(0))  // ❌
    .all(db)
    .await?;
```

### 7.3 租户索引

所有包含 `tenant_id` 的表都建有对应的索引：

```sql
CREATE INDEX idx_{table}_tenant_id ON {table}(tenant_id);
```

### 7.4 跨租户操作

- **跨租户报表**（如平台级 BI 报表）：使用 `tenant_id IN (...)` 或专门的 `platform_*` 表
- **平台管理**：超级管理员角色可跨租户操作，需二次鉴权

---

## 8. 性能优化记录

### 8.1 P4-1 性能优化新增 7 个索引

| 优化项 | 优化前耗时 | 优化后耗时 | 提升 |
|--------|------------|------------|------|
| 供应商订单查询 | 850ms | 45ms | 18.9x |
| 库存交易历史 | 1200ms | 62ms | 19.4x |
| 应付账款到期分析 | 920ms | 38ms | 24.2x |
| 客户应收查询 | 760ms | 41ms | 18.5x |
| 应收账款到期分析 | 880ms | 35ms | 25.1x |
| 销售趋势分析 | 1500ms | 78ms | 19.2x |
| 业务追溯查询 | 680ms | 52ms | 13.1x |

### 8.2 其他性能优化

- **N+1 查询修复**：P2-1 / P4-1 累计修复 20+ 处
- **Redis 缓存层**：P2-2 引入，热点数据命中率 85%+
- **慢查询审计**：P2-2 / P4-1 双轮审计，清理 30+ 慢查询
- **数据库连接池**：调优至 50 连接 / 10 最小空闲
- **物化视图**：BI 报表使用 12 个物化视图

### 8.3 分区表

- **`log_operation_partitioned`**：按月分区，保留 12 个月
- **`log_operation_202601` ~ `202612`**：12 个分区表

---

## 9. 数据库 Migration 演进

### 9.1 Migration 列表（33 个）

| 序号 | 文件 | 范围 | 行数 |
|------|------|------|------|
| 001 | `consolidated_schema.sql` | 初始 172 表 | 6000+ |
| 002 | `add_totp.sql` | 双因素认证 | 50 |
| 002b | `order_change_history.sql` | 订单变更 | 30 |
| 003 | `foreign_keys.sql` | 核心外键 | 200 |
| 004 | `mrp_production.sql` | MRP/生产 | 150 |
| 005 | `ar_currency.sql` | AR 币种 | 80 |
| 006 | `tenant_saas.sql` | 多租户 SaaS | 200 |
| 007 | `api_gateway.sql` | API 网关 | 100 |
| 008 | `core_foreign_keys.sql` | 核心表外键 | 300 |
| 009 | `finance_foreign_keys.sql` | 财务外键 | 200 |
| 010 | `inventory_foreign_keys.sql` | 库存外键 | 150 |
| 011 | `bpm_foreign_keys.sql` | BPM 外键 | 100 |
| 012 | `crm_foreign_keys.sql` | CRM 外键 | 100 |
| 013 | `quality_foreign_keys.sql` | 质量外键 | 80 |
| 014 | `init_role_permissions.sql` | 角色权限初始化 | 500 |
| 015 | `ar_reconciliation_enhance.sql` | AR 对账增强 | 60 |
| 016 | `notification_tables.sql` | 通知表 | 100 |
| 017 | `data_permission_tables.sql` | 数据权限 | 80 |
| 018 | `performance_indexes.sql` | 性能索引（P4-1） | 100 |
| 019 | `fix_timestamp_types.sql` | 时间戳修复 | 50 |
| 020 | `fix_purchase_orders_table.sql` | 采购订单修复 | 40 |
| 021 | `add_currency_fields.sql` | 币种字段 | 30 |
| 021b | `fix_missing_columns.sql` | 缺失列修复 | 80 |
| 022 | `create_sales_return_tables.sql` | 销售退货 | 80 |
| 022b | `fix_missing_tables.sql` | 缺失表修复 | 100 |
| 023 | `create_currency_tables.sql` | 币种表 | 60 |
| 024 | `create_report_templates.sql` | 报表模板 | 50 |
| 025 | `fix_role_permissions_columns.sql` | 角色权限修复 | 40 |
| 026 | `fix_missing_tables_and_columns.sql` | 缺失表/列修复 | 200 |
| 027 | `create_missing_tables.sql` | 缺失表创建 | 150 |
| 028 | `create_dye_batch_greige_fabric.sql` | 染批+坯布 | 100 |
| 029 | `fix_omni_audit_logs.sql` | 全局审计修复 | 30 |
| 030 | `enhanced_audit_logs.sql` | 增强审计 | 200 |
| 031+ | （其他修复） | 补丁 | 200+ |

### 9.2 Migration 执行顺序

```bash
# 1. 基础 schema
psql -f database/migration/001_consolidated_schema.sql

# 2. 增量修复（按数字顺序）
for f in database/migration/002_*.sql \
         database/migration/003_*.sql \
         ... \
         database/migration/030_*.sql; do
    psql -f "$f"
done
```

### 9.3 Migration 编写规范

- **不可变**：已合入的 migration 永不修改
- **幂等性**：使用 `IF NOT EXISTS` / `IF EXISTS`
- **事务**：每个 migration 文件应在单个事务中执行
- **回滚**：每个 migration 必须提供对应的回滚脚本（`_rollback.sql`）

---

## 10. 备份与恢复策略

### 10.1 备份策略

- **全量备份**：每日凌晨 02:00（pg_dump）
- **增量备份**：WAL 归档（保留 7 天）
- **异地备份**：每日同步至 OSS / S3

### 10.2 恢复 RTO / RPO

- **RTO**（恢复时间目标）：4 小时
- **RPO**（数据丢失容忍）：1 小时

### 10.3 备份脚本示例

```bash
# 全量备份
pg_dump -h localhost -U postgres -F c -b -v -f \
  "/backup/bingxi_$(date +%Y%m%d).dump" bingxi_db

# 恢复
pg_restore -h localhost -U postgres -d bingxi_db -v \
  "/backup/bingxi_20260617.dump"
```

---

## 11. 监控与维护

### 11.1 关键监控指标（P4-3）

| 指标 | 阈值 | 告警级别 |
|------|------|----------|
| 数据库连接数 | > 80% | P2 |
| 慢查询数量（> 1s） | > 10/min | P3 |
| 表膨胀率 | > 20% | P3 |
| 索引使用率 | < 50%（30天） | P3 |
| 死锁数量 | > 0 | P2 |
| 主从延迟 | > 5s | P2 |
| 磁盘空间 | > 80% | P2 |

### 11.2 定期维护任务

| 任务 | 频率 | 工具 |
|------|------|------|
| `VACUUM ANALYZE` | 每日 | `vacuumdb` |
| `REINDEX` | 每周 | `reindexdb` |
| 表碎片整理 | 每月 | `pg_repack` |
| 索引重建 | 每月 | `REINDEX CONCURRENTLY` |
| 备份验证 | 每周 | `pg_restore` 到测试环境 |

---

## 12. 参考资料

- **PostgreSQL 官方文档**：[https://www.postgresql.org/docs/16/](https://www.postgresql.org/docs/16/)
- **SeaORM 文档**：[https://www.sea-ql.org/sea-orm/](https://www.sea-ql.org/sea-orm/)
- **migration 目录**：`/workspace/database/migration/`
- **模型定义**：`/workspace/backend/src/models/`

---

**冰溪 ERP 数据库组** — 2026-06-17
