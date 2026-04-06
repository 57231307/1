-- 采购管理模块数据库迁移脚本
-- 创建时间：2026-03-15
-- 功能说明：创建采购订单、入库、退货、质检相关表及索引

-- =====================================================
-- 1. 采购订单表 (purchase_order)
-- =====================================================
CREATE TABLE purchase_order (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    order_no VARCHAR(50) NOT NULL UNIQUE,               -- 订单编号（PO20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    order_date DATE NOT NULL,                           -- 订单日期
    expected_delivery_date DATE,                        -- 预计交货日期
    actual_delivery_date DATE,                          -- 实际交货日期
    warehouse_id INTEGER NOT NULL,                      -- 入库仓库 ID
    department_id INTEGER NOT NULL,                     -- 采购部门 ID
    purchaser_id INTEGER NOT NULL,                      -- 采购员 ID
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 订单总金额（本位币）
    total_amount_foreign DECIMAL(18,2) DEFAULT 0.00,    -- 订单总金额（外币）
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总数量（辅助单位）
    order_status VARCHAR(20) DEFAULT 'DRAFT',           -- 订单状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，PARTIAL_RECEIVED=部分入库，COMPLETED=已完成，CLOSED=已关闭
    payment_terms TEXT,                                 -- 付款条件
    shipping_terms TEXT,                                -- 运输条款
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_purchaser
    FOREIGN KEY (purchaser_id) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);

-- 索引
CREATE INDEX idx_po_order_no ON purchase_order(order_no);
CREATE INDEX idx_po_supplier_id ON purchase_order(supplier_id);
CREATE INDEX idx_po_order_date ON purchase_order(order_date);
CREATE INDEX idx_po_order_status ON purchase_order(order_status);
CREATE INDEX idx_po_expected_delivery ON purchase_order(expected_delivery_date);
CREATE INDEX idx_po_created_by ON purchase_order(created_by);

-- 触发器：更新时间
CREATE TRIGGER update_purchase_order_updated_at
BEFORE UPDATE ON purchase_order
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 2. 采购订单明细表 (purchase_order_item)
-- =====================================================
CREATE TABLE purchase_order_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    order_id INTEGER NOT NULL,                          -- 订单 ID（外键）
    line_no INTEGER NOT NULL,                           -- 行号（10, 20, 30...）
    product_id INTEGER NOT NULL,                        -- 产品 ID
    material_code VARCHAR(50) NOT NULL,                 -- 物料编码
    material_name VARCHAR(200) NOT NULL,                -- 物料名称
    specification VARCHAR(200),                         -- 规格型号
    batch_no VARCHAR(50),                               -- 批次号（面料行业）
    color_code VARCHAR(50),                             -- 色号（面料行业）
    lot_no VARCHAR(50),                                 -- 缸号（面料行业）
    grade VARCHAR(10),                                  -- 等级（面料行业）
    gram_weight DECIMAL(8,2),                           -- 克重（g/m²）
    width DECIMAL(8,2),                                 -- 幅宽（cm）
    unit_price DECIMAL(18,6) NOT NULL,                  -- 单价
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    quantity_ordered DECIMAL(18,4) NOT NULL,            -- 订购数量（主单位）
    quantity_received DECIMAL(18,4) DEFAULT 0.0000,     -- 已入库数量（主单位）
    quantity_returned DECIMAL(18,4) DEFAULT 0.0000,     -- 已退货数量（主单位）
    unit_master VARCHAR(20) NOT NULL,                   -- 主单位（米）
    unit_alt VARCHAR(20),                               -- 辅助单位（公斤）
    conversion_factor DECIMAL(18,6),                    -- 换算系数
    quantity_alt_ordered DECIMAL(18,4),                 -- 订购数量（辅助单位）
    quantity_alt_received DECIMAL(18,4),                -- 已入库数量（辅助单位）
    amount DECIMAL(18,2) NOT NULL,                      -- 金额
    tax_rate DECIMAL(5,2) DEFAULT 13.00,                -- 税率（%）
    tax_amount DECIMAL(18,2) DEFAULT 0.00,              -- 税额
    delivery_date DATE,                                 -- 交货日期
    warehouse_id INTEGER,                               -- 仓库 ID
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 更新时间
);

-- 外键约束
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id) ON DELETE CASCADE;
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_material
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);

-- 唯一约束
ALTER TABLE purchase_order_item ADD CONSTRAINT uk_poi_order_line
    UNIQUE (order_id, line_no);

-- 索引
CREATE INDEX idx_poi_order_id ON purchase_order_item(order_id);
CREATE INDEX idx_poi_material_id ON purchase_order_item(material_id);
CREATE INDEX idx_poi_batch_no ON purchase_order_item(batch_no);
CREATE INDEX idx_poi_color_code ON purchase_order_item(color_code);
CREATE INDEX idx_poi_lot_no ON purchase_order_item(lot_no);

-- 触发器：更新时间
CREATE TRIGGER update_purchase_order_item_updated_at
BEFORE UPDATE ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 触发器：计算金额
CREATE OR REPLACE FUNCTION calc_purchase_order_item_amount()
RETURNS TRIGGER AS $$
BEGIN
    -- 计算金额 = 数量 * 单价
    NEW.amount := (NEW.quantity_ordered * NEW.unit_price);
    
    -- 计算税额
    NEW.tax_amount := (NEW.amount * NEW.tax_rate / 100);
    
    -- 计算辅助单位数量（如果有换算系数）
    IF NEW.conversion_factor IS NOT NULL AND NEW.conversion_factor > 0 THEN
        NEW.quantity_alt_ordered := (NEW.quantity_ordered / NEW.conversion_factor);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER calc_purchase_order_item_amount_before_insert
BEFORE INSERT ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_order_item_amount();

CREATE TRIGGER calc_purchase_order_item_amount_before_update
BEFORE UPDATE ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_order_item_amount();


-- =====================================================
-- 3. 采购入库表 (purchase_receipt)
-- =====================================================
CREATE TABLE purchase_receipt (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    receipt_no VARCHAR(50) NOT NULL UNIQUE,             -- 入库单号（GR20260315001）
    order_id INTEGER,                                   -- 采购订单 ID（外键）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    receipt_date DATE NOT NULL,                         -- 入库日期
    warehouse_id INTEGER NOT NULL,                      -- 仓库 ID
    department_id INTEGER,                              -- 收货部门 ID
    receiver_id INTEGER,                                -- 收货人 ID
    inspector_id INTEGER,                               -- 质检员 ID
    inspection_status VARCHAR(20) DEFAULT 'PENDING',    -- 质检状态：PENDING=待质检，INSPECTING=质检中，PASSED=合格，FAILED=不合格
    receipt_status VARCHAR(20) DEFAULT 'DRAFT',         -- 入库状态：DRAFT=草稿，CONFIRMED=已确认，CANCELLED=已取消
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总入库数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总入库数量（辅助单位）
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 总金额
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    confirmed_at TIMESTAMP,                             -- 确认时间
    confirmed_by INTEGER                                -- 确认人 ID
);

-- 外键约束
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_receiver
    FOREIGN KEY (receiver_id) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_inspector
    FOREIGN KEY (inspector_id) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);

-- 索引
CREATE INDEX idx_pr_receipt_no ON purchase_receipt(receipt_no);
CREATE INDEX idx_pr_order_id ON purchase_receipt(order_id);
CREATE INDEX idx_pr_supplier_id ON purchase_receipt(supplier_id);
CREATE INDEX idx_pr_receipt_date ON purchase_receipt(receipt_date);
CREATE INDEX idx_pr_warehouse_id ON purchase_receipt(warehouse_id);
CREATE INDEX idx_pr_receipt_status ON purchase_receipt(receipt_status);

-- 触发器：更新时间
CREATE TRIGGER update_purchase_receipt_updated_at
BEFORE UPDATE ON purchase_receipt
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 4. 采购入库明细表 (purchase_receipt_item)
-- =====================================================
CREATE TABLE purchase_receipt_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_item_id INTEGER,                              -- 订单明细 ID（外键）
    line_no INTEGER NOT NULL,                           -- 行号
    product_id INTEGER NOT NULL,                       -- 物料 ID
    material_code VARCHAR(50) NOT NULL,                 -- 物料编码
    material_name VARCHAR(200) NOT NULL,                -- 物料名称
    batch_no VARCHAR(50),                               -- 批次号
    color_code VARCHAR(50),                             -- 色号
    lot_no VARCHAR(50),                                 -- 缸号
    grade VARCHAR(10),                                  -- 等级
    gram_weight DECIMAL(8,2),                           -- 克重
    width DECIMAL(8,2),                                 -- 幅宽
    quantity DECIMAL(18,4) NOT NULL,                    -- 入库数量（主单位）
    quantity_alt DECIMAL(18,4),                         -- 入库数量（辅助单位）
    unit_master VARCHAR(20) NOT NULL,                   -- 主单位
    unit_alt VARCHAR(20),                               -- 辅助单位
    unit_price DECIMAL(18,6),                           -- 单价
    amount DECIMAL(18,2),                               -- 金额
    location_code VARCHAR(50),                          -- 库位编码
    package_no VARCHAR(50),                             -- 包号
    production_date DATE,                               -- 生产日期
    shelf_life INTEGER,                                 -- 保质期（天）
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 外键约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id) ON DELETE CASCADE;
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_order_item
    FOREIGN KEY (order_item_id) REFERENCES purchase_order_item(id);
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_material
    FOREIGN KEY (product_id) REFERENCES products(id);

-- 唯一约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT uk_pri_receipt_line
    UNIQUE (receipt_id, line_no);

-- 索引
CREATE INDEX idx_pri_receipt_id ON purchase_receipt_item(receipt_id);
CREATE INDEX idx_pri_order_item_id ON purchase_receipt_item(order_item_id);
CREATE INDEX idx_pri_material_id ON purchase_receipt_item(material_id);
CREATE INDEX idx_pri_batch_no ON purchase_receipt_item(batch_no);
CREATE INDEX idx_pri_color_code ON purchase_receipt_item(color_code);
CREATE INDEX idx_pri_lot_no ON purchase_receipt_item(lot_no);

-- 触发器：计算金额
CREATE OR REPLACE FUNCTION calc_purchase_receipt_item_amount()
RETURNS TRIGGER AS $$
BEGIN
    -- 计算金额 = 数量 * 单价
    IF NEW.unit_price IS NOT NULL THEN
        NEW.amount := (NEW.quantity * NEW.unit_price);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER calc_purchase_receipt_item_amount_before_insert
BEFORE INSERT ON purchase_receipt_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_receipt_item_amount();

CREATE TRIGGER calc_purchase_receipt_item_amount_before_update
BEFORE UPDATE ON purchase_receipt_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_receipt_item_amount();


-- =====================================================
-- 5. 采购退货表 (purchase_return)
-- =====================================================
CREATE TABLE purchase_return (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    return_no VARCHAR(50) NOT NULL UNIQUE,              -- 退货单号（RT20260315001）
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_id INTEGER,                                   -- 采购订单 ID
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    return_date DATE NOT NULL,                          -- 退货日期
    warehouse_id INTEGER NOT NULL,                      -- 仓库 ID
    department_id INTEGER,                              -- 退货部门 ID
    reason_type VARCHAR(20) NOT NULL,                   -- 退货原因类型：QUALITY_ISSUE=质量问题，WRONG_ITEM=发错货，DAMAGED=破损，OTHER=其他
    reason_detail TEXT,                                 -- 退货原因详情
    return_status VARCHAR(20) DEFAULT 'DRAFT',          -- 退货状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，COMPLETED=已完成
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总退货数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总退货数量（辅助单位）
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 总退货金额
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);

-- 索引
CREATE INDEX idx_pret_return_no ON purchase_return(return_no);
CREATE INDEX idx_pret_receipt_id ON purchase_return(receipt_id);
CREATE INDEX idx_pret_order_id ON purchase_return(order_id);
CREATE INDEX idx_pret_supplier_id ON purchase_return(supplier_id);
CREATE INDEX idx_pret_return_date ON purchase_return(return_date);
CREATE INDEX idx_pret_return_status ON purchase_return(return_status);

-- 触发器：更新时间
CREATE TRIGGER update_purchase_return_updated_at
BEFORE UPDATE ON purchase_return
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 6. 采购质检表 (purchase_inspection)
-- =====================================================
CREATE TABLE purchase_inspection (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    inspection_no VARCHAR(50) NOT NULL UNIQUE,          -- 质检单号（IQ20260315001）
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_id INTEGER,                                   -- 采购订单 ID
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    inspection_date DATE NOT NULL,                      -- 质检日期
    inspector_id INTEGER NOT NULL,                      -- 质检员 ID
    inspection_type VARCHAR(20) DEFAULT 'NORMAL',       -- 质检类型：NORMAL=常规检验，URGENT=紧急检验，SAMPLING=抽样检验
    sample_size INTEGER,                                -- 抽样数量
    defect_count INTEGER DEFAULT 0,                     -- 不合格数量
    pass_quantity DECIMAL(18,4),                        -- 合格数量（主单位）
    reject_quantity DECIMAL(18,4),                      -- 不合格数量（主单位）
    inspection_status VARCHAR(20) DEFAULT 'PENDING',    -- 质检状态：PENDING=待质检，INSPECTING=质检中，COMPLETED=已完成
    inspection_result VARCHAR(20),                      -- 质检结果：PASSED=合格，REJECTED=不合格，CONDITIONAL_PASS=让步接收
    quality_score DECIMAL(5,2),                         -- 质量得分（0-100）
    defect_description TEXT,                            -- 缺陷描述
    attachment_urls TEXT[],                             -- 附件 URL 列表
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    completed_at TIMESTAMP,                             -- 完成时间
    completed_by INTEGER                                -- 完成人 ID
);

-- 外键约束
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_inspector
    FOREIGN KEY (inspector_id) REFERENCES users(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_completed_by
    FOREIGN KEY (completed_by) REFERENCES users(id);

-- 索引
CREATE INDEX idx_pi_inspection_no ON purchase_inspection(inspection_no);
CREATE INDEX idx_pi_receipt_id ON purchase_inspection(receipt_id);
CREATE INDEX idx_pi_order_id ON purchase_inspection(order_id);
CREATE INDEX idx_pi_inspection_date ON purchase_inspection(inspection_date);
CREATE INDEX idx_pi_inspector_id ON purchase_inspection(inspector_id);
CREATE INDEX idx_pi_inspection_status ON purchase_inspection(inspection_status);

-- 触发器：更新时间
CREATE TRIGGER update_purchase_inspection_updated_at
BEFORE UPDATE ON purchase_inspection
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 7. 数据字典表 - 采购订单状态
-- =====================================================
CREATE TABLE purchase_order_status (
    id SERIAL PRIMARY KEY,
    status_code VARCHAR(20) NOT NULL UNIQUE,            -- 状态编码
    status_name VARCHAR(50) NOT NULL,                   -- 状态名称
    description TEXT,                                   -- 状态描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_order_status (status_code, status_name, description, sort_order) VALUES
('DRAFT', '草稿', '订单尚未提交，可以编辑和删除', 10),
('SUBMITTED', '已提交', '订单已提交待审批', 20),
('APPROVED', '已审批', '订单已审批通过，可以执行', 30),
('REJECTED', '已拒绝', '订单审批被拒绝', 25),
('PARTIAL_RECEIVED', '部分入库', '订单部分物料已入库', 40),
('COMPLETED', '已完成', '订单全部物料已入库', 50),
('CLOSED', '已关闭', '订单已关闭，不可再操作', 60);


-- =====================================================
-- 8. 数据字典表 - 入库单状态
-- =====================================================
CREATE TABLE purchase_receipt_status (
    id SERIAL PRIMARY KEY,
    status_code VARCHAR(20) NOT NULL UNIQUE,            -- 状态编码
    status_name VARCHAR(50) NOT NULL,                   -- 状态名称
    description TEXT,                                   -- 状态描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_receipt_status (status_code, status_name, description, sort_order) VALUES
('DRAFT', '草稿', '入库单尚未确认', 10),
('CONFIRMED', '已确认', '入库单已确认，库存已更新', 20),
('CANCELLED', '已取消', '入库单已取消', 30);


-- =====================================================
-- 9. 数据字典表 - 退货原因类型
-- =====================================================
CREATE TABLE purchase_return_reason (
    id SERIAL PRIMARY KEY,
    reason_code VARCHAR(20) NOT NULL UNIQUE,            -- 原因编码
    reason_name VARCHAR(100) NOT NULL,                  -- 原因名称
    description TEXT,                                   -- 原因描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_return_reason (reason_code, reason_name, description, sort_order) VALUES
('QUALITY_ISSUE', '质量问题', '物料质量不符合要求', 10),
('WRONG_ITEM', '发错货', '供应商发错物料', 20),
('DAMAGED', '破损', '物料在运输过程中破损', 30),
('EXCESS_DELIVERY', '超额送货', '供应商送货数量超过订单数量', 40),
('OTHER', '其他', '其他退货原因', 90);


-- =====================================================
-- 10. 物化视图 - 采购订单统计
-- =====================================================
CREATE MATERIALIZED VIEW mv_purchase_order_stats AS
SELECT 
    supplier_id,
    COUNT(*) as total_orders,                           -- 总订单数
    SUM(total_amount) as total_amount,                  -- 总金额
    AVG(total_amount) as avg_order_amount,              -- 平均订单金额
    MAX(order_date) as last_order_date,                 -- 最后订单日期
    SUM(CASE WHEN order_status = 'COMPLETED' THEN 1 ELSE 0 END) as completed_orders,  -- 已完成订单数
    SUM(CASE WHEN order_status = 'PARTIAL_RECEIVED' THEN 1 ELSE 0 END) as partial_orders  -- 部分入库订单数
FROM purchase_order
WHERE order_status IN ('COMPLETED', 'CLOSED', 'PARTIAL_RECEIVED')
GROUP BY supplier_id;

-- 创建索引
CREATE INDEX idx_mv_po_stats_supplier ON mv_purchase_order_stats(supplier_id);


-- =====================================================
-- 11. 注释
-- =====================================================
COMMENT ON TABLE purchase_order IS '采购订单表';
COMMENT ON TABLE purchase_order_item IS '采购订单明细表';
COMMENT ON TABLE purchase_receipt IS '采购入库表';
COMMENT ON TABLE purchase_receipt_item IS '采购入库明细表';
COMMENT ON TABLE purchase_return IS '采购退货表';
COMMENT ON TABLE purchase_inspection IS '采购质检表';
COMMENT ON TABLE purchase_order_status IS '采购订单状态字典';
COMMENT ON TABLE purchase_receipt_status IS '采购入库状态字典';
COMMENT ON TABLE purchase_return_reason IS '采购退货原因字典';
COMMENT ON MATERIALIZED VIEW mv_purchase_order_stats IS '采购订单统计视图';

-- =====================================================
-- 迁移完成
-- =====================================================
