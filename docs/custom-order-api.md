# 定制订单全流程跟踪 - API 文档

> **版本**: v1.0
> **时间**: 2026-06-17
> **基础 URL**: `/api/v1/erp/custom-orders`

---

## 1. 总览

定制订单全流程跟踪模块提供 16 个 REST 端点，覆盖：

- CRUD（5 端点）：list / create / get / update / cancel
- 流程推进（4 端点）：advance / add_node / update_node / advance_node / add_log
- 工艺时间线（1 端点）：timeline
- 质检（3 端点）：report_issue / list_issues / resolve_issue
- 售后（3 端点）：create / list / update

所有端点：
- 需要 JWT 认证（Authorization: Bearer <token>）
- 强制多租户隔离（从 JWT 提取 tenant_id）
- 响应统一包装：`{ code: 0, data: ..., message: "ok" }`

---

## 2. CRUD 接口

### 2.1 列出定制订单

```
GET /api/v1/erp/custom-orders
```

**Query 参数**：
- `page` (optional, default=1)
- `page_size` (optional, default=20)
- `status` (optional)：draft / yarn_purchasing / dyeing / ...
- `customer_id` (optional)
- `keyword` (optional)：订单号模糊匹配

**响应**：
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "order_no": "CO202606170001",
        "customer_id": 1,
        "product_id": 1,
        "spec": "100% 棉 200g/m²",
        "quantity": 100.00,
        "unit": "m",
        "status": "draft",
        "currency": "CNY",
        "created_at": "2026-06-17T10:00:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "page_size": 20
  }
}
```

### 2.2 创建定制订单（草稿）

```
POST /api/v1/erp/custom-orders
```

**Body**：
```json
{
  "customer_id": 1,
  "product_id": 1,
  "color_id": 5,
  "spec": "100% 棉 200g/m² 幅宽 1.5m",
  "quantity": 100.00,
  "unit": "m",
  "custom_requirements": {
    "special_weight": "220g/m²",
    "width": "1.6m",
    "color_fastness": "level_4"
  },
  "yarn_spec": "32S 精梳",
  "dye_method": "reactive",
  "finishing_method": "softening",
  "expected_delivery_date": "2026-07-15",
  "total_amount": 5000.00,
  "currency": "CNY",
  "sales_order_id": null
}
```

**响应**：
```json
{
  "code": 0,
  "data": { "id": 1, "order_no": "CO202606170001", "status": "draft", ... }
}
```

**副作用**：
- 自动生成 5 阶段工艺节点（yarn_purchasing / dyeing / finishing / delivery / after_sales）

### 2.3 获取订单详情

```
GET /api/v1/erp/custom-orders/:id
```

**响应**（包含节点/异常/售后）：
```json
{
  "code": 0,
  "data": {
    "id": 1,
    "order_no": "CO202606170001",
    "status": "yarn_purchasing",
    "process_nodes": [
      { "id": 1, "node_type": "yarn_purchasing", "status": "in_progress", "sequence": 1 }
    ],
    "quality_issues": [],
    "after_sales": []
  }
}
```

### 2.4 更新定制订单（仅草稿）

```
PUT /api/v1/erp/custom-orders/:id
```

**Body**：同 create 字段（均为可选）

**错误**：
- 404 Not Found
- 409 Conflict：当前状态不允许修改（非 draft）

### 2.5 取消定制订单

```
DELETE /api/v1/erp/custom-orders/:id
```

**Body**：
```json
{ "reason": "客户主动取消" }
```

---

## 3. 流程推进接口

### 3.1 推进到下一阶段

```
POST /api/v1/erp/custom-orders/:id/advance
```

**Body**：
```json
{
  "operator_id": 1,
  "notes": "纱线已采购完成，进入染整"
}
```

**行为**：
- 状态机推进（draft → yarn_purchasing → ...）
- 当前节点标记为 completed
- 下一节点标记为 in_progress
- 记录工艺日志

### 3.2 添加工艺节点

```
POST /api/v1/erp/custom-orders/:id/nodes
```

**Body**：
```json
{
  "node_type": "custom_step",
  "node_name": "特殊印花",
  "sequence": 3,
  "planned_start_date": "2026-07-01T00:00:00Z",
  "planned_end_date": "2026-07-05T00:00:00Z"
}
```

### 3.3 更新工艺节点

```
PUT /api/v1/erp/custom-orders/:id/nodes/:nid
```

**Body**：
```json
{
  "status": "blocked",
  "notes": "原料供应延迟"
}
```

### 3.4 推进工艺节点

```
POST /api/v1/erp/custom-orders/:id/nodes/:nid/advance
```

**Body**：
```json
{
  "action": "start",   // start / pause / resume / complete / block / unblock
  "operator_id": 1,
  "notes": "开始纱线采购",
  "attachments": ["https://example.com/receipt.pdf"]
}
```

### 3.5 添加节点日志

```
POST /api/v1/erp/custom-orders/:id/nodes/:nid/logs
```

### 3.6 获取工艺时间线

```
GET /api/v1/erp/custom-orders/:id/timeline
```

**响应**：
```json
{
  "code": 0,
  "data": {
    "order_id": 1,
    "order_no": "CO202606170001",
    "current_status": "dyeing",
    "nodes": [
      {
        "id": 1,
        "node_name": "纱线采购",
        "status": "completed",
        "logs": [
          {
            "id": 1,
            "action": "complete",
            "before_status": "in_progress",
            "after_status": "completed",
            "log_time": "2026-06-17T11:00:00Z"
          }
        ]
      }
    ]
  }
}
```

---

## 4. 质检接口

### 4.1 上报质量异常

```
POST /api/v1/erp/custom-orders/:id/issues
```

**Body**：
```json
{
  "custom_order_id": 1,
  "process_node_id": 2,
  "issue_type": "color_diff",
  "severity": "high",
  "description": "批次色差 ΔE=4.2 超过阈值",
  "color_delta_e": 4.2
}
```

**行业规则**：
- 异常类型 color_diff 必须填色差 ΔE
- 异常类型 color_fastness 必须填等级 1-5
- severity 必须是 low/medium/high/critical 之一

### 4.2 列出订单异常

```
GET /api/v1/erp/custom-orders/:id/issues?page=1&page_size=20
```

### 4.3 解决异常

```
PUT /api/v1/erp/custom-orders/issues/:id/resolve
```

**Body**：
```json
{
  "resolution": "调整染整工艺参数，重新生产",
  "operator_id": 1
}
```

---

## 5. 售后接口

### 5.1 创建售后工单

```
POST /api/v1/erp/custom-orders/:id/after-sales
```

**Body**：
```json
{
  "custom_order_id": 1,
  "customer_id": 1,
  "issue_type": "refund",   // complaint / repair / exchange / refund
  "description": "客户对染色效果不满意",
  "refund_amount": 2500.00
}
```

**业务规则**：
- issue_type = "refund" 时 refund_amount 必填
- issue_type 必须是 4 种之一

### 5.2 列出售后工单

```
GET /api/v1/erp/custom-orders/:id/after-sales?page=1&page_size=20
```

### 5.3 更新售后工单

```
PUT /api/v1/erp/custom-orders/after-sales/:id
```

**Body**：
```json
{
  "status": "resolved",
  "resolution": "已退款 2500 元"
}
```

**状态转换**：
- opened → processing / rejected / closed
- processing → resolved / closed / rejected
- resolved → closed

---

## 6. 错误码

| 状态码 | 含义 | 说明 |
|--------|------|------|
| 200 | OK | 成功 |
| 400 | Bad Request | 参数校验失败 |
| 401 | Unauthorized | 未认证或 token 失效 |
| 403 | Forbidden | 跨租户访问 |
| 404 | Not Found | 资源不存在 |
| 409 | Conflict | 状态机不允许当前操作 |
| 500 | Internal Server Error | 系统错误 |

---

## 7. 多租户隔离

- 所有 API 自动从 JWT 提取 `tenant_id`
- 所有数据库查询自动追加 `WHERE tenant_id = ?` 条件
- 严禁客户端通过 URL/Body 传入 tenant_id
- 跨租户访问统一返回 403
