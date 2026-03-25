# P2 级服务 API 文档

## 📋 概述

本文档描述面料 ERP 系统 P2 级服务的所有 REST API 接口。所有接口都遵循统一的规范和格式。

**基础路径**: `/api/v1/erp`  
**认证方式**: JWT Bearer Token  
**响应格式**: JSON

---

## 🔐 认证

所有接口都需要在请求头中携带 JWT Token：

```
Authorization: Bearer <your_jwt_token>
```

---

## 📊 通用响应格式

### 成功响应

```json
{
  "code": 200,
  "message": "success",
  "data": { ... }
}
```

### 错误响应

```json
{
  "code": 400,
  "message": "错误信息",
  "data": null
}
```

---

## 1. 采购价格服务 (Purchase Price Service)

### 1.1 查询采购价格列表

**接口**: `GET /api/v1/erp/purchases/prices`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 | 示例 |
|------|------|------|------|------|
| product_id | integer | 否 | 产品 ID | 1 |
| supplier_id | integer | 否 | 供应商 ID | 1 |
| price_type | string | 否 | 价格类型 | standard |
| status | string | 否 | 状态 | approved |
| page | integer | 否 | 页码（从 0 开始） | 0 |
| page_size | integer | 否 | 每页数量 | 10 |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": [
    {
      "id": 1,
      "product_id": 1,
      "supplier_id": 1,
      "price": "100.00",
      "currency": "CNY",
      "unit": "kg",
      "price_type": "standard",
      "status": "approved",
      "effective_date": "2026-03-16",
      "created_at": "2026-03-16T10:00:00Z"
    }
  ]
}
```

### 1.2 创建采购价格

**接口**: `POST /api/v1/erp/purchases/prices`

**请求体**:
```json
{
  "product_id": 1,
  "supplier_id": 1,
  "price": "100.00",
  "currency": "CNY",
  "unit": "kg",
  "min_order_qty": "100",
  "price_type": "standard",
  "effective_date": "2026-03-16",
  "expiry_date": "2027-03-16"
}
```

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "id": 1,
    "product_id": 1,
    "supplier_id": 1,
    "price": "100.00",
    "status": "pending"
  }
}
```

### 1.3 获取采购价格详情

**接口**: `GET /api/v1/erp/purchases/prices/:id`

**路径参数**:
- `id` - 价格记录 ID

### 1.4 更新采购价格

**接口**: `PUT /api/v1/erp/purchases/prices/:id`

**请求体**:
```json
{
  "price": "105.00",
  "expiry_date": "2027-12-31",
  "status": "approved"
}
```

### 1.5 删除采购价格

**接口**: `DELETE /api/v1/erp/purchases/prices/:id`

### 1.6 审批采购价格

**接口**: `POST /api/v1/erp/purchases/prices/:id/approve`

**请求体**:
```json
{
  "approved": true,
  "remark": "价格合理的"
}
```

### 1.7 获取价格历史

**接口**: `GET /api/v1/erp/purchases/prices/history/:product_id/:supplier_id`

**路径参数**:
- `product_id` - 产品 ID
- `supplier_id` - 供应商 ID

**查询参数**:
- `limit` - 返回记录数量限制（默认 20）

### 1.8 价格趋势分析

**接口**: `GET /api/v1/erp/purchases/prices/trend/:product_id/:supplier_id`

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "product_id": 1,
    "supplier_id": 1,
    "current_price": "100.00",
    "average_price": "98.50",
    "min_price": "95.00",
    "max_price": "105.00",
    "price_change_rate": "1.52",
    "trend_direction": "up",
    "history_count": 10
  }
}
```

---

## 2. 销售价格服务 (Sales Price Service)

### 2.1 查询销售价格列表

**接口**: `GET /api/v1/erp/sales/prices`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| product_id | integer | 否 | 产品 ID |
| customer_id | integer | 否 | 客户 ID |
| customer_type | string | 否 | 客户类型 | retail/wholesale/vip |
| price_level | string | 否 | 价格等级 |
| status | string | 否 | 状态 |

### 2.2 创建销售价格

**接口**: `POST /api/v1/erp/sales/prices`

**请求体**:
```json
{
  "product_id": 1,
  "customer_id": 1,
  "customer_type": "vip",
  "price": "150.00",
  "currency": "CNY",
  "unit": "kg",
  "price_type": "standard",
  "price_level": "A",
  "effective_date": "2026-03-16"
}
```

### 2.3 获取客户价格等级

**接口**: `GET /api/v1/erp/sales/prices/customer-level/:customer_type`

**路径参数**:
- `customer_type` - 客户类型（retail/wholesale/vip）

### 2.4 获取价格策略

**接口**: `GET /api/v1/erp/sales/prices/strategies`

**查询参数**:
- `customer_type` - 客户类型（可选）

---

## 3. 销售分析服务 (Sales Analysis Service)

### 3.1 获取销售统计列表

**接口**: `GET /api/v1/erp/sales/analysis/stats`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| statistic_type | string | 否 | 统计类型 |
| period | string | 否 | 期间 |
| dimension_type | string | 否 | 维度类型 |

### 3.2 获取销售趋势

**接口**: `GET /api/v1/erp/sales/analysis/trends`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| product_id | integer | 否 | 产品 ID |
| customer_id | integer | 否 | 客户 ID |
| period | string | 否 | 期间 |
| limit | integer | 否 | 返回数量限制 |

### 3.3 获取业绩排行

**接口**: `GET /api/v1/erp/sales/analysis/rankings`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| ranking_type | string | 是 | 排行类型 |
| period | string | 是 | 期间 |
| limit | integer | 否 | 返回数量限制 |

### 3.4 获取销售目标

**接口**: `GET /api/v1/erp/sales/analysis/targets`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| target_type | string | 否 | 目标类型 |
| period | string | 否 | 期间 |

### 3.5 创建销售目标

**接口**: `POST /api/v1/erp/sales/analysis/targets`

**请求体**:
```json
{
  "target_type": "revenue",
  "target_period": "2026-Q1",
  "department_id": 1,
  "target_amount": "1000000.00"
}
```

### 3.6 更新销售目标完成度

**接口**: `PUT /api/v1/erp/sales/analysis/targets/:id/achievement`

**路径参数**:
- `id` - 目标 ID

**请求体**:
```json
{
  "actual_amount": "850000.00"
}
```

---

## 4. 质量检验服务 (Quality Inspection Service)

### 4.1 查询检验标准列表

**接口**: `GET /api/v1/erp/quality/standards`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| inspection_type | string | 否 | 检验类型 |
| status | string | 否 | 状态 |
| product_id | integer | 否 | 产品 ID |
| supplier_id | integer | 否 | 供应商 ID |

### 4.2 创建检验标准

**接口**: `POST /api/v1/erp/quality/standards`

**请求体**:
```json
{
  "standard_name": "来料检验标准",
  "standard_code": "IQC-STD-001",
  "product_id": 1,
  "inspection_type": "IQC",
  "sampling_method": "AQL",
  "sampling_rate": "10.0",
  "acceptance_criteria": "AQL 2.5"
}
```

### 4.3 获取检验标准详情

**接口**: `GET /api/v1/erp/quality/standards/:id`

### 4.4 查询检验记录列表

**接口**: `GET /api/v1/erp/quality/records`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| inspection_type | string | 否 | 检验类型 |
| status | string | 否 | 状态 |
| product_id | integer | 否 | 产品 ID |
| supplier_id | integer | 否 | 供应商 ID |

### 4.5 创建检验记录

**接口**: `POST /api/v1/erp/quality/records`

**请求体**:
```json
{
  "inspection_no": "IQC20260316001",
  "inspection_type": "IQC",
  "product_id": 1,
  "batch_no": "BATCH001",
  "supplier_id": 1,
  "inspection_date": "2026-03-16",
  "total_qty": "1000",
  "inspected_qty": "500",
  "qualified_qty": "480",
  "unqualified_qty": "20",
  "inspection_result": "qualified"
}
```

### 4.6 获取检验记录详情

**接口**: `GET /api/v1/erp/quality/records/:id`

### 4.7 获取质量统计

**接口**: `GET /api/v1/erp/quality/statistics`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| period | string | 是 | 期间 |
| product_id | integer | 否 | 产品 ID |
| supplier_id | integer | 否 | 供应商 ID |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "period": "2026-03",
    "inspection_count": 50,
    "total_qty": "50000",
    "qualified_qty": "49000",
    "unqualified_qty": "1000",
    "qualification_rate": "98.00"
  }
}
```

---

## 5. 财务分析服务 (Financial Analysis Service)

### 5.1 获取财务指标列表

**接口**: `GET /api/v1/erp/finance/analysis/indicators`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| indicator_type | string | 否 | 指标类型 |
| status | string | 否 | 状态 |

### 5.2 创建财务指标

**接口**: `POST /api/v1/erp/finance/analysis/indicators`

**请求体**:
```json
{
  "indicator_name": "流动比率",
  "indicator_code": "CURRENT_RATIO",
  "indicator_type": "liquidity",
  "formula": "流动资产 / 流动负债",
  "unit": "%"
}
```

### 5.3 获取财务指标详情

**接口**: `GET /api/v1/erp/finance/analysis/indicators/:id`

### 5.4 创建财务分析结果

**接口**: `POST /api/v1/erp/finance/analysis/results`

**请求体**:
```json
{
  "analysis_type": "ratio_analysis",
  "period": "2026-03",
  "indicator_id": 1,
  "indicator_value": "15.5",
  "target_value": "15.0"
}
```

### 5.5 获取财务趋势

**接口**: `GET /api/v1/erp/finance/analysis/trends/:indicator_id/:limit`

**路径参数**:
- `indicator_id` - 指标 ID
- `limit` - 返回数量限制

### 5.6 财务比率分析

**接口**: `GET /api/v1/erp/finance/analysis/ratios`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| period | string | 是 | 期间 |
| company_id | integer | 是 | 公司 ID |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "period": "2026-03",
    "company_id": 1,
    "偿债能力": {
      "current_ratio": "2.00",
      "quick_ratio": "1.50",
      "cash_ratio": "0.80",
      "asset_liability_ratio": "45.00"
    },
    "盈利能力": {
      "gross_profit_margin": "30.00",
      "net_profit_margin": "15.00",
      "roa": "8.50",
      "roe": "12.00"
    }
  }
}
```

### 5.7 杜邦分析

**接口**: `GET /api/v1/erp/finance/analysis/dupont`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| period | string | 是 | 期间 |
| company_id | integer | 是 | 公司 ID |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "period": "2026-03",
    "company_id": 1,
    "roe": "15.00",
    "销售净利率": "15.00",
    "资产周转率": "0.50",
    "权益乘数": "2.00",
    "总资产": "2000000",
    "净资产": "1000000",
    "净利润": "150000",
    "销售收入": "1000000",
    "分解公式": "ROE = 销售净利率 × 资产周转率 × 权益乘数"
  }
}
```

---

## 6. 供应商评估服务 (Supplier Evaluation Service)

### 6.1 获取评估指标列表

**接口**: `GET /api/v1/erp/suppliers/eval/indicators`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| category | string | 否 | 类别 |
| status | string | 否 | 状态 |

### 6.2 创建评估指标

**接口**: `POST /api/v1/erp/suppliers/eval/indicators`

**请求体**:
```json
{
  "indicator_name": "质量水平",
  "indicator_code": "QUALITY",
  "category": "quality",
  "weight": "0.35",
  "max_score": 100,
  "evaluation_method": "score"
}
```

### 6.3 获取评估指标详情

**接口**: `GET /api/v1/erp/suppliers/eval/indicators/:id`

### 6.4 创建供应商评估记录

**接口**: `POST /api/v1/erp/suppliers/eval/evaluations`

**请求体**:
```json
{
  "supplier_id": 1,
  "evaluation_period": "2026-Q1",
  "indicator_id": 1,
  "score": "85.5",
  "remark": "表现良好"
}
```

### 6.5 计算综合评分

**接口**: `GET /api/v1/erp/suppliers/eval/scores`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| supplier_id | integer | 是 | 供应商 ID |
| evaluation_period | string | 是 | 评估期间 |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "supplier_id": 1,
    "evaluation_period": "2026-Q1",
    "total_score": "88.50",
    "quality_score": "90.00",
    "delivery_score": "85.00",
    "price_score": "88.00",
    "service_score": "92.00",
    "tech_score": "85.00",
    "grade": "A",
    "rank": 1
  }
}
```

### 6.6 供应商等级评定

**接口**: `GET /api/v1/erp/suppliers/eval/grade`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| supplier_id | integer | 是 | 供应商 ID |
| evaluation_period | string | 是 | 评估期间 |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "supplier_id": 1,
    "evaluation_period": "2026-Q1",
    "grade": "A"
  }
}
```

### 6.7 获取供应商排名

**接口**: `GET /api/v1/erp/suppliers/eval/rankings`

**请求参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| evaluation_period | string | 是 | 评估期间 |
| limit | integer | 否 | 返回数量限制 |

**响应示例**:
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "evaluation_period": "2026-Q1",
    "total": 10,
    "rankings": [
      {
        "rank": 1,
        "supplier_id": 1,
        "total_score": "88.50",
        "grade": "A"
      }
    ]
  }
}
```

---

## 📊 状态码说明

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未授权 |
| 403 | 禁止访问 |
| 404 | 资源不存在 |
| 500 | 服务器内部错误 |

---

## 🔑 认证说明

所有接口都需要在请求头中携带 JWT Token：

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

Token 过期时，接口将返回 401 错误。

---

## 📝 数据格式说明

### 日期格式
- 日期：`YYYY-MM-DD` (例如：`2026-03-16`)
- 日期时间：`YYYY-MM-DDTHH:MM:SSZ` (例如：`2026-03-16T10:00:00Z`)

### 数字格式
- 金额：字符串格式，保留两位小数 (例如：`"100.00"`)
- 百分比：字符串格式，保留两位小数 (例如：`"15.50"`)
- 数量：字符串格式，整数 (例如：`"1000"`)

### 枚举值

**检验类型**:
- `IQC` - 来料检验
- `IPQC` - 过程检验
- `OQC` - 出货检验

**客户类型**:
- `retail` - 零售
- `wholesale` - 批发
- `vip` - VIP 客户

**价格状态**:
- `pending` - 待审批
- `approved` - 已批准
- `rejected` - 已拒绝

---

**文档时间**: 2026-03-16  
**开发者**: AI Assistant  
**项目**: 面料 ERP 系统  
**版本**: v1.0
