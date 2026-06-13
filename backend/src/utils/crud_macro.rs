/// 通用 Service 结构体生成宏
/// 用于减少各个 service 中重复的结构体定义和 new 方法
#[macro_export]
macro_rules! define_service {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            db: std::sync::Arc<sea_orm::DatabaseConnection>,
        }

        impl $name {
            pub fn new(db: std::sync::Arc<sea_orm::DatabaseConnection>) -> Self {
                Self { db }
            }
        }
    };
}

/// 通用单号生成函数宏
/// 用于减少各个 service 中重复的 generate_*_no 函数模板代码
#[macro_export]
macro_rules! impl_generate_no {
    ($fn_name:ident, $prefix:expr, $entity:ty, $column:expr) => {
        pub async fn $fn_name(&self) -> Result<String, $crate::utils::error::AppError> {
            $crate::utils::number_generator::DocumentNumberGenerator::generate_no(
                &*self.db,
                $prefix,
                <$entity>::default(),
                $column,
            )
            .await
        }
    };
    ($fn_name:ident, $prefix:expr, $entity:ty, $column:expr, $conn:ident) => {
        pub async fn $fn_name(
            &self,
            $conn: &sea_orm::DatabaseTransaction,
        ) -> Result<String, $crate::utils::error::AppError> {
            $crate::utils::number_generator::DocumentNumberGenerator::generate_no(
                $conn,
                $prefix,
                <$entity>::default(),
                $column,
            )
            .await
        }
    };
}

/// 通用 CRUD Handler 生成宏
/// 用于减少各个实体基础增删改查路由的模板代码
///
/// 要求目标 Service 实现以下方法：
/// - `list(query) -> PaginatedResponse<T>`
/// - `get(id) -> T`（如返回 Option，需使用 `define_tenant_crud_handlers!` 变体）
/// - `create(req) -> T`
/// - `update(id, req) -> T`
/// - `delete(id) -> ()`
#[macro_export]
macro_rules! define_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty
    ) => {
        pub async fn list(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Query(params): axum::extract::Query<$query_params>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&params) {
                return Err($crate::utils::error::AppError::validation(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let result = service.list(params).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                serde_json::to_value(result).map_err($crate::utils::error::AppError::from)?,
            )))
        }

        pub async fn get(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let item = service.get(id).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
            )))
        }

        pub async fn create(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::Json(req): axum::Json<$create_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::validation(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
                    "创建成功",
                ),
            ))
        }

        pub async fn update(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
            axum::Json(req): axum::Json<$update_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::validation(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.update(id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
                    "更新成功",
                ),
            ))
        }

        pub async fn delete(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<()>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            service.delete(id).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message((), "删除成功"),
            ))
        }
    };
}

/// 租户隔离版 CRUD Handler 生成宏
///
/// 与 `define_crud_handlers!` 的差异：
/// - 从 `AuthContext` 中提取 `tenant_id`，用于多租户隔离
/// - `list(tenant_id, query) -> (Vec<T>, u64)` 返回元组，由宏包装为 `{items, total}` 结构
/// - `get_by_id(id) -> Option<T>` 返回 `Option<T>`，由宏转换为未找到错误
/// - `create(tenant_id, user_id, req) -> T` 接收额外的 `tenant_id` 与 `user_id`
/// - `update(id, req) -> T`、`delete(id) -> ()` 与基础版一致
///
/// 适用于租户维度数据隔离的常见业务对象（如报表订阅、邮件模板、字典配置等）。
#[macro_export]
macro_rules! define_tenant_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty,
        $not_found_msg:expr
    ) => {
        /// 列表查询（带租户隔离）
        pub async fn list(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Query(params): axum::extract::Query<$query_params>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            let tenant_id = $crate::middleware::tenant::extract_tenant_id(&auth)?;
            let service = <$service_ty>::new(state.db.clone());
            let (items, total) = service.list(tenant_id, params).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                serde_json::json!({
                    "items": items,
                    "total": total,
                }),
            )))
        }

        /// 详情查询（自动处理未找到场景）
        pub async fn get(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let item = service
                .get_by_id(id)
                .await?
                .ok_or_else(|| $crate::utils::error::AppError::not_found($not_found_msg))?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
            )))
        }

        /// 创建（自动注入 tenant_id 与 user_id）
        pub async fn create(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::Json(req): axum::Json<$create_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::validation(e.to_string()));
            }
            let tenant_id = $crate::middleware::tenant::extract_tenant_id(&auth)?;
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(tenant_id, auth.user_id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
                    "创建成功",
                ),
            ))
        }

        /// 更新
        pub async fn update(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
            axum::Json(req): axum::Json<$update_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::validation(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.update(id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    serde_json::to_value(item).map_err($crate::utils::error::AppError::from)?,
                    "更新成功",
                ),
            ))
        }

        /// 删除
        pub async fn delete(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<()>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            service.delete(id).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message((), "删除成功"),
            ))
        }
    };
}
