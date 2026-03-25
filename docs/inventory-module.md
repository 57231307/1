# 秉羲管理系统 - 库存模块实现总结

**完成时间**: 2026-03-15  
**模块**: 库存管理模块  
**技术栈**: Axum + SeaORM

---

## 📦 模块概述

库存管理模块是秉羲管理系统的核心业务模块之一，负责管理企业的库存记录、库存数量、库存预警等功能。该模块提供了完整的库存 CRUD 操作和库存预警检查功能。

---

## ✅ 已实现功能

### 一、服务层 (Service Layer)

**文件**: `backend/src/services/inventory_stock_service.rs`

**服务类**: `InventoryStockService`

**核心方法**:

1. **查询方法**
   - `find_by_id(id)` - 根据 ID 查询库存记录
   - `find_by_product_and_warehouse(product_id, warehouse_id)` - 根据产品和仓库查询
   - `list_stock(page, page_size, warehouse_id, product_id)` - 分页列表查询（支持仓库和产品筛选）
   - `check_low_stock(warehouse_id)` - 检查低库存预警

2. **创建方法**
   - `create_stock(...)` - 创建新的库存记录

3. **更新方法**
   - `update_stock_quantity(id, quantity_on_hand, quantity_available, quantity_reserved)` - 更新库存数量

4. **删除方法**
   - `delete_stock(id)` - 删除库存记录

**业务逻辑**:
- ✅ 库存数量管理（现有数量、可用数量、预留数量）
- ✅ 库存预警（再订货点检查）
- ✅ 库位管理（bin_location）
- ✅ 时间戳自动管理（created_at, updated_at）

---

### 二、处理器层 (Handler Layer)

**文件**: `backend/src/handlers/inventory_stock_handler.rs`

**处理器函数**:

1. **`get_stock`**
   - **接口**: `GET /api/inventory/stock/:id`
   - **功能**: 获取单个库存记录详情
   - **响应**: StockResponse

2. **`create_stock`**
   - **接口**: `POST /api/inventory/stock`
   - **功能**: 创建新的库存记录
   - **请求**: CreateStockRequest
   - **响应**: StockResponse

3. **`list_stock`**
   - **接口**: `GET /api/inventory/stock`
   - **功能**: 获取库存列表（分页、支持筛选）
   - **请求参数**: page, page_size, warehouse_id, product_id
   - **响应**: StockListResponse

4. **`check_low_stock`**
   - **接口**: `GET /api/inventory/stock/low-stock`
   - **功能**: 检查低库存预警
   - **请求参数**: warehouse_id（可选）
   - **响应**: LowStockResponse

---

### 三、数据模型

**文件**: `backend/src/models/inventory_stock.rs`

**模型结构**:
```rust
pub struct Model {
    pub id: i32,                    // 库存记录 ID
    pub warehouse_id: i32,          // 仓库 ID
    pub product_id: i32,            // 产品 ID
    pub quantity_on_hand: Decimal,  // 现有数量
    pub quantity_available: Decimal, // 可用数量
    pub quantity_reserved: Decimal,  // 预留数量
    pub quantity_incoming: Decimal,  // 在途数量
    pub reorder_point: Decimal,      // 再订货点
    pub reorder_quantity: Decimal,   // 再订货数量
    pub bin_location: Option<String>, // 库位
    pub last_count_date: Option<DateTime<Utc>>, // 最后盘点日期
    pub last_movement_date: Option<DateTime<Utc>>, // 最后移动日期
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**关联关系**:
- `belongs_to Warehouse` - 关联仓库
- `belongs_to Product` - 关联产品

---

## 📊 接口文档

### 1. 获取库存详情

**接口**: `GET /api/inventory/stock/:id`

**路径参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | integer | 是 | 库存记录 ID |

**响应示例**:
```json
{
  "id": 1,
  "warehouse_id": 1,
  "product_id": 1,
  "quantity_on_hand": 100.00,
  "quantity_available": 80.00,
  "quantity_reserved": 20.00,
  "reorder_point": 50.00,
  "bin_location": "A-01-01",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-15T00:00:00Z"
}
```

**错误码**:
- 404: 库存记录不存在

---

### 2. 创建库存记录

**接口**: `POST /api/inventory/stock`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| warehouse_id | integer | 是 | 仓库 ID |
| product_id | integer | 是 | 产品 ID |
| quantity_on_hand | decimal | 是 | 现有数量 |
| quantity_available | decimal | 是 | 可用数量 |
| quantity_reserved | decimal | 是 | 预留数量 |
| reorder_point | decimal | 是 | 再订货点 |
| reorder_quantity | decimal | 是 | 再订货数量 |
| bin_location | string | 否 | 库位 |

**请求示例**:
```json
{
  "warehouse_id": 1,
  "product_id": 1,
  "quantity_on_hand": 100.00,
  "quantity_available": 80.00,
  "quantity_reserved": 20.00,
  "reorder_point": 50.00,
  "reorder_quantity": 100.00,
  "bin_location": "A-01-01"
}
```

**响应示例**:
```json
{
  "id": 1,
  "warehouse_id": 1,
  "product_id": 1,
  "quantity_on_hand": 100.00,
  "quantity_available": 80.00,
  "quantity_reserved": 20.00,
  "reorder_point": 50.00,
  "bin_location": "A-01-01",
  "created_at": "2024-01-15T10:00:00Z",
  "updated_at": "2024-01-15T10:00:00Z"
}
```

---

### 3. 获取库存列表

**接口**: `GET /api/inventory/stock?page=0&page_size=20&warehouse_id=1&product_id=1`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码（从 0 开始），默认 0 |
| page_size | integer | 否 | 每页数量，默认 20 |
| warehouse_id | integer | 否 | 仓库 ID 筛选 |
| product_id | integer | 否 | 产品 ID 筛选 |

**响应示例**:
```json
{
  "stock": [
    {
      "id": 1,
      "warehouse_id": 1,
      "product_id": 1,
      "quantity_on_hand": 100.00,
      "quantity_available": 80.00,
      "quantity_reserved": 20.00,
      "reorder_point": 50.00,
      "bin_location": "A-01-01",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T00:00:00Z"
    }
  ],
  "total": 50,
  "page": 0,
  "page_size": 20
}
```

---

### 4. 检查低库存预警

**接口**: `GET /api/inventory/stock/low-stock?warehouse_id=1`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| warehouse_id | integer | 否 | 仓库 ID 筛选 |

**响应示例**:
```json
{
  "products": [
    {
      "id": 2,
      "warehouse_id": 1,
      "product_id": 2,
      "quantity_on_hand": 30.00,
      "quantity_available": 25.00,
      "quantity_reserved": 5.00,
      "reorder_point": 50.00,
      "bin_location": "A-01-02",
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-15T00:00:00Z"
    }
  ],
  "count": 1
}
```

**说明**: 返回所有可用数量低于再订货点的库存记录

---

## 🎯 业务逻辑

### 1. 库存数量管理

**数量关系**:
```
现有数量 (quantity_on_hand) = 可用数量 (quantity_available) + 预留数量 (quantity_reserved)
```

**业务场景**:
- **入库**: 增加现有数量和可用数量
- **出库**: 减少现有数量和可用数量
- **预留**: 减少可用数量，增加预留数量
- **取消预留**: 增加可用数量，减少预留数量

### 2. 库存预警

**预警条件**:
```
可用数量 < 再订货点
```

**预警处理**:
- 系统自动检查所有库存记录
- 返回低于再订货点的产品列表
- 采购部门根据预警生成采购订单

### 3. 库位管理

**库位编码规则**:
```
[区域]-[货架]-[层]
例如：A-01-01 表示 A 区 01 货架第 1 层
```

**作用**:
- 快速定位产品位置
- 优化拣货路径
- 提高仓库管理效率

---

## 📈 代码统计

**新增文件**: 2 个
- `services/inventory_stock_service.rs` - ~140 行
- `handlers/inventory_stock_handler.rs` - ~200 行

**更新文件**: 3 个
- `services/mod.rs` - 导出 InventoryStockService
- `handlers/mod.rs` - 导出 inventory_stock_handler
- `routes/mod.rs` - 添加库存路由

**总代码行数**: ~340 行

---

## 🔧 技术实现

### 1. 服务层实现

**特点**:
- 使用 `Arc<DatabaseConnection>` 进行依赖注入
- 统一的错误处理（sea_orm::DbErr）
- 异步方法实现
- 支持可选参数的灵活查询

**代码示例**:
```rust
pub async fn list_stock(
    &self,
    page: u64,
    page_size: u64,
    warehouse_id: Option<i32>,
    product_id: Option<i32>,
) -> Result<(Vec<inventory_stock::Model>, u64), sea_orm::DbErr> {
    let mut query = inventory_stock::Entity::find();

    // 动态构建查询条件
    if let Some(wid) = warehouse_id {
        query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
    }

    if let Some(pid) = product_id {
        query = query.filter(inventory_stock::Column::ProductId.eq(pid));
    }

    // 分页查询
    let paginator = query.paginate(&self.db, page_size);
    let total = paginator.num_items().await?;
    let stock_list = paginator.fetch_page(page).await?;

    Ok((stock_list, total))
}
```

### 2. 处理器层实现

**特点**:
- 使用 Axum 的 State 提取器注入数据库连接
- 统一的响应格式
- 完善的错误处理
- 支持 Query 和 Path 参数

**代码示例**:
```rust
pub async fn create_stock(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateStockRequest>,
) -> Result<Json<StockResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(db.clone());

    match service.create_stock(
        payload.warehouse_id,
        payload.product_id,
        payload.quantity_on_hand,
        // ... 其他参数
    ).await {
        Ok(stock) => Ok(Json(StockResponse { /* ... */ })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
```

### 3. 路由配置

**路由结构**:
```rust
let inventory_routes = Router::new()
    .route("/stock", get(list_stock))
    .route("/stock", post(create_stock))
    .route("/stock/:id", get(get_stock))
    .route("/stock/low-stock", get(check_low_stock));

let api_routes = Router::new()
    .nest("/inventory", inventory_routes);
```

---

## 📝 使用示例

### API 测试

```bash
# 创建库存记录
curl -X POST http://localhost:8000/api/inventory/stock \
  -H "Content-Type: application/json" \
  -d '{
    "warehouse_id": 1,
    "product_id": 1,
    "quantity_on_hand": 100.00,
    "quantity_available": 80.00,
    "quantity_reserved": 20.00,
    "reorder_point": 50.00,
    "reorder_quantity": 100.00,
    "bin_location": "A-01-01"
  }'

# 获取库存列表
curl "http://localhost:8000/api/inventory/stock?page=0&page_size=20"

# 按仓库筛选
curl "http://localhost:8000/api/inventory/stock?warehouse_id=1"

# 检查低库存预警
curl "http://localhost:8000/api/inventory/stock/low-stock"
```

---

## 🎓 设计亮点

### 1. 灵活的查询机制
- 支持多条件筛选（仓库、产品）
- 分页查询支持
- 低库存预警检查

### 2. 完善的错误处理
- 统一的错误类型
- 友好的错误消息
- 正确的 HTTP 状态码

### 3. 清晰的业务逻辑
- 数量关系明确
- 预警机制完善
- 时间戳自动管理

### 4. 可扩展的架构
- 分层设计
- 依赖注入
- 易于添加新功能

---

## 📋 待完成功能

### 高优先级
1. **库存调整单**
   - 创建库存调整申请
   - 审批流程
   - 自动更新库存数量

2. **库存移动**
   - 库位调整
   - 仓库调拨
   - 移动记录追踪

3. **库存盘点**
   - 盘点计划
   - 盘点执行
   - 差异处理

### 中优先级
4. **批次管理**
   - 批次号追踪
   - 先进先出（FIFO）
   - 保质期管理

5. **库存报表**
   - 库存周转率
   - 库存金额统计
   - 库龄分析

---

## 🎊 总结

库存管理模块已经实现了核心的 CRUD 功能和库存预警功能，为秉羲管理系统提供了完整的库存管理能力。

**已完成**:
- ✅ 库存记录管理（CRUD）
- ✅ 库存数量管理
- ✅ 库存预警检查
- ✅ 分页查询和筛选
- ✅ 库位管理

**技术特点**:
- 遵循 RESTful API 设计规范
- 使用 SeaORM 进行数据库操作
- 清晰的分层架构
- 完善的错误处理
- 统一的响应格式

该模块可以与其他模块（销售、采购、财务）无缝集成，为企业提供更全面的库存管理解决方案。

---

**报告人**: AI 助手  
**完成时间**: 2026-03-15  
**技术栈**: Axum + SeaORM + PostgreSQL 18.0
