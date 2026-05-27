# 服务层 (Services)

服务层是冰溪 ERP 后端的核心业务逻辑层，负责实现所有业务功能。服务层接收来自处理器层的请求，执行业务逻辑，调用数据访问层，并返回结果。

## 模块职责

- 实现核心业务逻辑
- 处理业务规则验证
- 管理事务和数据一致性
- 调用外部服务
- 发布领域事件

## 结构

```
services/
├── auth_service.rs           # 认证服务
├── user_service.rs           # 用户服务
├── product_service.rs        # 产品服务
├── sales_order_service.rs    # 销售订单服务
├── purchase_order_service.rs # 采购订单服务
├── inventory_service.rs      # 库存服务
├── voucher_service.rs        # 财务凭证服务
├── customer_service.rs       # 客户服务
├── supplier_service.rs       # 供应商服务
├── bpm_service.rs            # BPM 工作流服务
├── tenant_service.rs         # 多租户服务
├── notification_service.rs   # 通知服务
├── import_export_service.rs  # 导入导出服务
├── report_engine_service.rs  # 报表引擎服务
├── ai_analysis_service.rs    # AI 分析服务
├── event_bus.rs              # 事件总线
├── event_notification_service.rs # 事件通知服务
├── email_service.rs          # 邮件服务
├── mrp_engine_service.rs     # MRP 引擎服务
├── scheduling_service.rs     # 排程服务
├── capacity_service.rs       # 产能服务
├── material_shortage_service.rs # 缺料预警服务
├── five_dimension_service.rs # 五维管理服务
├── dual_unit_converter.rs    # 双单位换算服务
├── transaction_helper.rs     # 事务辅助
└── metrics_service.rs        # 指标服务
```

## 关键文件

| 文件 | 目的 |
|------|------|
| `auth_service.rs` | JWT 令牌生成/验证、登录认证、密码管理 |
| `user_service.rs` | 用户 CRUD、密码重置、角色分配 |
| `sales_order_service.rs` | 销售订单生命周期管理、价格计算、库存预留 |
| `inventory_service.rs` | 库存管理、调拨、盘点、调整、预警 |
| `voucher_service.rs` | 财务凭证管理、借贷平衡、记账处理 |
| `bpm_service.rs` | BPM 工作流引擎、审批流程管理 |
| `event_bus.rs` | 事件总线、异步事件监听/分发 |
| `transaction_helper.rs` | 数据库事务管理、原子操作 |

## 依赖

**本模块依赖**:
- `models/` - 数据模型和实体定义
- `database/` - 数据库连接池
- `utils/` - 工具函数和辅助类
- `middleware/` - 中间件（获取当前用户、权限等）

**依赖本模块的**:
- `handlers/` - HTTP 处理器调用服务层
- `grpc/` - gRPC 服务调用服务层
- `bin/` - CLI 工具调用服务层

## 规范

### 文件命名

- 服务: `[entity]_service.rs`（如 `user_service.rs`）
- 引擎: `[feature]_engine_service.rs`（如 `mrp_engine_service.rs`）
- 辅助: `[feature]_helper.rs`（如 `transaction_helper.rs`）

### 代码模式

**服务类模式**:
```rust
pub struct UserService;

impl UserService {
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<User>, AppError> {
        User::find_by_id(id)
            .one(db)
            .await
            .map_err(AppError::Database)
    }
    
    pub async fn create(
        db: &DatabaseConnection,
        request: CreateUserRequest,
    ) -> Result<User, AppError> {
        // 1. 验证请求
        request.validate()?;
        
        // 2. 检查业务规则
        Self::check_duplicate_username(db, &request.username).await?;
        
        // 3. 创建用户
        let user = User::create(db, request).await?;
        
        // 4. 发布事件
        EventBus::publish(UserCreatedEvent { user_id: user.id });
        
        Ok(user)
    }
}
```

**事务模式**:
```rust
pub async fn create_order_with_items(
    db: &DatabaseConnection,
    request: CreateOrderRequest,
) -> Result<Order, AppError> {
    let txn = db.begin().await?;
    
    // 1. 创建订单
    let order = Order::create(&txn, request.order_data).await?;
    
    // 2. 创建订单项
    for item_data in request.items {
        OrderItem::create(&txn, order.id, item_data).await?;
    }
    
    // 3. 预留库存
    InventoryService::reserve_stock(&txn, order.id, request.items).await?;
    
    // 4. 提交事务
    txn.commit().await?;
    
    Ok(order)
}
```

### 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("用户不存在")]
    UserNotFound,
    
    #[error("用户名已存在")]
    DuplicateUsername,
    
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("验证错误: {0}")]
    Validation(String),
    
    #[error("权限不足")]
    PermissionDenied,
    
    #[error("业务规则违反: {0}")]
    BusinessRuleViolation(String),
}
```

### 日志

```rust
use tracing::{info, warn, error, debug};

// 包含上下文
info!(
    user_id = %user.id,
    username = %user.username,
    "用户创建成功"
);

// 使用适当级别
debug!(query = %sql, "执行数据库查询");
warn!(retry_count = count, "请求重试");
error!(error = %err, user_id = %user_id, "用户创建失败");
```

## 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_find_user_by_id() {
        let db = MockDatabase::new()
            .append_query_results(vec![vec![user_model()]])
            .into_connection();
        
        let result = UserService::find_by_id(&db, Uuid::new_v4()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_validation() {
        let db = MockDatabase::new().into_connection();
        
        let invalid_data = CreateUserRequest {
            email: "invalid-email".to_string(),
            ..Default::default()
        };
        
        let result = UserService::create(&db, invalid_data).await;
        assert!(result.is_err());
    }
}
```

## 添加新服务

### 添加新 [实体] 服务

1. 创建 `services/[entity]_service.rs` 文件
2. 实现基本 CRUD 操作
3. 添加业务逻辑方法
4. 从 `services/mod.rs` 导出
5. 在 `handlers/[entity]_handler.rs` 中使用
6. 添加单元测试

**检查清单**:
- [ ] 遵循命名约定
- [ ] 实现错误处理
- [ ] 添加日志记录
- [ ] 有对应测试文件
- [ ] 从 mod.rs 导出
- [ ] 定义了错误类型