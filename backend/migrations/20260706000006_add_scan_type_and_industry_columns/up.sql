-- v11 批次 153 P2-A：为 inventory_piece 表添加 scan_type 列，支持扫码历史按类型筛选
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS scan_type VARCHAR(50);

-- v11 批次 153 P2-A：为 crm_lead 表添加 industry 列，支持客户池按行业筛选
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS industry VARCHAR(100);
