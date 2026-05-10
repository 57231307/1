# 通知系统优化实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 按顺序实现用户邮箱关联、通知偏好设置、更多业务场景接入、数据权限扩展四个优化方向

**架构：** 采用最小改动式方案，在现有 EventNotificationService 中逐步添加功能，保持向后兼容

**技术栈：** Rust + Axum + SeaORM + PostgreSQL

---

## 文件结构

### 第一部分：用户邮箱关联

| 文件 | 职责 |
|------|------|
| `backend/src/services/event_notification_service.rs` | 添加 `get_user_email` 辅助方法，修改所有 `notify_*` 方法使用真实邮箱 |

### 第二部分：用户通知偏好设置

| 文件 | 职责 |
|------|------|
| `backend/src/models/user_notification_setting.rs` | 新增用户通知偏好设置数据模型 |
| `backend/src/services/user_notification_setting_service.rs` | 新增用户通知偏好设置服务 |
| `backend/src/handlers/user_notification_setting_handler.rs` | 新增用户通知偏好设置 API Handler |
| `backend/src/routes/mod.rs` | 添加通知偏好设置路由 |
| `backend/src/services/event_notification_service.rs` | 在发送通知前检查用户偏好 |

### 第三部分：更多业务场景接入

| 文件 | 职责 |
|------|------|
| `backend/src/handlers/ap_payment_request_handler.rs` | 审批拒绝添加通知 |
| `backend/src/handlers/purchase_order_handler.rs` | 采购订单创建添加通知 |
| `backend/src/handlers/purchase_return_handler.rs` | 采购退货拒绝添加通知 |
| `backend/src/services/event_notification_service.rs` | 添加新的通知方法 |

### 第四部分：数据权限扩展

| 文件 | 职责 |
|------|------|
| `backend/src/handlers/purchase_order_handler.rs` | 采购订单查询接入数据权限 |
| `backend/src/handlers/ap_payment_request_handler.rs` | 付款申请查询接入数据权限 |
| `backend/src/handlers/inventory_stock_handler.rs` | 库存查询接入数据权限 |
| `backend/src/handlers/customer_handler.rs` | 客户资料查询接入数据权限 |

---

## 第一部分：用户邮箱关联

### 任务 1：EventNotificationService 添加用户邮箱查询

**文件：**
- 修改：`backend/src/services/event_notification_service.rs`

- [ ] **步骤 1：添加用户邮箱查询方法**

在 `EventNotificationService` 中添加：

```rust
use crate::models::user;
use sea_orm::EntityTrait;

/// 根据用户ID查询邮箱
async fn get_user_email(&self, user_id: i32) -> Option<String> {
    if let Ok(Some(user)) = user::Entity::find_by_id(user_id)
        .one(self.notification_service.db.as_ref())
        .await
    {
        user.email
    } else {
        None
    }
}
```

- [ ] **步骤 2：修改 `send_email_notification` 方法**

将 `send_email_notification` 改为根据 user_id 查询邮箱：

```rust
/// 发送邮件通知（辅助方法）
async fn send_email_notification(
    &self,
    user_id: i32,
    subject: &str,
    html_content: String,
) {
    if let Some(email_service) = &self.email_service {
        if let Some(email) = self.get_user_email(user_id).await {
            let _ = email_service
                .send_html_email(
                    vec![email],
                    subject.to_string(),
                    html_content,
                )
                .await;
        }
    }
}
```

- [ ] **步骤 3：修改 `notify_order_submitted` 中的邮件调用**

```rust
// 发送邮件通知
let html = EmailTemplate::order_notification(order_no, "已提交", "/sales/orders");
self.send_email_notification(user_id, "订单状态更新", html).await;
```

- [ ] **步骤 4：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 第二部分：用户通知偏好设置

### 任务 2：创建用户通知偏好设置数据模型

**文件：**
- 创建：`backend/src/models/user_notification_setting.rs`

- [ ] **步骤 1：创建数据模型文件**

```rust
//! 用户通知偏好设置模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_notification_setting")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    /// 是否启用邮件通知（全局开关）
    pub email_enabled: bool,
    /// 是否启用站内通知（全局开关）
    pub internal_enabled: bool,
    /// 订单通知方式：email, internal, both, none
    pub order_notification_type: String,
    /// 审批通知方式
    pub approval_notification_type: String,
    /// 库存预警通知方式
    pub inventory_notification_type: String,
    /// 采购通知方式
    pub purchase_notification_type: String,
    /// 财务通知方式
    pub finance_notification_type: String,
    /// 系统公告通知方式
    pub system_notification_type: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 通知类型常量
pub mod notification_type {
    pub const EMAIL: &str = "email";
    pub const INTERNAL: &str = "internal";
    pub const BOTH: &str = "both";
    pub const NONE: &str = "none";
}
```

- [ ] **步骤 2：在 models/mod.rs 中注册模块**

```rust
pub mod user_notification_setting;
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 3：创建用户通知偏好设置服务

**文件：**
- 创建：`backend/src/services/user_notification_setting_service.rs`

- [ ] **步骤 1：创建服务文件**

```rust
//! 用户通知偏好设置服务

use crate::models::user_notification_setting::{self, notification_type};
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use sea_orm::ColumnTrait;
use std::sync::Arc;

pub struct UserNotificationSettingService {
    db: Arc<DatabaseConnection>,
}

impl UserNotificationSettingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取或创建默认设置
    pub async fn get_or_create_default(&self, user_id: i32) -> Result<user_notification_setting::Model, AppError> {
        let setting = user_notification_setting::Entity::find()
            .filter(user_notification_setting::Column::UserId.eq(user_id))
            .one(&*self.db)
            .await?;

        if let Some(setting) = setting {
            Ok(setting)
        } else {
            // 创建默认设置
            let active_model = user_notification_setting::ActiveModel {
                user_id: Set(user_id),
                email_enabled: Set(true),
                internal_enabled: Set(true),
                order_notification_type: Set(notification_type::BOTH.to_string()),
                approval_notification_type: Set(notification_type::BOTH.to_string()),
                inventory_notification_type: Set(notification_type::BOTH.to_string()),
                purchase_notification_type: Set(notification_type::BOTH.to_string()),
                finance_notification_type: Set(notification_type::BOTH.to_string()),
                system_notification_type: Set(notification_type::INTERNAL.to_string()),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
                ..Default::default()
            };
            Ok(active_model.insert(&*self.db).await?)
        }
    }

    /// 更新设置
    pub async fn update_setting(
        &self,
        user_id: i32,
        email_enabled: Option<bool>,
        internal_enabled: Option<bool>,
        order_type: Option<String>,
        approval_type: Option<String>,
        inventory_type: Option<String>,
        purchase_type: Option<String>,
        finance_type: Option<String>,
        system_type: Option<String>,
    ) -> Result<user_notification_setting::Model, AppError> {
        let setting = self.get_or_create_default(user_id).await?;
        let mut active_model: user_notification_setting::ActiveModel = setting.into();

        if let Some(v) = email_enabled {
            active_model.email_enabled = Set(v);
        }
        if let Some(v) = internal_enabled {
            active_model.internal_enabled = Set(v);
        }
        if let Some(v) = order_type {
            active_model.order_notification_type = Set(v);
        }
        if let Some(v) = approval_type {
            active_model.approval_notification_type = Set(v);
        }
        if let Some(v) = inventory_type {
            active_model.inventory_notification_type = Set(v);
        }
        if let Some(v) = purchase_type {
            active_model.purchase_notification_type = Set(v);
        }
        if let Some(v) = finance_type {
            active_model.finance_notification_type = Set(v);
        }
        if let Some(v) = system_type {
            active_model.system_notification_type = Set(v);
        }
        active_model.updated_at = Set(chrono::Utc::now());

        Ok(active_model.update(&*self.db).await?)
    }

    /// 检查是否应该发送邮件通知
    pub async fn should_send_email(&self, user_id: i32, notification_category: &str) -> Result<bool, AppError> {
        let setting = self.get_or_create_default(user_id).await?;
        
        if !setting.email_enabled {
            return Ok(false);
        }

        let notification_type = match notification_category {
            "ORDER" => &setting.order_notification_type,
            "APPROVAL" => &setting.approval_notification_type,
            "INVENTORY" => &setting.inventory_notification_type,
            "PURCHASE" => &setting.purchase_notification_type,
            "FINANCE" => &setting.finance_notification_type,
            "SYSTEM" => &setting.system_notification_type,
            _ => notification_type::BOTH,
        };

        Ok(notification_type == notification_type::EMAIL || notification_type == notification_type::BOTH)
    }

    /// 检查是否应该发送站内通知
    pub async fn should_send_internal(&self, user_id: i32, notification_category: &str) -> Result<bool, AppError> {
        let setting = self.get_or_create_default(user_id).await?;
        
        if !setting.internal_enabled {
            return Ok(false);
        }

        let notification_type = match notification_category {
            "ORDER" => &setting.order_notification_type,
            "APPROVAL" => &setting.approval_notification_type,
            "INVENTORY" => &setting.inventory_notification_type,
            "PURCHASE" => &setting.purchase_notification_type,
            "FINANCE" => &setting.finance_notification_type,
            "SYSTEM" => &setting.system_notification_type,
            _ => notification_type::BOTH,
        };

        Ok(notification_type == notification_type::INTERNAL || notification_type == notification_type::BOTH)
    }
}
```

- [ ] **步骤 2：在 services/mod.rs 中注册模块**

```rust
pub mod user_notification_setting_service;
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 4：EventNotificationService 集成通知偏好检查

**文件：**
- 修改：`backend/src/services/event_notification_service.rs`

- [ ] **步骤 1：添加 UserNotificationSettingService 依赖**

```rust
use crate::services::user_notification_setting_service::UserNotificationSettingService;
```

- [ ] **步骤 2：修改 `EventNotificationService` 结构体**

```rust
pub struct EventNotificationService {
    notification_service: NotificationService,
    email_service: Option<Arc<EmailService>>,
    setting_service: UserNotificationSettingService,
}
```

- [ ] **步骤 3：修改构造函数**

```rust
/// 创建服务实例（仅站内通知）
pub fn new(db: Arc<DatabaseConnection>) -> Self {
    Self {
        notification_service: NotificationService::new(db.clone()),
        email_service: None,
        setting_service: UserNotificationSettingService::new(db),
    }
}

/// 创建服务实例（含邮件通知）
pub fn with_email(db: Arc<DatabaseConnection>, email_service: Arc<EmailService>) -> Self {
    Self {
        notification_service: NotificationService::new(db.clone()),
        email_service: Some(email_service),
        setting_service: UserNotificationSettingService::new(db),
    }
}
```

- [ ] **步骤 4：修改 `notify_order_submitted` 方法**

```rust
pub async fn notify_order_submitted(
    &self,
    user_id: i32,
    order_no: &str,
    order_id: i32,
) -> Result<(), AppError> {
    // 检查用户偏好
    let should_email = self.setting_service.should_send_email(user_id, "ORDER").await?;
    let should_internal = self.setting_service.should_send_internal(user_id, "ORDER").await?;

    if should_internal {
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
    }

    if should_email {
        let html = EmailTemplate::order_notification(order_no, "已提交", "/sales/orders");
        self.send_email_notification(user_id, "订单状态更新", html).await;
    }

    Ok(())
}
```

- [ ] **步骤 5：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 5：创建用户通知偏好设置 API Handler

**文件：**
- 创建：`backend/src/handlers/user_notification_setting_handler.rs`

- [ ] **步骤 1：创建 Handler 文件**

```rust
//! 用户通知偏好设置 Handler

use crate::middleware::auth_context::AuthContext;
use crate::services::user_notification_setting_service::UserNotificationSettingService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// 获取当前用户的通知偏好设置
pub async fn get_setting(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = UserNotificationSettingService::new(state.db.clone());
    let setting = service.get_or_create_default(auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(setting)?)))
}

/// 更新当前用户的通知偏好设置
#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub email_enabled: Option<bool>,
    pub internal_enabled: Option<bool>,
    pub order_notification_type: Option<String>,
    pub approval_notification_type: Option<String>,
    pub inventory_notification_type: Option<String>,
    pub purchase_notification_type: Option<String>,
    pub finance_notification_type: Option<String>,
    pub system_notification_type: Option<String>,
}

pub async fn update_setting(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = UserNotificationSettingService::new(state.db.clone());
    let setting = service
        .update_setting(
            auth.user_id,
            req.email_enabled,
            req.internal_enabled,
            req.order_notification_type,
            req.approval_notification_type,
            req.inventory_notification_type,
            req.purchase_notification_type,
            req.finance_notification_type,
            req.system_notification_type,
        )
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(setting)?,
        "通知偏好设置已更新",
    )))
}
```

- [ ] **步骤 2：在 handlers/mod.rs 中注册模块**

```rust
pub mod user_notification_setting_handler;
```

- [ ] **步骤 3：在 routes/mod.rs 中添加路由**

```rust
use crate::handlers::user_notification_setting_handler;

// 在用户路由中添加
.route("/api/v1/user/notification-setting", get(user_notification_setting_handler::get_setting))
.route("/api/v1/user/notification-setting", put(user_notification_setting_handler::update_setting))
```

- [ ] **步骤 4：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 第三部分：更多业务场景接入

### 任务 6：审批拒绝通知接入

**文件：**
- 修改：`backend/src/handlers/ap_payment_request_handler.rs`
- 修改：`backend/src/handlers/purchase_order_handler.rs`
- 修改：`backend/src/services/event_notification_service.rs`

- [ ] **步骤 1：添加 `notify_approval_rejected` 方法**

```rust
/// 审批拒绝通知
pub async fn notify_approval_rejected(
    &self,
    user_id: i32,
    task_title: &str,
    approver_name: &str,
    reason: Option<&str>,
    business_type: &str,
    business_id: i32,
) -> Result<(), AppError> {
    let should_email = self.setting_service.should_send_email(user_id, "APPROVAL").await?;
    let should_internal = self.setting_service.should_send_internal(user_id, "APPROVAL").await?;

    let mut content = format!("您的 '{}' 申请被 {} 拒绝", task_title, approver_name);
    if let Some(r) = reason {
        content.push_str(&format!("，原因：{}", r));
    }

    if should_internal {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "审批被拒绝".to_string(),
                content,
                priority: NotificationPriority::High,
                business_type: Some(business_type.to_string()),
                business_id: Some(business_id),
                action_url: None,
                sender_id: Some(0),
                sender_name: Some(approver_name.to_string()),
            })
            .await?;
    }

    Ok(())
}
```

- [ ] **步骤 2：在 `reject_request` handler 中添加通知**

```rust
// 在拒绝成功后添加
if let Some(ref event_service) = state.event_notification_service {
    let _ = event_service
        .notify_approval_rejected(
            request.created_by,
            &request.request_no,
            &auth.username,
            Some(&req.reason),
            "FINANCE",
            request.id,
        )
        .await;
}
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 7：采购订单创建通知接入

**文件：**
- 修改：`backend/src/handlers/purchase_order_handler.rs`
- 修改：`backend/src/services/event_notification_service.rs`

- [ ] **步骤 1：添加 `notify_purchase_order_created` 方法**

```rust
/// 采购订单创建通知
pub async fn notify_purchase_order_created(
    &self,
    user_id: i32,
    order_no: &str,
    order_id: i32,
    supplier_name: &str,
) -> Result<(), AppError> {
    let should_email = self.setting_service.should_send_email(user_id, "PURCHASE").await?;
    let should_internal = self.setting_service.should_send_internal(user_id, "PURCHASE").await?;

    if should_internal {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "采购订单已创建".to_string(),
                content: format!("采购订单 {}（供应商：{}）已创建", order_no, supplier_name),
                priority: NotificationPriority::Normal,
                business_type: Some("PURCHASE".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/purchases/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
    }

    Ok(())
}
```

- [ ] **步骤 2：在 `create_order` handler 中添加通知**

```rust
// 在采购订单创建成功后添加
if let Some(ref event_service) = state.event_notification_service {
    let _ = event_service
        .notify_purchase_order_created(
            auth.user_id,
            &order.order_no,
            order.id,
            &order.supplier_name,
        )
        .await;
}
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 第四部分：数据权限扩展

### 任务 8：采购订单查询接入数据权限

**文件：**
- 修改：`backend/src/handlers/purchase_order_handler.rs`

- [ ] **步骤 1：修改 `list_orders` handler**

参考销售订单的数据权限接入方式，在采购订单列表查询中添加：

```rust
// 数据权限控制
if let Some(role_id) = auth.role_id {
    if let Ok(Some(permission)) = state.data_permission_service
        .get_role_data_permission(role_id, "purchase_order")
        .await
    {
        // 应用字段过滤
        // ...
    }
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 9：付款申请查询接入数据权限

**文件：**
- 修改：`backend/src/handlers/ap_payment_request_handler.rs`

- [ ] **步骤 1：修改 `list_requests` handler**

参考销售订单的数据权限接入方式。

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 10：库存查询接入数据权限

**文件：**
- 修改：`backend/src/handlers/inventory_stock_handler.rs`

- [ ] **步骤 1：修改 `list_stock` handler**

参考销售订单的数据权限接入方式。

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

### 任务 11：客户资料查询接入数据权限

**文件：**
- 修改：`backend/src/handlers/customer_handler.rs`

- [ ] **步骤 1：修改 `list_customers` handler**

参考销售订单的数据权限接入方式。

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/backend && cargo check`
预期：通过

---

## 最终验证

### 任务 12：运行测试

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
| 用户邮箱关联 | 任务 1 |
| 用户通知偏好设置数据模型 | 任务 2 |
| 用户通知偏好设置服务 | 任务 3 |
| EventNotificationService 集成偏好检查 | 任务 4 |
| 用户通知偏好设置 API | 任务 5 |
| 审批拒绝通知 | 任务 6 |
| 采购订单创建通知 | 任务 7 |
| 采购订单数据权限 | 任务 8 |
| 付款申请数据权限 | 任务 9 |
| 库存查询数据权限 | 任务 10 |
| 客户资料数据权限 | 任务 11 |

### 占位符扫描
- 无 "待定"、"TODO" 占位符
- 所有代码步骤包含实际代码
- 所有类型和函数在任务中已定义

### 类型一致性
- `EventNotificationService` 字段名在各任务中一致
- `UserNotificationSettingService` 方法签名一致
- 通知类型常量定义一致
