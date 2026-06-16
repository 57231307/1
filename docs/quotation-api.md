# 报价单 API 文档

> **基础路径**：`/api/v1/erp/quotations`
> **认证方式**：Bearer Token（放在 `Authorization` header）
> **数据格式**：JSON
> **字符编码**：UTF-8
> **适用版本**：冰溪 ERP 2026.1+
> **最后更新**：2026-06-16

---

## 目录

- [通用说明](#通用说明)
- [端点列表](#端点列表)
- [数据模型](#数据模型)
- [错误码](#错误码)
- [示例代码](#示例代码)

---

## 通用说明

### 响应格式

所有接口统一返回 `ApiResponse<T>` 结构：

```json
{
  "code": 200,
  "message": "success",
  "data": { ... },
  "timestamp": "2026-06-16T10:30:00Z"
}
```

- `code = 200`：成功
- `code = 4xx`：业务错误（参数错误、未授权、状态冲突等）
- `code = 5xx`：系统错误
- `code = 401`：未授权，前端会自动跳转登录

### 分页参数

列表接口支持分页：

| 参数 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `page` | int | 1 | 页码（从 1 开始） |
| `page_size` | int | 20 | 每页条数（最大 100） |

### 状态值

`status` 字段（7 种状态）：

| 值 | 中文 | 可执行操作 |
|----|------|------------|
| `draft` | 草稿 | 编辑、删除、提交 |
| `pending_approval` | 待审批 | 批准、拒绝 |
| `approved` | 已批准 | 转订单、取消 |
| `rejected` | 已拒绝 | 编辑、重新提交 |
| `expired` | 已过期 | 仅查看 |
| `converted` | 已转订单 | 仅查看 |
| `cancelled` | 已取消 | 仅查看 |

---

## 端点列表

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/quotations` | 列表（分页 + 筛选） |
| POST | `/quotations` | 创建报价单（草稿） |
| GET | `/quotations/:id` | 详情 |
| PUT | `/quotations/:id` | 更新（仅 draft / rejected） |
| POST | `/quotations/:id/submit` | 提交审批 |
| POST | `/quotations/:id/approve` | 审批通过 |
| POST | `/quotations/:id/reject` | 审批拒绝 |
| POST | `/quotations/:id/cancel` | 取消 |
| POST | `/quotations/:id/convert` | 转销售订单 |
| GET | `/quotations/:id/terms` | 获取贸易条款 |
| PUT | `/quotations/:id/terms` | 设置贸易条款（覆盖式） |
| GET | `/quotations/expiring` | 即将过期（7 天内） |
| GET | `/quotations/expired` | 已过期 |
| POST | `/quotations/calculate-price` | 价格预计算（不保存） |
| GET | `/quotations/color-prices/:id` | 色号价格列表 |
| POST | `/quotations/color-prices/:id` | 设置色号价格 |

---

## 1. 列表

```
GET /api/v1/erp/quotations
```

### 查询参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `page` | int | 否 | 页码，默认 1 |
| `page_size` | int | 否 | 每页条数，默认 20 |
| `status` | string | 否 | 状态筛选（见状态值表） |
| `customer_id` | int | 否 | 客户筛选 |

### 响应

```json
{
  "code": 200,
  "message": "success",
  "data": [
    {
      "id": 1,
      "quotation_no": "QT202606160001",
      "customer_id": 100,
      "customer_name": "ABC 服装有限公司",
      "sales_user_id": 5,
      "sales_user_name": "张三",
      "quotation_date": "2026-06-16",
      "valid_until": "2026-07-16",
      "currency": "CNY",
      "exchange_rate": 1.0,
      "base_currency": "CNY",
      "price_terms": "FOB",
      "incoterm_location": "上海港",
      "tax_inclusive": true,
      "tax_rate": 13.0,
      "moq": 1000.0,
      "lead_time_days": 30,
      "customer_level": "VIP",
      "status": "approved",
      "subtotal": 50000.00,
      "tax_amount": 0.00,
      "total_amount": 50000.00,
      "approved_by": 5,
      "approved_by_name": "李四（销售经理）",
      "approved_at": "2026-06-16T10:30:00Z",
      "items": [...],
      "terms": [...],
      "created_at": "2026-06-16T09:00:00Z",
      "updated_at": "2026-06-16T10:30:00Z"
    }
  ],
  "timestamp": "2026-06-16T11:00:00Z"
}
```

---

## 2. 创建

```
POST /api/v1/erp/quotations
Content-Type: application/json
```

### 请求体

```json
{
  "customer_id": 100,
  "sales_user_id": 5,
  "quotation_date": "2026-06-16",
  "valid_until": "2026-07-16",
  "currency": "CNY",
  "exchange_rate": 1.0,
  "base_currency": "CNY",
  "price_terms": "FOB",
  "incoterms_version": "2020",
  "incoterm_location": "上海港",
  "tax_inclusive": true,
  "tax_rate": 13.0,
  "moq": 1000.0,
  "lead_time_days": 30,
  "customer_level": "VIP",
  "notes": "大客户优先",
  "items": [
    {
      "product_id": 200,
      "color_id": 50,
      "specification": "100% 棉, 200g/m²",
      "unit": "米",
      "quantity": 1000,
      "unit_price": 50.0,
      "unit_price_with_tax": 56.5,
      "discount_rate": 0.0,
      "notes": "主流产品"
    }
  ],
  "terms": [
    {
      "term_type": "logistics",
      "term_key": "shipping_method",
      "term_value": "海运至汉堡港，预计 30 天到达",
      "sequence": 1
    },
    {
      "term_type": "payment",
      "term_key": "payment_terms",
      "term_value": "30% 定金，余款发货后 30 天付清",
      "sequence": 2
    }
  ]
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `customer_id` | int | 是 | 客户 ID |
| `sales_user_id` | int | 是 | 销售员用户 ID |
| `quotation_date` | string | 是 | YYYY-MM-DD |
| `valid_until` | string | 是 | YYYY-MM-DD，必须 > 报价日期 |
| `currency` | string | 是 | CNY / USD / EUR |
| `exchange_rate` | number | 是 | > 0 |
| `base_currency` | string | 是 | 基础币种（用于汇率换算） |
| `price_terms` | string | 是 | FOB / CIF / EXW / DDP / DAP |
| `incoterms_version` | string | 否 | 默认 2020 |
| `incoterm_location` | string | 否 | 港口/地点 |
| `tax_inclusive` | bool | 是 | 是否含税 |
| `tax_rate` | number | 是 | 0-100 |
| `moq` | number | 否 | 最小起订量 |
| `lead_time_days` | int | 否 | 交期（天） |
| `customer_level` | string | 否 | VIP / NORMAL |
| `notes` | string | 否 | 备注 |
| `items` | array | 是 | 至少 1 个 |
| `terms` | array | 否 | 4 类贸易条款 |

### 响应

- `201 Created`：成功，返回完整报价单对象
- `400 Bad Request`：参数错误（如客户不存在、价格条款无效）
- `409 Conflict`：客户被禁用等业务冲突

---

## 3. 详情

```
GET /api/v1/erp/quotations/:id
```

### 响应

返回完整 `QuotationResponseDto`（包含 items 和 terms 数组）。

---

## 4. 更新

```
PUT /api/v1/erp/quotations/:id
Content-Type: application/json
```

仅 **草稿（draft）** 和 **已拒绝（rejected）** 状态可更新。

请求体结构同「创建」。

---

## 5. 提交审批

```
POST /api/v1/erp/quotations/:id/submit
```

### 业务逻辑

- 仅 draft / rejected 状态可提交
- 金额 < 10 万：销售员自批（直接 approved）
- 金额 10-50 万：进入销售经理审批
- 金额 > 50 万：进入总经理审批
- 即使 BPM 模板未配置，提交也能成功

### 响应

- `200 OK`：成功
- `400 Bad Request`：状态不允许
- `404 Not Found`：报价单不存在

---

## 6. 审批通过

```
POST /api/v1/erp/quotations/:id/approve
```

仅 **待审批（pending_approval）** 状态可调用。

---

## 7. 审批拒绝

```
POST /api/v1/erp/quotations/:id/reject
Content-Type: application/json
```

### 请求体

```json
{
  "reason": "价格超出市场行情，建议下调 5%"
}
```

`reason` 必填，至少 1 个字符。

---

## 8. 取消

```
POST /api/v1/erp/quotations/:id/cancel
```

仅 draft / pending_approval / rejected / approved 状态可取消。

---

## 9. 转销售订单

```
POST /api/v1/erp/quotations/:id/convert
```

### 前置条件

- 报价单状态 = `approved`
- 报价单未过期

### 业务逻辑

1. 创建销售订单（订单号格式 `SO + YYYYMMDD + 4 位序号`）
2. 复制所有报价明细
3. 复制客户、价格、币种、汇率
4. 备注自动添加 `[源自报价单 XXXX]` 前缀
5. 报价单状态变为 `converted`

### 响应

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "id": 500,
    "order_no": "SO202606160001",
    "status": "draft"
  },
  "timestamp": "2026-06-16T11:00:00Z"
}
```

---

## 10. 贸易条款

### 获取

```
GET /api/v1/erp/quotations/:id/terms
```

### 设置（覆盖式）

```
PUT /api/v1/erp/quotations/:id/terms
Content-Type: application/json
```

请求体：

```json
{
  "terms": [
    {
      "term_type": "logistics",
      "term_key": "",
      "term_value": "海运 30 天",
      "sequence": 1
    }
  ]
}
```

---

## 11. 即将过期 / 已过期

```
GET /api/v1/erp/quotations/expiring
GET /api/v1/erp/quotations/expired
```

返回对应的报价单列表（最多 100 条）。

---

## 12. 价格预计算

```
POST /api/v1/erp/quotations/calculate-price
Content-Type: application/json
```

### 请求体

```json
{
  "customer_id": 100,
  "customer_level": "VIP",
  "product_id": 200,
  "color_id": 50,
  "quantity": 1500,
  "currency": "CNY",
  "quotation_date": "2026-06-16"
}
```

### 响应

```json
{
  "code": 200,
  "data": {
    "unit_price": 47.5,
    "unit_price_with_tax": 53.675,
    "tier_breakdown": [
      { "min_quantity": 1000, "max_quantity": 2000, "unit_price": 50.0 }
    ],
    "discount_applied": 2.5,
    "final_amount": 71250.0,
    "price_source": "color_price"
  }
}
```

`price_source` 可能值：
- `color_price`：色号价格表
- `product_price`：产品基础价
- `promotion`：促销

---

## 13. 色号价格

### 获取

```
GET /api/v1/erp/quotations/color-prices/:product_color_id
```

### 设置

```
POST /api/v1/erp/quotations/color-prices/:product_color_id
Content-Type: application/json
```

```json
{
  "base_price": 50.0,
  "min_quantity": 1,
  "effective_from": "2026-06-01",
  "effective_to": "2026-12-31",
  "customer_level": "VIP"
}
```

---

## 数据模型

### QuotationResponseDto

```typescript
interface QuotationResponseDto {
  id: number
  quotation_no: string
  customer_id: number
  customer_name?: string
  sales_user_id: number
  sales_user_name?: string
  quotation_date: string          // YYYY-MM-DD
  valid_until: string             // YYYY-MM-DD
  currency: string                // CNY / USD / EUR
  exchange_rate: number
  base_currency: string
  price_terms: string             // FOB / CIF / EXW / DDP / DAP
  incoterms_version?: string
  incoterm_location?: string
  tax_inclusive: boolean
  tax_rate: number
  moq?: number
  lead_time_days?: number
  customer_level?: string         // VIP / NORMAL
  status: QuotationStatus
  subtotal: number
  tax_amount: number
  total_amount: number
  approved_by?: number
  approved_by_name?: string
  approved_at?: string            // ISO 8601
  rejection_reason?: string
  converted_sales_order_id?: number
  converted_at?: string           // ISO 8601
  notes?: string
  items: QuotationItemResponseDto[]
  terms: QuotationTermResponseDto[]
  created_at: string              // ISO 8601
  updated_at: string              // ISO 8601
}
```

### QuotationItemResponseDto

```typescript
interface QuotationItemResponseDto {
  id: number
  product_id: number
  product_name?: string
  product_code?: string
  color_id?: number
  color_code?: string
  pantone_code?: string
  cncs_code?: string
  specification?: string
  unit: string
  quantity: number
  unit_price: number
  unit_price_with_tax: number
  amount: number
  amount_with_tax: number
  tier_pricing?: any              // JSONB
  discount_rate?: number
  discount_amount?: number
  notes?: string
  sequence: number
}
```

### QuotationTermResponseDto

```typescript
interface QuotationTermResponseDto {
  id: number
  term_type: 'logistics' | 'payment' | 'sample' | 'inspection'
  term_key: string
  term_value: string
  sequence: number
}
```

---

## 错误码

| HTTP | code | 说明 |
|------|------|------|
| 200 | 200 | 成功 |
| 201 | 200 | 创建成功 |
| 400 | 400 | 参数错误 |
| 401 | 401 | 未授权 |
| 403 | 403 | 无权限 |
| 404 | 404 | 报价单不存在 |
| 409 | 409 | 状态冲突（如已审批后再更新） |
| 500 | 500 | 服务器内部错误 |
| 502 | 502 | 网关错误 |
| 503 | 503 | 服务暂时不可用 |

### 错误响应示例

```json
{
  "code": 409,
  "message": "报价单状态不允许更新：approved",
  "data": null,
  "timestamp": "2026-06-16T11:00:00Z"
}
```

---

## 示例代码

### JavaScript / TypeScript（fetch）

```typescript
// 列表
async function listQuotations(params: { status?: string; page?: number }) {
  const qs = new URLSearchParams(params as any).toString()
  const res = await fetch(`/api/v1/erp/quotations?${qs}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  const json = await res.json()
  if (json.code !== 200) throw new Error(json.message)
  return json.data
}

// 创建
async function createQuotation(data: CreateQuotationDto) {
  const res = await fetch('/api/v1/erp/quotations', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(data),
  })
  const json = await res.json()
  if (json.code !== 200) throw new Error(json.message)
  return json.data
}

// 提交审批
async function submitQuotation(id: number) {
  const res = await fetch(`/api/v1/erp/quotations/${id}/submit`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}` },
  })
  const json = await res.json()
  if (json.code !== 200) throw new Error(json.message)
  return json.data
}
```

### cURL

```bash
# 列表
curl -X GET 'https://erp.example.com/api/v1/erp/quotations?status=approved&page=1' \
  -H 'Authorization: Bearer YOUR_TOKEN'

# 创建
curl -X POST 'https://erp.example.com/api/v1/erp/quotations' \
  -H 'Authorization: Bearer YOUR_TOKEN' \
  -H 'Content-Type: application/json' \
  -d '{
    "customer_id": 100,
    "sales_user_id": 5,
    "quotation_date": "2026-06-16",
    "valid_until": "2026-07-16",
    "currency": "CNY",
    "exchange_rate": 1.0,
    "base_currency": "CNY",
    "price_terms": "FOB",
    "tax_inclusive": true,
    "tax_rate": 13.0,
    "items": [
      {
        "product_id": 200,
        "unit": "米",
        "quantity": 1000,
        "unit_price": 50.0,
        "unit_price_with_tax": 56.5
      }
    ]
  }'

# 提交审批
curl -X POST 'https://erp.example.com/api/v1/erp/quotations/1/submit' \
  -H 'Authorization: Bearer YOUR_TOKEN'
```

### Python（requests）

```python
import requests

BASE_URL = 'https://erp.example.com/api/v1/erp'
HEADERS = {
    'Authorization': 'Bearer YOUR_TOKEN',
    'Content-Type': 'application/json',
}

# 列表
resp = requests.get(f'{BASE_URL}/quotations', headers=HEADERS, params={'status': 'approved'})
resp.raise_for_status()
data = resp.json()['data']

# 创建
payload = {
    'customer_id': 100,
    'sales_user_id': 5,
    'quotation_date': '2026-06-16',
    'valid_until': '2026-07-16',
    'currency': 'CNY',
    'exchange_rate': 1.0,
    'base_currency': 'CNY',
    'price_terms': 'FOB',
    'tax_inclusive': True,
    'tax_rate': 13.0,
    'items': [
        {
            'product_id': 200,
            'unit': '米',
            'quantity': 1000,
            'unit_price': 50.0,
            'unit_price_with_tax': 56.5,
        }
    ],
}
resp = requests.post(f'{BASE_URL}/quotations', headers=HEADERS, json=payload)
resp.raise_for_status()
quotation = resp.json()['data']

# 提交审批
resp = requests.post(f'{BASE_URL}/quotations/{quotation["id"]}/submit', headers=HEADERS)
resp.raise_for_status()
```

---

## 附录 A：变更日志

| 版本 | 日期 | 变更 |
|------|------|------|
| 2026.1.0 | 2026-06-16 | 初版发布 |

## 附录 B：相关文档

- 用户手册：[`docs/quotation-user-manual.md`](./quotation-user-manual.md)
- 设计规格：[`docs/superpowers/specs/2026-06-16-sales-quotation-design.md`](./superpowers/specs/2026-06-16-sales-quotation-design.md)
- 实施计划：[`docs/superpowers/plans/2026-06-16-sales-quotation-plan.md`](./superpowers/plans/2026-06-16-sales-quotation-plan.md)
