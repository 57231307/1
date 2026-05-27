-- 回滚批次4：供应商管理扩展 + 产品扩展
-- 创建时间: 2026-05-27
-- 描述: 删除供应商管理扩展表和产品扩展表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "product_supplier_mappings" CASCADE;
DROP TABLE IF EXISTS "piece_mapping" CASCADE;
DROP TABLE IF EXISTS "color_code_mapping" CASCADE;
DROP TABLE IF EXISTS "product_code_mapping" CASCADE;
DROP TABLE IF EXISTS "product_colors" CASCADE;
DROP TABLE IF EXISTS "supplier_product_colors" CASCADE;
DROP TABLE IF EXISTS "supplier_products" CASCADE;
DROP TABLE IF EXISTS "supplier_qualifications" CASCADE;
DROP TABLE IF EXISTS "supplier_blacklists" CASCADE;
DROP TABLE IF EXISTS "supplier_contacts" CASCADE;
