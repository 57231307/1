# 秉羲 ERP 功能模块全面融合计划 (含四级批次管理)

## 📋 项目概述

整合前期规划的 6 大模块 (OA/HRM/BPM/CRM/日志/报表) 与四级批次管理 (成品 - 色号 - 缸号 - 匹号)，打造一体化企业级管理系统。

---

## 🎯 一、融合范围

### 1.1 新增模块 (6+1 个)

| 模块 | 优先级 | 表数量 | 实施周期 |
|------|--------|--------|---------|
| **BPM 流程引擎** | P0 | 12 张 | 4 周 |
| **CRM 扩展** | P1 | 6 张 | 3 周 |
| **OA 协同办公** | P2 | 8 张 | 3 周 |
| **日志管理** | P0 | 4 张 | 2 周 |
| **数据可视化** | P2 | 6 张 | 3 周 |
| **四级批次管理** | P0 | 7 张 | 5 周 |

**总计**: 43 张新表，20 周实施周期

### 1.2 融合点统计

| 融合类型 | 融合点数量 | 优先级 |
|---------|-----------|--------|
| BPM 流程融合 | 20+ | P0 |
| CRM 扩展融合 | 18 | P1 |
| 四级批次融合 | 15 | P0 |
| OA 通知融合 | 15 | P2 |
| 报表融合 | 25+ | P2 |
| 日志融合 | 8 | P0 |

**总计**: 101+ 个融合点

---

## 🏗️ 二、整体融合架构

```
┌─────────────────────────────────────────────────────┐
│              统一门户层 (Yew 前端)                    │
│  统一登录 | 工作台 | 消息 | 待办 | 批次查询        │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│           BPM 流程引擎 (统一审批)                     │
│  采购审批 | 销售审批 | 财务审批 | 人事审批         │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│           业务中台层 (Axum 服务)                      │
│  ┌──────────────────────────────────────────────┐  │
│  │  供应链域                                     │  │
│  │  ├─ 采购管理 (含四级批次)                    │  │
│  │  ├─ 销售管理 (含四级批次)                    │  │
│  │  ├─ 库存管理 (四级批次核心)                  │  │
│  │  └─ 供应商管理                               │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │  财务域                                       │  │
│  │  ├─ 应收/应付                                │  │
│  │  └─ 总账                                     │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │  CRM 域                                       │  │
│  │  ├─ 客户管理                                 │  │
│  │  ├─ 线索/商机                                │  │
│  │  └─ 销售漏斗                                 │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│         数据中台层 (SeaORM + PostgreSQL)              │
│  ┌──────────────────────────────────────────────┐  │
│  │  统一数据模型                                 │  │
│  │  ├─ 用户/部门/角色                           │  │
│  │  ├─ 四级批次 (成品/色号/缸号/匹号)          │  │
│  │  └─ 编码映射 (我方/供应商)                   │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │  日志中心                                     │  │
│  │  ├─ 操作日志                                 │  │
│  │  ├─ API 日志                                  │  │
│  │  └─ 批次追溯日志                             │  │
│  └──────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────┐  │
│  │  报表中心                                     │  │
│  │  ├─ 财务报表                                 │  │
│  │  ├─ 销售报表                                 │  │
│  │  └─ 批次报表                                 │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 📊 三、四级批次管理融合 (更正版)

### 3.1 四级批次层级关系

```
成品 (Product)
  └── 色号 (Color No)
        └── 缸号 (Dye Lot No)
              └── 匹号 (Piece No)
```

### 3.2 关键业务场景 (更正)

#### **场景 1: 销售订单 (不需要缸号/匹号)**

```rust
// 销售订单明细
pub struct SalesOrderDetail {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_code: String,      // 我方成品编码
    pub color_no: String,          // 色号 (需要)
    pub quantity: Decimal,         // 数量
    pub unit: String,              // 单位
    pub price: Decimal,            // 单价
    
    // ❌ 不需要缸号和匹号
    // pub dye_lot_no: String,     // 删除
    // pub piece_nos: Vec<String>, // 删除
}
```

**说明**: 销售订单只需指定成品和色号，不需要指定具体的缸号和匹号。

---

#### **场景 2: 销售发货单 (需要缸号/匹号)**

```rust
// 销售发货单
pub struct SalesDelivery {
    pub id: i64,
    pub delivery_no: String,
    pub sales_order_id: i64,
    pub customer_id: i64,
    pub delivery_date: NaiveDate,
    
    // 发货明细 (含四级批次)
    pub details: Vec<SalesDeliveryDetail>,
}

// 销售发货单明细
pub struct SalesDeliveryDetail {
    pub id: i64,
    pub delivery_id: i64,
    pub product_id: i64,
    pub product_code: String,      // 我方成品编码
    pub color_no: String,          // 色号
    pub dye_lot_no: String,        // 缸号 ✅ (需要)
    pub piece_nos: Vec<String>,    // 匹号列表 ✅ (需要)
    pub total_length: Decimal,     // 总长度 (米)
    pub total_weight: Decimal,     // 总重量 (公斤)
    pub quantity: Decimal,         // 数量
}
```

**业务流程**:
```
销售订单 (成品 + 色号)
    ↓
库存分配 (根据色号分配库存)
    ↓
生成发货单
    ↓
拣货 (从库存中拣选具体缸号/匹号)
    ↓
发货单明细 (成品 + 色号 + 缸号 + 匹号)
    ↓
出库
```

---

#### **场景 3: 采购订单 (不需要缸号/匹号)**

```rust
// 采购订单明细 (供应商编码)
pub struct PurchaseOrderDetail {
    pub id: i64,
    pub order_id: i64,
    pub supplier_product_code: String,  // 供应商成品编码
    pub supplier_color_code: String,    // 供应商色号
    pub quantity: Decimal,
    pub unit: String,
    pub price: Decimal,
    
    // ❌ 不需要缸号和匹号
    // pub supplier_dye_lot_no: String,  // 删除
    // pub supplier_piece_nos: Vec<String>, // 删除
}
```

**说明**: 采购订单只需指定供应商成品编码和色号，不需要指定具体的缸号和匹号。

---

#### **场景 4: 采购收货单 (需要缸号/匹号)**

```rust
// 采购收货单
pub struct PurchaseReceipt {
    pub id: i64,
    pub receipt_no: String,
    pub purchase_order_id: i64,
    pub supplier_id: i64,
    pub receipt_date: NaiveDate,
    
    // 收货明细 (含四级批次)
    pub details: Vec<PurchaseReceiptDetail>,
}

// 采购收货单明细
pub struct PurchaseReceiptDetail {
    pub id: i64,
    pub receipt_id: i64,
    pub product_id: i64,
    pub supplier_product_code: String,  // 供应商成品编码
    pub supplier_color_code: String,    // 供应商色号
    pub supplier_dye_lot_no: String,    // 供应商缸号 ✅ (需要)
    pub supplier_piece_nos: Vec<String>, // 供应商匹号列表 ✅ (需要)
    
    // 我方编码 (用于入库)
    pub internal_product_code: String,  // 我方成品编码
    pub internal_color_no: String,      // 我方色号
    pub internal_dye_lot_no: String,    // 我方缸号
    pub internal_piece_nos: Vec<String>, // 我方匹号列表
    
    pub total_length: Decimal,          // 总长度 (米)
    pub total_weight: Decimal,          // 总重量 (公斤)
    pub quantity: Decimal,              // 数量
}
```

**业务流程**:
```
采购订单 (供应商成品 + 色号)
    ↓
供应商发货
    ↓
到货验收
    ↓
录入四级批次 (成品 + 色号 + 缸号 + 匹号)
    ↓
采购收货单明细 (含四级批次)
    ↓
入库 (创建库存记录，含四级批次)
```

---

### 3.3 数据库设计 (更正版)

#### **销售订单明细表 (不需要缸号/匹号)**

```sql
-- 扩展现有 sales_order_details 表
ALTER TABLE sales_order_details ADD COLUMN color_no VARCHAR(100) COMMENT '色号 (我方)';
ALTER TABLE sales_order_details ADD COLUMN total_length DECIMAL(12,2) COMMENT '总长度 (米)';
ALTER TABLE sales_order_details ADD COLUMN total_weight DECIMAL(12,2) COMMENT '总重量 (公斤)';

-- 创建索引
CREATE INDEX idx_sales_detail_color ON sales_order_details(color_no);
```

**说明**: 销售订单明细只需要色号，不需要缸号和匹号。

---

#### **销售发货单明细表 (需要缸号/匹号)**

```sql
-- 新增销售发货单表
CREATE TABLE sales_delivery (
    id BIGSERIAL PRIMARY KEY,
    delivery_no VARCHAR(100) NOT NULL UNIQUE COMMENT '发货单号',
    sales_order_id BIGINT NOT NULL COMMENT '销售订单 ID',
    customer_id BIGINT NOT NULL COMMENT '客户 ID',
    delivery_date DATE NOT NULL COMMENT '发货日期',
    
    -- 状态
    status VARCHAR(50) DEFAULT 'pending' COMMENT '状态：pending/partial/shipped',
    
    -- 系统字段
    created_by BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- 外键
    FOREIGN KEY (sales_order_id) REFERENCES sales_order(id),
    FOREIGN KEY (customer_id) REFERENCES customer(id)
);

COMMENT ON TABLE sales_delivery IS '销售发货单表';

-- 销售发货单明细表
CREATE TABLE sales_delivery_detail (
    id BIGSERIAL PRIMARY KEY,
    delivery_id BIGINT NOT NULL COMMENT '发货单 ID',
    product_id BIGINT NOT NULL COMMENT '成品 ID',
    product_code VARCHAR(100) NOT NULL COMMENT '我方成品编码',
    color_no VARCHAR(100) NOT NULL COMMENT '色号 (我方)',
    dye_lot_no VARCHAR(100) NOT NULL COMMENT '缸号 (我方)',
    piece_nos TEXT[] NOT NULL COMMENT '匹号列表 (我方)',
    total_length DECIMAL(12,2) COMMENT '总长度 (米)',
    total_weight DECIMAL(12,2) COMMENT '总重量 (公斤)',
    quantity DECIMAL(12,2) NOT NULL COMMENT '数量',
    unit VARCHAR(20) NOT NULL COMMENT '单位',
    
    -- 外键
    FOREIGN KEY (delivery_id) REFERENCES sales_delivery(id),
    FOREIGN KEY (product_id) REFERENCES product(id)
);

COMMENT ON TABLE sales_delivery_detail IS '销售发货单明细表';
COMMENT ON COLUMN sales_delivery_detail.dye_lot_no IS '缸号 (我方)';
COMMENT ON COLUMN sales_delivery_detail.piece_nos IS '匹号列表 (我方)';

-- 创建索引
CREATE INDEX idx_delivery_detail_delivery ON sales_delivery_detail(delivery_id);
CREATE INDEX idx_delivery_detail_color ON sales_delivery_detail(color_no);
CREATE INDEX idx_delivery_detail_dye_lot ON sales_delivery_detail(dye_lot_no);
CREATE INDEX idx_delivery_detail_piece ON sales_delivery_detail USING GIN(piece_nos);
```

---

#### **采购订单明细表 (不需要缸号/匹号)**

```sql
-- 扩展现有 purchase_order_details 表
ALTER TABLE purchase_order_details ADD COLUMN supplier_product_code VARCHAR(100) COMMENT '供应商成品编码';
ALTER TABLE purchase_order_details ADD COLUMN supplier_product_name VARCHAR(200) COMMENT '供应商成品名称';
ALTER TABLE purchase_order_details ADD COLUMN supplier_color_code VARCHAR(100) COMMENT '供应商色号编码';
ALTER TABLE purchase_order_details ADD COLUMN supplier_color_name VARCHAR(200) COMMENT '供应商色号名称';
ALTER TABLE purchase_order_details ADD COLUMN is_from_sales_order BOOLEAN DEFAULT FALSE COMMENT '是否来自销售订单';
ALTER TABLE purchase_order_details ADD COLUMN sales_order_id BIGINT COMMENT '销售订单 ID';
ALTER TABLE purchase_order_details ADD COLUMN sales_order_no VARCHAR(50) COMMENT '销售订单号';

-- 创建索引
CREATE INDEX idx_purchase_detail_supplier_code ON purchase_order_details(supplier_product_code);
CREATE INDEX idx_purchase_detail_supplier_color ON purchase_order_details(supplier_color_code);
CREATE INDEX idx_purchase_detail_sales_order ON purchase_order_details(sales_order_id);
```

**说明**: 采购订单明细只需要供应商成品编码和色号，不需要缸号和匹号。

---

#### **采购收货单明细表 (需要缸号/匹号)**

```sql
-- 扩展现有 purchase_receipt_details 表
ALTER TABLE purchase_receipt_details ADD COLUMN supplier_product_code VARCHAR(100) COMMENT '供应商成品编码';
ALTER TABLE purchase_receipt_details ADD COLUMN supplier_color_code VARCHAR(100) COMMENT '供应商色号编码';
ALTER TABLE purchase_receipt_details ADD COLUMN supplier_dye_lot_no VARCHAR(100) COMMENT '供应商缸号';
ALTER TABLE purchase_receipt_details ADD COLUMN supplier_piece_nos TEXT[] COMMENT '供应商匹号列表';

ALTER TABLE purchase_receipt_details ADD COLUMN internal_product_code VARCHAR(100) COMMENT '我方成品编码';
ALTER TABLE purchase_receipt_details ADD COLUMN internal_color_no VARCHAR(100) COMMENT '我方色号';
ALTER TABLE purchase_receipt_details ADD COLUMN internal_dye_lot_no VARCHAR(100) COMMENT '我方缸号';
ALTER TABLE purchase_receipt_details ADD COLUMN internal_piece_nos TEXT[] COMMENT '我方匹号列表';

ALTER TABLE purchase_receipt_details ADD COLUMN total_length DECIMAL(12,2) COMMENT '总长度 (米)';
ALTER TABLE purchase_receipt_details ADD COLUMN total_weight DECIMAL(12,2) COMMENT '总重量 (公斤)';

-- 创建索引
CREATE INDEX idx_receipt_detail_supplier_lot ON purchase_receipt_details(supplier_dye_lot_no);
CREATE INDEX idx_receipt_detail_supplier_piece ON purchase_receipt_details USING GIN(supplier_piece_nos);
CREATE INDEX idx_receipt_detail_internal_lot ON purchase_receipt_details(internal_dye_lot_no);
CREATE INDEX idx_receipt_detail_internal_piece ON purchase_receipt_details USING GIN(internal_piece_nos);
```

---

### 3.4 核心服务实现 (更正版)

#### **编码转换服务 (更正)**

```rust
// backend/src/services/code_conversion_service.rs

impl CodeConversionService {
    /// 销售订单→采购订单 (仅转换成品和色号)
    pub async fn convert_sales_order_to_purchase(
        &self,
        sales_order_id: i64,
        supplier_id: i64,
    ) -> Result<PurchaseOrderData, AppError> {
        // 1. 获取销售订单明细 (仅成品 + 色号)
        let sales_details = get_sales_order_details(&self.db, sales_order_id).await?;
        
        let mut purchase_details = Vec::new();
        
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
            
            // 创建采购订单明细 (仅成品 + 色号)
            let purchase_detail = PurchaseOrderDetail {
                supplier_product_code: product_mapping.supplier_product_code,
                supplier_product_name: product_mapping.supplier_product_name,
                supplier_color_code: color_mapping.as_ref().map(|m| m.supplier_color_code.clone()),
                supplier_color_name: color_mapping.as_ref().map(|m| m.supplier_color_name.clone()),
                quantity: detail.quantity,
                unit: detail.unit,
                price: detail.price,
                is_from_sales_order: true,
                sales_order_id: Some(sales_order_id),
                // ❌ 不需要缸号和匹号
            };
            
            purchase_details.push(purchase_detail);
        }
        
        Ok(PurchaseOrderData {
            details: purchase_details,
        })
    }
    
    /// 采购收货时转换四级批次
    pub async fn convert_purchase_receipt_batch(
        &self,
        supplier_product_code: &str,
        supplier_color_code: &str,
        supplier_dye_lot_no: &str,
        supplier_piece_nos: &[String],
        supplier_id: i64,
    ) -> Result<BatchConversionResult, AppError> {
        // 1. 转换成品编码
        let product_mapping = self.convert_product_to_supplier(
            supplier_product_code,
            supplier_id,
        ).await?;
        
        // 2. 转换色号
        let color_mapping = self.convert_supplier_color_to_internal(
            supplier_color_code,
            supplier_id,
        ).await?;
        
        // 3. 转换缸号
        let dye_lot_mapping = self.convert_supplier_dye_lot_to_internal(
            supplier_dye_lot_no,
            supplier_id,
        ).await?;
        
        // 4. 转换匹号
        let mut piece_mappings = Vec::new();
        for supplier_piece_no in supplier_piece_nos {
            let piece_mapping = self.convert_supplier_piece_to_internal(
                supplier_piece_no,
                supplier_id,
            ).await?;
            piece_mappings.push(piece_mapping);
        }
        
        Ok(BatchConversionResult {
            internal_product_code: product_mapping.internal_product_code,
            internal_color_no: color_mapping.internal_color_code,
            internal_dye_lot_no: dye_lot_mapping.internal_dye_lot_no,
            internal_piece_nos: piece_mappings.iter().map(|m| m.internal_piece_no.clone()).collect(),
        })
    }
}
```

---

#### **销售发货服务 (更正)**

```rust
// backend/src/services/sales_delivery_service.rs

pub struct SalesDeliveryService {
    db: PgPool,
}

impl SalesDeliveryService {
    /// 创建销售发货单
    pub async fn create_sales_delivery(
        &self,
        req: CreateSalesDeliveryRequest,
    ) -> Result<SalesDelivery, AppError> {
        // 1. 获取销售订单
        let sales_order = get_sales_order(&self.db, req.sales_order_id).await?;
        
        // 2. 根据销售订单明细分配库存 (含四级批次)
        let allocated_batches = self.allocate_inventory(
            sales_order.id,
            &sales_order.details,
        ).await?;
        
        // 3. 创建发货单
        let delivery = SalesDelivery {
            delivery_no: generate_delivery_no(),
            sales_order_id: sales_order.id,
            customer_id: sales_order.customer_id,
            delivery_date: Utc::now().naive_utc().date(),
            status: "pending".to_string(),
            created_by: req.created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let saved_delivery = save_delivery(&self.db, delivery).await?;
        
        // 4. 创建发货单明细 (含四级批次)
        for (detail, batch) in sales_order.details.iter().zip(allocated_batches.iter()) {
            let delivery_detail = SalesDeliveryDetail {
                delivery_id: saved_delivery.id,
                product_id: detail.product_id,
                product_code: detail.product_code.clone(),
                color_no: detail.color_no.clone(),
                dye_lot_no: batch.dye_lot_no.clone(),  // ✅ 缸号
                piece_nos: batch.piece_nos.clone(),    // ✅ 匹号列表
                total_length: batch.total_length,
                total_weight: batch.total_weight,
                quantity: detail.quantity,
                unit: detail.unit.clone(),
            };
            
            save_delivery_detail(&self.db, delivery_detail).await?;
        }
        
        // 5. 扣减库存
        for batch in allocated_batches {
            deduct_inventory(&self.db, &batch).await?;
        }
        
        Ok(saved_delivery)
    }
    
    /// 分配库存 (含四级批次)
    async fn allocate_inventory(
        &self,
        sales_order_id: i64,
        details: &[SalesOrderDetail],
    ) -> Result<Vec<AllocatedBatch>, AppError> {
        let mut allocated_batches = Vec::new();
        
        for detail in details {
            // 根据成品 + 色号查询可用库存
            let available_inventory = query_available_inventory(
                &self.db,
                detail.product_id,
                &detail.color_no,
            ).await?;
            
            // 分配库存 (先进先出原则)
            let mut remaining = detail.quantity;
            let mut allocated = Vec::new();
            
            for inv in available_inventory {
                if remaining <= Decimal::ZERO {
                    break;
                }
                
                let allocate_qty = inv.quantity.min(remaining);
                allocated.push(AllocatedBatch {
                    dye_lot_no: inv.dye_lot_no.clone(),
                    piece_nos: inv.piece_nos.clone(),
                    total_length: inv.total_length,
                    total_weight: inv.total_weight,
                    quantity: allocate_qty,
                });
                
                remaining -= allocate_qty;
            }
            
            if remaining > Decimal::ZERO {
                return Err(AppError::ValidationError(
                    format!("库存不足，缺少 {}", remaining)
                ));
            }
            
            allocated_batches.extend(allocated);
        }
        
        Ok(allocated_batches)
    }
}
```

---

#### **采购收货服务 (更正)**

```rust
// backend/src/services/purchase_receipt_service.rs

pub struct PurchaseReceiptService {
    db: PgPool,
    code_conversion: Arc<CodeConversionService>,
}

impl PurchaseReceiptService {
    /// 创建采购收货单
    pub async fn create_purchase_receipt(
        &self,
        req: CreatePurchaseReceiptRequest,
    ) -> Result<PurchaseReceipt, AppError> {
        // 1. 获取采购订单
        let purchase_order = get_purchase_order(&self.db, req.purchase_order_id).await?;
        
        // 2. 创建收货单
        let receipt = PurchaseReceipt {
            receipt_no: generate_receipt_no(),
            purchase_order_id: purchase_order.id,
            supplier_id: purchase_order.supplier_id,
            receipt_date: Utc::now().naive_utc().date(),
            status: "pending".to_string(),
            created_by: req.created_by,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let saved_receipt = save_receipt(&self.db, receipt).await?;
        
        // 3. 创建收货单明细 (录入四级批次)
        for detail in req.details {
            // 转换四级批次编码 (供应商→我方)
            let batch_conversion = self.code_conversion
                .convert_purchase_receipt_batch(
                    &detail.supplier_product_code,
                    &detail.supplier_color_code,
                    &detail.supplier_dye_lot_no,
                    &detail.supplier_piece_nos,
                    purchase_order.supplier_id,
                )
                .await?;
            
            let receipt_detail = PurchaseReceiptDetail {
                receipt_id: saved_receipt.id,
                product_id: detail.product_id,
                supplier_product_code: detail.supplier_product_code.clone(),
                supplier_color_code: detail.supplier_color_code.clone(),
                supplier_dye_lot_no: detail.supplier_dye_lot_no.clone(),
                supplier_piece_nos: detail.supplier_piece_nos.clone(),
                internal_product_code: batch_conversion.internal_product_code,
                internal_color_no: batch_conversion.internal_color_no,
                internal_dye_lot_no: batch_conversion.internal_dye_lot_no,
                internal_piece_nos: batch_conversion.internal_piece_nos,
                total_length: detail.total_length,
                total_weight: detail.total_weight,
                quantity: detail.quantity,
            };
            
            save_receipt_detail(&self.db, receipt_detail).await?;
            
            // 4. 创建库存记录
            create_inventory_from_receipt(&self.db, &receipt_detail).await?;
        }
        
        Ok(saved_receipt)
    }
}
```

---

## 📅 四、整合实施路线图 (20 周)

### 阶段一：基础框架 (第 1-4 周) ⭐⭐⭐

#### 第 1-2 周：数据库建设
- [ ] 创建四级批次表 (7 张)
- [ ] 扩展销售/采购表
- [ ] 创建 BPM 流程表 (12 张)
- [ ] 创建日志表 (4 张)
- [ ] 准备测试数据

**交付物**: 完整的数据库结构

#### 第 3-4 周：基础服务
- [ ] 实现批次管理服务
- [ ] 实现编码转换服务
- [ ] 实现 BPM 基础服务
- [ ] 实现日志服务
- [ ] 单元测试

**交付物**: 基础服务层

---

### 阶段二：BPM 流程融合 (第 5-8 周) ⭐⭐⭐

#### 第 5-6 周：采购审批流
- [ ] 实现采购订单审批流程
- [ ] 修改采购订单 Handler
- [ ] 前端添加审批 UI
- [ ] 联调测试

#### 第 7-8 周：销售审批流
- [ ] 实现销售订单审批流程
- [ ] 实现信用审批流程
- [ ] 修改销售订单 Handler
- [ ] 前端添加审批 UI

**交付物**: BPM 流程引擎 + 采购/销售审批

---

### 阶段三：四级批次融合 (第 9-13 周) ⭐⭐⭐

#### 第 9-10 周：采购收货四级批次
- [ ] 实现采购收货服务
- [ ] 实现四级批次录入
- [ ] 实现编码转换
- [ ] 前端添加收货页面
- [ ] 联调测试

#### 第 11-12 周：销售发货四级批次
- [ ] 实现销售发货服务
- [ ] 实现库存分配
- [ ] 实现四级批次扣减
- [ ] 前端添加发货页面
- [ ] 联调测试

#### 第 13 周：批次追溯
- [ ] 实现批次追溯服务
- [ ] 创建追溯日志
- [ ] 前端追溯查询
- [ ] 完整测试

**交付物**: 四级批次管理完整功能

---

### 阶段四：CRM 扩展 (第 14-16 周) ⭐⭐

#### 第 14 周：线索/商机管理
- [ ] 创建线索/商机表
- [ ] 实现线索服务
- [ ] 实现商机服务
- [ ] 前端页面

#### 第 15-16 周：销售漏斗
- [ ] 实现销售漏斗服务
- [ ] 与现有销售订单融合
- [ ] 前端漏斗报表
- [ ] 联调测试

**交付物**: CRM 扩展功能

---

### 阶段五：OA 和报表 (第 17-20 周) ⭐⭐

#### 第 17 周：OA 通知
- [ ] 实现通知公告服务
- [ ] 业务事件通知
- [ ] 前端通知页面

#### 第 18-19 周：报表系统
- [ ] 创建报表表
- [ ] 实现报表服务
- [ ] 财务报表
- [ ] 销售报表
- [ ] 批次报表

#### 第 20 周：集成测试
- [ ] 系统集成测试
- [ ] 性能测试
- [ ] 文档完善
- [ ] 上线准备

**交付物**: OA 通知 + 报表系统 + 完整生产系统

---

## ✅ 五、验收标准

### 5.1 功能验收

| 功能模块 | 验收标准 | 优先级 |
|---------|---------|--------|
| **BPM 采购审批** | 采购订单可发起审批 | P0 |
| **BPM 销售审批** | 销售订单可发起信用审批 | P0 |
| **四级批次采购收货** | 收货时录入缸号/匹号 | P0 |
| **四级批次销售发货** | 发货时分配缸号/匹号 | P0 |
| **编码转换** | 供应商/我方编码准确转换 | P0 |
| **批次追溯** | 支持正反向追溯 | P1 |
| **CRM 线索管理** | 线索→商机→订单转化 | P1 |
| **OA 通知** | 业务事件自动通知 | P2 |
| **财务报表** | 自动生成财务 dashboard | P2 |
| **批次报表** | 四级批次统计报表 | P1 |

### 5.2 数据一致性验收

- ✅ 销售订单：仅成品 + 色号
- ✅ 销售发货单：成品 + 色号 + 缸号 + 匹号
- ✅ 采购订单：仅供应商成品 + 色号
- ✅ 采购收货单：供应商 + 我方四级批次
- ✅ 编码转换准确率 100%
- ✅ 批次追溯完整率 100%

---

## 📊 六、关键更正点总结

### 6.1 销售流程

```
销售订单 (成品 + 色号)
    ↓ 不需要缸号/匹号
库存分配
    ↓
销售发货单 (成品 + 色号 + 缸号 + 匹号) ✅
    ↓
出库
```

### 6.2 采购流程

```
采购订单 (供应商成品 + 色号)
    ↓ 不需要缸号/匹号
供应商发货
    ↓
采购收货单 (供应商 + 我方四级批次) ✅
    ↓
入库
```

### 6.3 关键区别

| 单据类型 | 成品 | 色号 | 缸号 | 匹号 |
|---------|------|------|------|------|
| 销售订单 | ✅ | ✅ | ❌ | ❌ |
| 销售发货单 | ✅ | ✅ | ✅ | ✅ |
| 采购订单 | ✅ (供应商) | ✅ (供应商) | ❌ | ❌ |
| 采购收货单 | ✅ (双方) | ✅ (双方) | ✅ (双方) | ✅ (双方) |

---

**整合方案完成!** 接下来可以开始实施阶段一的数据库建设。🚀

---

**文档创建时间**: 2026-03-16  
**文档版本**: v1.0 (整合版)  
**适用范围**: 秉羲 ERP 功能模块全面集成项目
