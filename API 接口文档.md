# 秉羲 ERP 系统 - API 接口文档

## 文档说明

**版本**: 2026-03-15  
**基础路径**: `/api/v1/erp`  
**认证方式**: JWT Bearer Token

---

## 认证授权

### 1. 用户登录

**接口**: `POST /auth/login`

**请求**:
```json
{
  "username": "admin",
  "password": "admin123"
}
```

**响应** (成功):
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": 1,
      "username": "admin",
      "email": "admin@example.com",
      "role_id": 1
    }
  }
}
```

---

### 2. 用户注销

**接口**: `POST /auth/logout`

**请求头**:
```
Authorization: Bearer {token}
```

**响应**:
```json
{
  "success": true,
  "message": "注销成功"
}
```

---

### 3. 刷新 Token

**接口**: `POST /auth/refresh`

**请求头**:
```
Authorization: Bearer {old_token}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 86400
  }
}
```

---

## 用户管理

### 1. 查询用户列表

**接口**: `GET /users?page=0&page_size=20`

**响应**:
```json
{
  "success": true,
  "data": {
    "users": [
      {
        "id": 1,
        "username": "admin",
        "email": "admin@example.com",
        "phone": "13800138000",
        "role_id": 1,
        "department_id": 1,
        "is_active": true,
        "created_at": "2026-03-15T10:00:00Z"
      }
    ],
    "total": 10,
    "page": 0,
    "page_size": 20
  }
}
```

---

### 2. 查询用户详情

**接口**: `GET /users/:id`

**响应**:
```json
{
  "success": true,
  "data": {
    "id": 1,
    "username": "admin",
    "email": "admin@example.com",
    "phone": "13800138000",
    "role_id": 1,
    "department_id": 1,
    "is_active": true,
    "created_at": "2026-03-15T10:00:00Z"
  }
}
```

---

### 3. 创建用户

**接口**: `POST /users`

**请求**:
```json
{
  "username": "newuser",
  "password": "password123",
  "email": "user@example.com",
  "phone": "13800138000",
  "role_id": 2,
  "department_id": 1
}
```

---

### 4. 更新用户

**接口**: `PUT /users/:id`

**请求**:
```json
{
  "email": "newemail@example.com",
  "phone": "13900139000",
  "role_id": 2,
  "department_id": 2,
  "status": "active"
}
```

**说明**: 增量更新，只更新提供的字段

---

### 5. 删除用户

**接口**: `DELETE /users/:id`

**响应**:
```json
{
  "success": true,
  "message": "用户删除成功"
}
```

**说明**: 软删除，只设置非激活状态

---

## 库存调整

### 1. 创建调整单

**接口**: `POST /inventory/adjustments`

**请求**:
```json
{
  "warehouse_id": 1,
  "adjustment_date": "2026-03-15T10:00:00Z",
  "adjustment_type": "decrease",
  "reason_type": "damage",
  "reason_description": "库存损坏",
  "notes": "测试调整",
  "items": [
    {
      "stock_id": 1,
      "quantity": "10.00",
      "unit_cost": "100.00",
      "notes": "损坏扣减"
    }
  ]
}
```

**响应**:
```json
{
  "success": true,
  "data": {
    "id": 1,
    "adjustment_no": "ADJ202603150001",
    "warehouse_id": 1,
    "adjustment_type": "decrease",
    "status": "pending",
    "total_quantity": "10.00",
    "items": [...]
  }
}
```

---

### 2. 查询调整单列表

**接口**: `GET /inventory/adjustments?page=0&page_size=20`

**响应**:
```json
{
  "success": true,
  "data": {
    "adjustments": [
      {
        "id": 1,
        "adjustment_no": "ADJ202603150001",
        "warehouse_id": 1,
        "adjustment_type": "decrease",
        "reason_type": "damage",
        "status": "pending",
        "total_quantity": "10.00",
        "created_at": "2026-03-15T10:00:00Z"
      }
    ],
    "total": 5,
    "page": 0,
    "page_size": 20
  }
}
```

---

### 3. 查询调整单详情

**接口**: `GET /inventory/adjustments/:id`

**响应**:
```json
{
  "success": true,
  "data": {
    "id": 1,
    "adjustment_no": "ADJ202603150001",
    "warehouse_id": 1,
    "adjustment_type": "decrease",
    "status": "pending",
    "items": [
      {
        "id": 1,
        "stock_id": 1,
        "quantity": "10.00",
        "quantity_before": "100.00",
        "quantity_after": "90.00",
        "unit_cost": "100.00",
        "amount": "1000.00"
      }
    ]
  }
}
```

---

### 4. 审核调整单

**接口**: `POST /inventory/adjustments/:id/approve`

**响应**:
```json
{
  "success": true,
  "data": {
    "id": 1,
    "adjustment_no": "ADJ202603150001",
    "status": "approved",
    "approved_at": "2026-03-15T11:00:00Z"
  }
}
```

**说明**: 审核通过后自动更新库存数量

---

### 5. 驳回调整单

**接口**: `POST /inventory/adjustments/:id/reject`

**响应**:
```json
{
  "success": true,
  "data": {
    "id": 1,
    "adjustment_no": "ADJ202603150001",
    "status": "rejected"
  }
}
```

---

## 操作日志

### 1. 查询操作日志列表

**接口**: `GET /operation-logs?page=0&page_size=20`

**查询参数**:
- `module` (可选): 按模块筛选
- `user_id` (可选): 按用户筛选
- `status` (可选): 按状态筛选

**响应**:
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "id": 1,
        "user_id": 1,
        "username": "admin",
        "module": "inventory_adjustment",
        "action": "create",
        "description": "POST /api/v1/erp/inventory/adjustments",
        "request_ip": "192.168.1.100",
        "status": "success",
        "duration_ms": 45,
        "created_at": "2026-03-15T10:00:00Z"
      }
    ],
    "total": 100,
    "page": 0,
    "page_size": 20
  }
}
```

---

## 错误响应格式

**通用错误响应**:
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "请求参数验证失败"
  }
}
```

**HTTP 状态码**:
- `200` - 成功
- `400` - 请求参数错误
- `401` - 未授权
- `403` - 禁止访问
- `404` - 资源不存在
- `500` - 服务器内部错误

---

## 认证说明

所有接口（除登录外）都需要在请求头中携带 JWT Token：

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

Token 有效期为 24 小时，过期后需要使用刷新接口获取新 Token。

---

## 分页说明

所有列表接口都支持分页，使用以下查询参数：

- `page`: 页码（从 0 开始），默认 0
- `page_size`: 每页数量，默认 20，最大 100

**示例**:
```
GET /api/v1/erp/users?page=0&page_size=50
```

---

**文档更新时间**: 2026-03-15
