use sea_orm_migration::prelude::*;

// V15 P1 batch-08 法律合规修复（环保/劳动/财税法律合规）：
//
// 缺陷 10：sales_contracts 新增电子签章字段（signed_at/signed_by_user_id/signature_hash/signature_image_url/signature_certificate）
// 缺陷 13：outsourcing_voucher 新增进项税转出金额字段（tax_transfer_amount）
// 缺陷 14：新建出口退税相关表（export_refund_declarations/export_customs_declarations/foreign_exchange_verifications）
// 缺陷 15：新建污染物排放记录表（pollutant_discharge_records）
// 缺陷 18：新建排污许可证表（pollution_permits）
// 缺陷 19：新建污染物监测记录表（pollutant_monitoring_records/solid_waste_disposal_records）
// 缺陷 21：新建劳动合同表（labor_contracts）
// 缺陷 23：新建社保缴纳记录表（social_insurance_records）
// 缺陷 24：新建职业健康相关表（occupational_hazard_monitorings/occupational_health_exams/ppe_distribution_records）
//
// 蓝绿部署兼容性（规则 25.4-J）：
// - 所有新增字段 NULLABLE 或带 DEFAULT
// - 所有新表均允许 NULL 或带 DEFAULT 的非空字段

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS "ppe_distribution_records";
                DROP TABLE IF EXISTS "occupational_health_exams";
                DROP TABLE IF EXISTS "occupational_hazard_monitorings";
                DROP TABLE IF EXISTS "social_insurance_records";
                DROP TABLE IF EXISTS "labor_contracts";
                DROP TABLE IF EXISTS "solid_waste_disposal_records";
                DROP TABLE IF EXISTS "pollutant_monitoring_records";
                DROP TABLE IF EXISTS "pollution_permits";
                DROP TABLE IF EXISTS "pollutant_discharge_records";
                DROP TABLE IF EXISTS "export_refund_declarations";
                DROP TABLE IF EXISTS "foreign_exchange_verifications";
                DROP TABLE IF EXISTS "export_customs_declarations";

                ALTER TABLE "outsourcing_voucher" DROP COLUMN IF EXISTS "tax_transfer_amount";

                ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "signature_certificate";
                ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "signature_image_url";
                ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "signature_hash";
                ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "signed_by_user_id";
                ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "signed_at";
                "#,
            )
            .await?;

        Ok(())
    }
}
