//! V15 批次18：胚布/委外/质量
//!
//! 合并自: 5 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0076_add_export_audit_fields.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- V15 P1-3-3：audit_logs 表导出专属字段
                -- ============================================================

                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(20);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(128);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(100);

                -- 索引：按导出条数筛选大批量导出（合规审查常用）
                CREATE INDEX IF NOT EXISTS "idx_audit_log_export_count"
                    ON "audit_logs"("export_record_count");
                -- 索引：按审批 token 查询敏感数据导出追溯
                CREATE INDEX IF NOT EXISTS "idx_audit_log_approval_token"
                    ON "audit_logs"("export_approval_token");

                COMMENT ON COLUMN "audit_logs"."export_record_count" IS 'V15 P1-3-3：导出数据行数，用于大批量导出识别（>80% 上限触发告警）';
                COMMENT ON COLUMN "audit_logs"."export_query_filter" IS 'V15 P1-3-3：导出时的筛选条件 JSON，用于追溯导出数据范围';
                COMMENT ON COLUMN "audit_logs"."export_file_format" IS 'V15 P1-3-3：导出文件格式（xlsx/csv/pdf），格式合规审计';
                COMMENT ON COLUMN "audit_logs"."export_approval_token" IS 'V15 P1-3-3：二级审批 token（敏感数据导出），10 分钟有效期';
                COMMENT ON COLUMN "audit_logs"."export_watermark_user" IS 'V15 P1-3-3：导出文件水印中的用户名，二次泄露追溯';
                "#,
            )
            .await?;

        Ok(())
        // === m0077_add_oa_visibility_consent_retention.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P1 batch-16 缺陷 7.2：OA 公告可见性控制
                -- ============================================================

                -- visibility_scope：可见性范围枚举
                --   ALL=全员可见（默认）
                --   DEPT=指定部门可见（visible_scope_config = {"department_ids": [1,2,3]
        // === m0078_batch18_greige_outsourcing_quality_scheduling.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P1 batch-18 缺陷 1.1：胚布关联采购订单
                -- ============================================================
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_order_id" INTEGER;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_receipt_id" INTEGER;
                COMMENT ON COLUMN "greige_fabric"."purchase_order_id" IS '关联采购订单ID（NULL表示手工录入未走采购流程）';
                COMMENT ON COLUMN "greige_fabric"."purchase_receipt_id" IS '关联采购入库单ID（支持三单匹配）';

                -- ============================================================
                -- P1 batch-18 缺陷 1.2：胚布安全库存预警字段
                -- ============================================================
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "safety_stock" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "max_stock_point" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(12,2) DEFAULT 0;
                COMMENT ON COLUMN "greige_fabric"."safety_stock" IS '安全库存（公斤）';
                COMMENT ON COLUMN "greige_fabric"."reorder_point" IS '订货点（公斤，低于此值触发补货建议）';
                COMMENT ON COLUMN "greige_fabric"."max_stock_point" IS '最大库存（公斤，超过此值告警）';
                COMMENT ON COLUMN "greige_fabric"."reorder_quantity" IS '补货量（公斤）';

                -- ============================================================
                -- P1 batch-18 缺陷 2.1：委外发料关联胚布
                -- ============================================================
                ALTER TABLE "outsourcing_order_item" ADD COLUMN IF NOT EXISTS "greige_fabric_id" INTEGER;
                COMMENT ON COLUMN "outsourcing_order_item"."greige_fabric_id" IS '关联胚布ID（精确到卷/匹级追溯）';

                -- ============================================================
                -- P1 batch-18 缺陷 4.2：8D 根因分析方法
                -- ============================================================
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_method" VARCHAR(20);
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_detail" JSONB;
                COMMENT ON COLUMN "quality_issues"."root_cause_method" IS '根因分析方法：5why/fishbone/other';
                COMMENT ON COLUMN "quality_issues"."root_cause_detail" IS '根因分析过程结构化存储（5why层级/鱼骨图分支）';

                -- ============================================================
                -- P1 batch-18 缺陷 4.3：纠正预防措施跟踪
                -- ============================================================
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_owner" INTEGER;
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_due_date" DATE;
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_completed_at" TIMESTAMP;
                COMMENT ON COLUMN "quality_issues"."permanent_action_owner" IS '永久措施责任人user_id';
                COMMENT ON COLUMN "quality_issues"."permanent_action_due_date" IS '永久措施完成截止日期';
                COMMENT ON COLUMN "quality_issues"."permanent_action_completed_at" IS '永久措施实际完成时间';

                -- ============================================================
                -- P1 batch-18 缺陷 5.1：降级联动库存等级同步标记
                -- ============================================================
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_grade_synced" BOOLEAN NOT NULL DEFAULT FALSE;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER;
                COMMENT ON COLUMN "unqualified_products"."stock_grade_synced" IS '是否已同步库存等级（B级降级时联动 inventory_stocks.grade）';
                COMMENT ON COLUMN "unqualified_products"."stock_id" IS '关联库存记录ID（用于降级时定位库存）';

                -- ============================================================
                -- P1 batch-18 缺陷 5.3：报废二级审批（财务+总经理）
                -- ============================================================
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_approval_status" VARCHAR(20) NOT NULL DEFAULT 'not_required';
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_fin" INTEGER;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_gm" INTEGER;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_fin" TIMESTAMP;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_gm" TIMESTAMP;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_loss_amount" DECIMAL(12,2);
                COMMENT ON COLUMN "unqualified_products"."scrap_approval_status" IS '报废审批状态：not_required/pending_fin/pending_gm/approved/rejected';
                COMMENT ON COLUMN "unqualified_products"."approver_id_fin" IS '财务审批人user_id';
                COMMENT ON COLUMN "unqualified_products"."approver_id_gm" IS '总经理审批人user_id';
                COMMENT ON COLUMN "unqualified_products"."approved_at_fin" IS '财务审批通过时间';
                COMMENT ON COLUMN "unqualified_products"."approved_at_gm" IS '总经理审批通过时间';
                COMMENT ON COLUMN "unqualified_products"."scrap_loss_amount" IS '报废损失金额（审批通过后写入成本）';

                -- ============================================================
                -- P1 batch-18 缺陷 6.1：调拨分级审批
                -- ============================================================
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approval_level" VARCHAR(20);
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_by_role" VARCHAR(50);
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(14,2) DEFAULT 0;
                COMMENT ON COLUMN "inventory_transfers"."approval_level" IS '审批层级：L1=常规/L2=经理/L3=总监';
                COMMENT ON COLUMN "inventory_transfers"."approved_by_role" IS '审批人角色记录';
                COMMENT ON COLUMN "inventory_transfers"."total_amount" IS '调拨总金额（数量×unit_cost 累计）用于分级审批';

                -- ============================================================
                -- P1 batch-18 缺陷 7.1：补货策略字段
                -- ============================================================
                ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "replenishment_strategy" VARCHAR(20) NOT NULL DEFAULT 'reorder_point';
                COMMENT ON COLUMN "inventory_stocks"."replenishment_strategy" IS '补货策略：reorder_point/eoq/mrp';

                -- ============================================================
                -- P1 batch-18 缺陷 9.1：排程基于缸号批量约束
                -- ============================================================
                ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "schedule_batch_key" VARCHAR(100);
                COMMENT ON COLUMN "production_orders"."schedule_batch_key" IS '排程批量键（按 dye_lot_no 聚合，同缸号订单合并为一个排程单元）';

                -- ============================================================
                -- P1 batch-18 缺陷 10.1：产能模型字段
                -- ============================================================
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "standard_hours_per_unit" DECIMAL(10,2);
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "equipment_count" INTEGER DEFAULT 1;
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "worker_count" INTEGER DEFAULT 1;
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "shift_hours" DECIMAL(6,2) DEFAULT 8;
                COMMENT ON COLUMN "work_centers"."standard_hours_per_unit" IS '标准工时（小时/单位）';
                COMMENT ON COLUMN "work_centers"."equipment_count" IS '设备数';
                COMMENT ON COLUMN "work_centers"."worker_count" IS '人员数';
                COMMENT ON COLUMN "work_centers"."shift_hours" IS '班次工时（小时）';

                -- ============================================================
                -- P1 batch-18 缺陷 11.3：调度异常自动重排开关
                -- ============================================================
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "auto_reschedule_enabled" BOOLEAN NOT NULL DEFAULT TRUE;
                COMMENT ON COLUMN "work_centers"."auto_reschedule_enabled" IS '工作中心状态异常时是否自动重排受影响订单';

                -- ============================================================
                -- P1 batch-18 缺陷 11.1：工作中心关联实体表
                -- ============================================================

                -- 工作中心-设备关联表
                CREATE TABLE IF NOT EXISTS "work_center_equipment" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "equipment_name" VARCHAR(100) NOT NULL,
                    "equipment_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "capacity_per_hour" DECIMAL(10,2),
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_equipment_wc" ON "work_center_equipment"("work_center_id");

                -- 工作中心-人员关联表（含多技能）
                CREATE TABLE IF NOT EXISTS "work_center_worker" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "user_id" INTEGER NOT NULL,
                    "skills" JSONB,
                    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_worker_wc" ON "work_center_worker"("work_center_id");
                CREATE INDEX IF NOT EXISTS "idx_work_center_worker_user" ON "work_center_worker"("user_id");

                -- 工作中心-班次关联表
                CREATE TABLE IF NOT EXISTS "work_center_shift" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "shift_name" VARCHAR(50) NOT NULL,
                    "start_time" TIME NOT NULL,
                    "end_time" TIME NOT NULL,
                    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_shift_wc" ON "work_center_shift"("work_center_id");

                -- ============================================================
                -- P1 batch-18 缺陷 3.3：piece_mapping 表删除（改用 inventory_piece.parent_piece_id）
                -- ============================================================
                DROP TABLE IF EXISTS "piece_mapping";
                "#,
            )
            .await?;

        Ok(())
        // === m0079_batch08_compliance_legal_env_tax_labor.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- 缺陷 10：销售合同电子签章字段（《电子签名法》合规）
                -- ============================================================
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMP;
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_by_user_id" INTEGER;
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_hash" VARCHAR(64);
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_image_url" VARCHAR(500);
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_certificate" TEXT;
                COMMENT ON COLUMN "sales_contracts"."signed_at" IS '电子签章时间';
                COMMENT ON COLUMN "sales_contracts"."signed_by_user_id" IS '签章人用户ID';
                COMMENT ON COLUMN "sales_contracts"."signature_hash" IS '合同内容哈希（SHA-256，防篡改）';
                COMMENT ON COLUMN "sales_contracts"."signature_image_url" IS '电子签章图片URL';
                COMMENT ON COLUMN "sales_contracts"."signature_certificate" IS 'CA证书内容（PEM格式）';

                -- ============================================================
                -- 缺陷 13：委外凭证进项税转出字段（增值税合规）
                -- ============================================================
                ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "tax_transfer_amount" DECIMAL(14,4) NOT NULL DEFAULT 0;
                COMMENT ON COLUMN "outsourcing_voucher"."tax_transfer_amount" IS '进项税转出金额（非正常损耗对应的已抵扣进项税转出）';

                -- ============================================================
                -- 缺陷 14：出口退税（免抵退）核算表
                -- 依据：财税[2012]39号 出口货物劳务增值税和消费税政策
                -- ============================================================

                -- 出口报关单
                CREATE TABLE IF NOT EXISTS "export_customs_declarations" (
                    "id" SERIAL PRIMARY KEY,
                    "declaration_no" VARCHAR(50) NOT NULL UNIQUE,
                    "sales_order_id" INTEGER,
                    "customer_id" INTEGER,
                    "product_id" INTEGER,
                    "export_date" DATE NOT NULL,
                    "destination_country" VARCHAR(100),
                    "currency_code" VARCHAR(10),
                    "total_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exchange_rate" DECIMAL(10,4) NOT NULL DEFAULT 1,
                    "customs_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_export_customs_dec_order" ON "export_customs_declarations"("sales_order_id");
                CREATE INDEX IF NOT EXISTS "idx_export_customs_dec_status" ON "export_customs_declarations"("status");

                -- 外汇核销单
                CREATE TABLE IF NOT EXISTS "foreign_exchange_verifications" (
                    "id" SERIAL PRIMARY KEY,
                    "verification_no" VARCHAR(50) NOT NULL UNIQUE,
                    "customs_declaration_id" INTEGER,
                    "sales_order_id" INTEGER,
                    "verification_date" DATE NOT NULL,
                    "foreign_currency_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "rmb_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exchange_rate" DECIMAL(10,4) NOT NULL DEFAULT 1,
                    "bank_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_fx_verification_customs" ON "foreign_exchange_verifications"("customs_declaration_id");

                -- 出口退税申报表（免抵退）
                CREATE TABLE IF NOT EXISTS "export_refund_declarations" (
                    "id" SERIAL PRIMARY KEY,
                    "declaration_no" VARCHAR(50) NOT NULL UNIQUE,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "declaration_date" DATE NOT NULL,
                    "export_sales_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "refundable_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exempt_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "credit_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "actual_refund_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "carryforward_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "refund_rate" DECIMAL(6,4) NOT NULL DEFAULT 0,
                    "documents_complete" BOOLEAN NOT NULL DEFAULT FALSE,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_export_refund_period" ON "export_refund_declarations"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_export_refund_status" ON "export_refund_declarations"("status");

                -- ============================================================
                -- 缺陷 15：污染物排放记录表（环保税核算基础）
                -- 依据：《环境保护税法》印染企业废水/废气/固废排放
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "pollutant_discharge_records" (
                    "id" SERIAL PRIMARY KEY,
                    "discharge_type" VARCHAR(20) NOT NULL,
                    "pollutant_name" VARCHAR(50) NOT NULL,
                    "discharge_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
                    "discharge_unit" VARCHAR(20) NOT NULL DEFAULT 'kg',
                    "concentration" DECIMAL(14,4),
                    "concentration_unit" VARCHAR(20),
                    "tax_unit_equivalent" DECIMAL(14,4),
                    "tax_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "monitoring_point" VARCHAR(200),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollutant_discharge_period" ON "pollutant_discharge_records"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_discharge_type" ON "pollutant_discharge_records"("discharge_type");

                -- ============================================================
                -- 缺陷 18：排污许可证登记表
                -- 依据：《环境保护法》第45条 + 《排污许可管理条例》
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "pollution_permits" (
                    "id" SERIAL PRIMARY KEY,
                    "permit_no" VARCHAR(100) NOT NULL UNIQUE,
                    "permit_type" VARCHAR(30) NOT NULL,
                    "permit_category" VARCHAR(30),
                    "issue_date" DATE NOT NULL,
                    "expiry_date" DATE NOT NULL,
                    "issuing_authority" VARCHAR(200) NOT NULL,
                    "permitted_capacity" DECIMAL(14,4),
                    "capacity_unit" VARCHAR(20),
                    "permitted_pollutants" JSONB,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollution_permit_status" ON "pollution_permits"("status");
                CREATE INDEX IF NOT EXISTS "idx_pollution_permit_expiry" ON "pollution_permits"("expiry_date");

                -- ============================================================
                -- 缺陷 19：污染物监测记录表 + 固废处置联单
                -- 依据：《水污染防治法》《大气污染防治法》《固废污染防治法》
                -- ============================================================

                -- 污染物监测记录（实时/定期监测）
                CREATE TABLE IF NOT EXISTS "pollutant_monitoring_records" (
                    "id" SERIAL PRIMARY KEY,
                    "monitoring_type" VARCHAR(20) NOT NULL,
                    "monitoring_point" VARCHAR(200) NOT NULL,
                    "pollutant_name" VARCHAR(50) NOT NULL,
                    "measured_value" DECIMAL(14,4) NOT NULL,
                    "unit" VARCHAR(20) NOT NULL,
                    "limit_value" DECIMAL(14,4) NOT NULL,
                    "is_exceeding" BOOLEAN NOT NULL DEFAULT FALSE,
                    "exceeding_ratio" DECIMAL(10,4),
                    "monitoring_time" TIMESTAMP NOT NULL,
                    "monitoring_method" VARCHAR(50),
                    "equipment_id" INTEGER,
                    "operator_id" INTEGER,
                    "remarks" TEXT,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_type" ON "pollutant_monitoring_records"("monitoring_type");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_time" ON "pollutant_monitoring_records"("monitoring_time");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_exceeding" ON "pollutant_monitoring_records"("is_exceeding");

                -- 固废处置联单（危废转移联单制度）
                CREATE TABLE IF NOT EXISTS "solid_waste_disposal_records" (
                    "id" SERIAL PRIMARY KEY,
                    "manifest_no" VARCHAR(50) NOT NULL UNIQUE,
                    "waste_type" VARCHAR(50) NOT NULL,
                    "waste_category" VARCHAR(30) NOT NULL,
                    "waste_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
                    "waste_unit" VARCHAR(20) NOT NULL DEFAULT 'ton',
                    "generation_date" DATE NOT NULL,
                    "disposal_date" DATE,
                    "disposal_method" VARCHAR(50) NOT NULL,
                    "disposal_vendor_id" INTEGER,
                    "disposal_vendor_name" VARCHAR(200),
                    "transport_license_no" VARCHAR(100),
                    "disposal_license_no" VARCHAR(100),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_solid_waste_status" ON "solid_waste_disposal_records"("status");
                CREATE INDEX IF NOT EXISTS "idx_solid_waste_type" ON "solid_waste_disposal_records"("waste_type");

                -- ============================================================
                -- 缺陷 21：劳动合同电子化管理
                -- 依据：《劳动法》《劳动合同法》第10/19/20条
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "labor_contracts" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
                    "contract_type" VARCHAR(30) NOT NULL,
                    "start_date" DATE NOT NULL,
                    "end_date" DATE,
                    "probation_end_date" DATE,
                    "probation_salary" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "regular_salary" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "position" VARCHAR(100),
                    "department" VARCHAR(100),
                    "work_location" VARCHAR(200),
                    "working_hours_system" VARCHAR(30) NOT NULL DEFAULT 'standard',
                    "sign_date" DATE NOT NULL,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "termination_date" DATE,
                    "termination_reason" TEXT,
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_worker" ON "labor_contracts"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_status" ON "labor_contracts"("status");
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_end_date" ON "labor_contracts"("end_date");

                -- ============================================================
                -- 缺陷 23：社保公积金缴纳记录
                -- 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "social_insurance_records" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "base_amount" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "pension_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "pension_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "medical_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "medical_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "unemployment_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "unemployment_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "work_injury_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "maternity_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "housing_fund_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "housing_fund_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "total_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "total_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "payment_date" DATE,
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_social_insurance_worker" ON "social_insurance_records"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_social_insurance_period" ON "social_insurance_records"("period_year", "period_month");

                -- ============================================================
                -- 缺陷 24：职业健康合规
                -- 依据：《职业病防治法》第26/35条 + 《危险化学品安全管理条例》
                -- ============================================================

                -- 职业危害因素检测记录（苯/甲醛/噪声/粉尘）
                CREATE TABLE IF NOT EXISTS "occupational_hazard_monitorings" (
                    "id" SERIAL PRIMARY KEY,
                    "hazard_type" VARCHAR(30) NOT NULL,
                    "hazard_name" VARCHAR(50) NOT NULL,
                    "monitoring_point" VARCHAR(200) NOT NULL,
                    "measured_value" DECIMAL(14,4) NOT NULL,
                    "unit" VARCHAR(20) NOT NULL,
                    "limit_value" DECIMAL(14,4) NOT NULL,
                    "is_exceeding" BOOLEAN NOT NULL DEFAULT FALSE,
                    "exceeding_ratio" DECIMAL(10,4),
                    "monitoring_date" DATE NOT NULL,
                    "monitoring_organization" VARCHAR(200),
                    "monitoring_method" VARCHAR(100),
                    "report_url" VARCHAR(500),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_type" ON "occupational_hazard_monitorings"("hazard_type");
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_date" ON "occupational_hazard_monitorings"("monitoring_date");
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_exceeding" ON "occupational_hazard_monitorings"("is_exceeding");

                -- 职业健康体检档案（上岗前/在岗期间/离岗时）
                CREATE TABLE IF NOT EXISTS "occupational_health_exams" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "exam_type" VARCHAR(30) NOT NULL,
                    "exam_date" DATE NOT NULL,
                    "next_exam_date" DATE,
                    "exam_organization" VARCHAR(200),
                    "exam_result" VARCHAR(20) NOT NULL DEFAULT 'normal',
                    "hazard_exposure" JSONB,
                    "contraindications" TEXT,
                    "report_url" VARCHAR(500),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_worker" ON "occupational_health_exams"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_type" ON "occupational_health_exams"("exam_type");
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_next" ON "occupational_health_exams"("next_exam_date");

                -- 防护用品配备记录（PPE）
                CREATE TABLE IF NOT EXISTS "ppe_distribution_records" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "ppe_name" VARCHAR(100) NOT NULL,
                    "ppe_type" VARCHAR(50) NOT NULL,
                    "specification" VARCHAR(100),
                    "quantity" INTEGER NOT NULL DEFAULT 1,
                    "distribution_date" DATE NOT NULL,
                    "expiry_date" DATE,
                    "hazard_type" VARCHAR(30),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'distributed',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_ppe_worker" ON "ppe_distribution_records"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_ppe_date" ON "ppe_distribution_records"("distribution_date");
                "#,
            )
            .await?;

        Ok(())
        // === m0080_create_collection_templates.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 催收模板表（V15 P1 17.3-D5 创建）
                -- 按催收类型与逾期阶段配置标准化话术
                CREATE TABLE IF NOT EXISTS "collection_templates" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "name" VARCHAR(100) NOT NULL,
                    "task_type" VARCHAR(20) NOT NULL,
                    "overdue_stage" VARCHAR(20) NOT NULL DEFAULT 'all',
                    "title" VARCHAR(200),
                    "content" TEXT NOT NULL,
                    "is_enabled" BOOLEAN NOT NULL DEFAULT TRUE,
                    "sort_order" INTEGER NOT NULL DEFAULT 0,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "uk_ct_name" UNIQUE ("name"),
                    CONSTRAINT "chk_ct_task_type" CHECK (
                        "task_type" IN ('phone', 'visit', 'email', 'letter')
                    ),
                    CONSTRAINT "chk_ct_overdue_stage" CHECK (
                        "overdue_stage" IN ('early', 'middle', 'late', 'all')
                    )
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_ctpl_task_type" ON "collection_templates"("task_type");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_overdue_stage" ON "collection_templates"("overdue_stage");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_is_enabled" ON "collection_templates"("is_enabled");

                COMMENT ON TABLE "collection_templates" IS '催收模板表 - 按类型/阶段配置标准化话术';
                COMMENT ON COLUMN "collection_templates"."task_type" IS '催收类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)';
                COMMENT ON COLUMN "collection_templates"."overdue_stage" IS '适用逾期阶段：early(0-30天) / middle(31-90天) / late(90+天) / all(全部)';

                -- 预置 4 套默认模板（按催收类型）
                INSERT INTO "collection_templates" ("name", "task_type", "overdue_stage", "title", "content", "is_enabled", "sort_order", "remark") VALUES
                ('电话催收-早期模板', 'phone', 'early', NULL, '您好，我是XX公司的财务专员，您有一笔账款已逾期{overdue_days
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
