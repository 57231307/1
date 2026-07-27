-- ============================================================================
-- Migration 055: V15 P1 工资明细新增加班工时字段
-- 依据：V15 审计报告 类八 P1（batch-08 维度 8.8 缺陷项 22：工时与加班合规缺失）
-- 业务背景：《劳动法》第 41 条规定每月加班不得超过 36 小时；
--   第 44 条规定加班工资支付标准：工作日 1.5 倍、休息日 2 倍（不能补休）、法定节假日 3 倍。
--   当前 wage_record_detail 表仅有 duration_minutes，无法区分正常工时与加班工时，
--   导致加班费计算错误、月加班超 36 小时无预警，违反《劳动法》。
-- 修复策略：
--   1. wage_record_detail 表新增 weekday_overtime_minutes/weekend_overtime_minutes/
--      holiday_overtime_minutes/overtime_pay 四个字段
--   2. wage_service.calculate_wage_for_step 增加加班费计算逻辑
--   3. wage_calculation_service 月加班超 36 小时预警（后续批次）
-- 关联文件：backend/src/models/wage_record_detail.rs
--             backend/src/services/wage_service.rs
--             backend/src/services/wage_ops/calculation.rs
-- ============================================================================

-- ============================================================================
-- 1. wage_record_detail 表新增加班工时字段
-- ============================================================================
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekday_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekend_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "holiday_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "overtime_pay" DECIMAL(12, 2) NOT NULL DEFAULT 0.00;

-- ============================================================================
-- 2. 字段注释
-- ============================================================================
COMMENT ON COLUMN "wage_record_detail"."weekday_overtime_minutes" IS '工作日加班工时（分钟，《劳动法》第 44 条第 1 款：支付不低于工资 150% 的工资报酬）';
COMMENT ON COLUMN "wage_record_detail"."weekend_overtime_minutes" IS '休息日加班工时（分钟，《劳动法》第 44 条第 2 款：不能安排补休的支付不低于工资 200% 的工资报酬）';
COMMENT ON COLUMN "wage_record_detail"."holiday_overtime_minutes" IS '法定节假日加班工时（分钟，《劳动法》第 44 条第 3 款：支付不低于工资 300% 的工资报酬）';
COMMENT ON COLUMN "wage_record_detail"."overtime_pay" IS '加班费（weekday_ot × 1.5 + weekend_ot × 2 + holiday_ot × 3，按计时单价 × 等级系数计算）';

-- ============================================================================
-- 3. 索引（按工人查询月加班总时长，用于 36 小时预警）
-- ============================================================================
CREATE INDEX IF NOT EXISTS "idx_wage_record_detail_worker_id" ON "wage_record_detail" ("worker_id") WHERE "is_deleted" = false;
CREATE INDEX IF NOT EXISTS "idx_wage_record_detail_step_record_id" ON "wage_record_detail" ("step_record_id") WHERE "is_deleted" = false;
