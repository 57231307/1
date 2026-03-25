# 面料 ERP 系统 - 核心业务 API 接口文档

## 文档说明

本文档描述了面料 ERP 系统第一阶段核心业务闭环的所有 HTTP API 接口。

### 基础信息

- **API 版本**: v1
- **基础路径**: `/api/v1/erp`
- **认证方式**: JWT Token（通过 Authorization header 传递）
- **数据格式**: JSON
- **响应格式**: 统一使用 `ApiResponse<T>` 格式

### 统一响应格式

```json
{
  "success": true,
  "message": "操作成功",
  "data": {},
  "timestamp": "2024-01-01T00:00:00Z"
}
```

---

## 一、应收账款管理 (AR Invoice)

### 1.1 查询应收单列表

**接口**: `GET /api/v1/erp/ar/invoices`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| customer_id | int | 否 | 客户 ID |
| status | string | 否 | 状态：active, partially_paid, fully_paid |
| approval_status | string | 否 | 审核状态：pending, approved, rejected |
| batch_no | string | 否 | 批次号（模糊查询） |
| color_no | string | 否 | 色号（模糊查询） |
| page | int | 否 | 页码（从 0 开始），默认 0 |
| page_size | int | 否 | 每页数量，默认 20 |

**响应示例**:
```json
{
  "success": true,
  "message": "查询成功",
  "data": {
    "items": [...],
    "total": 100
  }
}
```

### 1.2 创建应收单

**接口**: `POST /api/v1/erp/ar/invoices`

**请求体**:
```json
{
  "invoiceDate": "2024-01-15",
  "dueDate": "2024-02-15",
  "customerId": 1,
  "customerName": "某某纺织厂",
  "customerCode": "C001",
  "sourceType": "SALES_ORDER",
  "sourceBillId": 100,
  "sourceBillNo": "SO20240115001",
  "invoiceAmount": "10000.00",
  "batchNo": "B20240115",
  "colorNo": "C001",
  "dyeLotNo": "DL20240115",
  "salesOrderNo": "SO20240115001",
  "quantityMeters": "1000.00",
  "quantityKg": "500.00",
  "unitPrice": "10.00",
  "taxAmount": "1300.00"
}
```

**字段说明**:
- `invoiceDate`: 发票日期（格式：YYYY-MM-DD）
- `dueDate`: 到期日期（格式：YYYY-MM-DD）
- `invoiceAmount`: 发票金额（Decimal 字符串）
- `batchNo`, `colorNo`, `dyeLotNo`: 面料行业特色字段（批次、色号、缸号）
- `quantityMeters`, `quantityKg`: 双计量单位（米/公斤）

**响应**: 返回创建的应收单对象

### 1.3 更新应收单

**接口**: `PUT /api/v1/erp/ar/invoices/:id`

**路径参数**: `id` - 应收单 ID

**请求体** (所有字段可选):
```json
{
  "customerName": "新客户名称",
  "customerCode": "C002",
  "invoiceAmount": "12000.00",
  "batchNo": "B20240116",
  "colorNo": "C002",
  "dyeLotNo": "DL20240116",
  "salesOrderNo": "SO20240116001",
  "quantityMeters": "1200.00",
  "quantityKg": "600.00",
  "unitPrice": "10.00",
  "taxAmount": "1560.00"
}
```

**注意**: 只有状态为 `active` 的应收单可以修改

### 1.4 审核应收单

**接口**: `POST /api/v1/erp/ar/invoices/:id/review`

**路径参数**: `id` - 应收单 ID

**响应**: 返回审核后的应收单对象

**业务规则**:
- 只有状态为 `pending` 的应收单可以审核
- 审核后状态变为 `approved`

### 1.5 收款核销

**接口**: `POST /api/v1/erp/ar/invoices/write-off`

**请求体**:
```json
{
  "invoiceId": 1,
  "collectionId": 100,
  "writeOffAmount": "5000.00"
}
```

**字段说明**:
- `invoiceId`: 发票 ID
- `collectionId`: 收款单 ID
- `writeOffAmount`: 核销金额

**业务规则**:
- 核销金额不能超过未付金额
- 核销金额不能超过收款金额
- 自动更新发票状态：
  - 全部核销 → `fully_paid`
  - 部分核销 → `partially_paid`

### 1.6 账龄分析

**接口**: `GET /api/v1/erp/ar/invoices/aging-analysis`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| customer_id | int | 否 | 客户 ID（不传则分析所有客户） |

**响应示例**:
```json
{
  "success": true,
  "data": [
    {
      "customerId": 1,
      "customerName": "某某纺织厂",
      "currentAmount": "5000.00",
      "overdue1To30Days": "2000.00",
      "overdue31To60Days": "1000.00",
      "overdue61To90Days": "500.00",
      "overdueOver90Days": "0.00",
      "totalUnpaidAmount": "8500.00"
    }
  ]
}
```

**账龄分段**:
- `currentAmount`: 未到期金额
- `overdue1To30Days`: 逾期 1-30 天
- `overdue31To60Days`: 逾期 31-60 天
- `overdue61To90Days`: 逾期 61-90 天
- `overdueOver90Days`: 逾期超过 90 天

### 1.7 获取客户未付总额

**接口**: `GET /api/v1/erp/ar/invoices/customer/:id/unpaid-total`

**路径参数**: `id` - 客户 ID

**响应示例**:
```json
{
  "success": true,
  "data": {
    "customerId": 1,
    "totalUnpaidAmount": "8500.00"
  }
}
```

---

## 二、成本归集管理 (Cost Collection)

### 2.1 查询成本归集列表

**接口**: `GET /api/v1/erp/cost/collections`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| batch_no | string | 否 | 批次号（模糊查询） |
| color_no | string | 否 | 色号（模糊查询） |
| workshop | string | 否 | 车间 |
| start_date | string | 否 | 开始日期（YYYY-MM-DD） |
| end_date | string | 否 | 结束日期（YYYY-MM-DD） |
| page | int | 否 | 页码，默认 0 |
| page_size | int | 否 | 每页数量，默认 20 |

### 2.2 创建成本归集

**接口**: `POST /api/v1/erp/cost/collections`

**请求体**:
```json
{
  "collectionDate": "2024-01-15",
  "costObjectType": "BATCH",
  "costObjectId": 1,
  "costObjectNo": "B20240115",
  "batchNo": "B20240115",
  "colorNo": "C001",
  "dyeLotNo": "DL20240115",
  "workshop": "染色车间",
  "productionOrderNo": "PO20240115001",
  "directMaterial": "5000.00",
  "directLabor": "2000.00",
  "manufacturingOverhead": "1500.00",
  "processingFee": "1000.00",
  "dyeingFee": "800.00",
  "outputQuantityMeters": "1000.00",
  "outputQuantityKg": "500.00"
}
```

**成本构成**:
- `directMaterial`: 直接材料
- `directLabor`: 直接人工
- `manufacturingOverhead`: 制造费用
- `processingFee`: 加工费
- `dyeingFee`: 染色费

**自动计算**:
- `totalCost`: 总成本（自动求和）
- `unitCostMeters`: 米单位成本（总成本/产量米）
- `unitCostKg`: 公斤单位成本（总成本/产量公斤）

### 2.3 更新成本归集

**接口**: `PUT /api/v1/erp/cost/collections/:id`

**请求体** (所有字段可选):
```json
{
  "batchNo": "B20240116",
  "colorNo": "C002",
  "dyeLotNo": "DL20240116",
  "workshop": "新车间",
  "directMaterial": "6000.00",
  "directLabor": "2500.00",
  "manufacturingOverhead": "1800.00",
  "processingFee": "1200.00",
  "dyeingFee": "900.00",
  "outputQuantityMeters": "1200.00",
  "outputQuantityKg": "600.00"
}
```

**自动重新计算**: 总成本和单位成本

### 2.4 成本分配

**接口**: `POST /api/v1/erp/cost/collections/allocate`

**请求体**:
```json
{
  "sourceCollectionId": 1,
  "targetCollectionIds": [2, 3, 4],
  "allocationMethod": "BY_OUTPUT"
}
```

**分配方法**:
- `BY_OUTPUT`: 按产量比例分配

**业务规则**:
- 将源成本归集的成本按目标归集的产量比例分配
- 源归集状态变为 `allocated`
- 目标归集成本累加

### 2.5 成本分析

**接口**: `GET /api/v1/erp/cost/collections/analyze`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| batch_no | string | 否 | 批次号 |
| color_no | string | 否 | 色号 |
| start_date | string | 否 | 开始日期 |
| end_date | string | 否 | 结束日期 |

**响应示例**:
```json
{
  "success": true,
  "data": [
    {
      "batchNo": "B20240115",
      "colorNo": "C001",
      "totalCost": "10300.00",
      "directMaterial": "5000.00",
      "directLabor": "2000.00",
      "manufacturingOverhead": "1500.00",
      "processingFee": "1000.00",
      "dyeingFee": "800.00",
      "outputQuantityMeters": "1000.00",
      "outputQuantityKg": "500.00",
      "unitCostMeters": "10.30",
      "unitCostKg": "20.60",
      "costBreakdown": {
        "directMaterialRatio": "48.54",
        "directLaborRatio": "19.42",
        "manufacturingOverheadRatio": "14.56",
        "processingFeeRatio": "9.71",
        "dyeingFeeRatio": "7.77"
      }
    }
  ]
}
```

**成本构成比例**: 以百分比表示各项成本占比

### 2.6 获取成本构成比例

**接口**: `GET /api/v1/erp/cost/collections/:id/breakdown`

**路径参数**: `id` - 成本归集 ID

**响应**: 返回成本构成比例对象

---

## 三、库存调整管理 (Inventory Adjustment)

### 3.1 查询库存调整单列表

**接口**: `GET /api/v1/erp/inventory/adjustments`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| adjustment_no | string | 否 | 调整单号 |
| adjustment_type | string | 否 | 调整类型：increase, decrease |
| warehouse_id | int | 否 | 仓库 ID |
| page | int | 否 | 页码 |
| page_size | int | 否 | 每页数量 |

### 3.2 创建库存调整单

**接口**: `POST /api/v1/erp/inventory/adjustments`

**请求体**:
```json
{
  "adjustmentNo": "ADJ20240115001",
  "adjustmentType": "increase",
  "adjustmentDate": "2024-01-15",
  "warehouseId": 1,
  "reason": "盘点差异调整",
  "remarks": "备注信息",
  "items": [
    {
      "productId": 1,
      "batchNo": "B20240115",
      "quantity": "100.00",
      "unit": "米",
      "unitPrice": "10.00"
    }
  ]
}
```

**调整类型**:
- `increase`: 库存增加
- `decrease`: 库存减少

### 3.3 查询库存调整单详情

**接口**: `GET /api/v1/erp/inventory/adjustments/:id`

**路径参数**: `id` - 调整单 ID

### 3.4 审核并更新库存

**接口**: `POST /api/v1/erp/inventory/adjustments/:id/approve`

**路径参数**: `id` - 调整单 ID

**业务规则**:
- 只有状态为 `draft` 的调整单可以审核
- 审核时自动更新库存数量
- 减少操作会检查库存充足性
- 自动生成库存交易流水

**响应**:
```json
{
  "success": true,
  "message": "库存调整单已审核并更新库存"
}
```

### 3.5 仅审核（不更新库存）

**接口**: `POST /api/v1/erp/inventory/adjustments/:id/review`

**说明**: 仅改变审核状态，不触发库存更新

### 3.6 生成凭证

**接口**: `POST /api/v1/erp/inventory/adjustments/:id/generate-voucher`

**业务规则**:
- 自动生成财务凭证
- 凭证单号格式：`ADJ+ 时间戳`
- 凭证金额为调整单总金额

---

## 四、采购入库管理 (Purchase Receipt)

### 4.1 查询采购入库单列表

**接口**: `GET /api/v1/erp/purchases/receipts`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| page | int | 否 | 页码 |
| page_size | int | 否 | 每页数量 |
| status | string | 否 | 状态：PENDING, CONFIRMED, CANCELLED |
| supplier_id | int | 否 | 供应商 ID |
| order_id | int | 否 | 采购订单 ID |

### 4.2 创建采购入库单

**接口**: `POST /api/v1/erp/purchases/receipts`

**请求体**:
```json
{
  "receiptNo": "RCV20240115001",
  "receiptDate": "2024-01-15",
  "orderId": 1,
  "supplierId": 1,
  "warehouseId": 1,
  "receiptStatus": "PENDING",
  "items": [...]
}
```

### 4.3 确认采购入库单

**接口**: `POST /api/v1/erp/purchases/receipts/:id/confirm`

**业务规则**:
- 确认后自动更新库存
- 自动更新采购订单的已入库数量
- 采购订单状态联动更新（pending → partial → completed）
- 自动生成库存交易流水

### 4.4 批次追溯查询

**接口**: `GET /api/v1/erp/purchases/receipts/trace/batch`

**请求参数**:
| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| batch_no | string | 是 | 批次号 |
| color_code | string | 否 | 色号 |

**响应**: 返回包含该批次的所有入库单

**应用场景**:
- 质量追溯
- 批次跟踪
- 供应商评估

### 4.5 获取关联采购订单信息

**接口**: `GET /api/v1/erp/purchases/receipts/:id/order-info`

**路径参数**: `id` - 入库单 ID

**响应**: 返回关联的采购订单详情（如果有）

---

## 五、错误处理

### 常见错误码

| 错误类型 | HTTP 状态码 | 说明 |
|---------|------------|------|
| ValidationError | 400 | 参数验证失败 |
| ResourceNotFound | 404 | 资源不存在 |
| Unauthorized | 401 | 未授权 |
| Forbidden | 403 | 禁止访问 |
| InternalError | 500 | 服务器内部错误 |

### 错误响应格式

```json
{
  "success": false,
  "message": "错误描述信息",
  "error": "详细错误信息（可选）",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

---

## 六、业务规则总结

### 6.1 应收账款
- 单号自动生成：`AR + 日期 + 序号`
- 状态流转：`pending` → `approved` → `active` → `partially_paid` / `fully_paid`
- 支持多次核销，直到全部付清

### 6.2 成本归集
- 单号自动生成：`CC + 日期 + 序号`
- 自动计算总成本和单位成本
- 支持按产量比例分配成本

### 6.3 库存调整
- 支持增加/减少两种类型
- 审核时自动更新库存
- 自动生成库存交易流水
- 支持生成财务凭证

### 6.4 采购入库
- 确认后自动更新库存和采购订单
- 支持批次追溯
- 双计量单位（米/公斤）
- 面料行业特色字段：批次、色号、缸号

---

## 七、认证与授权

### JWT Token 使用

所有接口都需要在请求头中携带 JWT Token：

```
Authorization: Bearer <your_jwt_token>
```

### Token 获取

通过登录接口获取 Token（具体接口见认证模块文档）。

---

## 八、版本历史

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 2024-01-15 | 初始版本，包含第一阶段核心业务接口 |

---

## 联系支持

如有问题，请联系技术支持团队。
