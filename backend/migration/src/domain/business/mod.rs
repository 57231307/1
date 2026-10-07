//! 业务表：生产/采购/库存/销售/财务
//!
//! 合并自: 8 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0007_add_mrp_production_bom;
mod m0008_add_supplier_and_product_extensions;
mod m0009_add_purchase_extensions;
mod m0010_add_inventory_extensions;
mod m0011_add_sales_and_logistics_extensions;
mod m0012_add_ap_ar_finance_analysis;
mod m0013_add_business_process_and_traceability;
mod m0014_add_saas_notification_report_email_oa;
mod m0015_seed_supplier_product_catalog;
mod m0016_add_contract_remark;
// 数组语义列 TEXT→text[]/integer[] 归一（12 列，模型无 Json 属性的
// Vec 字段 vs 生效 DDL 标量 TEXT 的列型漂移族）。注册在域 up 链尾：目标表全部由
// 本域 m0009/m0012/m0013 建表，必须晚于建表；production 域对 crm_lead.tags 的
// JSONB ADD IF NOT EXISTS 恒 no-op、不构成生效定义（依据与存量探测策略见文件头注释）。
mod m0071_normalize_array_columns;
// product-categories:read 存量库补授（roles/role_permissions 属 system 域、
// 早于本域，故可直接注册在 business 域 up 链尾/down 链首）
mod m0072_grant_product_categories_read;
// customer_type 渠道词表收口：m0077 只读点名（现状有哪些脏值/各多少行），
// m0078 才回填 + 加 DEFAULT/NOT NULL/CHECK。两者必须同域且 m0077 在前——"回填没
// 干净就不许进 CHECK"的判据依据就是 m0077 留在迁移日志里的逐值分账。
mod m0077_report_customer_type_dirty_values;
mod m0078_finalize_customer_type_domain;
// 合同/价格 reject 存量库补授（roles/role_permissions 属 system 域、早于
// 本域，注册在 business 域 up 链尾/down 链首，同 m0072 授权补种范式）
mod m0081_grant_contract_price_reject;
// customer_credit_ratings 补 customer_id 全表唯一 + customers 外键（本表由本域 m0012
// 建、参照表 customers 由 system 域 m0001 建，均早于链尾；守卫与语义论证见文件头）
mod m0082_add_customer_credit_uniqueness_and_fk;
// 让步接收/复检改判通道：purchase_receipt 真实列 + 检验状态 DB CHECK（本表由本域
// m0009 建、值域归一由 production 域 m0057 完成，均早于链尾；守卫见文件头。
// 编号让位于同域先登记的 m0083 分层列，取 m0084）
mod m0084_concession_receiving_channel;
// 大客户分层列：customers 加 tier（可空+四值 CHECK）+ 按 customer_credit_ratings
// 评级单档映射回填（customers 由 system 域 m0001 建、回填参照表由本域 m0012 建、
// 其单行唯一由本域 m0082 钉死，全部早于链尾；词表与回填依据见文件头）
mod m0083_add_customer_tier_column;
// PII 按需揭示留痕表：自包含新表（不引用其它表、不给业务表加列、不回填），
// 注册在本域 up 链尾；列语义与「无外键/不存原文」判据见文件头。
mod m0085_add_pii_reveal_audit;
// customers:reveal 存量库补授（roles/role_permissions 属 system 域、早于本域，
// 注册紧随 m0085，同 m0069/m0072/m0081 授权补种范式；
// 受授集合与三通道口径见文件头）
mod m0086_grant_customers_pii_reveal;
// 疵点处置理由载体列：理由此前只校验不落库，补一列承接；列语义见文件头。
mod m0087_add_unqualified_handling_reason;
// 采购收货让步接收/复检改判两枚显式权限键的存量库补授：role_permissions 由 system 域
// 先建，注册紧随 m0087，岗位集合与 init 矩阵、e2e 三通道同口径（见文件头）。
mod m0090_grant_purchase_receipt_concession_rejudge;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_business_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0007_add_mrp_production_bom::Migration.up(manager).await?;
        m0008_add_supplier_and_product_extensions::Migration
            .up(manager)
            .await?;
        m0009_add_purchase_extensions::Migration.up(manager).await?;
        m0010_add_inventory_extensions::Migration
            .up(manager)
            .await?;
        m0011_add_sales_and_logistics_extensions::Migration
            .up(manager)
            .await?;
        m0012_add_ap_ar_finance_analysis::Migration
            .up(manager)
            .await?;
        m0013_add_business_process_and_traceability::Migration
            .up(manager)
            .await?;
        m0014_add_saas_notification_report_email_oa::Migration
            .up(manager)
            .await?;
        let sql = r#"ALTER TABLE "ar_invoices" ADD COLUMN IF NOT EXISTS "salesperson_id" INTEGER;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "batch_trace_log" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "batch_trace_log" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "from_status" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "to_status" VARCHAR(255);
ALTER TABLE "bom_items" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "boms" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "account_subject_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "budget_year" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "planned_amount" DECIMAL(14,2);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "custom_fields" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "industry" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "batch_dye_lot_id" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "dye_lot_mapping" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "internal_dye_lot_no" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "mapping_date" DATE;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "product_code" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "supplier_dye_lot_no" VARCHAR(255);
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "supplier_id" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validated_at" TIMESTAMPTZ;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validated_by" INTEGER;
ALTER TABLE "dye_lot_mapping" ADD COLUMN IF NOT EXISTS "validation_status" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "attachments" JSONB;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "html_content" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "text_content" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "gain_loss" DECIMAL(18,4);
ALTER TABLE "fixed_assets" ADD COLUMN IF NOT EXISTS "asset_category_id" INTEGER;
ALTER TABLE "fixed_assets" ADD COLUMN IF NOT EXISTS "depreciation_start_date" DATE;
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "available_balance" DECIMAL(18,4);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "frozen_balance" DECIMAL(18,4);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "opened_date" DATE;
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "fund_accounts" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "inventory_piece" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "inventory_piece" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inventory_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "package_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_seq" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "position_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "scan_type" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "shelf_life" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "supplier_piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "width" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "distance_km" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_bearer" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_rate" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_photo_url" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_receipt_url" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "sign_remark" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "signed_by" INTEGER;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_volume" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_weight" DECIMAL(18,4);
ALTER TABLE "notification_settings" ADD COLUMN IF NOT EXISTS "enable_webhook" BOOLEAN;
ALTER TABLE "notifications" ADD COLUMN IF NOT EXISTS "dedup_key" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visibility_scope" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visible_scope_config" JSONB;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "production_orders" SET "batch_no" = '' WHERE "batch_no" IS NULL;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "production_orders" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "production_orders" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "original_batch_id" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "schedule_batch_key" VARCHAR(255);
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "purchase_return_item" SET "batch_no" = '' WHERE "batch_no" IS NULL;
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "purchase_return_item" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "purchase_return_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "purchase_return_item" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "max_retries" INTEGER;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "cache_ttl_seconds" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "category" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "data_source" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "refresh_strategy" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "required_permission" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "supported_formats" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "template_id" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "breach_liability" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "dispute_resolution" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "performance_period" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "quality_terms" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_certificate" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_hash" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_image_url" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_by_user_id" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "stamp_tax_amount" DECIMAL(18,4);
ALTER TABLE "sales_delivery_item" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "sales_delivery_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "sales_delivery_item" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "sales_return_item" SET "batch_no" = '' WHERE "batch_no" IS NULL;
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "sales_return_item" SET "color_no" = '' WHERE "color_no" IS NULL;
ALTER TABLE "sales_return_item" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255) NOT NULL DEFAULT '';
UPDATE "sales_return_item" SET "dye_lot_no" = '' WHERE "dye_lot_no" IS NULL;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_fin" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_gm" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_fin" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_gm" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_result" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_approval_status" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_loss_amount" DECIMAL(18,4);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_grade_synced" BOOLEAN;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "auto_reschedule_enabled" BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "equipment_count" INTEGER;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "shift_hours" DECIMAL(6,2);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "standard_hours_per_unit" DECIMAL(10,2);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "worker_count" INTEGER;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // 合同表头备注列：依赖 m0009/m0011 已建两张合同主表
        m0016_add_contract_remark::Migration.up(manager).await?;
        // 供应商商品/色号目录种子：必须晚于 m0008（建两表）与 system 的 suppliers 表，
        // 放在 business.up() 末尾，保证依赖表与补列全部就绪后再灌演示数据。
        m0015_seed_supplier_product_catalog::Migration
            .up(manager)
            .await?;
        // 数组列归一，注册在 business 域 up 链尾——目标表全部由本域
        // m0009/m0012/m0013 建表（含 attachment_urls/tags/product_* /internal_piece_*
        // 的生效 TEXT 定义），归一必须晚于建表、且早于任何读写这些列的后续域。
        m0071_normalize_array_columns::Migration.up(manager).await?;
        // 采购岗补 product-categories:read，注册在本域 up 链尾（角色表与
        // 权限表均由 system 域先建）
        m0072_grant_product_categories_read::Migration
            .up(manager)
            .await?;
        // customer_type 渠道词表收口注册在本域 up 链尾——目标表 customers 由
        // system/m0001 建（VARCHAR(20)、可空、无 CHECK，m0001:332），点名与回填必须
        // 晚于建表；finance 域那段 `ADD COLUMN IF NOT EXISTS customer_type VARCHAR(255)`
        // 恒 no-op（列早已存在），排在其前不构成干扰。
        m0077_report_customer_type_dirty_values::Migration
            .up(manager)
            .await?;
        m0078_finalize_customer_type_domain::Migration
            .up(manager)
            .await?;
        // 合同/价格 reject 存量库补授，注册在本域 up 链尾（角色表与权限表
        // 均由 system 域先建，见 m0081 文件头）
        m0081_grant_contract_price_reject::Migration
            .up(manager)
            .await?;
        // customer_credit_ratings 单行唯一 + 客户外键（建表在本域 m0012、参照表在
        // system 域 m0001，均先于此处的链尾约束）
        m0082_add_customer_credit_uniqueness_and_fk::Migration
            .up(manager)
            .await?;
        // customers.tier 分层列 + 评级单档映射回填 + 四值 CHECK，注册在本域 up 链最末：
        // 依赖的 m0082 单行唯一必须先建立（回填按 customer_id 关联单行评级，多行会致
        // 同一客户取档不确定），见 m0083 文件头。
        m0083_add_customer_tier_column::Migration
            .up(manager)
            .await?;
        // 让步接收/复检改判通道：注册在本域 up 链最末（purchase_receipt 建表在本域
        // m0009、检验状态归一在 production 域 m0057，均先于此处；与分层列迁移无依赖）
        m0084_concession_receiving_channel::Migration
            .up(manager)
            .await?;
        // PII 按需揭示留痕表：自包含新表，无上游依赖，注册在本域 up 链最末
        m0085_add_pii_reveal_audit::Migration.up(manager).await?;
        // customers:reveal 存量库补授：roles/role_permissions 由 system 域先建，
        // 注册紧随 m0085（见 m0086 文件头三通道口径）
        m0086_grant_customers_pii_reveal::Migration
            .up(manager)
            .await?;
        // 疵点处置理由列：自包含加列，无上游依赖
        m0087_add_unqualified_handling_reason::Migration
            .up(manager)
            .await?;
        // 让步接收/复检改判两枚键补授：紧随其后注册在本域 up 链最末
        m0090_grant_purchase_receipt_concession_rejudge::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        // 让步/改判补授：最后应用者最先回滚（只按角色码回收本迁移授予的两枚键）
        m0090_grant_purchase_receipt_concession_rejudge::Migration
            .down(manager)
            .await?;
        // 疵点处置理由列：撤本迁移施加的列
        m0087_add_unqualified_handling_reason::Migration
            .down(manager)
            .await?;
        // customers:reveal 补授：只回收本迁移按角色码授予的
        // reveal 键，人工另授的 customers 键不动
        m0086_grant_customers_pii_reveal::Migration
            .down(manager)
            .await?;
        // PII 揭示留痕表：撤表（索引与约束随表消失；审计数据仅在刻意回滚整域时
        // 一并回收，与建表的对称语义，见 m0085 文件头）
        m0085_add_pii_reveal_audit::Migration.down(manager).await?;
        // 让步接收通道：只移除本迁移施加的 CHECK 与新增列，不触碰数据行
        m0084_concession_receiving_channel::Migration
            .down(manager)
            .await?;
        // customers.tier 分层列：先撤 CHECK 再删列，源评级数据未动
        m0083_add_customer_tier_column::Migration
            .down(manager)
            .await?;
        // customer_credit_ratings 约束：只移除本迁移施加的唯一索引与 FK，不触碰数据行
        m0082_add_customer_credit_uniqueness_and_fk::Migration
            .down(manager)
            .await?;
        // 审批改造波：最后应用者最先回滚（只回收本迁移按角色码授予的四对 reject 键）
        m0081_grant_contract_price_reject::Migration
            .down(manager)
            .await?;
        // 最后应用者最先回滚——先撤 CHECK/NOT NULL/DEFAULT 并自备份列还原原值，
        // 再回滚只读点名（其 up 对库内状态零改变，down 是显式 no-op）。
        m0078_finalize_customer_type_domain::Migration
            .down(manager)
            .await?;
        m0077_report_customer_type_dirty_values::Migration
            .down(manager)
            .await?;
        // 最后应用者最先回滚（只回收本迁移按角色码授予的 read 键）
        m0072_grant_product_categories_read::Migration
            .down(manager)
            .await?;
        // 逆序首位——数组列回退 TEXT（多元素行存在时拒滚，见 m0071 文件头）
        m0071_normalize_array_columns::Migration
            .down(manager)
            .await?;
        m0015_seed_supplier_product_catalog::Migration
            .down(manager)
            .await?;
        m0016_add_contract_remark::Migration.down(manager).await?;
        m0014_add_saas_notification_report_email_oa::Migration
            .down(manager)
            .await?;
        m0013_add_business_process_and_traceability::Migration
            .down(manager)
            .await?;
        m0012_add_ap_ar_finance_analysis::Migration
            .down(manager)
            .await?;
        m0011_add_sales_and_logistics_extensions::Migration
            .down(manager)
            .await?;
        m0010_add_inventory_extensions::Migration
            .down(manager)
            .await?;
        m0009_add_purchase_extensions::Migration
            .down(manager)
            .await?;
        m0008_add_supplier_and_product_extensions::Migration
            .down(manager)
            .await?;
        m0007_add_mrp_production_bom::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
