# 管理服务 API 文档

## 概述

本文档描述了四个管理服务的 HTTP API 接口，包括采购合同、销售合同、固定资产和预算管理。

**基础信息:**
- API 版本：v1
- 基础路径：`/api/v1/erp/`
- 认证方式：JWT Bearer Token
- 响应格式：JSON

**响应格式:**
```json
{
  "success": true,
  "message": "操作成功",
  "data": {},
  "total": 0
}
```

---

## 采购合同管理 (Purchase Contracts)

### 1. 获取采购合同列表

**端点:** `GET /api/v1/erp/purchase-contracts`

**认证:** 必需

**查询参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| keyword | string | 否 | 搜索关键字 (合同编号/合同名称) |
| status | string | 否 | 合同状态 (draft/approved/executing/completed/cancelled) |
| supplier_id | integer | 否 | 供应商 ID |
| page | integer | 否 | 页码，默认 1 |
| page_size | integer | 否 | 每页数量，默认 10 |

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": [
    {
      "id": 1,
      "contract_no": "PC20260316001",
      "contract_name": "面料采购合同",
      "contract_type": "purchase",
      "supplier_id": 1,
      "supplier_name": "供应商 A",
      "total_amount": "10000.00",
      "status": "approved",
      "created_at": "2026-03-16T10:00:00Z"
    }
  ],
  "total": 1
}
```

---

### 2. 获取单个采购合同

**端点:** `GET /api/v1/erp/purchase-contracts/{id}`

**认证:** 必需

**路径参数:**
| 参数 | 类型 | 说明 |
|------|------|------|
| id | integer | 合同 ID |

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "id": 1,
    "contract_no": "PC20260316001",
    "contract_name": "面料采购合同",
    "contract_type": "purchase",
    "supplier_id": 1,
    "supplier_name": "供应商 A",
    "total_amount": "10000.00",
    "payment_terms": "30 天账期",
    "delivery_date": "2026-04-01",
    "status": "approved",
    "signed_date": "2026-03-16",
    "effective_date": "2026-03-16",
    "created_by": 1,
    "created_at": "2026-03-16T10:00:00Z",
    "updated_at": "2026-03-16T10:00:00Z"
  }
}
```

---

### 3. 创建采购合同

**端点:** `POST /api/v1/erp/purchase-contracts`

**认证:** 必需

**请求体:**
```json
{
  "contract_no": "PC20260316001",
  "contract_name": "面料采购合同",
  "supplier_id": 1,
  "total_amount": "10000.00",
  "payment_terms": "30 天账期",
  "delivery_date": "2026-04-01",
  "remark": "备注信息"
}
```

**字段说明:**
| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| contract_no | string | 是 | 合同编号 (唯一) |
| contract_name | string | 是 | 合同名称 |
| supplier_id | integer | 是 | 供应商 ID |
| total_amount | decimal | 是 | 合同总金额 |
| payment_terms | string | 否 | 付款条件 |
| delivery_date | date | 是 | 交货日期 (YYYY-MM-DD) |
| remark | string | 否 | 备注 |

**响应示例:**
```json
{
  "success": true,
  "message": "创建成功",
  "data": {
    "id": 1,
    "contract_no": "PC20260316001",
    "status": "draft"
  }
}
```

---

### 4. 审核采购合同

**端点:** `POST /api/v1/erp/purchase-contracts/{id}/approve`

**认证:** 必需

**路径参数:**
| 参数 | 类型 | 说明 |
|------|------|------|
| id | integer | 合同 ID |

**响应示例:**
```json
{
  "success": true,
  "message": "审核成功"
}
```

---

### 5. 执行采购合同

**端点:** `POST /api/v1/erp/purchase-contracts/{id}/execute`

**认证:** 必需

**请求体:**
```json
{
  "execution_type": "purchase_order",
  "execution_amount": "5000.00",
  "related_bill_type": "purchase_order",
  "related_bill_id": 1,
  "remark": "执行备注"
}
```

**响应示例:**
```json
{
  "success": true,
  "message": "执行成功"
}
```

---

### 6. 取消采购合同

**端点:** `POST /api/v1/erp/purchase-contracts/{id}/cancel`

**认证:** 必需

**请求体:**
```json
{
  "reason": "取消原因"
}
```

**响应示例:**
```json
{
  "success": true,
  "message": "取消成功"
}
```

---

### 7. 删除采购合同

**端点:** `DELETE /api/v1/erp/purchase-contracts/{id}`

**认证:** 必需

**说明:** 仅草稿状态的合同可以删除

**响应示例:**
```json
{
  "success": true,
  "message": "删除成功"
}
```

---

## 销售合同管理 (Sales Contracts)

### 1. 获取销售合同列表

**端点:** `GET /api/v1/erp/sales-contracts`

**认证:** 必需

**查询参数:** 同采购合同

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": [
    {
      "id": 1,
      "contract_no": "SC20260316001",
      "contract_name": "销售合同",
      "customer_id": 1,
      "customer_name": "客户 A",
      "total_amount": "15000.00",
      "status": "approved"
    }
  ],
  "total": 1
}
```

---

### 2. 获取单个销售合同

**端点:** `GET /api/v1/erp/sales-contracts/{id}`

**认证:** 必需

**响应:** 同采购合同结构

---

### 3. 创建销售合同

**端点:** `POST /api/v1/erp/sales-contracts`

**认证:** 必需

**请求体:**
```json
{
  "contract_no": "SC20260316001",
  "contract_name": "销售合同",
  "customer_id": 1,
  "total_amount": "15000.00",
  "payment_terms": "预付款 30%",
  "delivery_date": "2026-04-01",
  "remark": "备注"
}
```

---

### 4. 审核销售合同

**端点:** `POST /api/v1/erp/sales-contracts/{id}/approve`

**认证:** 必需

---

### 5. 执行销售合同

**端点:** `POST /api/v1/erp/sales-contracts/{id}/execute`

**认证:** 必需

---

### 6. 取消销售合同

**端点:** `POST /api/v1/erp/sales-contracts/{id}/cancel`

**认证:** 必需

---

### 7. 删除销售合同

**端点:** `DELETE /api/v1/erp/sales-contracts/{id}`

**认证:** 必需

---

## 固定资产管理 (Fixed Assets)

### 1. 获取固定资产列表

**端点:** `GET /api/v1/erp/fixed-assets`

**认证:** 必需

**查询参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| keyword | string | 否 | 搜索关键字 (资产编号/资产名称) |
| status | string | 否 | 资产状态 (active/disposed/scrapped) |
| asset_category | string | 否 | 资产类别 |
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": [
    {
      "id": 1,
      "asset_no": "FA20260316001",
      "asset_name": "生产设备",
      "asset_category": "equipment",
      "original_value": "50000.00",
      "accumulated_depreciation": "5000.00",
      "net_value": "45000.00",
      "status": "active",
      "purchase_date": "2026-03-16"
    }
  ],
  "total": 1
}
```

---

### 2. 获取单个固定资产

**端点:** `GET /api/v1/erp/fixed-assets/{id}`

**认证:** 必需

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "id": 1,
    "asset_no": "FA20260316001",
    "asset_name": "生产设备",
    "asset_category": "equipment",
    "specification": "规格型号",
    "original_value": "50000.00",
    "salvage_value": "5000.00",
    "salvage_rate": "0.10",
    "depreciable_value": "45000.00",
    "depreciation_method": "straight_line",
    "useful_life": 60,
    "monthly_depreciation": "750.00",
    "accumulated_depreciation": "5000.00",
    "net_value": "45000.00",
    "status": "active",
    "purchase_date": "2026-03-16",
    "in_service_date": "2026-03-16",
    "supplier_id": 1,
    "supplier_name": "供应商 A"
  }
}
```

---

### 3. 创建固定资产

**端点:** `POST /api/v1/erp/fixed-assets`

**认证:** 必需

**请求体:**
```json
{
  "asset_no": "FA20260316001",
  "asset_name": "生产设备",
  "asset_category": "equipment",
  "specification": "规格型号",
  "location": "车间 A",
  "original_value": "50000.00",
  "useful_life": 60,
  "depreciation_method": "straight_line",
  "purchase_date": "2026-03-16",
  "put_in_date": "2026-03-16",
  "supplier_id": 1,
  "remark": "备注"
}
```

**字段说明:**
| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| asset_no | string | 是 | 资产编号 (唯一) |
| asset_name | string | 是 | 资产名称 |
| asset_category | string | 是 | 资产类别 |
| specification | string | 否 | 规格型号 |
| location | string | 否 | 存放地点 |
| original_value | decimal | 是 | 原值 |
| useful_life | integer | 是 | 使用年限 (月) |
| depreciation_method | string | 是 | 折旧方法 |
| purchase_date | date | 是 | 购买日期 |
| put_in_date | date | 是 | 投入使用日期 |
| supplier_id | integer | 否 | 供应商 ID |
| remark | string | 否 | 备注 |

**响应示例:**
```json
{
  "success": true,
  "message": "创建成功",
  "data": {
    "id": 1,
    "asset_no": "FA20260316001",
    "status": "active"
  }
}
```

---

### 4. 计提折旧

**端点:** `POST /api/v1/erp/fixed-assets/{id}/depreciate`

**认证:** 必需

**响应示例:**
```json
{
  "success": true,
  "message": "折旧成功"
}
```

---

### 5. 处置资产

**端点:** `POST /api/v1/erp/fixed-assets/{id}/dispose`

**认证:** 必需

**请求体:**
```json
{
  "disposal_type": "sale",
  "disposal_value": "10000.00",
  "disposal_date": "2026-12-31",
  "reason": "设备更新",
  "buyer_info": "买家信息"
}
```

**响应示例:**
```json
{
  "success": true,
  "message": "处置成功"
}
```

---

### 6. 删除固定资产

**端点:** `DELETE /api/v1/erp/fixed-assets/{id}`

**认证:** 必需

**说明:** 仅草稿状态的资产可以删除

---

## 预算管理 (Budget Management)

### 1. 获取预算科目列表

**端点:** `GET /api/v1/erp/budget-items`

**认证:** 必需

**查询参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| item_type | string | 否 | 科目类型 (income/expense) |
| status | string | 否 | 状态 (active/draft/archived) |
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |

**响应示例:**
```json
{
  "success": true,
  "message": "获取成功",
  "data": [
    {
      "id": 1,
      "item_code": "BI001",
      "item_name": "原材料采购",
      "item_type": "expense",
      "level": 1,
      "status": "active"
    }
  ],
  "total": 1
}
```

---

### 2. 获取单个预算科目

**端点:** `GET /api/v1/erp/budget-items/{id}`

**认证:** 必需

---

### 3. 创建预算科目

**端点:** `POST /api/v1/erp/budget-items`

**认证:** 必需

**请求体:**
```json
{
  "item_code": "BI001",
  "item_name": "原材料采购",
  "item_type": "expense",
  "parent_id": null,
  "level": 1
}
```

**字段说明:**
| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| item_code | string | 是 | 科目编码 (唯一) |
| item_name | string | 是 | 科目名称 |
| item_type | string | 是 | 科目类型 (income/expense) |
| parent_id | integer | 否 | 父级科目 ID |
| level | integer | 是 | 科目级别 |

**响应示例:**
```json
{
  "success": true,
  "message": "创建成功",
  "data": {
    "id": 1,
    "item_code": "BI001"
  }
}
```

---

### 4. 更新预算科目

**端点:** `PUT /api/v1/erp/budget-items/{id}`

**认证:** 必需

**请求体:**
```json
{
  "item_name": "新名称",
  "item_type": "expense",
  "status": "active"
}
```

---

### 5. 删除预算科目

**端点:** `DELETE /api/v1/erp/budget-items/{id}`

**认证:** 必需

---

### 6. 获取预算方案列表

**端点:** `GET /api/v1/erp/budget-plans`

**认证:** 必需

**查询参数:**
| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| budget_year | integer | 否 | 预算年度 |
| status | string | 否 | 状态 |
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |

---

### 7. 创建预算方案

**端点:** `POST /api/v1/erp/budget-plans`

**认证:** 必需

**请求体:**
```json
{
  "plan_no": "BP2026001",
  "plan_name": "2026 年度预算",
  "budget_year": 2026,
  "department_id": 1,
  "total_amount": "1000000.00",
  "start_date": "2026-01-01",
  "end_date": "2026-12-31",
  "remark": "年度预算方案"
}
```

---

### 8. 审核预算方案

**端点:** `POST /api/v1/erp/budget-plans/{id}/approve`

**认证:** 必需

**请求体:**
```json
{
  "approval_comment": "审核意见"
}
```

---

### 9. 执行预算方案

**端点:** `POST /api/v1/erp/budget-plans/{id}/execute`

**认证:** 必需

**请求体:**
```json
{
  "actual_amount": "50000.00",
  "expense_type": "原材料",
  "expense_date": "2026-03-16",
  "remark": "执行备注"
}
```

---

## 错误响应

### 通用错误格式

```json
{
  "success": false,
  "message": "错误描述",
  "error_code": "ERROR_CODE"
}
```

### 常见错误码

| 错误码 | HTTP 状态码 | 说明 |
|--------|------------|------|
| UNAUTHORIZED | 401 | 未授权访问 |
| FORBIDDEN | 403 | 权限不足 |
| NOT_FOUND | 404 | 资源不存在 |
| BAD_REQUEST | 400 | 请求参数错误 |
| CONFLICT | 409 | 资源冲突 (如编号重复) |
| INTERNAL_ERROR | 500 | 服务器内部错误 |

---

## 认证说明

所有 API 端点都需要 JWT Token 认证。

**请求头:**
```
Authorization: Bearer <your_jwt_token>
```

**获取 Token:**
通过登录接口获取 JWT Token。

---

## 分页说明

所有列表接口都支持分页。

**请求参数:**
- `page`: 页码，从 1 开始，默认 1
- `page_size`: 每页数量，默认 10，最大 100

**响应包含:**
- `data`: 数据列表
- `total`: 总记录数

---

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-03-16 | 初始版本，包含四个管理服务 |
