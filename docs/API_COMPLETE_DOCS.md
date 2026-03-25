# 秉羲 ERP 系统 - API 接口完整文档

## 📌 接口规范

### 基础信息
- **基础路径**: `/api/v1/erp`
- **数据格式**: JSON
- **认证方式**: JWT Bearer Token
- **字符编码**: UTF-8

### 请求头
```http
Authorization: Bearer <token>
Content-Type: application/json
```

### 响应格式
```json
{
    "code": 200,
    "message": "操作成功",
    "data": { ... }
}
```

### 错误码说明
| 错误码 | 说明 |
|--------|------|
| 200 | 成功 |
| 400 | 请求参数错误 |
| 401 | 未授权 |
| 403 | 禁止访问 |
| 404 | 资源不存在 |
| 500 | 服务器内部错误 |

---

## 1️⃣ 认证模块 (Auth)

**Handler 文件**: `backend/src/handlers/auth_handler.rs`

### 1.1 用户登录
```http
POST /api/v1/erp/auth/login
```

**请求参数**:
```json
{
    "username": "string, 必填，用户名",
    "password": "string, 必填，密码"
}
```

**响应示例**:
```json
{
    "code": 200,
    "message": "登录成功",
    "data": {
        "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
        "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
        "user_info": {
            "id": 1,
            "username": "admin",
            "real_name": "管理员",
            "role_id": 1,
            "role_name": "超级管理员"
        }
    }
}
```

### 1.2 用户登出
```http
POST /api/v1/erp/auth/logout
```

**请求参数**:
```json
{
    "token": "string, 必填，当前有效的 token"
}
```

**响应示例**:
```json
{
    "code": 200,
    "message": "登出成功",
    "data": null
}
```

### 1.3 刷新令牌
```http
POST /api/v1/erp/auth/refresh
```

**请求参数**:
```json
{
    "refresh_token": "string, 必填，刷新令牌"
}
```

**响应示例**:
```json
{
    "code": 200,
    "message": "令牌刷新成功",
    "data": {
        "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
        "refresh_token": "dGhpcyBpcyBhIG5ldyByZWZyZXNoIHRva2Vu..."
    }
}
```

---

## 2️⃣ 用户管理模块 (User)

**Handler 文件**: `backend/src/handlers/user_handler.rs`  
**Service 文件**: `backend/src/services/user_service.rs`  
**Model 文件**: `backend/src/models/user.rs`

### 2.1 获取用户列表
```http
GET /api/v1/erp/users
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码，默认 1 |
| page_size | integer | 否 | 每页数量，默认 10 |
| search | string | 否 | 搜索关键词（用户名/姓名） |
| role_id | integer | 否 | 按角色筛选 |
| department_id | integer | 否 | 按部门筛选 |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "items": [
            {
                "id": 1,
                "username": "admin",
                "real_name": "管理员",
                "email": "admin@example.com",
                "phone": "13800138000",
                "role_id": 1,
                "department_id": 1,
                "is_active": true,
                "created_at": "2024-01-01T00:00:00Z"
            }
        ],
        "total": 100,
        "page": 1,
        "page_size": 10
    }
}
```

### 2.2 获取用户详情
```http
GET /api/v1/erp/users/:id
```

**路径参数**:
- `id`: integer, 必填，用户 ID

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "id": 1,
        "username": "admin",
        "real_name": "管理员",
        "email": "admin@example.com",
        "phone": "13800138000",
        "role_id": 1,
        "department_id": 1,
        "is_active": true,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }
}
```

### 2.3 创建用户
```http
POST /api/v1/erp/users
```

**请求参数**:
```json
{
    "username": "string, 必填，用户名（3-20 位字母数字下划线）",
    "password": "string, 必填，密码（6-20 位）",
    "real_name": "string, 必填，真实姓名",
    "email": "string, 可选，邮箱",
    "phone": "string, 可选，手机号",
    "role_id": "integer, 必填，角色 ID",
    "department_id": "integer, 可选，部门 ID"
}
```

**响应示例**:
```json
{
    "code": 200,
    "message": "用户创建成功",
    "data": {
        "id": 2,
        "username": "zhangsan",
        "real_name": "张三",
        "email": "zhangsan@example.com",
        "role_id": 2,
        "department_id": 1,
        "is_active": true,
        "created_at": "2024-01-02T00:00:00Z"
    }
}
```

### 2.4 更新用户
```http
PUT /api/v1/erp/users/:id
```

**路径参数**:
- `id`: integer, 必填，用户 ID

**请求参数**:
```json
{
    "real_name": "string, 可选，真实姓名",
    "email": "string, 可选，邮箱",
    "phone": "string, 可选，手机号",
    "role_id": "integer, 可选，角色 ID",
    "department_id": "integer, 可选，部门 ID",
    "is_active": "boolean, 可选，是否启用"
}
```

### 2.5 删除用户
```http
DELETE /api/v1/erp/users/:id
```

**路径参数**:
- `id`: integer, 必填，用户 ID

**响应示例**:
```json
{
    "code": 200,
    "message": "用户删除成功",
    "data": null
}
```

---

## 3️⃣ 产品管理模块 (Product)

**Handler 文件**: `backend/src/handlers/product_handler.rs`  
**Service 文件**: `backend/src/services/product_service.rs`  
**Model 文件**: `backend/src/models/product.rs`

### 3.1 获取产品列表
```http
GET /api/v1/erp/products
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码，默认 1 |
| page_size | integer | 否 | 每页数量，默认 10 |
| category_id | integer | 否 | 按分类筛选 |
| search | string | 否 | 搜索关键词（产品编号/名称） |
| product_type | string | 否 | 产品类型（坯布/成品布/辅料） |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "items": [
            {
                "id": 1,
                "product_no": "P20240101001",
                "product_name": "全棉斜纹面料",
                "product_type": "成品布",
                "category_id": 1,
                "fabric_composition": "100% 棉",
                "yarn_count": "40S",
                "density": "133x72",
                "width": "150",
                "gram_weight": "120",
                "structure": "斜纹",
                "unit": "米",
                "standard_price": "25.00",
                "is_active": true
            }
        ],
        "total": 50,
        "page": 1,
        "page_size": 10
    }
}
```

### 3.2 获取产品详情
```http
GET /api/v1/erp/products/:id
```

**路径参数**:
- `id`: integer, 必填，产品 ID

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "id": 1,
        "product_no": "P20240101001",
        "product_name": "全棉斜纹面料",
        "product_type": "成品布",
        "category_id": 1,
        "fabric_composition": "100% 棉",
        "yarn_count": "40S",
        "density": "133x72",
        "width": "150",
        "gram_weight": "120",
        "structure": "斜纹",
        "finish": "防水",
        "unit": "米",
        "standard_price": "25.00",
        "min_order_quantity": "1000",
        "lead_time": 15,
        "is_active": true,
        "created_at": "2024-01-01T00:00:00Z",
        "colors": [
            {
                "id": 1,
                "color_no": "C001",
                "color_name": "本白",
                "pantone_code": "PANTONE 11-0601",
                "is_active": true
            }
        ]
    }
}
```

### 3.3 创建产品
```http
POST /api/v1/erp/products
```

**请求参数**:
```json
{
    "product_name": "string, 必填，产品名称",
    "product_type": "string, 必填，产品类型",
    "category_id": "integer, 必填，分类 ID",
    "fabric_composition": "string, 可选，面料成分",
    "yarn_count": "string, 可选，纱支",
    "density": "string, 可选，密度",
    "width": "decimal, 可选，幅宽",
    "gram_weight": "decimal, 可选，克重",
    "structure": "string, 可选，组织结构",
    "finish": "string, 可选，后整理",
    "unit": "string, 必填，单位",
    "standard_price": "decimal, 必填，标准价格",
    "min_order_quantity": "decimal, 可选，最小起订量",
    "lead_time": "integer, 可选，交货期（天）"
}
```

### 3.4 更新产品
```http
PUT /api/v1/erp/products/:id
```

**路径参数**:
- `id`: integer, 必填，产品 ID

**请求参数**: 同创建产品，所有字段可选

### 3.5 删除产品
```http
DELETE /api/v1/erp/products/:id
```

### 3.6 获取产品色号列表
```http
GET /api/v1/erp/products/:id/colors
```

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": [
        {
            "id": 1,
            "product_id": 1,
            "color_no": "C001",
            "color_name": "本白",
            "pantone_code": "PANTONE 11-0601",
            "color_type": "常规色",
            "extra_cost": "0.00",
            "is_active": true
        }
    ]
}
```

### 3.7 创建产品色号
```http
POST /api/v1/erp/products/:id/colors
```

**请求参数**:
```json
{
    "color_no": "string, 必填，色号",
    "color_name": "string, 必填，颜色名称",
    "pantone_code": "string, 可选，潘通色号",
    "color_type": "string, 可选，颜色类型",
    "extra_cost": "decimal, 可选，特殊色号加价"
}
```

---

## 4️⃣ 销售订单模块 (Sales Order)

**Handler 文件**: `backend/src/handlers/sales_order_handler.rs`  
**Service 文件**: `backend/src/services/sales_service.rs`  
**Model 文件**: `backend/src/models/sales_order.rs`

### 4.1 获取订单列表
```http
GET /api/v1/erp/sales/orders
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |
| customer_id | integer | 否 | 按客户筛选 |
| status | string | 否 | 按状态筛选 |
| order_date_start | date | 否 | 订单开始日期 |
| order_date_end | date | 否 | 订单结束日期 |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "items": [
            {
                "id": 1,
                "order_no": "SO20240101001",
                "customer_id": 1,
                "customer_name": "某某纺织有限公司",
                "order_date": "2024-01-01",
                "status": "pending",
                "total_amount": "50000.00",
                "paid_amount": "0.00",
                "balance_amount": "50000.00",
                "created_at": "2024-01-01T10:00:00Z"
            }
        ],
        "total": 25,
        "page": 1,
        "page_size": 10
    }
}
```

### 4.2 获取订单详情
```http
GET /api/v1/erp/sales/orders/:id
```

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "id": 1,
        "order_no": "SO20240101001",
        "customer_id": 1,
        "customer_name": "某某纺织有限公司",
        "order_date": "2024-01-01",
        "delivery_date": "2024-01-15",
        "status": "pending",
        "total_amount": "50000.00",
        "paid_amount": "0.00",
        "balance_amount": "50000.00",
        "notes": "加急订单",
        "items": [
            {
                "id": 1,
                "product_id": 1,
                "product_name": "全棉斜纹面料",
                "color_no": "C001",
                "color_name": "本白",
                "quantity": "2000",
                "unit": "米",
                "unit_price": "25.00",
                "amount": "50000.00"
            }
        ]
    }
}
```

### 4.3 创建订单
```http
POST /api/v1/erp/sales/orders
```

**请求参数**:
```json
{
    "customer_id": "integer, 必填，客户 ID",
    "order_date": "date, 必填，订单日期",
    "delivery_date": "date, 必填，交货日期",
    "notes": "string, 可选，备注",
    "items": [
        {
            "product_id": "integer, 必填，产品 ID",
            "color_no": "string, 必填，色号",
            "quantity": "decimal, 必填，数量",
            "unit_price": "decimal, 必填，单价"
        }
    ]
}
```

---

## 5️⃣ 库存管理模块 (Inventory)

**Handler 文件**: `backend/src/handlers/inventory_stock_handler.rs`  
**Service 文件**: `backend/src/services/inventory_stock_service.rs`  
**Model 文件**: `backend/src/models/inventory_stock.rs`

### 5.1 获取库存列表
```http
GET /api/v1/erp/inventory/stock
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| warehouse_id | integer | 否 | 按仓库筛选 |
| batch_no | string | 否 | 按批次筛选 |
| color_no | string | 否 | 按色号筛选 |
| product_id | integer | 否 | 按产品筛选 |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": [
        {
            "id": 1,
            "warehouse_id": 1,
            "warehouse_name": "成品库",
            "location_id": 1,
            "product_id": 1,
            "product_name": "全棉斜纹面料",
            "batch_no": "B20240101001",
            "color_no": "C001",
            "color_name": "本白",
            "dye_lot_no": "D20240101001",
            "grade": "一等品",
            "quantity_meters": "5000.00",
            "quantity_kg": "600.00",
            "unit": "米",
            "created_at": "2024-01-01T00:00:00Z"
        }
    ]
}
```

---

## 6️⃣ 仓库管理模块 (Warehouse)

**Handler 文件**: `backend/src/handlers/warehouse_handler.rs`  
**Service 文件**: `backend/src/services/warehouse_service.rs`  
**Model 文件**: `backend/src/models/warehouse.rs`

### 6.1 获取仓库列表
```http
GET /api/v1/erp/warehouses
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |
| type | string | 否 | 仓库类型 |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "items": [
            {
                "id": 1,
                "warehouse_no": "WH001",
                "warehouse_name": "成品库",
                "type": "finished_goods",
                "address": "江苏省苏州市 XX 路 XX 号",
                "capacity": "10000",
                "manager": "张三",
                "phone": "13800138000",
                "is_active": true
            }
        ],
        "total": 5,
        "page": 1,
        "page_size": 10
    }
}
```

### 6.2 创建仓库
```http
POST /api/v1/erp/warehouses
```

**请求参数**:
```json
{
    "warehouse_no": "string, 必填，仓库编号",
    "warehouse_name": "string, 必填，仓库名称",
    "type": "string, 必填，仓库类型",
    "address": "string, 可选，地址",
    "capacity": "decimal, 可选，容量",
    "manager": "string, 可选，管理员",
    "phone": "string, 可选，联系电话"
}
```

---

## 7️⃣ 采购管理模块 (Purchase Order)

**Handler 文件**: `backend/src/handlers/purchase_order_handler.rs`  
**Service 文件**: `backend/src/services/purchase_order_service.rs`  
**Model 文件**: `backend/src/models/purchase_order.rs`

### 7.1 获取采购订单列表
```http
GET /api/v1/erp/purchase/orders
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |
| supplier_id | integer | 否 | 按供应商筛选 |
| status | string | 否 | 按状态筛选 |

**响应示例**:
```json
{
    "code": 200,
    "message": "获取成功",
    "data": {
        "items": [
            {
                "id": 1,
                "order_no": "PO20240101001",
                "supplier_id": 1,
                "supplier_name": "某某纺织厂",
                "order_date": "2024-01-01",
                "status": "pending",
                "total_amount": "30000.00",
                "received_amount": "0.00",
                "balance_amount": "30000.00"
            }
        ]
    }
}
```

---

## 8️⃣ 供应商管理模块 (Supplier)

**Handler 文件**: `backend/src/handlers/supplier_handler.rs`  
**Service 文件**: `backend/src/services/supplier_service.rs`  
**Model 文件**: `backend/src/models/supplier.rs`

### 8.1 获取供应商列表
```http
GET /api/v1/erp/suppliers
```

**查询参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | integer | 否 | 页码 |
| page_size | integer | 否 | 每页数量 |
| search | string | 否 | 搜索关键词 |
| grade | string | 否 | 按等级筛选 |

### 8.2 创建供应商
```http
POST /api/v1/erp/suppliers
```

**请求参数**:
```json
{
    "supplier_no": "string, 必填，供应商编号",
    "supplier_name": "string, 必填，供应商名称",
    "contact_person": "string, 可选，联系人",
    "contact_phone": "string, 可选，联系电话",
    "address": "string, 可选，地址",
    "grade": "string, 可选，等级",
    "category_id": "integer, 可选，分类 ID"
}
```

---

## 9️⃣ 应付管理模块 (AP)

### 9.1 获取应付单列表
**Handler 文件**: `backend/src/handlers/ap_invoice_handler.rs`

```http
GET /api/v1/erp/ap/invoices
```

### 9.2 创建应付单
```http
POST /api/v1/erp/ap/invoices
```

### 9.3 获取付款申请列表
**Handler 文件**: `backend/src/handlers/ap_payment_request_handler.rs`

```http
GET /api/v1/erp/ap/payment-requests
```

### 9.4 创建付款申请
```http
POST /api/v1/erp/ap/payment-requests
```

---

## 🔟 应收管理模块 (AR)

### 10.1 获取应收单列表
**Handler 文件**: `backend/src/handlers/ar_invoice_handler.rs`

```http
GET /api/v1/erp/ar/invoices
```

### 10.2 创建应收单
```http
POST /api/v1/erp/ar/invoices
```

---

## 1️⃣1️⃣ 总账管理模块 (GL)

### 11.1 获取会计科目列表
**Handler 文件**: `backend/src/handlers/account_subject_handler.rs`

```http
GET /api/v1/erp/gl/account-subjects
```

### 11.2 获取凭证列表
**Handler 文件**: `backend/src/handlers/voucher_handler.rs`

```http
GET /api/v1/erp/gl/vouchers
```

---

## 📊 完整 API 列表（按模块）

### 基础管理（9 个模块，约 45 个接口）
1. 认证 - 3 个接口
2. 用户 - 5 个接口
3. 角色 - 7 个接口
4. 部门 - 5 个接口
5. 仪表板 - 3 个接口
6. 健康检查 - 2 个接口
7. 批量处理（旧）- 3 个接口
8. 批量处理（新）- 6 个接口
9. 客户 - 5 个接口

### 产品与仓库（7 个模块，约 35 个接口）
10. 产品 - 10 个接口
11. 产品分类 - 6 个接口
12. 仓库 - 7 个接口
13. 库存 - 5 个接口
14. 库存调拨 - 6 个接口
15. 库存盘点 - 6 个接口
16. 库存调整 - 5 个接口

### 销售管理（5 个模块，约 25 个接口）
17. 销售订单 - 7 个接口
18. 面料订单 - 5 个接口
19. 销售合同 - 5 个接口
20. 销售分析 - 3 个接口
21. 销售价格 - 5 个接口

### 采购管理（6 个模块，约 30 个接口）
22. 采购订单 - 7 个接口
23. 采购入库 - 5 个接口
24. 采购退货 - 5 个接口
25. 采购质检 - 5 个接口
26. 采购合同 - 5 个接口
27. 采购价格 - 3 个接口

### 供应商管理（2 个模块，约 10 个接口）
28. 供应商 - 5 个接口
29. 供应商评估 - 5 个接口

### 财务管理（8 个模块，约 40 个接口）
30. 财务付款 - 5 个接口
31. 财务发票 - 5 个接口
32. 应付单 - 5 个接口
33. 付款申请 - 5 个接口
34. 应付付款 - 5 个接口
35. 应付核销 - 5 个接口
36. 供应商对账 - 5 个接口
37. 应付报表 - 5 个接口

### 其他模块（25 个模块，约 125 个接口）
38-62. 应收、总账、成本、固定资产等

**总计**: 约 **310 个 API 接口**

---

## 📝 附录

### A. 请求示例（cURL）

#### 登录
```bash
curl -X POST http://localhost:8000/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"123456"}'
```

#### 获取产品列表
```bash
curl -X GET "http://localhost:8000/api/v1/erp/products?page=1&page_size=10" \
  -H "Authorization: Bearer <token>"
```

#### 创建销售订单
```bash
curl -X POST http://localhost:8000/api/v1/erp/sales/orders \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "customer_id": 1,
    "order_date": "2024-01-01",
    "delivery_date": "2024-01-15",
    "items": [
      {
        "product_id": 1,
        "color_no": "C001",
        "quantity": "2000",
        "unit_price": "25.00"
      }
    ]
  }'
```

### B. 数据类型说明

| 类型 | 说明 | 示例 |
|------|------|------|
| integer | 整数 | 1, 100, -5 |
| decimal | 小数 | 25.00, 120.50 |
| string | 字符串 | "admin", "全棉面料" |
| boolean | 布尔值 | true, false |
| date | 日期 | "2024-01-01" |
| datetime | 日期时间 | "2024-01-01T10:00:00Z" |

### C. 通用查询参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| page | 页码 | 1 |
| page_size | 每页数量 | 10 |
| search | 搜索关键词 | - |
| sort_by | 排序字段 | created_at |
| sort_order | 排序方向 | desc |

---

**文档版本**: v1.0  
**最后更新**: 2026-03-21  
**维护者**: 秉羲团队
