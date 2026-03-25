# 面料四级批次管理 - 执行摘要

## 📋 项目概述

实现**成品 - 多色号 - 缸号 - 匹号**四级批次管理体系，并建立与供应商系统的编码映射和自动转换机制，确保销售开单到采购单生成的全流程自动化。

---

## 🎯 核心需求

### 需求 1: 四级批次层级化管理
- ✅ 支持成品 - 色号 - 缸号 - 匹号四级管理
- ✅ 信息录入、存储、查询、维护功能
- ✅ 完整的批次追溯能力

### 需求 2: 供应商端系统对接
- ✅ 供应商内部成品 - 色号管理
- ✅ 我方与供应商编码对应关系
- ✅ 双方信息准确匹配

### 需求 3: 销售开单流程
- ✅ 使用我方内部编码录入
- ✅ 支持四级批次选择

### 需求 4: 采购单自动生成
- ✅ 销售开单后自动触发
- ✅ 我方编码→供应商编码转换
- ✅ 仅显示供应商编码，隐藏我方信息

### 需求 5: 数据映射机制
- ✅ 成品编码映射
- ✅ 色号编码映射
- ✅ 缸号编码映射
- ✅ 匹号编码映射
- ✅ 转换准确无误

### 需求 6: 数据一致性校验
- ✅ 转换前自动验证
- ✅ 防止采购错误

### 需求 7: 操作日志记录
- ✅ 完整记录全过程
- ✅ 数据转换记录
- ✅ 审计和问题追溯

---

## 🏗️ 解决方案架构

### 整体架构

```
销售开单 (我方编码)
    ↓
┌─────────────────────────────────────┐
│  四级批次管理系统                    │
│  ├─ 成品管理 (Product)              │
│  ├─ 色号管理 (Color No)             │
│  ├─ 缸号管理 (Dye Lot No)           │
│  └─ 匹号管理 (Piece No)             │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  编码映射转换系统                    │
│  ├─ 成品编码映射                    │
│  ├─ 色号编码映射                    │
│  ├─ 缸号编码映射                    │
│  └─ 匹号编码映射                    │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  数据一致性校验                      │
│  ├─ 映射关系验证                    │
│  ├─ 数量验证                        │
│  └─ 业务规则验证                    │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  采购单生成 (供应商编码)             │
│  - 仅显示供应商编码                  │
│  - 隐藏我方内部信息                  │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  操作日志记录                        │
│  - 完整记录转换过程                  │
│  - 支持审计追溯                      │
└─────────────────────────────────────┘
```

---

## 📊 数据库设计要点

### 核心表 (7 张新增表)

| 表名 | 用途 | 关键字段 |
|------|------|---------|
| **batch_dye_lot** | 缸号管理 | dye_lot_no, product_id, color_id, supplier_dye_lot_no |
| **inventory_piece** | 匹号管理 | piece_no, dye_lot_id, supplier_piece_no, length, weight |
| **product_code_mapping** | 成品编码映射 | internal_product_code, supplier_product_code |
| **color_code_mapping** | 色号编码映射 | internal_color_code, supplier_color_code |
| **dye_lot_mapping** | 缸号编码映射 | internal_dye_lot_no, supplier_dye_lot_no |
| **piece_mapping** | 匹号编码映射 | internal_piece_no, supplier_piece_no |
| **batch_trace_log** | 追溯日志 | trace_no, business_type, conversion_details |

### 扩展现有表 (4 张表)

| 表名 | 新增字段 |
|------|---------|
| **product** | supplier_product_code, supplier_product_name, category_type |
| **product_color** | supplier_color_code, supplier_color_name, pantone_code |
| **sales_order_details** | color_no, dye_lot_no, piece_nos, total_length, total_weight |
| **purchase_order_details** | supplier_product_code, supplier_color_code, supplier_dye_lot_no, sales_order_id |

---

## 🔧 核心服务 (3 个)

### 1. 批次管理服务 (`BatchManagementService`)

**功能**:
- ✅ 创建缸号批次
- ✅ 创建匹号
- ✅ 四级批次查询
- ✅ 正反向批次追溯

**关键方法**:
```rust
// 创建缸号
pub async fn create_dye_lot(&self, req: CreateBatchDyeLotRequest) -> Result<BatchDyeLot>

// 创建匹号
pub async fn create_piece(&self, req: CreateInventoryPieceRequest) -> Result<InventoryPiece>

// 查询四级批次
pub async fn query_batch_hierarchy(...) -> Result<BatchHierarchy>

// 正向追溯 (匹号→销售)
pub async fn trace_forward(&self, piece_no: &str) -> Result<BatchTraceResult>

// 反向追溯 (成品→匹号)
pub async fn trace_backward(&self, product_id: i64, ...) -> Result<Vec<BatchTraceResult>>
```

---

### 2. 编码转换服务 (`CodeConversionService`)

**功能**:
- ✅ 我方编码→供应商编码转换
- ✅ 批量转换 (销售→采购)
- ✅ 数据一致性校验

**关键方法**:
```rust
// 成品编码转换
pub async fn convert_product_to_supplier(
    &self,
    internal_product_code: &str,
    supplier_id: i64,
) -> Result<ProductCodeMapping>

// 色号编码转换
pub async fn convert_color_to_supplier(
    &self,
    internal_color_code: &str,
    supplier_id: i64,
) -> Result<ColorCodeMapping>

// 批量转换 (销售→采购)
pub async fn convert_sales_to_purchase(
    &self,
    sales_order_id: i64,
    supplier_id: i64,
) -> Result<PurchaseOrderData>

// 数据一致性校验
pub async fn validate_conversion(
    &self,
    sales_order_id: i64,
    supplier_id: i64,
) -> Result<ValidationResult>
```

**校验逻辑**:
```rust
pub async fn validate_conversion(&self, sales_order_id: i64, supplier_id: i64) -> ValidationResult {
    let mut errors = Vec::new();
    
    // 1. 校验成品编码映射
    if !self.product_exists(internal_code, supplier_id) {
        errors.push("成品编码映射不存在");
    }
    
    // 2. 校验色号映射
    if !self.color_exists(internal_color, supplier_id) {
        errors.push("色号编码映射不存在");
    }
    
    // 3. 校验缸号映射
    if !self.dye_lot_exists(internal_dye_lot, supplier_id) {
        errors.push("缸号编码映射不存在");
    }
    
    // 4. 校验匹号映射
    for piece_no in piece_nos {
        if !self.piece_exists(piece_no, supplier_id) {
            errors.push(format!("匹号 {} 映射不存在", piece_no));
        }
    }
    
    // 5. 校验数量
    if quantity <= 0.0 {
        errors.push("数量必须大于 0");
    }
    
    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings: vec![],
    }
}
```

---

### 3. 销售转采购服务 (`SalesToPurchaseService`)

**功能**:
- ✅ 从销售订单自动生成采购订单
- ✅ 编码转换
- ✅ 数据校验
- ✅ 追溯日志记录

**关键方法**:
```rust
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
        details: purchase_data.details,  // 仅包含供应商编码
        total_amount: purchase_data.calculate_total(),
        is_from_sales: true,
        source_sales_order_id: Some(sales_order_id),
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
        
        // 我方编码
        internal_product_code: sales_order.product_code,
        internal_color_no: sales_order.color_no,
        internal_dye_lot_no: sales_order.dye_lot_no,
        internal_piece_nos: sales_order.piece_nos,
        
        // 供应商编码
        supplier_product_code: saved_purchase.supplier_product_code,
        supplier_color_code: saved_purchase.supplier_color_code,
        supplier_dye_lot_no: saved_purchase.supplier_dye_lot_no,
        supplier_piece_nos: saved_purchase.supplier_piece_nos,
        
        // 转换详情
        conversion_details: serde_json::to_value(&purchase_data.conversion_logs)?,
        validation_result: "passed".to_string(),
        validation_errors: vec![],
        
        // 目标信息
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
```

---

## 📅 实施路线图 (5 周)

### 第 1 周：数据库建设
- [ ] 创建 7 张新表
- [ ] 扩展 4 张现有表
- [ ] 创建索引和约束
- [ ] 准备测试数据

**交付物**: 完整的数据库结构

---

### 第 2 周：批次管理服务
- [ ] 实现 BatchManagementService
- [ ] 实现缸号创建功能
- [ ] 实现匹号创建功能
- [ ] 实现批次查询功能
- [ ] 实现批次追溯功能

**交付物**: 批次管理服务 (含单元测试)

---

### 第 3 周：编码转换服务
- [ ] 实现 CodeConversionService
- [ ] 实现 4 种编码映射
- [ ] 实现批量转换功能
- [ ] 实现数据校验功能
- [ ] 前端添加映射管理页面

**交付物**: 编码转换服务 + 前端映射管理

---

### 第 4 周：销售转采购服务
- [ ] 实现 SalesToPurchaseService
- [ ] 扩展销售订单 Model
- [ ] 扩展采购订单 Model
- [ ] 实现自动生成逻辑
- [ ] 前端添加销售开单四级批次选择

**交付物**: 销售转采购完整流程

---

### 第 5 周：追溯和日志
- [ ] 实现 BatchTraceService
- [ ] 创建追溯日志表
- [ ] 实现操作日志记录
- [ ] 前端添加追溯查询页面
- [ ] 完整测试和文档

**交付物**: 完整的追溯系统 + 文档

---

## ✅ 验收标准

### 功能验收

| 功能点 | 验收标准 | 优先级 |
|--------|---------|--------|
| 四级批次录入 | 支持成品 - 色号 - 缸号 - 匹号完整录入 | P0 |
| 编码映射管理 | 支持我方/供应商编码映射管理 | P0 |
| 销售开单 | 使用我方四级批次编码开单 | P0 |
| 采购单生成 | 自动生成，仅显示供应商编码 | P0 |
| 数据校验 | 转换前自动校验，阻止错误 | P0 |
| 操作日志 | 完整记录转换全过程 | P0 |
| 批次追溯 | 支持正反向追溯查询 | P1 |

### 性能验收

- ✅ 编码转换时间 < 100ms
- ✅ 采购单生成时间 < 2s
- ✅ 追溯查询时间 < 1s
- ✅ 并发支持 > 100 QPS

### 数据一致性验收

- ✅ 映射关系准确率 100%
- ✅ 转换错误率 0%
- ✅ 追溯数据完整率 100%
- ✅ 无数据丢失

---

## 💡 关键设计要点

### 1. 双编码体系隔离

```rust
// 我方编码体系 (内部使用)
struct InternalCode {
    product_code: String,   // 我方成品编码
    color_no: String,       // 我方色号
    dye_lot_no: String,     // 我方缸号
    piece_no: String,       // 我方匹号
}

// 供应商编码体系 (对外使用)
struct SupplierCode {
    product_code: String,   // 供应商成品编码
    color_code: String,     // 供应商色号
    dye_lot_no: String,     // 供应商缸号
    piece_no: String,       // 供应商匹号
}

// 映射关系
struct CodeMapping {
    internal: InternalCode,
    supplier: SupplierCode,
    supplier_id: i64,
    is_active: bool,
}
```

### 2. 采购单数据隔离

```rust
// 采购订单明细 (仅显示供应商编码)
struct PurchaseOrderDetail {
    // 供应商编码 (显示)
    supplier_product_code: String,
    supplier_product_name: String,
    supplier_color_code: String,
    supplier_color_name: String,
    supplier_dye_lot_no: String,
    
    // 不显示我方编码
    // ❌ internal_product_code: String,
    // ❌ internal_color_no: String,
    // ❌ internal_dye_lot_no: String,
    
    // 来源信息 (内部使用)
    is_from_sales_order: bool,
    sales_order_id: Option<i64>,
}
```

### 3. 完整的追溯链

```
销售订单 (我方编码)
    ↓ 转换
编码映射表 (映射关系)
    ↓ 验证
数据校验日志 (验证记录)
    ↓ 生成
采购订单 (供应商编码)
    ↓ 记录
追溯日志 (完整记录)
    ↓ 追溯
正反向追溯查询
```

---

## 📊 预期收益

### 业务价值

1. **自动化**: 销售→采购转换 100% 自动化
2. **准确性**: 编码转换准确率 100%
3. **效率**: 开单到采购时间从小时级降至秒级
4. **追溯**: 完整的四级批次追溯能力
5. **合规**: 供应商数据隔离，符合商业保密

### 技术价值

1. **统一**: 统一的批次管理模型
2. **扩展**: 易于扩展新的供应商
3. **维护**: 映射关系可配置，无需改代码
4. **审计**: 完整的操作日志和追溯链

---

## 🎯 下一步行动

### 立即开始 (本周)

1. [ ] 阅读完整方案：[`BATCH_FOUR_LEVEL_INTEGRATION.md`](file:///e:/1/10/bingxi-rust/docs/BATCH_FOUR_LEVEL_INTEGRATION.md)
2. [ ] 创建 Git 分支：`feature/four-level-batch`
3. [ ] 准备开发环境
4. [ ] 开始第 1 周：数据库建设

### 本周目标

- [ ] 完成 7 张新表的创建
- [ ] 完成 4 张表的扩展
- [ ] 创建所有索引和约束
- [ ] 准备测试数据

---

**方案完成，开始实施!** 🚀

---

**文档创建时间**: 2026-03-16  
**文档版本**: v1.0  
**适用范围**: 秉羲 ERP 面料四级批次管理项目
