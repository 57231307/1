-- 迁移脚本: 004_mrp_production.sql
-- 描述: MRP生产计划管理模块数据库表
-- 日期: 2026-05-09
-- 依赖: 003_foreign_keys.sql

BEGIN;

-- ========================================================
-- MRP-001: 生产订单表
-- ========================================================
CREATE TABLE IF NOT EXISTS production_orders (
    id SERIAL PRIMARY KEY,
    order_no VARCHAR(50) UNIQUE NOT NULL,
    sales_order_id INTEGER REFERENCES sales_orders(id) ON DELETE SET NULL,
    product_id INTEGER NOT NULL,
    planned_quantity DECIMAL(15,4) NOT NULL,
    actual_quantity DECIMAL(15,4),
    planned_start_date DATE,
    planned_end_date DATE,
    actual_start_date DATE,
    actual_end_date DATE,
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    priority INTEGER DEFAULT 5,
    work_center_id INTEGER,
    remarks TEXT,
    created_by INTEGER NOT NULL,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE production_orders IS '生产订单表';
COMMENT ON COLUMN production_orders.status IS '状态: DRAFT=草稿, SCHEDULED=已排产, IN_PROGRESS=生产中, COMPLETED=已完成, CANCELLED=已取消';
COMMENT ON COLUMN production_orders.priority IS '优先级（1-10，1最高）';

CREATE INDEX idx_production_orders_status ON production_orders(status);
CREATE INDEX idx_production_orders_product_id ON production_orders(product_id);
CREATE INDEX idx_production_orders_sales_order_id ON production_orders(sales_order_id) WHERE sales_order_id IS NOT NULL;
CREATE INDEX idx_production_orders_planned_dates ON production_orders(planned_start_date, planned_end_date);

-- ========================================================
-- MRP-002: BOM物料清单表
-- ========================================================
CREATE TABLE IF NOT EXISTS boms (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL,
    version INTEGER DEFAULT 1,
    is_default BOOLEAN DEFAULT false,
    status VARCHAR(20) DEFAULT 'ACTIVE',
    remarks TEXT,
    created_by INTEGER NOT NULL,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE boms IS 'BOM物料清单表';
COMMENT ON COLUMN boms.status IS '状态: ACTIVE=生效中, INACTIVE=已失效, PENDING=审核中';

CREATE INDEX idx_boms_product_id ON boms(product_id);
CREATE INDEX idx_boms_status ON boms(status);
CREATE UNIQUE INDEX idx_boms_default ON boms(product_id) WHERE is_default = true AND is_deleted = false;

-- BOM明细表
CREATE TABLE IF NOT EXISTS bom_items (
    id SERIAL PRIMARY KEY,
    bom_id INTEGER REFERENCES boms(id) ON DELETE CASCADE,
    material_id INTEGER NOT NULL,
    quantity DECIMAL(15,4) NOT NULL,
    unit VARCHAR(20),
    scrap_rate DECIMAL(5,4) DEFAULT 0,
    sort_order INTEGER,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE bom_items IS 'BOM物料清单明细表';

CREATE INDEX idx_bom_items_bom_id ON bom_items(bom_id);
CREATE INDEX idx_bom_items_material_id ON bom_items(material_id);

-- ========================================================
-- MRP-004: MRP计算结果表
-- ========================================================
CREATE TABLE IF NOT EXISTS mrp_results (
    id SERIAL PRIMARY KEY,
    calculation_no VARCHAR(50) UNIQUE NOT NULL,
    product_id INTEGER NOT NULL,
    required_quantity DECIMAL(15,4) NOT NULL,
    required_date DATE,
    source_type VARCHAR(20) NOT NULL,
    source_id INTEGER,
    planned_order_quantity DECIMAL(15,4),
    planned_order_date DATE,
    status VARCHAR(20) DEFAULT 'PLANNED',
    remarks TEXT,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE mrp_results IS 'MRP计算结果表';
COMMENT ON COLUMN mrp_results.source_type IS '来源类型: SALES_ORDER=销售订单, FORECAST=预测, SAFETY_STOCK=安全库存';
COMMENT ON COLUMN mrp_results.status IS '状态: PLANNED=计划中, CONFIRMED=已确认, RELEASED=已下达, COMPLETED=已完成';

CREATE INDEX idx_mrp_results_product_id ON mrp_results(product_id);
CREATE INDEX idx_mrp_results_status ON mrp_results(status);
CREATE INDEX idx_mrp_results_required_date ON mrp_results(required_date);

-- ========================================================
-- MRP-005: 工作中心表
-- ========================================================
CREATE TABLE IF NOT EXISTS work_centers (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    work_center_type VARCHAR(50),
    daily_capacity DECIMAL(15,4),
    capacity_unit VARCHAR(20),
    status VARCHAR(20) DEFAULT 'ACTIVE',
    remarks TEXT,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE work_centers IS '工作中心表';
COMMENT ON COLUMN work_centers.status IS '状态: ACTIVE=正常, MAINTENANCE=维修中, INACTIVE=停用';

CREATE INDEX idx_work_centers_status ON work_centers(status);

COMMIT;

-- 验证迁移
-- SELECT 'Migration 004_mrp_production completed successfully' AS status;
