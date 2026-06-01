-- 023_create_currency_tables.sql
-- 注意：currencies 和 exchange_rates 表已在 005_ar_currency.sql 中创建
-- 此文件只添加缺失的数据

-- 插入额外的币种（如果不存在）
INSERT INTO currencies (code, name, symbol, is_base, precision, is_active) VALUES
('GBP', '英镑', '£', false, 2, true),
('JPY', '日元', '¥', false, 0, true)
ON CONFLICT (code) DO NOTHING;

-- 插入额外的汇率（如果不存在）
INSERT INTO exchange_rates (from_currency, to_currency, rate, effective_date) VALUES
('CNY', 'GBP', 0.1091, CURRENT_DATE),
('GBP', 'CNY', 9.1700, CURRENT_DATE),
('CNY', 'JPY', 20.8300, CURRENT_DATE),
('JPY', 'CNY', 0.0480, CURRENT_DATE)
ON CONFLICT DO NOTHING;
