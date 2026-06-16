# 面料多色号定价扩展 API 文档

> 冰溪 ERP P0-5 行业功能 - 16 个 REST API 端点
> 基础路径：`/api/v1/erp/color-prices`
> 更新日期: 2026-06-18

---

## 1. 通用说明

- **认证方式**：Bearer Token（Authorization header）
- **租户隔离**：所有端点强制 `tenant_id` 隔离
- **响应格式**：`{ code, message, data }` 统一格式
- **错误码**：
  - `400` 参数错误
  - `401` 未认证
  - `403` 无权限
  - `404` 资源不存在
  - `409` 业务冲突
  - `500` 服务器错误

---

## 2. 色号价格 CRUD

### 2.1 GET /api/v1/erp/color-prices - 列表

**Query 参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| page | int | 页码（默认 1） |
| page_size | int | 每页条数（默认 20） |
| product_id | int | 产品 ID |
| color_id | int | 色号 ID |
| customer_id | int | 客户 ID |
| customer_level | string | 客户等级（VIP/NORMAL/GOLD/SILVER） |
| season | string | 季节（SS/AW/HOLIDAY） |
| currency | string | 币种（CNY/USD/EUR） |
| is_active | bool | 是否启用 |
| approval_status | string | 审批状态（PENDING/APPROVED/REJECTED） |
| keyword | string | 关键字 |

**响应**：

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "product_id": 100,
        "color_id": 200,
        "currency": "CNY",
        "base_price": "50.000000",
        "customer_level": "VIP",
        "season": "SS",
        "is_active": true,
        "priority": 0,
        "approval_status": "APPROVED",
        ...
      }
    ],
    "total": 100,
    "page": 1,
    "page_size": 20
  }
}
```

### 2.2 POST /api/v1/erp/color-prices - 创建

**请求体**：

```json
{
  "product_id": 100,
  "color_id": 200,
  "currency": "CNY",
  "base_price": 50.00,
  "effective_from": "2026-01-01",
  "effective_to": null,
  "customer_level": "VIP",
  "min_quantity": 1,
  "max_quantity": 1000,
  "customer_id": null,
  "season": "SS",
  "priority": 0,
  "notes": "VIP 春夏客户价"
}
```

**响应**：`ColorPriceDetail`

### 2.3 GET /api/v1/erp/color-prices/:id - 详情

**响应**：

```json
{
  "id": 1,
  "product_id": 100,
  "color_id": 200,
  "currency": "CNY",
  "base_price": "50.000000",
  "effective_from": "2026-01-01",
  "effective_to": null,
  "customer_level": "VIP",
  "min_quantity": "1.00",
  "max_quantity": "1000.00",
  "customer_id": null,
  "season": "SS",
  "is_active": true,
  "priority": 0,
  "notes": "...",
  "created_by": 1,
  "approved_by": null,
  "approved_at": null,
  "approval_status": "APPROVED",
  "tenant_id": 1,
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

### 2.4 PUT /api/v1/erp/color-prices/:id - 更新

**请求体**：同创建，所有字段可选

### 2.5 DELETE /api/v1/erp/color-prices/:id - 软删除

**响应**：`ColorPriceDetail`（`is_active = false`）

---

## 3. 批量调价

### 3.1 POST /api/v1/erp/color-prices/batch-adjust

**请求体**：

```json
{
  "items": [
    {
      "price_id": 1,
      "adjustment_type": "percentage",
      "adjustment_value": 0.05
    },
    {
      "price_id": 2,
      "adjustment_type": "fixed",
      "adjustment_value": 1.5
    }
  ],
  "change_reason": "原材料涨价 5%"
}
```

**响应**：

```json
{
  "code": 0,
  "data": {
    "auto_approved": [2],
    "pending_approval": [1],
    "total": 2
  }
}
```

**调价规则**：
- 涨跌幅 ≤ 10% → `auto_approved`
- 涨跌幅 > 10% → `pending_approval`

### 3.2 POST /api/v1/erp/color-prices/:id/approve

**请求体**：

```json
{
  "decision": "APPROVED",
  "comments": "成本已核实，同意调价"
}
```

**响应**：`ColorPriceDetail`（`approval_status` 更新为 APPROVED/REJECTED）

---

## 4. 价格历史

### 4.1 GET /api/v1/erp/color-prices/:id/history

**响应**：

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "product_color_price_id": 1,
        "old_price": "50.000000",
        "new_price": "52.500000",
        "currency": "CNY",
        "change_type": "batch",
        "change_reason": "原材料涨价 5%",
        "change_percent": "0.0500",
        "quantity": null,
        "operated_by": 1,
        "operated_at": "2026-01-15T10:00:00Z",
        "approved_by": 2,
        "approved_at": "2026-01-15T11:00:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "page_size": 100
  }
}
```

---

## 5. 价格计算

### 5.1 GET /api/v1/erp/color-prices/calculate

**Query 参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| product_id | int | 是 | 产品 ID |
| color_id | int | 是 | 色号 ID |
| customer_id | int | 否 | 客户 ID |
| customer_level | string | 否 | 客户等级 |
| quantity | decimal | 是 | 数量（米） |
| season | string | 否 | 季节（SS/AW/HOLIDAY） |
| product_category_id | int | 否 | 产品品类 ID |
| currency | string | 否 | 币种（默认 CNY） |
| calc_date | date | 否 | 计算日期（默认今天） |

**响应**：

```json
{
  "code": 0,
  "data": {
    "base_price": "100.000000",
    "tier_price": "90.000000",
    "level_price": "85.500000",
    "season_price": "94.050000",
    "special_price": null,
    "final_price": "94.050000",
    "currency": "CNY",
    "applied_rule": "seasonal",
    "breakdown": [
      {
        "step": "基础价",
        "before": "100.000000",
        "after": "100.000000",
        "rule": "基础价 100.00 CNY"
      },
      {
        "step": "阶梯价",
        "before": "100.000000",
        "after": "90.000000",
        "rule": "数量 1000 命中阶梯价 90.00 CNY"
      },
      {
        "step": "客户等级",
        "before": "90.000000",
        "after": "85.500000",
        "rule": "VIP 等级 0.950 折"
      },
      {
        "step": "季节调价",
        "before": "85.500000",
        "after": "94.050000",
        "rule": "SS 规则 percentage 0.1000"
      }
    ]
  }
}
```

---

## 6. 阶梯价

### 6.1 GET /api/v1/erp/color-prices/tiers/:price_id - 列表

**响应**：

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "product_color_price_id": 1,
        "min_quantity": "1.00",
        "max_quantity": "99.00",
        "tier_price": "100.00",
        "customer_level": null,
        "sequence": 0
      },
      {
        "id": 2,
        "product_color_price_id": 1,
        "min_quantity": "100.00",
        "max_quantity": "499.00",
        "tier_price": "95.00",
        "customer_level": null,
        "sequence": 1
      }
    ],
    "total": 4
  }
}
```

### 6.2 POST /api/v1/erp/color-prices/tiers - 新建

**请求体**：

```json
{
  "product_color_price_id": 1,
  "min_quantity": 1000,
  "max_quantity": null,
  "tier_price": 85.00,
  "customer_level": null,
  "sequence": 3
}
```

### 6.3 DELETE /api/v1/erp/color-prices/tiers/item/:tier_id - 删除

**响应**：

```json
{ "code": 0, "data": { "deleted": 1 } }
```

---

## 7. 客户专属价

### 7.1 GET /api/v1/erp/color-prices/customer-special

**响应**：

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "customer_id": 100,
        "product_id": 100,
        "color_id": 200,
        "special_price": "80.00",
        "discount_percent": "0.85",
        "currency": "CNY",
        "valid_from": "2026-01-01",
        "valid_until": "2026-12-31"
      }
    ],
    "total": 1
  }
}
```

### 7.2 POST /api/v1/erp/color-prices/customer-special

**请求体**：

```json
{
  "customer_id": 100,
  "product_id": 100,
  "color_id": 200,
  "special_price": 80.00,
  "discount_percent": 0.85,
  "currency": "CNY",
  "valid_from": "2026-01-01",
  "valid_until": "2026-12-31",
  "notes": "战略客户 A 协议价"
}
```

---

## 8. 季节调价规则

### 8.1 GET /api/v1/erp/color-prices/seasonal-rules

**Query 参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| page | int | 页码 |
| page_size | int | 每页条数 |
| season | string | SS / AW / HOLIDAY |
| is_active | bool | 是否启用 |
| product_category_id | int | 品类 ID |

**响应**：

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "rule_name": "春夏新品 +10%",
        "season": "SS",
        "product_category_id": 1,
        "adjustment_type": "percentage",
        "adjustment_value": "0.10",
        "valid_from": "2026-03-01",
        "valid_until": "2026-08-31",
        "is_active": true
      }
    ],
    "total": 1,
    "page": 1,
    "page_size": 20
  }
}
```

### 8.2 POST /api/v1/erp/color-prices/seasonal-rules

**请求体**：

```json
{
  "rule_name": "2026 春夏新品 +10%",
  "season": "SS",
  "product_category_id": 1,
  "adjustment_type": "percentage",
  "adjustment_value": 0.10,
  "valid_from": "2026-03-01",
  "valid_until": "2026-08-31",
  "description": "新品上市季节性提价"
}
```

### 8.3 DELETE /api/v1/erp/color-prices/seasonal-rules/:id

**响应**：`{ "deleted": 1 }`（软删除，is_active = false）

---

## 9. 错误响应

```json
{
  "code": 400,
  "message": "参数校验失败: 无效的币种: ABC（允许: CNY / USD / EUR）",
  "data": null
}
```

---

## 10. 调用示例（cURL）

```bash
# 1. 列表
curl -H "Authorization: Bearer <token>" \
  'http://localhost:8080/api/v1/erp/color-prices?page=1&page_size=20&is_active=true'

# 2. 创建
curl -X POST -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"product_id":100,"color_id":200,"currency":"CNY","base_price":50.00,"effective_from":"2026-01-01"}' \
  http://localhost:8080/api/v1/erp/color-prices

# 3. 批量调价
curl -X POST -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"items":[{"price_id":1,"adjustment_type":"percentage","adjustment_value":0.05}],"change_reason":"原材料涨价"}' \
  http://localhost:8080/api/v1/erp/color-prices/batch-adjust

# 4. 价格计算
curl -H "Authorization: Bearer <token>" \
  'http://localhost:8080/api/v1/erp/color-prices/calculate?product_id=100&color_id=200&customer_level=VIP&quantity=1000&season=SS'
```
