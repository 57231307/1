-- 迁移脚本: 035_v14_quality_grade_and_dyelot_validation.sql
-- 描述: v14 复审批次 421 - 面料行业质检 A/B/C 级分级判定 + 缸号同订单校验支持字段
-- 日期: 2026-07-15
-- 修复: T-P1-4（质检分级缺失：A 级合格/B 级让步接收/C 级不合格）+ T-P1-5（缸号同订单校验缺失）
-- 调研依据: fabric-industry-research.md §4.7 质量检验模块 + §2.3 约束 5 缸号同订单校验

BEGIN;

-- =====================================================
-- 1. quality_inspection_records 表：添加 grade 字段（T-P1-4）
-- =====================================================
-- 面料行业质检结果分级：A级（合格）/ B级（让步接收，降级销售）/ C级（不合格，返工或报废）
-- 依据：fabric-industry-research.md §4.7 质量检验模块
-- 业务规则：
--   A 级：qualification_rate >= 95% 或 ΔE <= 1.2，正常入库销售
--   B 级：qualification_rate >= 80% 且 < 95%，让步接收，降级销售（影响定价）
--   C 级：qualification_rate < 80%，不合格，需返工或报废
ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS grade VARCHAR(2);

-- 添加 grade 索引便于按等级筛选质检记录
CREATE INDEX IF NOT EXISTS idx_quality_inspection_records_grade
    ON quality_inspection_records(grade) WHERE grade IS NOT NULL;

-- 添加 color_no/dye_lot_no 字段支持按缸号追溯质检结果
-- 依据：fabric-industry-research.md §2.1 四层级联关系 - 每缸号独立质检
ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS color_no VARCHAR(50);
ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);

CREATE INDEX IF NOT EXISTS idx_quality_inspection_records_color_no
    ON quality_inspection_records(color_no) WHERE color_no IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_quality_inspection_records_dye_lot_no
    ON quality_inspection_records(dye_lot_no) WHERE dye_lot_no IS NOT NULL;

-- =====================================================
-- 2. unqualified_products 表：添加 grade 字段（T-P1-4）
-- =====================================================
-- 不合格品处理需记录对应等级，B 级降级销售/C 级返工报废分支处理
-- 依据：fabric-industry-research.md §4.7 - B 级让步接收降级销售，C 级不合格返工或报废
ALTER TABLE unqualified_products ADD COLUMN IF NOT EXISTS grade VARCHAR(2);

CREATE INDEX IF NOT EXISTS idx_unqualified_products_grade
    ON unqualified_products(grade) WHERE grade IS NOT NULL;

-- 添加 handling_result 字段记录最终处理结果（降级销售单价/返工工时/报废损失）
ALTER TABLE unqualified_products ADD COLUMN IF NOT EXISTS handling_result VARCHAR(50);

COMMIT;

-- 验证迁移
SELECT 'Migration 035_v14_quality_grade_and_dyelot_validation completed successfully' AS status;
