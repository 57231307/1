-- P0-1 历史数据订正脚本（2026-06-26）
--
-- 背景：2026-06-25 综合审计发现，自动生成 AP 发票时曾误用 Decimal::new(1, 2) = 0.01
-- 作为本位币汇率，导致下游按汇率换算本位币金额被缩小 100 倍。
-- 代码侧已在 ap_invoice_service.rs 修复（常量 DEFAULT_BASE_CURRENCY_EXCHANGE_RATE = 1.0），
-- 本脚本订正修复前已落库的历史数据。
--
-- 修复范围：exchange_rate = 0.01 且币种为本位币（CNY）的 AP 发票记录。
-- 安全说明：仅订正 exchange_rate 字段，不动其他字段；幂等设计（重复执行无副作用）。
-- 前置条件：执行前请备份 ap_invoice 表。

-- 订正前：打印受影响记录数（可选，仅用于审计）
-- SELECT COUNT(*) AS affected_count FROM ap_invoice WHERE exchange_rate = 0.01 AND currency = 'CNY';

-- 订正：将历史误用的 0.01 汇率修正为 1.0
UPDATE ap_invoice
SET exchange_rate = 1.000000,
    updated_at = NOW()
WHERE exchange_rate = 0.010000
  AND currency = 'CNY';

-- 订正后：验证无残留（可选，仅用于审计）
-- SELECT COUNT(*) AS remaining_count FROM ap_invoice WHERE exchange_rate = 0.01 AND currency = 'CNY';
