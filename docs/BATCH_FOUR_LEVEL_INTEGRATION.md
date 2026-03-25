# 面料四级批次管理与供应商对接融合方案

## 📋 需求概述

实现**成品 - 多色号 - 缸号 - 匹号**四级批次管理，并建立与供应商系统的对接机制，确保销售开单到采购单生成的全流程自动化和数据准确性。

---

## 🎯 一、核心业务流程

### 1.1 四级批次层级关系

```
成品 (Product)
  └── 色号 (Color No)
        └── 缸号 (Dye Lot No)
              └── 匹号 (Piece No)
```

### 1.2 双编码体系

```
我方内部编码体系          供应商编码体系
├── 成品编号              ├── 供应商成品编号
├── 色号                  ├── 供应商色号
├── 缸号                  ├── 供应商缸号
└── 匹号                  └── 供应商匹号
        ↕ 数据映射转换 ↕
```

### 1.3 核心业务流程

```
销售开单 (使用我方编码)
    ↓
数据转换 (我方→供应商)
    ↓
数据校验 (一致性验证)
    ↓
生成采购单 (仅显示供应商编码)
    ↓
供应商生产/发货
    ↓
采购入库 (四级批次录入)
    ↓
库存管理 (四级批次追溯)
    ↓
销售出库 (四级批次扣减)
```

---

## 🗂️ 二、数据库设计

### 2.1 四级批次核心表

#### **2.1.1 成品主表扩展**

```sql
-- 扩展现有 product 表
ALTER TABLE product ADD COLUMN category_type VARCHAR(50) COMMENT '产品类型：fabric/finished_goods';
ALTER TABLE product ADD COLUMN is_batch_managed BOOLEAN DEFAULT TRUE COMMENT '是否批次管理';
ALTER TABLE product ADD COLUMN supplier_product_code VARCHAR(100) COMMENT '供应商成品编码';
ALTER TABLE product ADD COLUMN supplier_product_name VARCHAR(200) COMMENT '供应商成品名称';
ALTER TABLE product ADD COLUMN unit_conversion_rate DECIMAL(10,4) COMMENT '单位换算率 (米/公斤)';

-- 创建索引
CREATE INDEX idx_product_batch ON product(is_batch_managed);
CREATE INDEX idx_product_supplier_code ON product(supplier_product_code);
```

#### **2.1.2 色号管理表扩展**

```sql
-- 扩展现有 product_color 表
ALTER TABLE product_color ADD COLUMN supplier_color_code VARCHAR(100) COMMENT '供应商色号编码';
ALTER TABLE product_color ADD COLUMN supplier_color_name VARCHAR(200) COMMENT '供应商色号名称';
ALTER TABLE product_color ADD COLUMN pantone_code VARCHAR(50) COMMENT '潘通色号';
ALTER TABLE product_color ADD COLUMN color_system VARCHAR(50) COMMENT '色系：internal/supplier/pantone';

-- 创建索引
CREATE INDEX idx_color_supplier_code ON product_color(supplier_color_code);
CREATE INDEX idx_color_pantone ON product_color(pantone_code);
```

#### **2.1.3 缸号管理表 (新增)**

```sql
CREATE TABLE batch_dye_lot (
    id BIGSERIAL PRIMARY KEY,
    dye_lot_no VARCHAR(100) NOT NULL COMMENT '缸号 (我方)',
    product_id BIGINT NOT NULL COMMENT '成品 ID',
    color_id BIGINT NOT NULL COMMENT '色号 ID',
    
    -- 供应商信息
    supplier_dye_lot_no VARCHAR(100) COMMENT '供应商缸号',
    supplier_id BIGINT COMMENT '供应商 ID',
    
    -- 生产信息
    production_date DATE COMMENT '生产日期',
    machine_no VARCHAR(50) COMMENT '机台号',
    dyeing_vat_no VARCHAR(50) COMMENT '染缸号',
    
    -- 质量信息
    grade VARCHAR(20) DEFAULT 'A' COMMENT '等级：A/B/C',
    quality_status VARCHAR(50) DEFAULT 'pending' COMMENT '质量状态：pending/qualified/unqualified',
    
    -- 数量信息
    total_quantity DECIMAL(12,2) COMMENT '总数量',
    total_length DECIMAL(12,2) COMMENT '总长度 (米)',
    total_weight DECIMAL(12,2) COMMENT '总重量 (公斤)',
    piece_count INTEGER DEFAULT 0 COMMENT '匹数',
    
    -- 状态
    status VARCHAR(50) DEFAULT 'active' COMMENT '状态：active/locked/archived',
    
    -- 系统字段
    created_by BIGINT COMMENT '创建人',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (color_id) REFERENCES product_color(id),
    FOREIGN KEY (supplier_id) REFERENCES supplier(id),
    
    -- 唯一约束
    UNIQUE KEY uk_dye_lot (dye_lot_no, product_id, color_id)
);

COMMENT ON TABLE batch_dye_lot IS '缸号管理表';
COMMENT ON COLUMN batch_dye_lot.dye_lot_no IS '缸号 (我方)';
COMMENT ON COLUMN batch_dye_lot.product_id IS '成品 ID';
COMMENT ON COLUMN batch_dye_lot.color_id IS '色号 ID';
COMMENT ON COLUMN batch_dye_lot.supplier_dye_lot_no IS '供应商缸号';
COMMENT ON COLUMN batch_dye_lot.supplier_id IS '供应商 ID';
COMMENT ON COLUMN batch_dye_lot.production_date IS '生产日期';
COMMENT ON COLUMN batch_dye_lot.machine_no IS '机台号';
COMMENT ON COLUMN batch_dye_lot.dyeing_vat_no IS '染缸号';
COMMENT ON COLUMN batch_dye_lot.grade IS '等级';
COMMENT ON COLUMN batch_dye_lot.quality_status IS '质量状态';
COMMENT ON COLUMN batch_dye_lot.total_quantity IS '总数量';
COMMENT ON COLUMN batch_dye_lot.total_length IS '总长度';
COMMENT ON COLUMN batch_dye_lot.total_weight IS '总重量';
COMMENT ON COLUMN batch_dye_lot.piece_count IS '匹数';
COMMENT ON COLUMN batch_dye_lot.status IS '状态';

-- 创建索引
CREATE INDEX idx_dye_lot_no ON batch_dye_lot(dye_lot_no);
CREATE INDEX idx_dye_lot_product ON batch_dye_lot(product_id);
CREATE INDEX idx_dye_lot_color ON batch_dye_lot(color_id);
CREATE INDEX idx_dye_lot_supplier ON batch_dye_lot(supplier_id);
CREATE INDEX idx_dye_lot_supplier_lot ON batch_dye_lot(supplier_dye_lot_no);
CREATE INDEX idx_dye_lot_status ON batch_dye_lot(status);
```

#### **2.1.4 匹号管理表 (新增)**

```sql
CREATE TABLE inventory_piece (
    id BIGSERIAL PRIMARY KEY,
    piece_no VARCHAR(100) NOT NULL UNIQUE COMMENT '匹号 (我方)',
    dye_lot_id BIGINT NOT NULL COMMENT '缸号 ID',
    
    -- 供应商信息
    supplier_piece_no VARCHAR(100) COMMENT '供应商匹号',
    supplier_code VARCHAR(100) COMMENT '供应商编码',
    
    -- 规格信息
    length DECIMAL(10,2) COMMENT '长度 (米)',
    weight DECIMAL(10,2) COMMENT '重量 (公斤)',
    width DECIMAL(10,2) COMMENT '门幅 (cm)',
    gsm DECIMAL(8,2) COMMENT '克重 (g/m²)',
    
    -- 位置信息
    warehouse_id BIGINT COMMENT '仓库 ID',
    location_code VARCHAR(50) COMMENT '库位编码',
    
    -- 状态信息
    status VARCHAR(50) DEFAULT 'available' COMMENT '状态：available/locked/sold/used',
    quality_grade VARCHAR(20) DEFAULT 'A' COMMENT '质量等级：A/B/C',
    is_locked BOOLEAN DEFAULT FALSE COMMENT '是否锁定',
    lock_reason TEXT COMMENT '锁定原因',
    
    -- 关联信息
    sales_order_id BIGINT COMMENT '销售订单 ID',
    sales_order_no VARCHAR(50) COMMENT '销售订单号',
    purchase_order_id BIGINT COMMENT '采购订单 ID',
    purchase_order_no VARCHAR(50) COMMENT '采购订单号',
    inventory_stock_id BIGINT COMMENT '库存 ID',
    
    -- 追溯信息
    five_dimension_id VARCHAR(100) COMMENT '五维 ID',
    trace_id VARCHAR(100) COMMENT '追溯 ID',
    
    -- 系统字段
    created_by BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id),
    FOREIGN KEY (warehouse_id) REFERENCES warehouse(id),
    FOREIGN KEY (sales_order_id) REFERENCES sales_order(id),
    FOREIGN KEY (purchase_order_id) REFERENCES purchase_order(id),
    FOREIGN KEY (inventory_stock_id) REFERENCES inventory_stock(id)
);

COMMENT ON TABLE inventory_piece IS '匹号管理表';
COMMENT ON COLUMN inventory_piece.piece_no IS '匹号 (我方)';
COMMENT ON COLUMN inventory_piece.dye_lot_id IS '缸号 ID';
COMMENT ON COLUMN inventory_piece.supplier_piece_no IS '供应商匹号';
COMMENT ON COLUMN inventory_piece.supplier_code IS '供应商编码';
COMMENT ON COLUMN inventory_piece.length IS '长度 (米)';
COMMENT ON COLUMN inventory_piece.weight IS '重量 (公斤)';
COMMENT ON COLUMN inventory_piece.width IS '门幅';
COMMENT ON COLUMN inventory_piece.gsm IS '克重';
COMMENT ON COLUMN inventory_piece.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_piece.location_code IS '库位编码';
COMMENT ON COLUMN inventory_piece.status IS '状态';
COMMENT ON COLUMN inventory_piece.quality_grade IS '质量等级';
COMMENT ON COLUMN inventory_piece.is_locked IS '是否锁定';
COMMENT ON COLUMN inventory_piece.sales_order_id IS '销售订单 ID';
COMMENT ON COLUMN inventory_piece.purchase_order_id IS '采购订单 ID';
COMMENT ON COLUMN inventory_piece.inventory_stock_id IS '库存 ID';
COMMENT ON COLUMN inventory_piece.five_dimension_id IS '五维 ID';
COMMENT ON COLUMN inventory_piece.trace_id IS '追溯 ID';

-- 创建索引
CREATE INDEX idx_piece_no ON inventory_piece(piece_no);
CREATE INDEX idx_piece_dye_lot ON inventory_piece(dye_lot_id);
CREATE INDEX idx_piece_supplier_piece ON inventory_piece(supplier_piece_no);
CREATE INDEX idx_piece_status ON inventory_piece(status);
CREATE INDEX idx_piece_sales_order ON inventory_piece(sales_order_id);
CREATE INDEX idx_piece_purchase_order ON inventory_piece(purchase_order_id);
CREATE INDEX idx_piece_warehouse ON inventory_piece(warehouse_id);
CREATE INDEX idx_piece_five_dimension ON inventory_piece(five_dimension_id);
```

---

### 2.2 数据映射表

#### **2.2.1 成品编码映射表**

```sql
CREATE TABLE product_code_mapping (
    id BIGSERIAL PRIMARY KEY,
    product_id BIGINT NOT NULL COMMENT '成品 ID',
    supplier_id BIGINT NOT NULL COMMENT '供应商 ID',
    
    -- 我方编码
    internal_product_code VARCHAR(100) NOT NULL COMMENT '我方成品编码',
    internal_product_name VARCHAR(200) COMMENT '我方成品名称',
    
    -- 供应商编码
    supplier_product_code VARCHAR(100) NOT NULL COMMENT '供应商成品编码',
    supplier_product_name VARCHAR(200) COMMENT '供应商成品名称',
    
    -- 映射关系
    mapping_type VARCHAR(50) DEFAULT 'one_to_one' COMMENT '映射类型：one_to_one/one_to_many',
    conversion_rate DECIMAL(10,4) COMMENT '换算率',
    
    -- 状态
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否启用',
    validated_at TIMESTAMP COMMENT '验证时间',
    validated_by BIGINT COMMENT '验证人',
    
    -- 系统字段
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (supplier_id) REFERENCES supplier(id),
    
    -- 唯一约束
    UNIQUE KEY uk_mapping (product_id, supplier_id, internal_product_code, supplier_product_code)
);

COMMENT ON TABLE product_code_mapping IS '成品编码映射表';
COMMENT ON COLUMN product_code_mapping.product_id IS '成品 ID';
COMMENT ON COLUMN product_code_mapping.supplier_id IS '供应商 ID';
COMMENT ON COLUMN product_code_mapping.internal_product_code IS '我方成品编码';
COMMENT ON COLUMN product_code_mapping.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN product_code_mapping.mapping_type IS '映射类型';
COMMENT ON COLUMN product_code_mapping.conversion_rate IS '换算率';
COMMENT ON COLUMN product_code_mapping.is_active IS '是否启用';

-- 创建索引
CREATE INDEX idx_mapping_product ON product_code_mapping(product_id);
CREATE INDEX idx_mapping_supplier ON product_code_mapping(supplier_id);
CREATE INDEX idx_mapping_internal ON product_code_mapping(internal_product_code);
CREATE INDEX idx_mapping_supplier_code ON product_code_mapping(supplier_product_code);
```

#### **2.2.2 色号编码映射表**

```sql
CREATE TABLE color_code_mapping (
    id BIGSERIAL PRIMARY KEY,
    color_id BIGINT NOT NULL COMMENT '色号 ID',
    supplier_id BIGINT NOT NULL COMMENT '供应商 ID',
    
    -- 我方编码
    internal_color_code VARCHAR(100) NOT NULL COMMENT '我方色号编码',
    internal_color_name VARCHAR(200) COMMENT '我方色号名称',
    
    -- 供应商编码
    supplier_color_code VARCHAR(100) NOT NULL COMMENT '供应商色号编码',
    supplier_color_name VARCHAR(200) COMMENT '供应商色号名称',
    
    -- 颜色信息
    pantone_code VARCHAR(50) COMMENT '潘通色号',
    color_formula TEXT COMMENT '染色配方',
    
    -- 状态
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否启用',
    validated_at TIMESTAMP COMMENT '验证时间',
    
    -- 系统字段
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (color_id) REFERENCES product_color(id),
    FOREIGN KEY (supplier_id) REFERENCES supplier(id),
    
    -- 唯一约束
    UNIQUE KEY uk_color_mapping (color_id, supplier_id, internal_color_code, supplier_color_code)
);

COMMENT ON TABLE color_code_mapping IS '色号编码映射表';

-- 创建索引
CREATE INDEX idx_color_mapping_color ON color_code_mapping(color_id);
CREATE INDEX idx_color_mapping_supplier ON color_code_mapping(supplier_id);
CREATE INDEX idx_color_mapping_internal ON color_code_mapping(internal_color_code);
CREATE INDEX idx_color_mapping_supplier_code ON color_code_mapping(supplier_color_code);
```

#### **2.2.3 缸号编码映射表**

```sql
CREATE TABLE dye_lot_mapping (
    id BIGSERIAL PRIMARY KEY,
    dye_lot_id BIGINT NOT NULL COMMENT '缸号 ID',
    supplier_id BIGINT NOT NULL COMMENT '供应商 ID',
    
    -- 我方缸号
    internal_dye_lot_no VARCHAR(100) NOT NULL COMMENT '我方缸号',
    
    -- 供应商缸号
    supplier_dye_lot_no VARCHAR(100) NOT NULL COMMENT '供应商缸号',
    supplier_production_batch VARCHAR(100) COMMENT '供应商生产批号',
    
    -- 生产信息
    production_date DATE COMMENT '生产日期',
    machine_no VARCHAR(50) COMMENT '机台号',
    
    -- 状态
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否启用',
    
    -- 系统字段
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id),
    FOREIGN KEY (supplier_id) REFERENCES supplier(id),
    
    -- 唯一约束
    UNIQUE KEY uk_dye_lot_mapping (dye_lot_id, supplier_id, internal_dye_lot_no, supplier_dye_lot_no)
);

COMMENT ON TABLE dye_lot_mapping IS '缸号编码映射表';

-- 创建索引
CREATE INDEX idx_dye_lot_mapping_dye_lot ON dye_lot_mapping(dye_lot_id);
CREATE INDEX idx_dye_lot_mapping_supplier ON dye_lot_mapping(supplier_id);
CREATE INDEX idx_dye_lot_mapping_internal ON dye_lot_mapping(internal_dye_lot_no);
CREATE INDEX idx_dye_lot_mapping_supplier_lot ON dye_lot_mapping(supplier_dye_lot_no);
```

#### **2.2.4 匹号编码映射表**

```sql
CREATE TABLE piece_mapping (
    id BIGSERIAL PRIMARY KEY,
    piece_id BIGINT NOT NULL COMMENT '匹号 ID',
    supplier_id BIGINT NOT NULL COMMENT '供应商 ID',
    
    -- 我方匹号
    internal_piece_no VARCHAR(100) NOT NULL COMMENT '我方匹号',
    
    -- 供应商匹号
    supplier_piece_no VARCHAR(100) NOT NULL COMMENT '供应商匹号',
    supplier_barcode VARCHAR(100) COMMENT '供应商条码',
    
    -- 规格信息
    length DECIMAL(10,2) COMMENT '长度 (米)',
    weight DECIMAL(10,2) COMMENT '重量 (公斤)',
    
    -- 状态
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否启用',
    
    -- 系统字段
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (piece_id) REFERENCES inventory_piece(id),
    FOREIGN KEY (supplier_id) REFERENCES supplier(id),
    
    -- 唯一约束
    UNIQUE KEY uk_piece_mapping (piece_id, supplier_id, internal_piece_no, supplier_piece_no)
);

COMMENT ON TABLE piece_mapping IS '匹号编码映射表';

-- 创建索引
CREATE INDEX idx_piece_mapping_piece ON piece_mapping(piece_id);
CREATE INDEX idx_piece_mapping_supplier ON piece_mapping(supplier_id);
CREATE INDEX idx_piece_mapping_internal ON piece_mapping(internal_piece_no);
CREATE INDEX idx_piece_mapping_supplier_piece ON piece_mapping(supplier_piece_no);
```

---

### 2.3 销售/采购订单扩展

#### **2.3.1 销售订单明细扩展**

```sql
-- 扩展现有 sales_order_details 表
ALTER TABLE sales_order_details ADD COLUMN color_no VARCHAR(100) COMMENT '色号 (我方)';
ALTER TABLE sales_order_details ADD COLUMN dye_lot_no VARCHAR(100) COMMENT '缸号 (我方)';
ALTER TABLE sales_order_details ADD COLUMN piece_nos TEXT[] COMMENT '匹号列表 (我方)';
ALTER TABLE sales_order_details ADD COLUMN total_length DECIMAL(12,2) COMMENT '总长度 (米)';
ALTER TABLE sales_order_details ADD COLUMN total_weight DECIMAL(12,2) COMMENT '总重量 (公斤)';
ALTER TABLE sales_order_details ADD COLUMN is_batch_allocated BOOLEAN DEFAULT FALSE COMMENT '是否已分配批次';

-- 创建索引
CREATE INDEX idx_sales_detail_color ON sales_order_details(color_no);
CREATE INDEX idx_sales_detail_dye_lot ON sales_order_details(dye_lot_no);
CREATE INDEX idx_sales_detail_batch_allocated ON sales_order_details(is_batch_allocated);
```

#### **2.3.2 采购订单明细扩展**

```sql
-- 扩展现有 purchase_order_details 表
ALTER TABLE purchase_order_details ADD COLUMN supplier_product_code VARCHAR(100) COMMENT '供应商成品编码';
ALTER TABLE purchase_order_details ADD COLUMN supplier_product_name VARCHAR(200) COMMENT '供应商成品名称';
ALTER TABLE purchase_order_details ADD COLUMN supplier_color_code VARCHAR(100) COMMENT '供应商色号编码';
ALTER TABLE purchase_order_details ADD COLUMN supplier_color_name VARCHAR(200) COMMENT '供应商色号名称';
ALTER TABLE purchase_order_details ADD COLUMN supplier_dye_lot_no VARCHAR(100) COMMENT '供应商缸号';
ALTER TABLE purchase_order_details ADD COLUMN expected_piece_count INTEGER COMMENT '预计匹数';
ALTER TABLE purchase_order_details ADD COLUMN is_from_sales_order BOOLEAN DEFAULT FALSE COMMENT '是否来自销售订单';
ALTER TABLE purchase_order_details ADD COLUMN sales_order_id BIGINT COMMENT '销售订单 ID';
ALTER TABLE purchase_order_details ADD COLUMN sales_order_no VARCHAR(50) COMMENT '销售订单号';

-- 创建索引
CREATE INDEX idx_purchase_detail_supplier_code ON purchase_order_details(supplier_product_code);
CREATE INDEX idx_purchase_detail_supplier_color ON purchase_order_details(supplier_color_code);
CREATE INDEX idx_purchase_detail_sales_order ON purchase_order_details(sales_order_id);
CREATE INDEX idx_purchase_detail_from_sales ON purchase_order_details(is_from_sales_order);
```

---

### 2.4 操作日志表

#### **2.4.1 批次追溯日志表**

```sql
CREATE TABLE batch_trace_log (
    id BIGSERIAL PRIMARY KEY,
    trace_no VARCHAR(100) NOT NULL UNIQUE COMMENT '追溯单号',
    
    -- 业务信息
    business_type VARCHAR(50) NOT NULL COMMENT '业务类型：sales_to_purchase/purchase_to_sales',
    business_id BIGINT NOT NULL COMMENT '业务 ID',
    business_no VARCHAR(100) NOT NULL COMMENT '业务单号',
    
    -- 四级批次信息 (我方)
    internal_product_code VARCHAR(100) COMMENT '我方成品编码',
    internal_color_no VARCHAR(100) COMMENT '我方色号',
    internal_dye_lot_no VARCHAR(100) COMMENT '我方缸号',
    internal_piece_nos TEXT[] COMMENT '我方匹号列表',
    
    -- 四级批次信息 (供应商)
    supplier_product_code VARCHAR(100) COMMENT '供应商成品编码',
    supplier_color_code VARCHAR(100) COMMENT '供应商色号',
    supplier_dye_lot_no VARCHAR(100) COMMENT '供应商缸号',
    supplier_piece_nos TEXT[] COMMENT '供应商匹号列表',
    
    -- 转换信息
    conversion_details JSONB COMMENT '转换详情',
    validation_result VARCHAR(50) COMMENT '验证结果：passed/failed',
    validation_errors TEXT[] COMMENT '验证错误列表',
    
    -- 目标信息
    target_type VARCHAR(50) COMMENT '目标类型：purchase_order',
    target_id BIGINT COMMENT '目标 ID',
    target_no VARCHAR(100) COMMENT '目标单号',
    
    -- 操作信息
    operator_id BIGINT COMMENT '操作人 ID',
    operator_name VARCHAR(100) COMMENT '操作人姓名',
    operation_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '操作时间',
    
    -- 系统字段
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (business_id) REFERENCES sales_order(id)
);

COMMENT ON TABLE batch_trace_log IS '批次追溯日志表';
COMMENT ON COLUMN batch_trace_log.trace_no IS '追溯单号';
COMMENT ON COLUMN batch_trace_log.business_type IS '业务类型';
COMMENT ON COLUMN batch_trace_log.business_id IS '业务 ID';
COMMENT ON COLUMN batch_trace_log.business_no IS '业务单号';
COMMENT ON COLUMN batch_trace_log.internal_product_code IS '我方成品编码';
COMMENT ON COLUMN batch_trace_log.internal_color_no IS '我方色号';
COMMENT ON COLUMN batch_trace_log.internal_dye_lot_no IS '我方缸号';
COMMENT ON COLUMN batch_trace_log.internal_piece_nos IS '我方匹号列表';
COMMENT ON COLUMN batch_trace_log.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN batch_trace_log.supplier_color_code IS '供应商色号';
COMMENT ON COLUMN batch_trace_log.supplier_dye_lot_no IS '供应商缸号';
COMMENT ON COLUMN batch_trace_log.supplier_piece_nos IS '供应商匹号列表';
COMMENT ON COLUMN batch_trace_log.conversion_details IS '转换详情';
COMMENT ON COLUMN batch_trace_log.validation_result IS '验证结果';
COMMENT ON COLUMN batch_trace_log.validation_errors IS '验证错误列表';
COMMENT ON COLUMN batch_trace_log.target_type IS '目标类型';
COMMENT ON COLUMN batch_trace_log.target_id IS '目标 ID';
COMMENT ON COLUMN batch_trace_log.target_no IS '目标单号';
COMMENT ON COLUMN batch_trace_log.operator_id IS '操作人 ID';
COMMENT ON COLUMN batch_trace_log.operator_name IS '操作人姓名';
COMMENT ON COLUMN batch_trace_log.operation_time IS '操作时间';

-- 创建索引
CREATE INDEX idx_trace_no ON batch_trace_log(trace_no);
CREATE INDEX idx_trace_business ON batch_trace_log(business_type, business_id);
CREATE INDEX idx_trace_business_no ON batch_trace_log(business_no);
CREATE INDEX idx_trace_target ON batch_trace_log(target_type, target_id);
CREATE INDEX idx_trace_operation_time ON batch_trace_log(operation_time);
CREATE INDEX idx_trace_internal_product ON batch_trace_log(internal_product_code);
CREATE INDEX idx_trace_supplier_product ON batch_trace_log(supplier_product_code);
```

---

## 🔧 三、核心服务实现

### 3.1 批次管理服务

```rust
// backend/src/services/batch_management_service.rs
use crate::models::batch_dye_lot::{BatchDyeLot, CreateBatchDyeLotRequest};
use crate::models::inventory_piece::{InventoryPiece, CreateInventoryPieceRequest};
use crate::models::product_code_mapping::ProductCodeMapping;
use crate::models::color_code_mapping::ColorCodeMapping;
use crate::models::dye_lot_mapping::DyeLotMapping;
use crate::models::piece_mapping::PieceMapping;

pub struct BatchManagementService {
    db: PgPool,
}

impl BatchManagementService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
    
    /// 创建缸号批次
    pub async fn create_dye_lot(
        &self,
        req: CreateBatchDyeLotRequest,
    ) -> Result<BatchDyeLot, AppError> {
        let dye_lot = BatchDyeLot {
            dye_lot_no: req.dye_lot_no,
            product_id: req.product_id,
            color_id: req.color_id,
            supplier_dye_lot_no: req.supplier_dye_lot_no,
            supplier_id: req.supplier_id,
            production_date: req.production_date,
            machine_no: req.machine_no,
            dyeing_vat_no: req.dyeing_vat_no,
            grade: req.grade.unwrap_or("A".to_string()),
            quality_status: "pending".to_string(),
            total_quantity: req.total_quantity,
            total_length: req.total_length,
            total_weight: req.total_weight,
            piece_count: 0,
            status: "active".to_string(),
            created_by: req.created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // 保存到数据库
        let saved = save_dye_lot(&self.db, dye_lot).await?;
        
        // 创建映射关系
        if let Some(supplier_dye_lot_no) = &req.supplier_dye_lot_no {
            let mapping = DyeLotMapping {
                dye_lot_id: saved.id,
                supplier_id: req.supplier_id,
                internal_dye_lot_no: saved.dye_lot_no.clone(),
                supplier_dye_lot_no: supplier_dye_lot_no.clone(),
                supplier_production_batch: req.supplier_production_batch,
                production_date: req.production_date,
                machine_no: req.machine_no,
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            save_dye_lot_mapping(&self.db, mapping).await?;
        }
        
        Ok(saved)
    }
    
    /// 创建匹号
    pub async fn create_piece(
        &self,
        req: CreateInventoryPieceRequest,
    ) -> Result<InventoryPiece, AppError> {
        // 生成五维 ID
        let five_dimension_id = fabric_five_dimension::generate_id(
            &req.piece_no,
            "PIECE",
            &Utc::now().naive_utc().date(),
        );
        
        let piece = InventoryPiece {
            piece_no: req.piece_no,
            dye_lot_id: req.dye_lot_id,
            supplier_piece_no: req.supplier_piece_no,
            supplier_code: req.supplier_code,
            length: req.length,
            weight: req.weight,
            width: req.width,
            gsm: req.gsm,
            warehouse_id: req.warehouse_id,
            location_code: req.location_code,
            status: "available".to_string(),
            quality_grade: req.quality_grade.unwrap_or("A".to_string()),
            is_locked: false,
            lock_reason: None,
            sales_order_id: None,
            sales_order_no: None,
            purchase_order_id: None,
            purchase_order_no: None,
            inventory_stock_id: None,
            five_dimension_id: Some(five_dimension_id.clone()),
            trace_id: Some(generate_trace_id()),
            created_by: req.created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let saved = save_piece(&self.db, piece).await?;
        
        // 更新缸号的匹数
        update_dye_lot_piece_count(&self.db, req.dye_lot_id, 1).await?;
        
        // 创建库存记录
        create_inventory_stock_from_piece(&self.db, &saved).await?;
        
        Ok(saved)
    }
    
    /// 查询四级批次信息
    pub async fn query_batch_hierarchy(
        &self,
        product_id: i64,
        color_no: Option<&str>,
        dye_lot_no: Option<&str>,
        piece_no: Option<&str>,
    ) -> Result<BatchHierarchy, AppError> {
        // 实现四级批次查询逻辑
        // 返回完整的层级结构
    }
    
    /// 批次追溯 (正向)
    pub async fn trace_forward(
        &self,
        piece_no: &str,
    ) -> Result<BatchTraceResult, AppError> {
        // 从匹号追溯到缸号、色号、成品
        // 并追溯到销售订单、出库记录等
    }
    
    /// 批次追溯 (反向)
    pub async fn trace_backward(
        &self,
        product_id: i64,
        color_no: Option<&str>,
        dye_lot_no: Option<&str>,
    ) -> Result<Vec<BatchTraceResult>, AppError> {
        // 从成品/色号/缸号追溯到所有匹号
        // 并追溯到采购订单、入库记录等
    }
}
```

---

### 3.2 编码转换服务

```rust
// backend/src/services/code_conversion_service.rs
use crate::models::product_code_mapping::ProductCodeMapping;
use crate::models::color_code_mapping::ColorCodeMapping;
use crate::models::dye_lot_mapping::DyeLotMapping;
use crate::models::piece_mapping::PieceMapping;

pub struct CodeConversionService {
    db: PgPool,
}

impl CodeConversionService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
    
    /// 成品编码转换 (我方→供应商)
    pub async fn convert_product_to_supplier(
        &self,
        internal_product_code: &str,
        supplier_id: i64,
    ) -> Result<ProductCodeMapping, AppError> {
        let mapping = get_product_mapping(
            &self.db,
            internal_product_code,
            supplier_id,
        ).await?;
        
        if !mapping.is_active {
            return Err(AppError::ValidationError(
                "成品编码映射已禁用".to_string()
            ));
        }
        
        Ok(mapping)
    }
    
    /// 色号编码转换 (我方→供应商)
    pub async fn convert_color_to_supplier(
        &self,
        internal_color_code: &str,
        supplier_id: i64,
    ) -> Result<ColorCodeMapping, AppError> {
        let mapping = get_color_mapping(
            &self.db,
            internal_color_code,
            supplier_id,
        ).await?;
        
        if !mapping.is_active {
            return Err(AppError::ValidationError(
                "色号编码映射已禁用".to_string()
            ));
        }
        
        Ok(mapping)
    }
    
    /// 缸号编码转换 (我方→供应商)
    pub async fn convert_dye_lot_to_supplier(
        &self,
        internal_dye_lot_no: &str,
        supplier_id: i64,
    ) -> Result<DyeLotMapping, AppError> {
        let mapping = get_dye_lot_mapping(
            &self.db,
            internal_dye_lot_no,
            supplier_id,
        ).await?;
        
        if !mapping.is_active {
            return Err(AppError::ValidationError(
                "缸号编码映射已禁用".to_string()
            ));
        }
        
        Ok(mapping)
    }
    
    /// 匹号编码转换 (我方→供应商)
    pub async fn convert_piece_to_supplier(
        &self,
        internal_piece_no: &str,
        supplier_id: i64,
    ) -> Result<PieceMapping, AppError> {
        let mapping = get_piece_mapping(
            &self.db,
            internal_piece_no,
            supplier_id,
        ).await?;
        
        if !mapping.is_active {
            return Err(AppError::ValidationError(
                "匹号编码映射已禁用".to_string()
            ));
        }
        
        Ok(mapping)
    }
    
    /// 批量转换 (销售→采购)
    pub async fn convert_sales_to_purchase(
        &self,
        sales_order_id: i64,
        supplier_id: i64,
    ) -> Result<PurchaseOrderData, AppError> {
        // 1. 获取销售订单明细
        let sales_details = get_sales_order_details(&self.db, sales_order_id).await?;
        
        // 2. 遍历明细进行转换
        let mut purchase_details = Vec::new();
        let mut conversion_logs = Vec::new();
        
        for detail in sales_details {
            // 转换成品编码
            let product_mapping = self.convert_product_to_supplier(
                &detail.product_code,
                supplier_id,
            ).await?;
            
            // 转换色号
            let color_mapping = if let Some(color_no) = &detail.color_no {
                Some(self.convert_color_to_supplier(color_no, supplier_id).await?)
            } else {
                None
            };
            
            // 转换缸号
            let dye_lot_mapping = if let Some(dye_lot_no) = &detail.dye_lot_no {
                Some(self.convert_dye_lot_to_supplier(dye_lot_no, supplier_id).await?)
            } else {
                None
            };
            
            // 转换匹号
            let piece_mappings = if let Some(piece_nos) = &detail.piece_nos {
                let mut mappings = Vec::new();
                for piece_no in piece_nos {
                    mappings.push(self.convert_piece_to_supplier(piece_no, supplier_id).await?);
                }
                Some(mappings)
            } else {
                None
            };
            
            // 创建采购订单明细
            let purchase_detail = PurchaseOrderDetail {
                supplier_product_code: product_mapping.supplier_product_code,
                supplier_product_name: product_mapping.supplier_product_name,
                supplier_color_code: color_mapping.as_ref().map(|m| m.supplier_color_code.clone()),
                supplier_color_name: color_mapping.as_ref().map(|m| m.supplier_color_name.clone()),
                supplier_dye_lot_no: dye_lot_mapping.as_ref().map(|m| m.supplier_dye_lot_no.clone()),
                quantity: detail.quantity,
                unit: detail.unit,
                price: detail.price,
                // ...
                is_from_sales_order: true,
                sales_order_id: Some(sales_order_id),
                sales_order_no: Some(detail.sales_order_no.clone()),
            };
            
            purchase_details.push(purchase_detail);
            
            // 记录转换日志
            conversion_logs.push(ConversionLog {
                sales_order_id,
                product_code: detail.product_code,
                color_no: detail.color_no.clone(),
                dye_lot_no: detail.dye_lot_no.clone(),
                piece_nos: detail.piece_nos.clone(),
                supplier_product_code: product_mapping.supplier_product_code,
                supplier_color_code: color_mapping.as_ref().map(|m| m.supplier_color_code.clone()),
                supplier_dye_lot_no: dye_lot_mapping.as_ref().map(|m| m.supplier_dye_lot_no.clone()),
                conversion_time: Utc::now(),
            });
        }
        
        Ok(PurchaseOrderData {
            details: purchase_details,
            conversion_logs,
        })
    }
    
    /// 数据一致性校验
    pub async fn validate_conversion(
        &self,
        sales_order_id: i64,
        supplier_id: i64,
    ) -> Result<ValidationResult, AppError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // 1. 获取销售订单明细
        let sales_details = get_sales_order_details(&self.db, sales_order_id).await?;
        
        for detail in sales_details {
            // 校验成品编码映射
            match self.convert_product_to_supplier(&detail.product_code, supplier_id).await {
                Ok(_) => {},
                Err(e) => errors.push(format!(
                    "成品 {} 编码映射不存在：{}",
                    detail.product_code, e
                )),
            }
            
            // 校验色号映射
            if let Some(color_no) = &detail.color_no {
                match self.convert_color_to_supplier(color_no, supplier_id).await {
                    Ok(_) => {},
                    Err(e) => errors.push(format!(
                        "色号 {} 编码映射不存在：{}",
                        color_no, e
                    )),
                }
            }
            
            // 校验缸号映射
            if let Some(dye_lot_no) = &detail.dye_lot_no {
                match self.convert_dye_lot_to_supplier(dye_lot_no, supplier_id).await {
                    Ok(_) => {},
                    Err(e) => errors.push(format!(
                        "缸号 {} 编码映射不存在：{}",
                        dye_lot_no, e
                    )),
                }
            }
            
            // 校验匹号映射
            if let Some(piece_nos) = &detail.piece_nos {
                for piece_no in piece_nos {
                    match self.convert_piece_to_supplier(piece_no, supplier_id).await {
                        Ok(_) => {},
                        Err(e) => errors.push(format!(
                            "匹号 {} 编码映射不存在：{}",
                            piece_no, e
                        )),
                    }
                }
            }
            
            // 校验数量一致性
            if detail.quantity <= 0.0 {
                errors.push(format!("数量必须大于 0: {}", detail.quantity));
            }
        }
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }
}
```

---

### 3.3 销售转采购服务

```rust
// backend/src/services/sales_to_purchase_service.rs
use crate::services::code_conversion_service::CodeConversionService;
use crate::services::batch_trace_service::BatchTraceService;

pub struct SalesToPurchaseService {
    db: PgPool,
    code_conversion: Arc<CodeConversionService>,
    batch_trace: Arc<BatchTraceService>,
}

impl SalesToPurchaseService {
    pub fn new(
        db: PgPool,
        code_conversion: Arc<CodeConversionService>,
        batch_trace: Arc<BatchTraceService>,
    ) -> Self {
        Self {
            db,
            code_conversion,
            batch_trace,
        }
    }
    
    /// 从销售订单生成采购订单
    pub async fn generate_purchase_from_sales(
        &self,
        sales_order_id: i64,
        supplier_id: i64,
        operator_id: i64,
    ) -> Result<PurchaseOrder, AppError> {
        let mut transaction = self.db.begin().await?;
        
        // 1. 数据一致性校验
        let validation = self.code_conversion
            .validate_conversion(sales_order_id, supplier_id)
            .await?;
        
        if !validation.is_valid {
            return Err(AppError::ValidationError(
                format!("数据校验失败：{:?}", validation.errors)
            ));
        }
        
        // 2. 获取销售订单
        let sales_order = get_sales_order(&mut *transaction, sales_order_id).await?;
        
        // 3. 转换编码
        let purchase_data = self.code_conversion
            .convert_sales_to_purchase(sales_order_id, supplier_id)
            .await?;
        
        // 4. 创建采购订单 (仅显示供应商编码)
        let purchase_order = PurchaseOrder {
            order_no: generate_purchase_order_no(),
            supplier_id,
            order_date: Utc::now().naive_utc().date(),
            details: purchase_data.details,
            total_amount: purchase_data.calculate_total(),
            status: "pending".to_string(),
            is_from_sales: true,
            source_sales_order_id: Some(sales_order_id),
            source_sales_order_no: Some(sales_order.order_no.clone()),
            created_by: operator_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let saved_purchase = save_purchase_order(&mut *transaction, purchase_order).await?;
        
        // 5. 记录追溯日志
        let trace_log = BatchTraceLog {
            trace_no: generate_trace_no(),
            business_type: "sales_to_purchase".to_string(),
            business_id: sales_order_id,
            business_no: sales_order.order_no.clone(),
            internal_product_code: sales_order.product_code,
            internal_color_no: sales_order.color_no,
            internal_dye_lot_no: sales_order.dye_lot_no,
            internal_piece_nos: sales_order.piece_nos,
            supplier_product_code: saved_purchase.supplier_product_code,
            supplier_color_code: saved_purchase.supplier_color_code,
            supplier_dye_lot_no: saved_purchase.supplier_dye_lot_no,
            supplier_piece_nos: saved_purchase.supplier_piece_nos,
            conversion_details: serde_json::to_value(&purchase_data.conversion_logs)?,
            validation_result: "passed".to_string(),
            validation_errors: vec![],
            target_type: "purchase_order".to_string(),
            target_id: saved_purchase.id,
            target_no: saved_purchase.order_no.clone(),
            operator_id,
            operator_name: get_user_name(&mut *transaction, operator_id).await?,
            operation_time: Utc::now(),
            created_at: Utc::now(),
        };
        
        save_trace_log(&mut *transaction, trace_log).await?;
        
        // 6. 提交事务
        transaction.commit().await?;
        
        Ok(saved_purchase)
    }
}
```

---

## 🎯 四、实施步骤

### 阶段一：基础数据建设 (第 1-2 周)
- [ ] 创建四级批次数据库表
- [ ] 创建编码映射表
- [ ] 实现批次管理服务
- [ ] 实现编码转换服务
- [ ] 前端添加四级批次管理页面

### 阶段二：销售开单融合 (第 3 周)
- [ ] 扩展销售订单 Model
- [ ] 修改销售开单 Handler
- [ ] 前端添加四级批次选择
- [ ] 实现批次分配功能

### 阶段三：采购自动生成 (第 4 周)
- [ ] 扩展采购订单 Model
- [ ] 实现销售转采购服务
- [ ] 实现数据校验功能
- [ ] 前端添加采购单生成页面

### 阶段四：追溯和日志 (第 5 周)
- [ ] 实现批次追溯服务
- [ ] 创建追溯日志表
- [ ] 实现操作日志记录
- [ ] 前端添加追溯查询页面

---

## ✅ 五、验收标准

### 5.1 功能验收

| 功能点 | 验收标准 | 状态 |
|--------|---------|------|
| 四级批次录入 | 支持成品 - 色号 - 缸号 - 匹号录入 | ⬜ |
| 编码映射管理 | 支持我方/供应商编码映射 | ⬜ |
| 销售开单 | 使用我方编码开单 | ⬜ |
| 采购单生成 | 自动生成，仅显示供应商编码 | ⬜ |
| 数据校验 | 转换前自动校验 | ⬜ |
| 操作日志 | 完整记录转换过程 | ⬜ |
| 批次追溯 | 支持正反向追溯 | ⬜ |

### 5.2 数据一致性验收

- ✅ 映射关系准确率 100%
- ✅ 转换错误率 0%
- ✅ 追溯数据完整率 100%

---

**方案完成!** 接下来可以开始实施阶段一的数据库表创建和服务开发。🚀
