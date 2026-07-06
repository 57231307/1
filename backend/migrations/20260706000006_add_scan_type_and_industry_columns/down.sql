-- v11 批次 153 P2-A：回滚 inventory_piece.scan_type 列
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS scan_type;

-- v11 批次 153 P2-A：回滚 crm_lead.industry 列
ALTER TABLE crm_lead DROP COLUMN IF EXISTS industry;
