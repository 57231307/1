//! 生产/质量/委外
//!
//! 合并自: 16 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0025_p4_1_perf_indexes;
mod m0026_extend_audit_log;
mod m0027_enable_pg_stat_statements;
mod m0028_create_slow_query_log;
mod m0029_drop_tenant_columns;
mod m0030_create_crm_recycle_rules;
mod m0041_create_import_tasks;
mod m0042_create_purchase_inspection_items;
mod m0043_add_scan_type_and_industry_columns;
mod m0044_integrate_unreferenced_migrations;
mod m0045_add_password_changed_at_to_users;
mod m0046_drop_audit_alert_rules;
mod m0047_add_last_payload_to_webhooks;
mod m0048_add_user_id_to_webhooks;
mod m0049_create_processed_events;
mod m0050_create_event_dead_letters;
mod m0051_add_piece_type_and_machine_no;
mod m0052_add_piece_no_to_flow_documents;
mod m0053_add_warehouse_contact_and_default;
mod m0054_add_import_task_file_fields;
mod m0055_create_system_update_tables;
mod m0056_normalize_stock_quality_status;
mod m0057_normalize_stock_status_domain;
// m0058 目标表 purchase_order_item / sales_contract_items 均在 v15 域内 CREATE，
// 而 production 域早于 v15 执行；故 m0058 的 up/down 改由 v15 域在建表之后调用
// （见 domain/v15/mod.rs）。此处仅保留类型定义供跨域引用，故提升可见性为 pub(crate)。
pub(crate) mod m0058_add_delivery_tolerance;
mod m0059_add_product_piece_roll_conversion;
mod m0060_add_so_item_tolerance;
mod m0061_custom_order_status_add_lab_dip_quotation;
mod m0062_add_dye_batch_actual_output;
// m0063 为 9 张单据表补单号列 UNIQUE 兜底：其中 outsourcing_order /
// outsourcing_receipt / finance_invoices 在 v15 域内建表，而 production 域
// 早于 v15 执行，直接注册本域会因 "relation ... does not exist" 中断迁移链。
// 照 m0058 先例（见上方注释与 domain/v15/mod.rs），up/down 由 v15 域在
// 全部建表完成后调用，此处仅保留定义，提升可见性为 pub(crate)。
pub(crate) mod m0063_add_document_no_unique_constraints;
// m0064 扩 chk_custom_order_status 取值集为权威模块 custom_order::ALL 的 11 值
// （+ change_pending）；目标表 custom_orders 由本域 m0044 建表（早于本迁移执行），
// 直接注册本域 up 末尾即可。
mod m0064_custom_order_status_add_change_pending;
// m0065 委外发料匹占用存量回填：目标表 outsourcing_order / outsourcing_order_item
// 在 v15 域内建表（v15/mod.rs:3227/:671），而 production 域早于 v15 执行，直接注册
// 本域会因 "relation ... does not exist" 中断迁移链。照 m0058/m0063 先例，
// up/down 由 domain/v15/mod.rs 在全部建表完成后调用，此处仅保留定义，
// 提升可见性为 pub(crate)。
pub(crate) mod m0065_backfill_outsourcing_reserved_pieces;
// m0066 调拨出库明细补匹号列（出库四维=缸/色/批/匹，用户 2026-10-02 纠正口径）：
// 目标表 inventory_transfer_items 由 system 域 m0001 建表，早于本域执行，直接注册本域。
mod m0066_add_piece_no_to_transfer_items;
// m0067 扩 chk_aftersales_status 取值集为写入方权威词表 AFTERSALES_ALL 的 7 值
// （+ accepted/evaluated，三端同源收口 CI #4669 65-01）；目标表 after_sales 由本域
// m0044 建表（早于本迁移执行），直接注册本域 up 末尾即可。
mod m0067_aftersales_status_add_accepted_evaluated;
// m0068 化学品三表编码列部分唯一索引：目标表 chemical_category/chemical_master/
// chemical_lot 均在 v15 域内建表（v15/mod.rs:2457/:2499/:2472），production 域早于
// v15 执行。照 m0058/m0063/m0065 先例，up/down 由 domain/v15/mod.rs 在建表完成后
// 调用，此处仅保留定义，提升可见性为 pub(crate)。
pub(crate) mod m0068_add_chemical_code_partial_unique_constraints;
// m0069 存量库补授 pieces:read / pieces:print（匹号领域权限键自始未注册，非 admin
// 访问 /inventory/pieces* 一律 403，详见文件头判责链）。目标表 role_permissions
// 由 system 域 m0005 建表、roles 由 m0001 建表并种 3 角色，均早于本域执行，直接注册本域。
mod m0069_grant_piece_read_and_print;
// m0073 收紧 color_cards.stock_quantity / issued_quantity 为 NOT NULL DEFAULT 0：
// 列由本域 inline SQL 先建成裸可空，finance 域的 NOT NULL DEFAULT 0 被
// ADD COLUMN IF NOT EXISTS 吃成恒 no-op（scan_out.txt SHADOWED 第 244 行），
// 而模型/出参都是非 Option i32 ⇒ NULL 行读取即 ColumnNull。判责链见文件头。
mod m0073_normalize_color_card_quantities;
// m0074 扩 chk_aftersales_type 取值集为写入方权威白名单的 5 值（+ return_goods，
// 三端同源：service create 白名单 = CHECK = 前端候选）；目标表 after_sales 由本域
// m0044 建表（早于本迁移执行），直接注册本域 up 末尾即可（与 m0067 同口径）。
mod m0074_aftersales_type_add_missing_values;
// m0075 委外收回入库单补三列打卷实测值（weight/width/gram_weight，收回→产匹透传的数据源）：
// 目标表 outsourcing_receipt 在 v15 域内建表（domain/v15/mod.rs:689），而 production 域早于 v15
// 执行，直接注册本域会因 "relation outsourcing_receipt does not exist" 中断迁移链。
// 照 m0058/m0063/m0065/m0068 先例，up/down 由 domain/v15/mod.rs 在全部建表完成后调用（up
// v15/mod.rs:4616、down :4625），此处仅保留定义，提升可见性为 pub(crate)。
pub(crate) mod m0075_add_outsourcing_receipt_measured_values;
// m0076 给 inventory_piece 的 weight/width/gram_weight 三列补值域 CHECK
// （`IS NULL OR > 0`，任务板 #246）：这三列是成品布入库标签（#220）的**直读源**，
// 服务层已有 >0 门（`outsourcing_ops/receipt.rs`、`inv/batch.rs`），CHECK 是并发/旁路
// 写入的兜底——否则 0 或负值可落库并被**印上标签**（0 属伪造实测值）。
// 注册链与 m0075 不同：目标表由 **business 域 m0010_add_inventory_extensions.rs:126** 建表，
// 而 lib.rs 域顺序为 system→business→sales_crm→**production**（business 早于本域执行），
// 故直接注册本域即可，无需像 m0075 那样挂到 v15 之后。
mod m0076_add_inventory_piece_measured_checks;
// m0079 交易域审批「通过/拒绝」双理由专列（价目/报价/合同/两订单/采购退货 8 表 14 列）：
// 目标表 sales_quotations 由 sales_crm 域建表（lib.rs 域序 system→business→sales_crm→
// production，sales_crm 早于本域），其余 7 表由 system/m0001 与 business/m0009、m0011
// 更早建表，故直接注册本域 up 链尾即可，全部目标表届时应存在。
// 链序约束：加列必须先于 price_vocab_check 的后继 CHECK 迁移（补列先于 CHECK 的既定链序）。
mod m0079_add_approval_reason_columns;
// m0083 出口商检权限键 export-inspections:{read,create,update,print} 存量库补授
// （资源段自始未注册，除 admin 外全员 403，详见该文件头判责链）。目标表 role_permissions
// 由 system 域 m0005 建表、roles 由 m0001 建表，均早于本域执行；本迁移不触碰 export_inspection
// 业务表，故直接注册本域 up 链尾 / down 链首，无需后置到 v15（口径同 m0069）。
mod m0083_grant_export_inspections_perms;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_production_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0025_p4_1_perf_indexes::Migration.up(manager).await?;
        m0026_extend_audit_log::Migration.up(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .up(manager)
            .await?;
        m0028_create_slow_query_log::Migration.up(manager).await?;
        // 顺序修复：m0044 必须在 m0029 之前执行。
        // m0044 创建 custom_orders / color_cards / process_nodes / sales_facts 等 31 张表，
        // m0029 随后对这些表执行 ALTER TABLE ... DROP COLUMN IF EXISTS "tenant_id"。
        // PostgreSQL 的 DROP COLUMN IF EXISTS 仅保护列不存在，不保护表不存在；
        // 若 m0029 先于 m0044 执行，会因 "relation xxx does not exist" 中断整个迁移链。
        // 这也符合 m0044 文件头注释的设计意图："注册在 m0028 之后、m0029 之前"。
        m0044_integrate_unreferenced_migrations::Migration
            .up(manager)
            .await?;
        // custom_orders ID 列统一为 BIGINT：对齐 Rust custom_order::Model 的
        // i64/Option<i64> 与 m0044 的建表声明。若列已是 BIGINT 则无副作用；
        // 若历史环境为 INT4（INTEGER），消除 SeaORM 解码
        // "Option<i64> (INT8) is not compatible with SQL type INT4" 的 500。
        // quotation_id/approval_instance_id 等列由后续迁移 ADD COLUMN，逐列存在性检查
        {
            let conn = manager.get_connection();
            conn.execute_unprepared(
                r#"
                DO $$
                BEGIN
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'customer_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "customer_id" TYPE BIGINT USING "customer_id"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'product_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "product_id" TYPE BIGINT USING "product_id"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'color_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "color_id" TYPE BIGINT USING "color_id"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'sales_order_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "sales_order_id" TYPE BIGINT USING "sales_order_id"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'quotation_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "quotation_id" TYPE BIGINT USING "quotation_id"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'created_by') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "created_by" TYPE BIGINT USING "created_by"::BIGINT;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.columns
                               WHERE table_name = 'custom_orders' AND column_name = 'approval_instance_id') THEN
                        ALTER TABLE "custom_orders" ALTER COLUMN "approval_instance_id" TYPE BIGINT USING "approval_instance_id"::BIGINT;
                    END IF;
                END $$;
                "#,
            )
            .await?;
        }
        m0029_drop_tenant_columns::Migration.up(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .up(manager)
            .await?;
        m0041_create_import_tasks::Migration.up(manager).await?;
        m0042_create_purchase_inspection_items::Migration
            .up(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .up(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .up(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration.up(manager).await?;
        m0047_add_last_payload_to_webhooks::Migration
            .up(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration.up(manager).await?;
        m0049_create_processed_events::Migration.up(manager).await?;
        m0050_create_event_dead_letters::Migration
            .up(manager)
            .await?;
        m0051_add_piece_type_and_machine_no::Migration
            .up(manager)
            .await?;
        m0052_add_piece_no_to_flow_documents::Migration
            .up(manager)
            .await?;
        m0053_add_warehouse_contact_and_default::Migration
            .up(manager)
            .await?;
        m0054_add_import_task_file_fields::Migration
            .up(manager)
            .await?;
        m0055_create_system_update_tables::Migration
            .up(manager)
            .await?;
        m0056_normalize_stock_quality_status::Migration
            .up(manager)
            .await?;
        m0057_normalize_stock_status_domain::Migration
            .up(manager)
            .await?;
        // 交货数量容差行级列（采购订单行 / 销售合同行）：目标表 purchase_order_item /
        // sales_contract_items 均在 v15 域内建表，production 先于 v15 执行会导致
        // "relation ... does not exist"，故 m0058.up() 已后置至 v15 域（见 domain/v15/mod.rs）。
        // 产品匹/卷换算元数据列（meters_per_piece / meters_per_roll），域内最后追加
        m0059_add_product_piece_roll_conversion::Migration
            .up(manager)
            .await?;
        // 销售订单行交货数量容差列（quantity_tolerance_pct），与 m0058 对称
        m0060_add_so_item_tolerance::Migration.up(manager).await?;
        // 定制订单状态 CHECK 补齐 lab_dip/quotation（与状态机 as_str() 同源），须在建表后执行
        m0061_custom_order_status_add_lab_dip_quotation::Migration
            .up(manager)
            .await?;
        // 缸号完工实际产出三列（dye_batch 表由 system 域 m0003 建表，早于 production 域执行）
        m0062_add_dye_batch_actual_output::Migration
            .up(manager)
            .await?;
        // 定制订单状态 CHECK 扩为权威词表 11 值（+ change_pending，写入方
        // submit_change_request），存量词表外取值 fail-visible 中止，须晚于
        // m0044 建表与 m0061 十值重建（同域顺序执行）
        m0064_custom_order_status_add_change_pending::Migration
            .up(manager)
            .await?;
        // 调拨出库明细补匹号列（出库四维=缸/色/批/匹 落库点，见文件头注释）
        m0066_add_piece_no_to_transfer_items::Migration
            .up(manager)
            .await?;
        // 售后工单状态 CHECK 补齐 accepted/evaluated（三端同源，见文件头注释），须晚于 m0044 建表
        m0067_aftersales_status_add_accepted_evaluated::Migration
            .up(manager)
            .await?;
        let sql = r#"ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "expires_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_hash" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_prefix" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "last_used_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "permissions" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "rate_limit_per_minute" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "action" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "created_at" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "duration_ms" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "new_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "old_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "response_status" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "address" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "company_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "contact_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "contact_title" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_customer_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "converted_opportunity_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "custom_fields" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "estimated_amount" DECIMAL(18,4);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "estimated_quantity" DECIMAL(18,4);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "expected_delivery_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "follow_up_plan" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "last_follow_up_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_no" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_source" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lead_status" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "lost_reason" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "mobile_phone" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "next_follow_up_date" DATE;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "owner_id" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "owner_name" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "priority" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "product_interest" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "qq" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "rating" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "requirement_desc" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "tags" JSONB;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "tel_phone" VARCHAR(255);
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "crm_lead" ADD COLUMN IF NOT EXISTS "wechat" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "created_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "event_payload" JSONB;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "failure_reason" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "first_failed_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "last_error" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "last_retry_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "max_retries" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "resolved_at" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "resolved_by" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "event_dead_letters" ADD COLUMN IF NOT EXISTS "updated_at" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "config_key" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "config_value" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "failover_config" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "from_state" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "latency_ms" INTEGER;
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "reason" VARCHAR(255);
ALTER TABLE "failover_event" ADD COLUMN IF NOT EXISTS "to_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "backup_type" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "circuit_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "consecutive_failures" INTEGER;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "current_state" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "function_name" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "last_success_at" TIMESTAMPTZ;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "last_switch_at" TIMESTAMPTZ;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "primary_url" VARCHAR(255);
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_backup_calls" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_primary_calls" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "total_switches" BIGINT;
ALTER TABLE "failover_status" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "barcode" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "inventory_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_length" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "original_weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "package_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "parent_piece_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "piece_seq" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "position_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "shelf_life" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "supplier_piece_no" VARCHAR(255);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "updated_by" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "weight" DECIMAL(18,4);
ALTER TABLE "inventory_piece" ADD COLUMN IF NOT EXISTS "width" DECIMAL(18,4);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "consumer_id" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "event_key" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "event_type" VARCHAR(255);
ALTER TABLE "processed_events" ADD COLUMN IF NOT EXISTS "processed_at" TIMESTAMPTZ;
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "assigned_to" VARCHAR(255);
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "jira_ticket" VARCHAR(255);
ALTER TABLE "slow_query_log" ADD COLUMN IF NOT EXISTS "optimization_status" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "agreed_to_terms_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "birth_date" DATE;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "department_id" INTEGER;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "gender" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "is_totp_enabled" BOOLEAN;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "last_login_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "password_hash" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "phone" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "role_id" INTEGER;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_recovery_codes" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_secret" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "address" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "city" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "country" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "email" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "manager_id" INTEGER;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "phone" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "postal_code" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "province" VARCHAR(255);
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "warehouses" ADD COLUMN IF NOT EXISTS "warehouse_code" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "events" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_status" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_triggered_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "secret" VARCHAR(255);
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "url" VARCHAR(255);

ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "accepted_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluated_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_comment" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_score" INTEGER;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "quality_issue_id" BIGINT;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_category" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_detail" VARCHAR(255);
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "production_recipe_id" INTEGER;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_avg_qualification_rate" DECIMAL(18,4);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_grade" VARCHAR(255);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_recorded_at" TIMESTAMPTZ;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_risk_level" VARCHAR(255);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "claim_amount" DECIMAL(18,4);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "claim_recorded_at" TIMESTAMPTZ;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "account_subject_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_code" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_name" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "item_type" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "level" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "parent_id" INTEGER;
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "budget_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "color_fastness_grade" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "dyeing_capability" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "issued_quantity" INTEGER;
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "printing_capability" VARCHAR(255);
ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "stock_quantity" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approval_instance_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approval_comment" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "lab_dip_request_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quality_standard_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quotation_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "rejection_reason" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "count_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "unit_cost" DECIMAL(12,2);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "count_no" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "bin_location" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "expiry_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_movement_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "layer_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "production_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_available" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_incoming" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_on_hand" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_reserved" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_shipped" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "replenishment_strategy" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "shelf_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "stock_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "base_price" DECIMAL(18,4);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "color_id" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "customer_level" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "effective_from" DATE;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "effective_to" DATE;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "min_quantity" DECIMAL(18,4);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "product_id" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_completed_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_due_date" DATE;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_owner" INTEGER;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_detail" JSONB;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_method" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "duty_cost" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "freight_cost" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "insurance_cost" DECIMAL(18,4);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // m0069 存量库补授 pieces:read / pieces:print（本域最后应用；依赖的
        // role_permissions/roles 由 system 域 m0005/m0001 建表，早于本域执行）
        m0069_grant_piece_read_and_print::Migration
            .up(manager)
            .await?;
        // m0073 收紧色卡数量列（本域最后应用；目标表 color_cards 由本域建表建列）
        m0073_normalize_color_card_quantities::Migration
            .up(manager)
            .await?;
        // m0074 售后工单类型 CHECK 补齐 return_goods（三端同源，见文件头注释），
        // 须晚于 m0044 建表；本域最后应用，down 最先回滚
        m0074_aftersales_type_add_missing_values::Migration
            .up(manager)
            .await?;
        // m0076 匹表三列实测值补 >0 CHECK（本域最后应用，故 down 最先回滚）
        m0076_add_inventory_piece_measured_checks::Migration
            .up(manager)
            .await?;
        // m0079 交易域审批双理由专列（8 表 14 列，全部可空 ADD IF NOT EXISTS；
        // 目标表均由更早的 system/business/sales_crm 域建表，注册本域 up 链尾）
        m0079_add_approval_reason_columns::Migration
            .up(manager)
            .await?;
        // m0083 出口商检权限键存量库补授（本域最后应用，故 down 最先回滚；
        // role_permissions/roles 由 system 域更早建表，直接注册本域，口径同 m0069）
        m0083_grant_export_inspections_perms::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        // m0083 最后应用故最先回滚（只回收本迁移按角色码授予的 export-inspections 键）
        m0083_grant_export_inspections_perms::Migration
            .down(manager)
            .await?;
        // m0079 最后应用故最先回滚（对称 DROP 本迁移新增的 14 列，不触碰既有列）
        m0079_add_approval_reason_columns::Migration
            .down(manager)
            .await?;
        // m0076 最后应用故最先回滚（撤三条值域 CHECK + 丢弃三列，带在途实测值时
        // fail-visible 拒滚，见该文件 down 注释）
        m0076_add_inventory_piece_measured_checks::Migration
            .down(manager)
            .await?;
        // m0074 最后应用故最先回滚（售后类型 CHECK 回 m0044 原 4 值，
        // 含 return_goods 在途行 fail-visible 拒滚）
        m0074_aftersales_type_add_missing_values::Migration
            .down(manager)
            .await?;
        // m0073 次后应用（撤 NOT NULL/DEFAULT，回到修复前的可空无默认形态）
        m0073_normalize_color_card_quantities::Migration
            .down(manager)
            .await?;
        // m0069 次后应用（只回收本迁移按角色码授予的 pieces 键）
        m0069_grant_piece_read_and_print::Migration
            .down(manager)
            .await?;
        // m0067 次后应用（售后状态 CHECK 回原 5 值，含在途行 fail-visible 拒滚）
        m0067_aftersales_status_add_accepted_evaluated::Migration
            .down(manager)
            .await?;
        // m0066 次后应用（调拨明细匹号列）
        m0066_add_piece_no_to_transfer_items::Migration
            .down(manager)
            .await?;
        // m0064 次后应用（带在途 change_pending 行的 fail-visible 拒滚检查）
        m0064_custom_order_status_add_change_pending::Migration
            .down(manager)
            .await?;
        m0062_add_dye_batch_actual_output::Migration
            .down(manager)
            .await?;
        m0061_custom_order_status_add_lab_dip_quotation::Migration
            .down(manager)
            .await?;
        m0060_add_so_item_tolerance::Migration.down(manager).await?;
        m0059_add_product_piece_roll_conversion::Migration
            .down(manager)
            .await?;
        // m0058 的 down() 已随之迁移至 v15 域（与 up 对称），此处不再回滚。
        m0057_normalize_stock_status_domain::Migration
            .down(manager)
            .await?;
        m0056_normalize_stock_quality_status::Migration
            .down(manager)
            .await?;
        m0055_create_system_update_tables::Migration
            .down(manager)
            .await?;
        m0054_add_import_task_file_fields::Migration
            .down(manager)
            .await?;
        m0053_add_warehouse_contact_and_default::Migration
            .down(manager)
            .await?;
        m0052_add_piece_no_to_flow_documents::Migration
            .down(manager)
            .await?;
        m0052_add_piece_no_to_flow_documents::Migration
            .down(manager)
            .await?;
        m0051_add_piece_type_and_machine_no::Migration
            .down(manager)
            .await?;
        m0050_create_event_dead_letters::Migration
            .down(manager)
            .await?;
        m0049_create_processed_events::Migration
            .down(manager)
            .await?;
        m0048_add_user_id_to_webhooks::Migration
            .down(manager)
            .await?;
        m0047_add_last_payload_to_webhooks::Migration
            .down(manager)
            .await?;
        m0046_drop_audit_alert_rules::Migration
            .down(manager)
            .await?;
        m0045_add_password_changed_at_to_users::Migration
            .down(manager)
            .await?;
        m0043_add_scan_type_and_industry_columns::Migration
            .down(manager)
            .await?;
        m0042_create_purchase_inspection_items::Migration
            .down(manager)
            .await?;
        m0041_create_import_tasks::Migration.down(manager).await?;
        m0030_create_crm_recycle_rules::Migration
            .down(manager)
            .await?;
        m0029_drop_tenant_columns::Migration.down(manager).await?;
        // 逆序：m0044 在 m0029 之后回滚（与 up 顺序相反）
        m0044_integrate_unreferenced_migrations::Migration
            .down(manager)
            .await?;
        m0028_create_slow_query_log::Migration.down(manager).await?;
        m0027_enable_pg_stat_statements::Migration
            .down(manager)
            .await?;
        m0026_extend_audit_log::Migration.down(manager).await?;
        m0025_p4_1_perf_indexes::Migration.down(manager).await?;
        Ok(())
    }
}
