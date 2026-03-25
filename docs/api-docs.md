# 秉羲管理系统 - API 接口文档

## 概述

本文档描述了秉羲管理系统的所有 REST API 接口。所有接口统一使用 `/api/v1/erp/` 前缀。

**基本信息**:
- **基础路径**: `/api/v1/erp`
- **协议**: HTTP/1.1
- **数据格式**: JSON
- **字符编码**: UTF-8
- **认证方式**: JWT Token

---

## 认证说明

### JWT Token 使用

除登录接口外，所有接口都需要在请求头中携带 JWT Token：

```
Authorization: Bearer <your_token_here>
```

### Token 获取

通过登录接口获取 Token，Token 有效期为 24 小时（可配置）。

---

## 接口清单

### 1. 认证模块 (Auth)

#### 1.1 用户登录

**接口**: `POST /api/v1/erp/auth/login`

**描述**: 用户登录并获取 JWT Token

**请求参数**:
```json
{
  "username": "string",     // 用户名（必填）
  "password": "string"      // 密码（必填）
}
```

**响应格式**:
- **成功** (200):
```json
{
  "success": true,
  "message": "登录成功",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": 1,
      "username": "admin",
      "email": "admin@bingxi.com",
      "role": "admin"
    }
  }
}
```

- **失败** (401):
```json
{
  "success": false,
  "message": "用户名或密码错误",
  "data": null
}
```

**错误码**:
- `401`: 用户名或密码错误
- `400`: 参数错误

**请求示例**:
```bash
curl -X POST http://localhost:8080/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123"
  }'
```

---

### 2. 用户管理模块 (Users)

#### 2.1 获取用户列表

**接口**: `GET /api/v1/erp/users`

**描述**: 获取用户列表（支持分页和过滤）

**请求参数**:
```
page: integer      // 页码，默认 1
page_size: integer // 每页数量，默认 10
status: string     // 状态过滤（可选）
role: string       // 角色过滤（可选）
```

**响应格式**:
- **成功** (200):
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "users": [
      {
        "id": 1,
        "username": "admin",
        "email": "admin@bingxi.com",
        "phone": "13800138000",
        "role": "admin",
        "status": "active",
        "last_login_at": "2026-03-15T10:00:00Z",
        "created_at": "2026-01-01T00:00:00Z"
      }
    ],
    "total": 10,
    "page": 1,
    "page_size": 10,
    "total_pages": 1
  }
}
```

**请求示例**:
```bash
curl -X GET "http://localhost:8080/api/v1/erp/users?page=1&page_size=10" \
  -H "Authorization: Bearer <token>"
```

---

#### 2.2 获取用户详情

**接口**: `GET /api/v1/erp/users/:id`

**描述**: 获取指定用户的详细信息

**路径参数**:
```
id: integer  // 用户 ID（必填）
```

**响应格式**:
- **成功** (200):
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "id": 1,
    "username": "admin",
    "email": "admin@bingxi.com",
    "phone": "13800138000",
    "role": {
      "id": 1,
      "name": "admin"
    },
    "department": {
      "id": 1,
      "name": "总经办"
    },
    "status": "active",
    "last_login_at": "2026-03-15T10:00:00Z",
    "created_at": "2026-01-01T00:00:00Z"
  }
}
```

**错误码**:
- `404`: 用户不存在

**请求示例**:
```bash
curl -X GET http://localhost:8080/api/v1/erp/users/1 \
  -H "Authorization: Bearer <token>"
```

---

#### 2.3 创建用户

**接口**: `POST /api/v1/erp/users`

**描述**: 创建新用户

**请求参数**:
```json
{
  "username": "string",        // 用户名（必填，唯一）
  "password": "string",        // 密码（必填）
  "email": "string",           // 邮箱（可选）
  "phone": "string",           // 手机号（可选）
  "role_id": 1,                // 角色 ID（必填）
  "department_id": 1           // 部门 ID（可选）
}
```

**响应格式**:
- **成功** (201):
```json
{
  "success": true,
  "message": "用户创建成功",
  "data": {
    "id": 2,
    "username": "new_user",
    "email": "user@example.com"
  }
}
```

**错误码**:
- `400`: 参数错误
- `409`: 用户名已存在

**请求示例**:
```bash
curl -X POST http://localhost:8080/api/v1/erp/users \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "zhangsan",
    "password": "password123",
    "email": "zhangsan@bingxi.com",
    "role_id": 2
  }'
```

---

### 3. 财务管理模块 (Finance)

#### 3.1 获取收款列表

**接口**: `GET /api/v1/erp/finance/payments`

**描述**: 获取财务收款列表

**请求参数**:
```
page: integer       // 页码
page_size: integer  // 每页数量
customer_name: string  // 客户名称（可选）
payment_date: string   // 收款日期（可选）
status: string         // 状态（可选）
```

**响应格式**:
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "payments": [
      {
        "id": 1,
        "payment_no": "PAY202603150001",
        "customer_name": "某某公司",
        "amount": 10000.00,
        "payment_method": "transfer",
        "payment_date": "2026-03-15",
        "status": "confirmed"
      }
    ],
    "total": 50
  }
}
```

**请求示例**:
```bash
curl -X GET "http://localhost:8080/api/v1/erp/finance/payments?page=1" \
  -H "Authorization: Bearer <token>"
```

---

#### 3.2 创建收款

**接口**: `POST /api/v1/erp/finance/payments`

**描述**: 创建财务收款记录

**请求参数**:
```json
{
  "order_id": 1,                // 关联订单 ID（可选）
  "customer_name": "string",    // 客户名称（必填）
  "payment_date": "2026-03-15", // 收款日期（必填）
  "payment_method": "transfer", // 收款方式（必填）
  "amount": 10000.00,           // 收款金额（必填）
  "bank_account": "6222001234567890", // 银行账户（可选）
  "remark": "string"            // 备注（可选）
}
```

**响应格式**:
```json
{
  "success": true,
  "message": "收款记录创建成功",
  "data": {
    "id": 1,
    "payment_no": "PAY202603150001"
  }
}
```

**请求示例**:
```bash
curl -X POST http://localhost:8080/api/v1/erp/finance/payments \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_name": "某某公司",
    "payment_date": "2026-03-15",
    "payment_method": "transfer",
    "amount": 10000.00
  }'
```

---

### 4. 销售管理模块 (Sales)

#### 4.1 获取订单列表

**接口**: `GET /api/v1/erp/sales/orders`

**描述**: 获取销售订单列表

**请求参数**:
```
page: integer       // 页码
page_size: integer  // 每页数量
order_no: string    // 订单编号（可选）
customer_name: string  // 客户名称（可选）
status: string         // 订单状态（可选）
order_date: string     // 订单日期（可选）
```

**响应格式**:
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "orders": [
      {
        "id": 1,
        "order_no": "SO202603150001",
        "customer_name": "某某公司",
        "total_amount": 50000.00,
        "paid_amount": 30000.00,
        "balance_amount": 20000.00,
        "status": "confirmed",
        "order_date": "2026-03-15"
      }
    ],
    "total": 100
  }
}
```

**请求示例**:
```bash
curl -X GET "http://localhost:8080/api/v1/erp/sales/orders?page=1" \
  -H "Authorization: Bearer <token>"
```

---

#### 4.2 创建订单

**接口**: `POST /api/v1/erp/sales/orders`

**描述**: 创建销售订单

**请求参数**:
```json
{
  "customer_name": "string",       // 客户名称（必填）
  "customer_contact": "string",    // 联系人（可选）
  "customer_phone": "string",      // 联系电话（可选）
  "order_date": "2026-03-15",      // 订单日期（必填）
  "delivery_date": "2026-03-20",   // 交货日期（可选）
  "shipping_address": "string",    // 收货地址（可选）
  "items": [                       // 订单明细（必填）
    {
      "product_id": 1,
      "batch_no": "B20260315001",
      "color_code": "C001",
      "quantity": 100,
      "unit": "米",
      "unit_price": 50.00
    }
  ]
}
```

**响应格式**:
```json
{
  "success": true,
  "message": "订单创建成功",
  "data": {
    "id": 1,
    "order_no": "SO202603150001"
  }
}
```

**请求示例**:
```bash
curl -X POST http://localhost:8080/api/v1/erp/sales/orders \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_name": "某某公司",
    "order_date": "2026-03-15",
    "items": [
      {
        "product_id": 1,
        "quantity": 100,
        "unit_price": 50.00
      }
    ]
  }'
```

---

### 5. 库存管理模块 (Inventory)

#### 5.1 获取库存列表

**接口**: `GET /api/v1/erp/inventory/stock`

**描述**: 获取库存列表

**请求参数**:
```
page: integer       // 页码
page_size: integer  // 每页数量
product_id: integer // 产品 ID（可选）
warehouse_id: integer // 仓库 ID（可选）
batch_no: string    // 批次号（可选）
color_code: string  // 色号（可选）
status: string      // 状态（可选）
```

**响应格式**:
```json
{
  "success": true,
  "message": "获取成功",
  "data": {
    "stocks": [
      {
        "id": 1,
        "product": {
          "id": 1,
          "name": "纯棉面料",
          "code": "P001"
        },
        "warehouse": {
          "id": 1,
          "name": "主仓库"
        },
        "batch_no": "B20260315001",
        "color_code": "C001",
        "color_name": "白色",
        "quantity": 1000.00,
        "unit": "米",
        "unit_price": 50.00,
        "total_amount": 50000.00,
        "status": "active"
      }
    ],
    "total": 200
  }
}
```

**请求示例**:
```bash
curl -X GET "http://localhost:8080/api/v1/erp/inventory/stock?page=1" \
  -H "Authorization: Bearer <token>"
```

---

#### 5.2 创建库存

**接口**: `POST /api/v1/erp/inventory/stock`

**描述**: 创建库存记录

**请求参数**:
```json
{
  "product_id": 1,            // 产品 ID（必填）
  "warehouse_id": 1,          // 仓库 ID（必填）
  "batch_no": "B20260315001", // 批次号（必填）
  "color_code": "C001",       // 色号（必填）
  "color_name": "白色",       // 颜色名称（可选）
  "quantity": 1000,           // 数量（必填）
  "unit": "米",               // 单位（必填）
  "unit_price": 50.00,        // 单价（可选）
  "production_date": "2026-03-15", // 生产日期（可选）
  "min_stock": 100            // 最低库存预警线（可选）
}
```

**响应格式**:
```json
{
  "success": true,
  "message": "库存记录创建成功",
  "data": {
    "id": 1
  }
}
```

**请求示例**:
```bash
curl -X POST http://localhost:8080/api/v1/erp/inventory/stock \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "product_id": 1,
    "warehouse_id": 1,
    "batch_no": "B20260315001",
    "color_code": "C001",
    "quantity": 1000,
    "unit": "米"
  }'
```

---

#### 5.3 低库存预警

**接口**: `GET /api/v1/erp/inventory/stock/low-stock`

**描述**: 获取低库存预警列表

**请求参数**: 无

**响应格式**:
```json
{
  "success": true,
  "message": "获取成功",
  "data": [
    {
      "id": 1,
      "batch_no": "B20260315001",
      "color_code": "C001",
      "product_name": "纯棉面料",
      "product_code": "P001",
      "warehouse_name": "主仓库",
      "quantity": 50,
      "min_stock": 100,
      "shortage": 50
    }
  ]
}
```

**请求示例**:
```bash
curl -X GET http://localhost:8080/api/v1/erp/inventory/stock/low-stock \
  -H "Authorization: Bearer <token>"
```

---

## 统一响应格式

所有接口统一使用以下响应格式：

```json
{
  "success": true/false,      // 是否成功
  "message": "string",        // 响应消息
  "data": {}                  // 响应数据（成功时）或 null（失败时）
}
```

---

## 错误码说明

### HTTP 状态码

- `200`: 成功
- `201`: 创建成功
- `400`: 请求参数错误
- `401`: 未授权（Token 无效或过期）
- `403`: 禁止访问（权限不足）
- `404`: 资源不存在
- `409`: 资源冲突（如用户名已存在）
- `500`: 服务器内部错误

### 业务错误码

| 错误码 | 说明 | 解决方案 |
|--------|------|----------|
| `AUTH_001` | 用户名或密码错误 | 检查用户名和密码 |
| `AUTH_002` | Token 无效或过期 | 重新登录获取 Token |
| `AUTH_003` | 权限不足 | 联系管理员分配权限 |
| `USER_001` | 用户不存在 | 检查用户 ID 是否正确 |
| `USER_002` | 用户名已存在 | 使用其他用户名 |
| `ORDER_001` | 订单不存在 | 检查订单 ID |
| `STOCK_001` | 库存不足 | 补充库存 |

---

## 最佳实践

### 1. 请求头设置

```javascript
// 示例：使用 fetch API
const headers = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${token}`
};

const response = await fetch('/api/v1/erp/users', { headers });
```

### 2. 错误处理

```javascript
try {
  const response = await fetch('/api/v1/erp/users');
  const data = await response.json();
  
  if (data.success) {
    // 处理成功响应
    console.log(data.data);
  } else {
    // 处理业务错误
    console.error(data.message);
  }
} catch (error) {
  // 处理网络错误
  console.error('网络错误:', error);
}
```

### 3. Token 刷新

```javascript
// 检查 Token 是否过期
if (response.status === 401) {
  // Token 过期，重新登录
  await logout();
  window.location.href = '/login';
}
```

---

## 更新日志

### v1.0.0 (2026-03-15)
- 初始版本
- 认证模块
- 用户管理模块
- 财务管理模块
- 销售管理模块
- 库存管理模块

---

**文档版本**: v1.0  
**最后更新**: 2026-03-15  
**维护者**: 秉羲团队
