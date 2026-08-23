//! 完整删除租户功能迁移
//!
//! 创建时间: 2026-06-28
//! 关联计划: 租户功能下线
//!
//! 本迁移用于完整删除系统中的多租户功能：
//! - 删除所有业务表上的 tenant_id 列及其相关索引
//! - 删除全部租户管理表（tenants / tenant_users / tenant_configs 等）
//!
//! 执行顺序：先删索引 → 再删业务表 tenant_id 列 → 最后删租户管理表。
//! 该迁移不可逆，回滚（down）不会恢复已删除的列与表。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 完整删除租户功能迁移（up）
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
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
        ALTER TABLE "sales_orders" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 库存
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_stocks') THEN
        ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 应收发票
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_invoices') THEN
        ALTER TABLE "ar_invoices" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 采购订单
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_orders') THEN
        ALTER TABLE "purchase_orders" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 库存预留
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_reservations') THEN
        ALTER TABLE "inventory_reservations" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 操作日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'operation_logs') THEN
        ALTER TABLE "operation_logs" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 用户
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') THEN
        ALTER TABLE "users" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- API 密钥
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'api_keys') THEN
        ALTER TABLE "api_keys" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- Webhooks
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'webhooks') THEN
        ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 邮件日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'email_logs') THEN
        ALTER TABLE "email_logs" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 邮件模板
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'email_templates') THEN
        ALTER TABLE "email_templates" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 报表模板
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'report_templates') THEN
        ALTER TABLE "report_templates" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 报表订阅
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'report_subscriptions') THEN
        ALTER TABLE "report_subscriptions" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 全渠道审计日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'omni_audit_logs') THEN
        ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 审计日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs') THEN
        ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 定制订单
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'custom_orders') THEN
        ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 流程节点
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'process_nodes') THEN
        ALTER TABLE "process_nodes" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 流程日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'process_logs') THEN
        ALTER TABLE "process_logs" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 质量问题
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'quality_issues') THEN
        ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 售后
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'after_sales') THEN
        ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 色卡
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'color_cards') THEN
        ALTER TABLE "color_cards" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 色卡明细
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'color_card_items') THEN
        ALTER TABLE "color_card_items" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 色卡借用记录
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'color_card_borrow_records') THEN
        ALTER TABLE "color_card_borrow_records" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- AI 工艺优化
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ai_process_optimizations') THEN
        ALTER TABLE "ai_process_optimizations" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- AI 质量预测
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ai_quality_predictions') THEN
        ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 销售事实表
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_facts') THEN
        ALTER TABLE "sales_facts" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 产品维度
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'dim_products') THEN
        ALTER TABLE "dim_products" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 客户维度
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'dim_customers') THEN
        ALTER TABLE "dim_customers" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 产品颜色价格
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'product_color_prices') THEN
        ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 颜色价格历史
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'color_price_history') THEN
        ALTER TABLE "color_price_history" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 颜色价格梯度
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'color_price_tiers') THEN
        ALTER TABLE "color_price_tiers" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 客户颜色价格
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'customer_color_prices') THEN
        ALTER TABLE "customer_color_prices" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 季节性价格规则
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'seasonal_price_rules') THEN
        ALTER TABLE "seasonal_price_rules" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 慢查询日志
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'slow_query_log') THEN
        ALTER TABLE "slow_query_log" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 故障转移事件
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'failover_event') THEN
        ALTER TABLE "failover_event" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- CRM 商机
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'crm_opportunity') THEN
        ALTER TABLE "crm_opportunity" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;
-- 分配历史
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'assignment_histories') THEN
        ALTER TABLE "assignment_histories" DROP COLUMN IF EXISTS "tenant_id";
    END IF;
END $$;

-- =====================================================================
-- 第三部分：删除租户管理表（先删子表，再删主表）
-- =====================================================================
-- 外键依赖关系（m0005 定义）：
--   tenant_invoices.subscription_id → tenant_subscriptions.id
--   tenant_invoices.tenant_id → tenants.id
--   tenant_subscriptions.tenant_id → tenants.id
--   tenant_subscriptions.plan_id → tenant_plans.id
--   tenant_usage.tenant_id → tenants.id
--   tenant_users.tenant_id → tenants.id
--   tenant_configs.tenant_id → tenants.id
--   tenants.plan_id → tenant_plans.id
-- 删除顺序：先删最深层子表，tenants 先于 tenant_plans 删除

-- 租户发票（依赖 tenant_subscriptions + tenants，最深子表）
DROP TABLE IF EXISTS "tenant_invoices";
-- 租户订阅（被 tenant_invoices 引用，依赖 tenants + tenant_plans）
DROP TABLE IF EXISTS "tenant_subscriptions";
-- 租户用量（依赖 tenants）
DROP TABLE IF EXISTS "tenant_usage";
-- 租户用户（依赖 tenants）
DROP TABLE IF EXISTS "tenant_users";
-- 租户配置（依赖 tenants）
DROP TABLE IF EXISTS "tenant_configs";
-- 租户主表（依赖 tenant_plans，必须先于 tenant_plans 删除）
DROP TABLE IF EXISTS "tenants";
-- 租户套餐（被 tenants 引用，最后删除）
DROP TABLE IF EXISTS "tenant_plans";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 完整删除租户功能迁移（down）
-- 创建时间: 2026-06-28
-- 关联计划: 租户功能下线
--
-- 该迁移为破坏性操作：
-- - 删除了所有业务表上的 tenant_id 列及其索引
-- - 删除了全部租户管理表（tenants / tenant_users / tenant_configs 等）
--
-- 列与表一旦删除，数据将永久丢失，无法通过 down 迁移恢复。
-- 如确需恢复多租户功能，请重新执行建表与建列迁移，并从备份中恢复数据。
-- 故本 down 迁移不执行任何 SQL。"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
