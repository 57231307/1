-- 完整删除租户功能迁移（up）
-- 创建时间: 2026-06-28
-- 关联计划: 租户功能下线
--
-- 本迁移用于完整删除系统中的多租户功能：
-- - 删除所有业务表上的 tenant_id 列及其相关索引
-- - 删除全部租户管理表（tenants / tenant_users / tenant_configs 等）
--
-- 执行顺序：先删索引 → 再删业务表 tenant_id 列 → 最后删租户管理表。
-- 该迁移不可逆。

-- =====================================================================
-- 第一部分：删除所有 tenant_id 相关索引
-- =====================================================================

-- 销售订单：租户 + 客户 + 状态联合索引
DROP INDEX IF EXISTS "idx_sales_orders_tenant_customer_status";
-- 库存：租户 + 仓库 + 产品联合索引
DROP INDEX IF EXISTS "idx_inventory_stocks_tenant_wh_product";
-- 应收发票：租户 + 客户 + 到期联合索引
DROP INDEX IF EXISTS "idx_ar_invoices_tenant_customer_due";
-- 采购订单：租户 + 供应商 + 状态联合索引
DROP INDEX IF EXISTS "idx_purchase_orders_tenant_supplier_status";
-- 库存预留：租户 + 产品 + 状态联合索引
DROP INDEX IF EXISTS "idx_inventory_reservations_tenant_product_status";
-- 操作日志：租户 + 创建时间索引
DROP INDEX IF EXISTS "idx_operation_logs_tenant_created";
-- 用户：租户 + 用户名唯一索引
DROP INDEX IF EXISTS "uq_users_tenant_username";
-- 租户用户：租户索引
DROP INDEX IF EXISTS "idx_tenant_users_tenant";
-- 租户配置：租户索引
DROP INDEX IF EXISTS "idx_tenant_configs_tenant";
-- 租户配置：配置键索引
DROP INDEX IF EXISTS "idx_tenant_configs_key";
-- 租户订阅：租户索引
DROP INDEX IF EXISTS "idx_tenant_subscriptions_tenant";
-- 租户用量：租户索引
DROP INDEX IF EXISTS "idx_tenant_usage_tenant";
-- 租户发票：租户索引
DROP INDEX IF EXISTS "idx_tenant_invoices_tenant";
-- API 密钥：租户索引
DROP INDEX IF EXISTS "idx_api_keys_tenant";
-- Webhooks：租户索引
DROP INDEX IF EXISTS "idx_webhooks_tenant";
-- 邮件日志：租户索引
DROP INDEX IF EXISTS "idx_email_logs_tenant";
-- 全渠道审计日志：租户索引
DROP INDEX IF EXISTS "idx_omni_audit_logs_tenant";
-- 审计日志：租户 + 创建时间索引
DROP INDEX IF EXISTS "idx_audit_log_tenant_created";
-- 定制订单：租户索引
DROP INDEX IF EXISTS "idx_custom_orders_tenant";
-- 流程节点：租户索引
DROP INDEX IF EXISTS "idx_process_nodes_tenant";
-- 流程日志：租户索引
DROP INDEX IF EXISTS "idx_process_logs_tenant";
-- 质量问题：租户索引
DROP INDEX IF EXISTS "idx_quality_issues_tenant";
-- 售后：租户索引
DROP INDEX IF EXISTS "idx_aftersales_tenant";
-- 色卡：租户索引
DROP INDEX IF EXISTS "idx_color_cards_tenant";
-- 色卡明细：租户索引
DROP INDEX IF EXISTS "idx_color_items_tenant";
-- 色卡借用：租户索引
DROP INDEX IF EXISTS "idx_borrow_tenant";
-- AI 工艺优化：租户 + 创建时间索引
DROP INDEX IF EXISTS "idx_ai_proc_tenant_created";
-- AI 工艺优化：颜色 + 面料索引
DROP INDEX IF EXISTS "idx_ai_proc_color_fabric";
-- AI 工艺优化：是否已应用索引
DROP INDEX IF EXISTS "idx_ai_proc_applied";
-- AI 工艺优化：来源索引
DROP INDEX IF EXISTS "idx_ai_proc_source";
-- AI 质量预测：租户 + 创建时间索引
DROP INDEX IF EXISTS "idx_ai_qual_tenant_created";
-- AI 质量预测：产品索引
DROP INDEX IF EXISTS "idx_ai_qual_product";
-- AI 质量预测：风险索引
DROP INDEX IF EXISTS "idx_ai_qual_risk";
-- AI 质量预测：确认索引
DROP INDEX IF EXISTS "idx_ai_qual_ack";
-- 销售事实表：租户 + 日期索引
DROP INDEX IF EXISTS "idx_sales_facts_tenant_date";
-- 销售事实表：租户 + 客户索引
DROP INDEX IF EXISTS "idx_sales_facts_tenant_customer";
-- 销售事实表：租户 + 产品索引
DROP INDEX IF EXISTS "idx_sales_facts_tenant_product";
-- 销售事实表：租户 + 区域索引
DROP INDEX IF EXISTS "idx_sales_facts_tenant_region";
-- 产品维度：租户 + 当前版本索引
DROP INDEX IF EXISTS "idx_dim_products_tenant_current";
-- 产品维度：租户 + 历史版本索引
DROP INDEX IF EXISTS "idx_dim_products_tenant_history";
-- 产品维度：租户 + 分类索引
DROP INDEX IF EXISTS "idx_dim_products_tenant_category";
-- 客户维度：租户 + 当前版本索引
DROP INDEX IF EXISTS "idx_dim_customers_tenant_current";
-- 客户维度：租户 + 区域索引
DROP INDEX IF EXISTS "idx_dim_customers_tenant_region";
-- 客户维度：租户 + 类型索引
DROP INDEX IF EXISTS "idx_dim_customers_tenant_type";
-- 颜色价格：租户索引
DROP INDEX IF EXISTS "idx_color_prices_tenant";
-- 价格历史：租户索引
DROP INDEX IF EXISTS "idx_price_history_tenant";
-- 价格梯度：租户索引
DROP INDEX IF EXISTS "idx_price_tiers_tenant";
-- 客户颜色价格：租户索引
DROP INDEX IF EXISTS "idx_cust_color_price_tenant";
-- 季节性价格：租户 + 启用索引
DROP INDEX IF EXISTS "idx_seasonal_tenant_active";
-- 故障转移事件：租户索引
DROP INDEX IF EXISTS "idx_failover_event_tenant";
-- 慢查询日志：租户索引
DROP INDEX IF EXISTS "idx_slow_query_tenant";

-- =====================================================================
-- 第二部分：删除业务表上的 tenant_id 列（保留表，仅删列）
-- =====================================================================

-- 销售订单
ALTER TABLE "sales_orders" DROP COLUMN IF EXISTS "tenant_id";
-- 库存
ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "tenant_id";
-- 应收发票
ALTER TABLE "ar_invoices" DROP COLUMN IF EXISTS "tenant_id";
-- 采购订单
ALTER TABLE "purchase_orders" DROP COLUMN IF EXISTS "tenant_id";
-- 库存预留
ALTER TABLE "inventory_reservations" DROP COLUMN IF EXISTS "tenant_id";
-- 操作日志
ALTER TABLE "operation_logs" DROP COLUMN IF EXISTS "tenant_id";
-- 用户
ALTER TABLE "users" DROP COLUMN IF EXISTS "tenant_id";
-- API 密钥
ALTER TABLE "api_keys" DROP COLUMN IF EXISTS "tenant_id";
-- Webhooks
ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "tenant_id";
-- 邮件日志
ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "tenant_id";
-- 邮件模板
ALTER TABLE "email_templates" DROP COLUMN IF EXISTS "tenant_id";
-- 报表模板
ALTER TABLE "report_templates" DROP COLUMN IF EXISTS "tenant_id";
-- 报表订阅
ALTER TABLE "report_subscriptions" DROP COLUMN IF EXISTS "tenant_id";
-- 全渠道审计日志
ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "tenant_id";
-- 审计日志
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "tenant_id";
-- 定制订单
ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "tenant_id";
-- 流程节点
ALTER TABLE "process_nodes" DROP COLUMN IF EXISTS "tenant_id";
-- 流程日志
ALTER TABLE "process_logs" DROP COLUMN IF EXISTS "tenant_id";
-- 质量问题
ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "tenant_id";
-- 售后
ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "tenant_id";
-- 色卡
ALTER TABLE "color_cards" DROP COLUMN IF EXISTS "tenant_id";
-- 色卡明细
ALTER TABLE "color_card_items" DROP COLUMN IF EXISTS "tenant_id";
-- 色卡借用记录
ALTER TABLE "color_card_borrow_records" DROP COLUMN IF EXISTS "tenant_id";
-- AI 工艺优化
ALTER TABLE "ai_process_optimizations" DROP COLUMN IF EXISTS "tenant_id";
-- AI 质量预测
ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "tenant_id";
-- 销售事实表
ALTER TABLE "sales_facts" DROP COLUMN IF EXISTS "tenant_id";
-- 产品维度
ALTER TABLE "dim_products" DROP COLUMN IF EXISTS "tenant_id";
-- 客户维度
ALTER TABLE "dim_customers" DROP COLUMN IF EXISTS "tenant_id";
-- 产品颜色价格
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "tenant_id";
-- 颜色价格历史
ALTER TABLE "color_price_history" DROP COLUMN IF EXISTS "tenant_id";
-- 颜色价格梯度
ALTER TABLE "color_price_tiers" DROP COLUMN IF EXISTS "tenant_id";
-- 客户颜色价格
ALTER TABLE "customer_color_prices" DROP COLUMN IF EXISTS "tenant_id";
-- 季节性价格规则
ALTER TABLE "seasonal_price_rules" DROP COLUMN IF EXISTS "tenant_id";
-- 慢查询日志
ALTER TABLE "slow_query_log" DROP COLUMN IF EXISTS "tenant_id";
-- 故障转移事件
ALTER TABLE "failover_event" DROP COLUMN IF EXISTS "tenant_id";
-- CRM 商机
ALTER TABLE "crm_opportunity" DROP COLUMN IF EXISTS "tenant_id";
-- 分配历史
ALTER TABLE "assignment_histories" DROP COLUMN IF EXISTS "tenant_id";

-- =====================================================================
-- 第三部分：删除租户管理表（先删子表，再删主表）
-- =====================================================================

-- 租户用户（子表，FK 指向 tenants）
DROP TABLE IF EXISTS "tenant_users";
-- 租户配置（子表）
DROP TABLE IF EXISTS "tenant_configs";
-- 租户订阅（子表）
DROP TABLE IF EXISTS "tenant_subscriptions";
-- 租户用量（子表）
DROP TABLE IF EXISTS "tenant_usage";
-- 租户发票（子表）
DROP TABLE IF EXISTS "tenant_invoices";
-- 租户套餐（被 tenants 引用）
DROP TABLE IF EXISTS "tenant_plans";
-- 租户主表
DROP TABLE IF EXISTS "tenants";
