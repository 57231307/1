# 处理器层 (Handlers)

处理器层是冰溪 ERP 后端的 HTTP 接口层，负责接收 HTTP 请求、验证参数、调用服务层，并返回 HTTP 响应。处理器层是客户端与后端交互的入口。

## 模块职责

- 接收 HTTP 请求
- 验证请求参数
- 调用服务层处理业务逻辑
- 格式化 HTTP 响应
- 处理错误和异常

## 结构

```
handlers/
├── auth_handler.rs           # 认证处理器
├── user_handler.rs           # 用户处理器
├── product_handler.rs        # 产品处理器
├── sales_order_handler.rs    # 销售订单处理器
├── purchase_order_handler.rs # 采购订单处理器
├── inventory_handler.rs      # 库存处理器
├── voucher_handler.rs        # 财务凭证处理器
├── customer_handler.rs       # 客户处理器
├── supplier_handler.rs       # 供应商处理器
├── bpm_handler.rs            # BPM 处理器
├── tenant_handler.rs         # 多租户处理器
├── notification_handler.rs   # 通知处理器
├── import_export_handler.rs  # 导入导出处理器
├── report_engine_handler.rs  # 报表引擎处理器
├── ai_analysis_handler.rs    # AI 分析处理器
├── health_handler.rs         # 健康检查处理器
├── init_handler.rs           # 系统初始化处理器
├── barcode_scanner_handler.rs # 条码扫描处理器
├── piece_split_handler.rs    # 匹号拆分处理器
├── print_handler.rs          # 打印处理器
├── tracking_handler.rs       # 追踪处理器
├── business_trace_handler.rs # 业务追溯处理器
├── five_dimension_handler.rs # 五维管理处理器
├── dual_unit_handler.rs      # 双单位处理器
├── data_permission_handler.rs # 数据权限处理器
├── field_permission_handler.rs # 字段权限处理器
├── omni_audit_handler.rs     # 全链路审计处理器
├── operation_log_handler.rs  # 操作日志处理器
├── email_handler.rs          # 邮件处理器
├── webhook_handler.rs        # Webhook 处理器
├── currency_handler.rs       # 币种处理器
├── dashboard_handler.rs      # 仪表盘处理器
├── crm_handler.rs            # CRM 处理器
├── quality_handler.rs        # 质量处理器
├── cost_handler.rs           # 成本处理器
├── budget_handler.rs         # 预算处理器
├── fund_handler.rs           # 资金处理器
├── fixed_asset_handler.rs    # 固定资产处理器
├── ap_handler.rs             # 应付处理器
├── ar_handler.rs             # 应收处理器
├── production_handler.rs     # 生产处理器
├── bom_handler.rs            # BOM 处理器
├── mrp_handler.rs            # MRP 处理器
├── scheduling_handler.rs     # 排程处理器
├── capacity_handler.rs       # 产能处理器
├── material_shortage_handler.rs # 缺料预警处理器
├── trading_handler.rs        # 交易处理器
├── advanced_handler.rs       # 高级功能处理器
├── financial_analysis_handler.rs # 财务分析处理器
├── sales_analysis_handler.rs # 销售分析处理器
├── sales_price_handler.rs    # 销售价格处理器
├── purchase_price_handler.rs # 采购价格处理器
├── sales_return_handler.rs   # 销售退货处理器
├── purchase_return_handler.rs # 采购退货处理器
├── purchase_receipt_handler.rs # 采购收货处理器
├── purchase_inspection_handler.rs # 采购检验处理器
├── inventory_transfer_handler.rs # 库存调拨处理器
├── inventory_count_handler.rs # 库存盘点处理器
├── inventory_adjustment_handler.rs # 库存调整处理器
├── inventory_reservation_handler.rs # 库存预留处理器
├── warehouse_handler.rs      # 仓库处理器
├── department_handler.rs     # 部门处理器
├── role_handler.rs           # 角色处理器
├── permission_handler.rs     # 权限处理器
├── audit_handler.rs          # 审计处理器
├── security_handler.rs       # 安全处理器
├── system_update_handler.rs  # 系统更新处理器
├── tenant_config_handler.rs  # 租户配置处理器
├── tenant_billing_handler.rs # 租户计费处理器
├── tenant_usage_handler.rs   # 租户使用处理器
├── customer_credit_handler.rs # 客户信用处理器
├── supplier_evaluation_handler.rs # 供应商评估处理器
├── greige_fabric_handler.rs  # 坯布处理器
├── dye_batch_handler.rs      # 缸号处理器
├── dye_recipe_handler.rs     # 染色配方处理器
├── account_subject_handler.rs # 会计科目处理器
├── accounting_period_handler.rs # 会计期间处理器
├── assist_accounting_handler.rs # 辅助核算处理器
├── finance_report_handler.rs # 财务报表处理器
└── template_handler.rs       # 模板处理器
```

## 关键文件

| 文件 | 目的 |
|------|------|
| `auth_handler.rs` | 登录/登出/刷新令牌/CSRF 令牌/TOTP 设置 |
| `user_handler.rs` | 用户 CRUD、密码管理、角色分配 |
| `sales_order_handler.rs` | 销售订单生命周期管理 |
| `inventory_handler.rs` | 库存管理、调拨、盘点、调整 |
| `health_handler.rs` | 健康检查（health/readiness/liveness） |
| `init_handler.rs` | 系统初始化、数据库测试连接 |

## 依赖

**本模块依赖**:
- `services/` - 业务逻辑层
- `models/` - 数据模型和 DTO
- `middleware/` - 中间件（获取当前用户、权限等）
- `utils/` - 工具函数和响应格式化

**依赖本模块的**:
- `routes/` - 路由配置调用处理器
- 客户端应用 - 通过 HTTP 调用处理器

## 规范

### 文件命名

- 处理器: `[entity]_handler.rs`（如 `user_handler.rs`）
- 处理器组: `[module]_handler.rs`（如 `inventory_handler.rs`）

### 代码模式

**处理器函数模式**:
```rust
use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::services::UserService;
use crate::utils::response::ApiResponse;

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    // 1. 调用服务层
    let user = UserService::find_by_id(&state.db, id).await?;
    
    // 2. 处理结果
    match user {
        Some(user) => Ok(Json(ApiResponse::success(user.into()))),
        None => Err(AppError::UserNotFound),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    // 1. 验证请求
    request.validate()?;
    
    // 2. 调用服务层
    let user = UserService::create(&state.db, request).await?;
    
    // 3. 返回响应
    Ok(Json(ApiResponse::created(user.into())))
}
```

**分页查询模式**:
```rust
pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<UserResponse>>>, AppError> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    
    let (users, total) = UserService::paginate(
        &state.db,
        page,
        page_size,
        params.filters,
    ).await?;
    
    Ok(Json(ApiResponse::success(PaginatedResponse {
        items: users.into_iter().map(|u| u.into()).collect(),
        total,
        page,
        page_size,
        total_pages: (total as f64 / page_size as f64).ceil() as u64,
    })))
}
```

### 错误处理

```rust
use axum::http::StatusCode;
use crate::utils::response::ErrorResponse;

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_code, message) = match self {
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "USER_NOT_FOUND",
                "用户不存在",
            ),
            AppError::DuplicateUsername => (
                StatusCode::CONFLICT,
                "DUPLICATE_USERNAME",
                "用户名已存在",
            ),
            AppError::Validation(errors) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                "请求参数错误",
            ),
            AppError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "PERMISSION_DENIED",
                "权限不足",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "服务器内部错误",
            ),
        };
        
        let body = ErrorResponse {
            code: status.as_u16(),
            error_code: error_code.to_string(),
            message: message.to_string(),
            details: None,
        };
        
        (status, Json(body)).into_response()
    }
}
```

### 日志

```rust
use tracing::{info, warn, error};

// 请求日志
info!(
    method = %method,
    path = %path,
    user_id = %user_id,
    "处理请求"
);

// 响应日志
info!(
    status = %status,
    duration_ms = %duration,
    "请求完成"
);

// 错误日志
error!(
    error = %err,
    user_id = %user_id,
    "请求处理失败"
);
```

## 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_user() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/erp/users/123")
                    .header("Authorization", "Bearer test-token")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_user_validation() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/erp/users")
                    .header("Authorization", "Bearer test-token")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&CreateUserRequest {
                            email: "invalid-email".to_string(),
                            ..Default::default()
                        }).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
```

## 添加新处理器

### 添加新 [实体] 处理器

1. 创建 `handlers/[entity]_handler.rs` 文件
2. 实现基本 CRUD 处理函数
3. 添加业务逻辑处理函数
4. 从 `handlers/mod.rs` 导出
5. 在 `routes/mod.rs` 中注册路由
6. 添加单元测试

**检查清单**:
- [ ] 遵循命名约定
- [ ] 实现错误处理
- [ ] 添加日志记录
- [ ] 有对应测试文件
- [ ] 从 mod.rs 导出
- [ ] 注册路由
- [ ] 添加 API 文档注释

## 处理器文件清单(自动维护)

<!-- AUTO-GENERATED-START: handlers -->
- `backend/src/handlers/health_handler.rs`
- `backend/src/handlers/bpm_handler.rs`
- `backend/src/handlers/ar_report_handler.rs`
- `backend/src/handlers/scheduling_handler.rs`
- `backend/src/handlers/ar_verification_handler.rs`
- `backend/src/handlers/print_handler.rs`
- `backend/src/handlers/ai_analysis_handler.rs`
- `backend/src/handlers/purchase_contract_handler.rs`
- `backend/src/handlers/finance_payment_handler.rs`
- `backend/src/handlers/currency_enhanced_handler.rs`
- `backend/src/handlers/account_subject_handler.rs`
- `backend/src/handlers/user_handler.rs`
- `backend/src/handlers/bpm_definition_handler.rs`
- `backend/src/handlers/ap_invoice_handler.rs`
- `backend/src/handlers/auth_handler.rs`
- `backend/src/handlers/dye_recipe_handler.rs`
- `backend/src/handlers/ar_reconciliation_enhanced_handler.rs`
- `backend/src/handlers/sales_fabric_order_handler.rs`
- `backend/src/handlers/notification_handler.rs`
- `backend/src/handlers/login_security_handler.rs`
- `backend/src/handlers/dye_batch_handler.rs`
- `backend/src/handlers/report_engine_handler.rs`
- `backend/src/handlers/ap_verification_handler.rs`
- `backend/src/handlers/crm_customer_handler.rs`
- `backend/src/handlers/import_export_handler.rs`
- `backend/src/handlers/missing_handlers.rs`
- `backend/src/handlers/sales_contract_handler.rs`
- `backend/src/handlers/api_key_handler.rs`
- `backend/src/handlers/inventory_batch_handler.rs`
- `backend/src/handlers/department_handler.rs`
- `backend/src/handlers/purchase_return_handler.rs`
- `backend/src/handlers/ap_payment_request_handler.rs`
- `backend/src/handlers/finance_report_handler.rs`
- `backend/src/handlers/bom_handler.rs`
- `backend/src/handlers/business_trace_handler.rs`
- `backend/src/handlers/inventory_reservation_handler.rs`
- `backend/src/handlers/init_handler.rs`
- `backend/src/handlers/material_shortage_handler.rs`
- `backend/src/handlers/fixed_asset_handler.rs`
- `backend/src/handlers/production_order_handler.rs`
- `backend/src/handlers/tenant_handler.rs`
- `backend/src/handlers/purchase_price_handler.rs`
- `backend/src/handlers/email_handler.rs`
- `backend/src/handlers/finance_invoice_handler.rs`
- `backend/src/handlers/ap_reconciliation_handler.rs`
- `backend/src/handlers/sales_order_handler.rs`
- `backend/src/handlers/quality_standard_handler.rs`
- `backend/src/handlers/system_update_handler.rs`
- `backend/src/handlers/audit_enhanced_handler.rs`
- `backend/src/handlers/purchase_receipt_handler.rs`
- `backend/src/handlers/logistics_handler.rs`
- `backend/src/handlers/inventory_stock_handler.rs`
- `backend/src/handlers/supplier_evaluation_handler.rs`
- `backend/src/handlers/accounting_period_handler.rs`
- `backend/src/handlers/inventory_adjustment_handler.rs`
- `backend/src/handlers/purchase_inspection_handler.rs`
- `backend/src/handlers/budget_management_handler.rs`
- `backend/src/handlers/user_notification_setting_handler.rs`
- `backend/src/handlers/product_category_handler.rs`
- `backend/src/handlers/financial_analysis_handler.rs`
- `backend/src/handlers/role_handler.rs`
- `backend/src/handlers/ar_reconciliation_handler.rs`
- `backend/src/handlers/crm_assignment_handler.rs`
- `backend/src/handlers/sales_return_handler.rs`
- `backend/src/handlers/bulk_product_handler.rs`
- `backend/src/handlers/ap_payment_handler.rs`
- `backend/src/handlers/voucher_handler.rs`
- `backend/src/handlers/capacity_handler.rs`
- `backend/src/handlers/supplier_handler.rs`
- `backend/src/handlers/inventory_transfer_handler.rs`
- `backend/src/handlers/sales_analysis_handler.rs`
- `backend/src/handlers/product_handler.rs`
- `backend/src/handlers/field_permission_handler.rs`
- `backend/src/handlers/inventory_count_handler.rs`
- `backend/src/handlers/tracking_handler.rs`
- `backend/src/handlers/customer_handler.rs`
- `backend/src/handlers/mrp_handler.rs`
- `backend/src/handlers/piece_split_handler.rs`
- `backend/src/handlers/fund_management_handler.rs`
- `backend/src/handlers/dashboard_handler.rs`
- `backend/src/handlers/data_permission_handler.rs`
- `backend/src/handlers/ar_invoice_handler.rs`
- `backend/src/handlers/sales_price_handler.rs`
- `backend/src/handlers/omni_audit_handler.rs`
- `backend/src/handlers/ar_payment_handler.rs`
- `backend/src/handlers/quality_inspection_handler.rs`
- `backend/src/handlers/dual_unit_converter_handler.rs`
- `backend/src/handlers/currency_handler.rs`
- `backend/src/handlers/five_dimension_handler.rs`
- `backend/src/handlers/greige_fabric_handler.rs`
- `backend/src/handlers/purchase_order_handler.rs`
- `backend/src/handlers/barcode_scanner_handler.rs`
- `backend/src/handlers/customer_credit_handler.rs`
- `backend/src/handlers/tenant_billing_handler.rs`
- `backend/src/handlers/assist_accounting_handler.rs`
- `backend/src/handlers/mod.rs`
- `backend/src/handlers/ap_report_handler.rs`
- `backend/src/handlers/webhook_handler.rs`
- `backend/src/handlers/advanced_handler.rs`
- `backend/src/handlers/cost_collection_handler.rs`
- `backend/src/handlers/webhook_integration_handler.rs`
- `backend/src/handlers/crm_pool_handler.rs`
- `backend/src/handlers/warehouse_handler.rs`
- `backend/src/handlers/tenant_config_handler.rs`
- `backend/src/handlers/crm_handler.rs`
- `backend/src/handlers/report_enhanced_handler.rs`
<!-- AUTO-GENERATED-END: handlers -->
