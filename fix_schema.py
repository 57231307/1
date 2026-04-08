import re

SQL_FILE = '/workspace/backend/database/migration/001_consolidated_schema.sql'

patches = """
-- =========================================================
-- ADDITIONAL COMPATIBILITY PATCHES (ROUND 2)
-- =========================================================

-- ap_reconciliation
ALTER TABLE ap_reconciliation ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- batch_dye_lot
ALTER TABLE batch_dye_lot ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50);
ALTER TABLE batch_dye_lot ADD COLUMN IF NOT EXISTS dye_date TIMESTAMPTZ;
ALTER TABLE batch_dye_lot ADD COLUMN IF NOT EXISTS quantity DECIMAL(12,2);
ALTER TABLE batch_dye_lot ADD COLUMN IF NOT EXISTS color_code VARCHAR(50);
ALTER TABLE batch_dye_lot ADD COLUMN IF NOT EXISTS status VARCHAR(20);

-- batch_trace_log
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS source_type VARCHAR(50);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS source_id INTEGER;
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS source_no VARCHAR(50);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS quantity DECIMAL(12,2);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS quantity_before DECIMAL(12,2);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS quantity_after DECIMAL(12,2);
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS operated_by INTEGER;
ALTER TABLE batch_trace_log ADD COLUMN IF NOT EXISTS operated_at TIMESTAMPTZ;

-- bpm_process_definition
ALTER TABLE bpm_process_definition ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE bpm_process_definition ADD COLUMN IF NOT EXISTS code VARCHAR(50);
ALTER TABLE bpm_process_definition ADD COLUMN IF NOT EXISTS category VARCHAR(50);
ALTER TABLE bpm_process_definition ADD COLUMN IF NOT EXISTS version INTEGER;
ALTER TABLE bpm_process_definition ADD COLUMN IF NOT EXISTS config JSONB;

-- bpm_process_instance
ALTER TABLE bpm_process_instance ADD COLUMN IF NOT EXISTS applicant_id INTEGER;
ALTER TABLE bpm_process_instance ADD COLUMN IF NOT EXISTS business_no VARCHAR(50);
ALTER TABLE bpm_process_instance ADD COLUMN IF NOT EXISTS current_node VARCHAR(50);
ALTER TABLE bpm_process_instance ADD COLUMN IF NOT EXISTS start_time TIMESTAMPTZ;
ALTER TABLE bpm_process_instance ADD COLUMN IF NOT EXISTS end_time TIMESTAMPTZ;

-- bpm_task
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS process_instance_id INTEGER;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS assignee_id INTEGER;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS candidate_ids JSONB;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS business_type VARCHAR(50);
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS business_id INTEGER;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS comment TEXT;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ;
ALTER TABLE bpm_task ADD COLUMN IF NOT EXISTS due_time TIMESTAMPTZ;

-- budget_adjustments
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS budget_id INTEGER;
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS adjustment_date TIMESTAMPTZ;
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS adjustment_type VARCHAR(50);
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS amount DECIMAL(12,2);
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS budget_before DECIMAL(12,2);
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS budget_after DECIMAL(12,2);
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS approval_status VARCHAR(20);
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS remarks TEXT;
ALTER TABLE budget_adjustments ADD COLUMN IF NOT EXISTS created_by INTEGER;

-- budget_plans
ALTER TABLE budget_plans ADD COLUMN IF NOT EXISTS start_date TIMESTAMPTZ;
ALTER TABLE budget_plans ADD COLUMN IF NOT EXISTS end_date TIMESTAMPTZ;
ALTER TABLE budget_plans ADD COLUMN IF NOT EXISTS created_by INTEGER;

-- color_code_mapping
ALTER TABLE color_code_mapping ADD COLUMN IF NOT EXISTS product_color_id INTEGER;
ALTER TABLE color_code_mapping ADD COLUMN IF NOT EXISTS customer_code VARCHAR(50);
ALTER TABLE color_code_mapping ADD COLUMN IF NOT EXISTS customer_name VARCHAR(100);
ALTER TABLE color_code_mapping ADD COLUMN IF NOT EXISTS customer_color_code VARCHAR(50);
ALTER TABLE color_code_mapping ADD COLUMN IF NOT EXISTS customer_color_name VARCHAR(100);

-- crm_lead
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS customer_name VARCHAR(100);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS contact_person VARCHAR(100);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS contact_phone VARCHAR(50);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS source VARCHAR(50);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS status VARCHAR(20);
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS remarks TEXT;

-- crm_opportunity
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS amount DECIMAL(12,2);
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS stage VARCHAR(50);
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS source VARCHAR(50);
ALTER TABLE crm_opportunity ADD COLUMN IF NOT EXISTS remarks TEXT;

-- dye_lot_mapping
ALTER TABLE dye_lot_mapping ADD COLUMN IF NOT EXISTS dye_batch_id INTEGER;
ALTER TABLE dye_lot_mapping ADD COLUMN IF NOT EXISTS lot_no VARCHAR(50);

-- finance_invoices
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS customer_id INTEGER;
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS customer_name VARCHAR(100);
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS invoice_type VARCHAR(50);
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS due_date TIMESTAMPTZ;

-- fund_accounts
ALTER TABLE fund_accounts ADD COLUMN IF NOT EXISTS bank_account VARCHAR(50);
ALTER TABLE fund_accounts ADD COLUMN IF NOT EXISTS remarks TEXT;
ALTER TABLE fund_accounts ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;
ALTER TABLE fund_accounts ADD COLUMN IF NOT EXISTS created_by INTEGER;

-- inventory_count_items
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS stock_id INTEGER;
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS warehouse_id INTEGER;
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS quantity_before DECIMAL(12,2);
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS quantity_difference DECIMAL(12,2);
ALTER TABLE inventory_count_items ADD COLUMN IF NOT EXISTS total_cost DECIMAL(12,2);

-- inventory_piece
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50);
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS product_id INTEGER;
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS location_id INTEGER;
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS status VARCHAR(20);

-- log_login
ALTER TABLE log_login ADD COLUMN IF NOT EXISTS status VARCHAR(20);
ALTER TABLE log_login ADD COLUMN IF NOT EXISTS fail_reason TEXT;

-- log_system
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS user_id INTEGER;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS username VARCHAR(50);
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS module VARCHAR(50);
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS operation VARCHAR(100);
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS method VARCHAR(20);
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS path TEXT;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS params JSONB;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS status_code INTEGER;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS ip_address VARCHAR(50);
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS user_agent TEXT;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS error_message TEXT;
ALTER TABLE log_system ADD COLUMN IF NOT EXISTS execution_time INTEGER;

-- oa_announcement
ALTER TABLE oa_announcement ADD COLUMN IF NOT EXISTS expiry_date TIMESTAMPTZ;
ALTER TABLE oa_announcement ADD COLUMN IF NOT EXISTS attachments JSONB;
ALTER TABLE oa_announcement ADD COLUMN IF NOT EXISTS remarks TEXT;

-- piece_mapping
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50);
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS product_id INTEGER;
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS piece_no VARCHAR(50);
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS length DECIMAL(10,2);
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS weight DECIMAL(10,2);
ALTER TABLE piece_mapping ADD COLUMN IF NOT EXISTS status VARCHAR(20);

-- product_code_mapping
ALTER TABLE product_code_mapping ADD COLUMN IF NOT EXISTS product_id INTEGER;
ALTER TABLE product_code_mapping ADD COLUMN IF NOT EXISTS customer_code VARCHAR(50);
ALTER TABLE product_code_mapping ADD COLUMN IF NOT EXISTS customer_name VARCHAR(100);
ALTER TABLE product_code_mapping ADD COLUMN IF NOT EXISTS customer_product_code VARCHAR(50);
ALTER TABLE product_code_mapping ADD COLUMN IF NOT EXISTS customer_product_name VARCHAR(100);

-- purchase_inspection
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS purchase_order_id INTEGER;
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS result VARCHAR(20);
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS qualified_quantity DECIMAL(12,2);
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS unqualified_quantity DECIMAL(12,2);
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS unqualified_reason TEXT;
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS remarks TEXT;
ALTER TABLE purchase_inspection ADD COLUMN IF NOT EXISTS created_by INTEGER;

-- purchase_receipt_item
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS internal_dye_lot_no VARCHAR(50);
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS internal_piece_ids JSONB;
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS internal_piece_nos JSONB;
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS supplier_dye_lot_no VARCHAR(50);
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS supplier_piece_nos JSONB;
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS batch_conversion_log_id INTEGER;

-- purchase_return
ALTER TABLE purchase_return ADD COLUMN IF NOT EXISTS purchase_order_id INTEGER;
ALTER TABLE purchase_return ADD COLUMN IF NOT EXISTS reason TEXT;
ALTER TABLE purchase_return ADD COLUMN IF NOT EXISTS status VARCHAR(20);
ALTER TABLE purchase_return ADD COLUMN IF NOT EXISTS remarks TEXT;

-- report_definition
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS report_code VARCHAR(50);
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS name VARCHAR(100);
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS data_source VARCHAR(50);
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS sql_query TEXT;
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS config JSONB;
ALTER TABLE report_definition ADD COLUMN IF NOT EXISTS description TEXT;

-- role_permissions
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS resource_type VARCHAR(50);
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS resource_id INTEGER;
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS action VARCHAR(50);
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS allowed BOOLEAN;
ALTER TABLE role_permissions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;

-- sales_order_items (wait, earlier it was sales_order_item)
ALTER TABLE sales_order_item ADD COLUMN IF NOT EXISTS total_amount DECIMAL(12,2);

-- MISSING TABLES
CREATE TABLE IF NOT EXISTS budget_executions (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS business_trace_assist_links (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS business_traces (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS dye_batch (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS dye_recipe (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS fixed_asset_disposals (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS fund_transfer_records (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS greige_fabric (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS log_api_accesses (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS purchase_contract_executions (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS sales_contract_executions (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS sales_return (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS sales_return_item (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS system_version (id SERIAL PRIMARY KEY, created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP);
"""

with open(SQL_FILE, 'a', encoding='utf-8') as f:
    f.write(patches)

print("Applied round 2 patches.")
