# 冰溪 ERP 接口文档

## 概述

冰溪 ERP 系统提供 RESTful API 和 gRPC 两种接口方式。REST API 遵循 `/api/v1/erp/` 前缀规范，支持 JSON 格式请求和响应。所有 API 需要 JWT Token 认证（除公开接口外），并支持 CSRF 防护、速率限制等安全机制。

## 认证方式

### JWT Token 认证

**登录获取 Token**:
```http
POST /api/v1/erp/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "admin123"
}
```

**响应**:
```json
{
  "code": 200,
  "message": "登录成功",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600,
    "user": {
      "id": "uuid",
      "username": "admin",
      "email": "admin@example.com",
      "roles": ["admin"]
    }
  }
}
```

**使用 Token 访问 API**:
```http
GET /api/v1/erp/users
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Cookie 认证

系统也支持通过 HttpOnly Cookie 进行认证，适用于 Web 前端自动携带凭证。

### CSRF 防护

对于状态变更操作（POST/PUT/DELETE），需要携带 CSRF Token：
```http
POST /api/v1/erp/sales/orders
X-CSRF-Token: <csrf_token>
```

## API 端点分类

### 认证模块 (`/api/v1/erp/auth`)

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | `/login` | 用户登录 |
| POST | `/logout` | 用户登出 |
| POST | `/refresh` | 刷新 Token |
| GET | `/csrf-token` | 获取 CSRF Token |
| POST | `/totp/setup` | 设置 TOTP 两步验证 |
| POST | `/totp/verify` | 验证 TOTP 验证码 |

### 用户管理 (`/api/v1/erp/users`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/` | 获取用户列表 |
| POST | `/` | 创建用户 |
| GET | `/{id}` | 获取用户详情 |
| PUT | `/{id}` | 更新用户信息 |
| DELETE | `/{id}` | 删除用户 |
| POST | `/{id}/reset-password` | 重置密码 |
| PUT | `/{id}/roles` | 分配角色 |

### 角色管理 (`/api/v1/erp/roles`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/` | 获取角色列表 |
| POST | `/` | 创建角色 |
| GET | `/{id}` | 获取角色详情 |
| PUT | `/{id}` | 更新角色 |
| DELETE | `/{id}` | 删除角色 |
| PUT | `/{id}/permissions` | 分配权限 |

### 产品管理 (`/api/v1/erp/products`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/` | 获取产品列表 |
| POST | `/` | 创建产品 |
| GET | `/{id}` | 获取产品详情 |
| PUT | `/{id}` | 更新产品 |
| DELETE | `/{id}` | 删除产品 |
| POST | `/import` | 批量导入产品 |
| GET | `/export` | 导出产品数据 |
| POST | `/{id}/colors` | 添加产品颜色 |
| GET | `/{id}/colors` | 获取产品颜色列表 |

### 销售管理 (`/api/v1/erp/sales`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/orders` | 获取销售订单列表 |
| POST | `/orders` | 创建销售订单 |
| GET | `/orders/{id}` | 获取订单详情 |
| PUT | `/orders/{id}` | 更新订单 |
| DELETE | `/orders/{id}` | 删除订单 |
| POST | `/orders/{id}/submit` | 提交订单审批 |
| POST | `/orders/{id}/approve` | 审批订单 |
| POST | `/orders/{id}/ship` | 订单发货 |
| POST | `/orders/{id}/complete` | 完成订单 |
| GET | `/contracts` | 获取销售合同列表 |
| POST | `/contracts` | 创建销售合同 |
| GET | `/prices` | 获取销售价格列表 |
| POST | `/prices` | 设置销售价格 |
| GET | `/returns` | 获取销售退货列表 |
| POST | `/returns` | 创建销售退货 |

### 采购管理 (`/api/v1/erp/purchases`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/orders` | 获取采购订单列表 |
| POST | `/orders` | 创建采购订单 |
| GET | `/orders/{id}` | 获取订单详情 |
| PUT | `/orders/{id}` | 更新订单 |
| DELETE | `/orders/{id}` | 删除订单 |
| POST | `/orders/{id}/receive` | 采购收货 |
| POST | `/orders/{id}/inspect` | 质量检验 |
| GET | `/contracts` | 获取采购合同列表 |
| POST | `/contracts` | 创建采购合同 |
| GET | `/prices` | 获取采购价格列表 |
| POST | `/prices` | 设置采购价格 |
| GET | `/returns` | 获取采购退货列表 |
| POST | `/returns` | 创建采购退货 |

### 库存管理 (`/api/v1/erp/inventory`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/stocks` | 获取库存列表 |
| GET | `/stocks/{id}` | 获取库存详情 |
| POST | `/transfers` | 创建库存调拨 |
| GET | `/transfers` | 获取调拨记录 |
| POST | `/counts` | 创建库存盘点 |
| GET | `/counts` | 获取盘点记录 |
| POST | `/adjustments` | 创建库存调整 |
| GET | `/adjustments` | 获取调整记录 |
| POST | `/reservations` | 创建库存预留 |
| GET | `/reservations` | 获取预留记录 |

### 财务管理 (`/api/v1/erp/finance`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/invoices` | 获取发票列表 |
| POST | `/invoices` | 创建发票 |
| GET | `/payments` | 获取付款列表 |
| POST | `/payments` | 创建付款 |
| GET | `/vouchers` | 获取凭证列表 |
| POST | `/vouchers` | 创建凭证 |
| GET | `/account-subjects` | 获取会计科目 |
| POST | `/account-subjects` | 创建会计科目 |
| GET | `/accounting-periods` | 获取会计期间 |
| POST | `/accounting-periods` | 创建会计期间 |

### 应付账款 (`/api/v1/erp/ap`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/invoices` | 获取应付发票列表 |
| POST | `/invoices` | 创建应付发票 |
| GET | `/payments` | 获取付款申请列表 |
| POST | `/payments` | 创建付款申请 |
| GET | `/reconciliations` | 获取对账单列表 |
| POST | `/reconciliations` | 创建对账单 |
| GET | `/verifications` | 获取核销记录 |
| POST | `/verifications` | 创建核销 |

### 应收账款 (`/api/v1/erp/ar`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/invoices` | 获取应收发票列表 |
| POST | `/invoices` | 创建应收发票 |
| GET | `/collections` | 获取收款记录 |
| POST | `/collections` | 创建收款 |
| GET | `/reconciliations` | 获取对账单列表 |
| POST | `/reconciliations` | 创建对账单 |
| GET | `/aging-analysis` | 获取账龄分析 |

### 生产管理 (`/api/v1/erp/production`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/orders` | 获取生产订单列表 |
| POST | `/orders` | 创建生产订单 |
| GET | `/orders/{id}` | 获取订单详情 |
| PUT | `/orders/{id}` | 更新订单 |
| GET | `/boms` | 获取 BOM 列表 |
| POST | `/boms` | 创建 BOM |
| GET | `/mrp/results` | 获取 MRP 运算结果 |
| POST | `/mrp/run` | 执行 MRP 运算 |
| GET | `/scheduling` | 获取排程结果 |
| POST | `/scheduling` | 创建排程 |
| GET | `/capacity` | 获取产能分析 |
| GET | `/material-shortage` | 获取缺料预警 |

### CRM 管理 (`/api/v1/erp/crm`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/leads` | 获取线索列表 |
| POST | `/leads` | 创建线索 |
| GET | `/opportunities` | 获取商机列表 |
| POST | `/opportunities` | 创建商机 |
| GET | `/pool` | 获取公海池客户 |
| POST | `/pool/assign` | 分配客户 |
| GET | `/assignments` | 获取分配记录 |

### BPM 审批 (`/api/v1/erp/bpm`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/definitions` | 获取流程定义列表 |
| POST | `/definitions` | 创建流程定义 |
| GET | `/instances` | 获取流程实例列表 |
| POST | `/instances` | 启动流程实例 |
| GET | `/tasks` | 获取待办任务 |
| POST | `/tasks/{id}/approve` | 审批任务 |
| POST | `/tasks/{id}/reject` | 拒绝任务 |
| POST | `/tasks/{id}/transfer` | 转交任务 |

### 系统管理 (`/api/v1/erp/system`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/departments` | 获取部门列表 |
| POST | `/departments` | 创建部门 |
| GET | `/data-permissions` | 获取数据权限 |
| POST | `/data-permissions` | 设置数据权限 |
| GET | `/field-permissions` | 获取字段权限 |
| POST | `/field-permissions` | 设置字段权限 |
| GET | `/notifications` | 获取通知列表 |
| POST | `/notifications/mark-read` | 标记通知已读 |

### AI 智能分析 (`/api/v1/erp/ai`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/sales-forecast` | 销售预测 |
| GET | `/inventory-optimization` | 库存优化建议 |
| GET | `/anomaly-detection` | 异常检测 |
| GET | `/recommendations` | 智能推荐 |

### 报表引擎 (`/api/v1/erp/reports`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/templates` | 获取报表模板列表 |
| POST | `/templates` | 创建报表模板 |
| POST | `/execute` | 执行报表查询 |
| GET | `/export/{format}` | 导出报表 (PDF/Excel) |

### 多租户管理 (`/api/v1/erp/tenants`)

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/` | 获取租户列表 |
| POST | `/` | 创建租户 |
| GET | `/{id}` | 获取租户详情 |
| PUT | `/{id}` | 更新租户 |
| GET | `/{id}/config` | 获取租户配置 |
| PUT | `/{id}/config` | 更新租户配置 |
| GET | `/{id}/usage` | 获取使用统计 |

## 请求/响应格式

### 标准响应格式

```json
{
  "code": 200,
  "message": "操作成功",
  "data": {
    // 业务数据
  }
}
```

### 分页响应格式

```json
{
  "code": 200,
  "message": "查询成功",
  "data": {
    "items": [
      // 数据列表
    ],
    "total": 100,
    "page": 1,
    "page_size": 20,
    "total_pages": 5
  }
}
```

### 错误响应格式

```json
{
  "code": 400,
  "message": "请求参数错误",
  "errors": [
    {
      "field": "username",
      "message": "用户名不能为空"
    }
  ]
}
```

### 常见 HTTP 状态码

| 状态码 | 描述 |
|--------|------|
| 200 | 请求成功 |
| 201 | 创建成功 |
| 400 | 请求参数错误 |
| 401 | 未认证或 Token 过期 |
| 403 | 无权限访问 |
| 404 | 资源不存在 |
| 409 | 资源冲突（如重复创建） |
| 422 | 请求格式正确但语义错误 |
| 429 | 请求过于频繁（限流） |
| 500 | 服务器内部错误 |

## 速率限制

系统对 API 请求进行速率限制：

- **认证接口**: 10 次/分钟（防暴力破解）
- **普通接口**: 100 次/分钟
- **批量操作**: 10 次/分钟

超限响应：
```json
{
  "code": 429,
  "message": "请求过于频繁，请稍后再试",
  "retry_after": 60
}
```

## gRPC 接口

系统同时提供 gRPC 接口，定义文件位于 `backend/proto/bingxi.proto`。gRPC 服务主要用于：

1. **高性能内部通信**: 微服务间调用
2. **流式数据传输**: 实时数据同步
3. **二进制协议**: 更高效的序列化

gRPC 服务地址：`localhost:50051`（默认）

## WebSocket 接口

系统支持 WebSocket 用于实时通知：

```javascript
// 连接 WebSocket
const ws = new WebSocket('ws://localhost:8080/ws');

// 订阅通知
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'notifications'
}));

// 接收通知
ws.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  console.log('收到通知:', notification);
};
```

## SDK 和客户端

### JavaScript/TypeScript SDK

```typescript
import { BingxiClient } from '@bingxi/sdk';

const client = new BingxiClient({
  baseURL: 'http://localhost:8080/api/v1/erp',
  token: 'your-jwt-token'
});

// 获取销售订单
const orders = await client.sales.getOrders({
  page: 1,
  pageSize: 20,
  status: 'pending'
});

// 创建采购订单
const purchaseOrder = await client.purchases.createOrder({
  supplier_id: 'uuid',
  items: [
    { product_id: 'uuid', quantity: 100, price: 10.5 }
  ]
});
```

### Python SDK

```python
from bingxi import BingxiClient

client = BingxiClient(
    base_url='http://localhost:8080/api/v1/erp',
    token='your-jwt-token'
)

# 获取库存信息
inventory = client.inventory.get_stocks(
    warehouse_id='uuid',
    product_id='uuid'
)

# 创建销售订单
order = client.sales.create_order(
    customer_id='uuid',
    items=[
        {'product_id': 'uuid', 'quantity': 50, 'price': 25.0}
    ]
)
```

## 错误处理

### 常见错误码

| 错误码 | 描述 | 处理建议 |
|--------|------|----------|
| `AUTH_001` | Token 过期 | 使用 Refresh Token 获取新 Token |
| `AUTH_002` | 无效 Token | 重新登录 |
| `AUTH_003` | 权限不足 | 检查用户角色和权限 |
| `BIZ_001` | 业务规则验证失败 | 检查业务逻辑约束 |
| `BIZ_002` | 数据不存在 | 检查资源 ID |
| `BIZ_003` | 数据冲突 | 检查唯一性约束 |
| `SYS_001` | 系统内部错误 | 联系管理员 |
| `SYS_002` | 数据库连接失败 | 检查数据库状态 |
| `SYS_003` | 外部服务调用失败 | 检查第三方服务状态 |

### 错误处理示例

```typescript
try {
  const result = await client.sales.createOrder(orderData);
  console.log('订单创建成功:', result);
} catch (error) {
  if (error.code === 'AUTH_001') {
    // Token 过期，刷新 Token
    await client.refreshToken();
    // 重试请求
    const result = await client.sales.createOrder(orderData);
  } else if (error.code === 'BIZ_001') {
    // 业务规则验证失败
    console.error('业务错误:', error.message);
  } else {
    // 其他错误
    console.error('系统错误:', error);
  }
}
```

## 测试和调试

### Swagger UI

启动后端服务后，访问以下地址查看 API 文档：
- **Swagger UI**: http://localhost:8080/swagger-ui
- **OpenAPI JSON**: http://localhost:8080/api-docs/openapi.json

### Postman 集合

系统提供 Postman 集合文件，位于 `backend/docs/postman_collection.json`，可直接导入使用。

### cURL 示例

```bash
# 登录获取 Token
curl -X POST http://localhost:8080/api/v1/erp/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# 使用 Token 访问 API
curl -X GET http://localhost:8080/api/v1/erp/products \
  -H "Authorization: Bearer <your_token>"

# 创建销售订单
curl -X POST http://localhost:8080/api/v1/erp/sales/orders \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "uuid",
    "items": [
      {"product_id": "uuid", "quantity": 100, "price": 10.5}
    ]
  }'
```

## 版本控制

API 版本通过 URL 路径管理：
- **当前版本**: `/api/v1/erp/`
- **未来版本**: `/api/v2/erp/`（向后兼容）

## 最佳实践

1. **Token 管理**: 定期刷新 Token，避免过期
2. **错误处理**: 实现统一的错误处理机制
3. **请求重试**: 对于网络错误实现指数退避重试
4. **数据缓存**: 合理缓存不常变化的数据
5. **分页查询**: 对于大数据量使用分页
6. **批量操作**: 使用批量接口减少请求次数
7. **日志记录**: 记录 API 调用日志便于调试