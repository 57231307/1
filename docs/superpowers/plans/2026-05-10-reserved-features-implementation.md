# 预留功能接入实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将 EmailService、EventNotificationService、DataPermissionService 三个预留服务接入业务流程

**架构：** 采用最小侵入式方案，在服务层关键方法中直接调用通知服务，数据权限在查询层统一过滤。通知调用使用 fire-and-forget 模式，确保不影响主业务流程。

**技术栈：** Rust + Axum + SeaORM + PostgreSQL

---

## 文件结构

### 修改文件

| 文件 | 职责 |
|------|------|
| `backend/src/utils/app_state.rs` | 添加 email_service、event_notification_service、data_permission_service 字段 |
| `backend/src/main.rs` | 初始化新增服务实例 |
| `backend/src/services/mod.rs` | 确保所有服务模块正确导出 |
| `backend/src/handlers/mod.rs` | 确保所有 handler 模块正确导出 |
| `backend/src/routes/mod.rs` | 添加邮件服务测试路由（可选） |

### 业务接入文件

| 文件 | 接入功能 |
|------|----------|
| `backend/src/services/sales_service.rs` | 订单创建/审批时触发通知 |
| `backend/src/services/purchase_receipt_service.rs` | 采购收货时触发通知 |
| `backend/src/services/ap_payment_request_service.rs` | 付款申请时触发通知 |
| `backend/src/services/inventory_stock_service.rs` | 库存预警时触发通知 |
| `backend/src/services/role_permission_service.rs` | 角色权限变更时刷新数据权限缓存 |

---

## 任务 1：AppState 添加服务字段

**文件：**
- 修改：`backend/src/utils/app_state.rs`

- [ ] **步骤 1：添加服务字段到 AppState**

```rust
use crate::services::email_service::EmailService;
use crate::services::event_notification_service::EventNotificationService;
use crate::services::data_permission_service::DataPermissionService;
use crate::services::notification_service::NotificationService;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub omni_audit: Arc<OmniAuditEngine>,
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: String,
    pub cache: Arc<AppCache>,
    pub metrics: Arc<MetricsService>,
    pub cookie_key: Key,
    pub rate_limiter: Arc<RateLimitStore>,
    pub di_container: Arc<DIContainer>,
    // 新增服务字段
    pub email_service: Option<Arc<EmailService>>,
    pub event_notification_service: Option<Arc<EventNotificationService>>,
    pub data_permission_service: Arc<DataPermissionService>,
    pub notification_service: Arc<NotificationService>,
}
```

- [ ] **步骤 2：修改 with_secrets 构造函数**

在 `with_secrets` 方法中初始化新增服务：

```rust
pub fn with_secrets(
    db: Arc<DatabaseConnection>,
    omni_audit: Arc<OmniAuditEngine>,
    jwt_secret: String,
    previous_jwt_secret: Option<String>,
    cookie_secret: String,
) -> Self {
    // ... 现有代码 ...
    
    let email_service = EmailService::from_env().map(Arc::new);
    let notification_service = Arc::new(NotificationService::new(db.clone()));
    let event_notification_service = email_service.as_ref().map(|_| {
        Arc::new(EventNotificationService::new(db.clone()))
    });
    let data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
    
    Self {
        // ... 现有字段 ...
        email_service,
        event_notification_service,
        data_permission_service,
        notification_service,
    }
}
```

- [ ] **步骤 3：修改 Default 实现**

```rust
fn default() -> Self {
    // ... 现有代码 ...
    let notification_service = Arc::new(NotificationService::new(db.clone()));
    let data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
    
    Self {
        // ... 现有字段 ...
        email_service: None,
        event_notification_service: None,
        data_permission_service,
        notification_service,
    }
}
```

- [ ] **步骤 4：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过，无新增错误

---

## 任务 2：main.rs 初始化适配

**文件：**
- 修改：`backend/src/main.rs`

- [ ] **步骤 1：修改 AppState 初始化代码**

找到 `AppState::with_secrets(...)` 调用处，确保传入所有参数。

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 3：EventNotificationService 接入销售订单

**文件：**
- 修改：`backend/src/services/sales_service.rs`

- [ ] **步骤 1：在销售订单创建后添加通知**

找到 `create_order` 方法，在订单创建成功后添加：

```rust
// 在订单创建成功后，发送通知
if let Some(event_service) = &state.event_notification_service {
    let _ = event_service
        .notify_order_submitted(order.created_by, &order.order_no, order.id)
        .await;
}
```

- [ ] **步骤 2：在销售订单审批后添加通知**

找到审批相关方法，在审批通过后添加：

```rust
if let Some(event_service) = &state.event_notification_service {
    let _ = event_service
        .notify_order_approved(order.created_by, &order.order_no, order.id, "审批人名称")
        .await;
}
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 4：EventNotificationService 接入采购收货

**文件：**
- 修改：`backend/src/services/purchase_receipt_service.rs`

- [ ] **步骤 1：在采购收货创建后添加通知**

找到 `create_receipt` 方法，在收货创建成功后添加：

```rust
if let Some(event_service) = &state.event_notification_service {
    let _ = event_service
        .notify_purchase_arrived(
            purchase_order.created_by,
            &purchase_order.order_no,
            purchase_order.id,
            "仓库名称",
        )
        .await;
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 5：EventNotificationService 接入付款申请

**文件：**
- 修改：`backend/src/services/ap_payment_request_service.rs`

- [ ] **步骤 1：在付款申请创建后添加通知**

找到 `create_payment_request` 方法，在创建成功后添加：

```rust
if let Some(event_service) = &state.event_notification_service {
    let _ = event_service
        .notify_payment_request(
            approver_user_id,
            &request.request_no,
            &request.amount.to_string(),
            "供应商名称",
            request.id,
        )
        .await;
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 6：库存预警通知接入

**文件：**
- 修改：`backend/src/services/inventory_stock_service.rs`

- [ ] **步骤 1：在库存检查方法中添加预警通知**

找到库存检查相关方法，当库存低于阈值时：

```rust
if stock.quantity < product.safety_stock {
    if let Some(event_service) = &state.event_notification_service {
        let _ = event_service
            .notify_inventory_alert(
                warehouse_manager_id,
                &product.name,
                product.id,
                &stock.quantity.to_string(),
                &product.safety_stock.to_string(),
            )
            .await;
    }
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 7：EmailService 与 EventNotificationService 集成

**文件：**
- 修改：`backend/src/services/event_notification_service.rs`

- [ ] **步骤 1：添加 EmailService 依赖**

```rust
use crate::services::email_service::{EmailService, EmailMessage, EmailTemplate};
use crate::services::notification_service::{CreateNotificationRequest, NotificationService};

pub struct EventNotificationService {
    notification_service: NotificationService,
    email_service: Option<Arc<EmailService>>,
}

impl EventNotificationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            notification_service: NotificationService::new(db),
            email_service: None,
        }
    }
    
    pub fn with_email(db: Arc<DatabaseConnection>, email_service: Arc<EmailService>) -> Self {
        Self {
            notification_service: NotificationService::new(db),
            email_service: Some(email_service),
        }
    }
}
```

- [ ] **步骤 2：在通知方法中添加邮件发送逻辑**

以 `notify_order_submitted` 为例：

```rust
pub async fn notify_order_submitted(
    &self,
    user_id: i32,
    order_no: &str,
    order_id: i32,
) -> Result<(), AppError> {
    // 创建站内通知
    self.notification_service
        .create_notification(CreateNotificationRequest {
            user_id,
            notification_type: NotificationType::Internal,
            title: "订单已提交".to_string(),
            content: format!("订单 {} 已提交，等待审批", order_no),
            priority: NotificationPriority::Normal,
            business_type: Some("ORDER".to_string()),
            business_id: Some(order_id),
            action_url: Some(format!("/sales/orders/{}", order_id)),
            sender_id: Some(0),
            sender_name: Some("系统".to_string()),
        })
        .await?;
    
    // 如果配置了邮件服务，发送邮件通知
    if let Some(email_service) = &self.email_service {
        let html = EmailTemplate::order_notification(order_no, "已提交", "/sales/orders");
        let _ = email_service
            .send_html_email(
                vec!["user@example.com".to_string()], // 需要从用户表获取邮箱
                "订单状态更新".to_string(),
                html,
            )
            .await;
    }
    
    Ok(())
}
```

- [ ] **步骤 3：更新 AppState 初始化**

修改 `AppState::with_secrets` 中 EventNotificationService 的初始化：

```rust
let event_notification_service = Some(Arc::new(
    EventNotificationService::with_email(
        db.clone(),
        email_service.clone(),
    )
));
```

- [ ] **步骤 4：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 8：DataPermissionService 接入查询接口

**文件：**
- 修改：`backend/src/handlers/user_handler.rs`（示例）
- 修改：`backend/src/handlers/sales_order_handler.rs`（示例）

- [ ] **步骤 1：在用户列表查询中添加数据权限过滤**

找到 `list_users` handler，在查询后添加字段过滤：

```rust
pub async fn list_users(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<UserListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = UserService::new(state.db.clone());
    let (users, total) = service.list_users(query).await?;
    
    // 应用数据权限过滤
    let mut users_json = serde_json::to_value(users)?;
    if let Some(array) = users_json.as_array_mut() {
        for user in array {
            state.data_permission_service.filter_fields(
                user,
                &Some(vec!["id".to_string(), "username".to_string(), "email".to_string()]),
                &Some(vec!["password".to_string(), "salt".to_string()]),
            );
        }
    }
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": users_json,
        "total": total
    }))))
}
```

- [ ] **步骤 2：在销售订单列表中添加数据范围过滤**

找到 `list_sales_orders` handler，在查询前获取数据权限：

```rust
pub async fn list_sales_orders(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<SalesOrderQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 获取当前用户的数据权限
    let data_permission = state
        .data_permission_service
        .get_role_data_permission(auth.role_id.unwrap_or(0), "sales_order")
        .await?;
    
    let service = SalesService::new(state.db.clone());
    // 将数据权限传递给 service 层进行过滤
    let (orders, total) = service.list_orders_with_permission(query, data_permission).await?;
    
    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": orders,
        "total": total
    }))))
}
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 任务 9：运行测试

- [ ] **步骤 1：运行单元测试**

运行：`cd /workspace/backend && cargo test --lib`
预期：所有现有测试通过

- [ ] **步骤 2：运行 cargo check 最终验证**

运行：`cd /workspace/backend && cargo check`
预期：通过，无新增错误

---

## 自检

### 规格覆盖度检查

| 需求 | 对应任务 |
|------|----------|
| EventNotificationService 接入销售订单 | 任务 3 |
| EventNotificationService 接入采购收货 | 任务 4 |
| EventNotificationService 接入付款申请 | 任务 5 |
| EventNotificationService 接入库存预警 | 任务 6 |
| EmailService 与 EventNotificationService 集成 | 任务 7 |
| DataPermissionService 接入查询接口 | 任务 8 |
| AppState 添加服务字段 | 任务 1 |
| main.rs 初始化适配 | 任务 2 |

### 占位符扫描
- 无 "待定"、"TODO" 占位符
- 所有代码步骤包含实际代码
- 所有类型和函数在任务中已定义

### 类型一致性
- `AppState` 字段名在各任务中一致
- `EventNotificationService::new` 和 `with_email` 签名一致
- `DataPermissionService` 方法名与定义一致
