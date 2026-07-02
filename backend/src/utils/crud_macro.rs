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
///
/// 提供两个变体：
/// - 无 txn 变体：用 `&*self.db`，适用于单号生成 + INSERT 不在同一事务的场景
///   （依赖数据库 UNIQUE 约束做最终去重防御）
/// - 带 txn 变体：用调用方传入的 `&DatabaseTransaction`，适用于单号生成 + INSERT
///   在同一事务内的场景。P1 5-10 修复（批次 60）：txn 变体改调 `generate_no_with_txn`
///   而非 `generate_no`，避免在 savepoint 上获取 advisory_xact_lock 导致锁提前释放。
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
            // P1 5-10 修复（批次 60）：调用 generate_no_with_txn 直接在传入 txn 上获取
            // advisory_xact_lock，避免在 savepoint 上加锁导致锁提前释放
            $crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
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
/// - `get(id) -> T`（如返回 Option，需使用 `define_tuple_crud_handlers!` 变体）
/// - `create(req) -> T`
/// - `update(id, req) -> T`
/// - `delete(id) -> ()`
///
/// 另有 `define_tuple_crud_handlers!` 变体适用于返回元组 `(Vec<T>, u64)` 与
/// `Option<T>` 的 Service（接口形态差异：list 返回元组、get_by_id 返回 Option、create 注入 user_id）。
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

/// 返回元组与 Option 的 CRUD Handler 生成宏
///
/// 与 `define_crud_handlers!` 的差异（仅接口形态不同）：
/// - `list(query) -> (Vec<T>, u64)` 返回元组，由宏包装为 `{items, total}` 结构
/// - `get_by_id(id) -> Option<T>` 返回 `Option<T>`，由宏转换为未找到错误
/// - `create(user_id, req) -> T` 接收额外的 `user_id`（用于审计字段写入）
/// - `update(id, req) -> T`、`delete(id) -> ()` 与基础版一致
///
/// 适用于 Service 返回元组与 Option 形态的业务对象（如报表订阅、邮件模板等）。
#[macro_export]
macro_rules! define_tuple_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty,
        $not_found_msg:expr
    ) => {
        /// 列表查询
        pub async fn list(
            axum::extract::State(state): axum::extract::State<$crate::utils::app_state::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Query(params): axum::extract::Query<$query_params>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<serde_json::Value>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let (items, total) = service.list(params).await?;
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

        /// 创建（自动注入 user_id）
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
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(auth.user_id, req).await?;
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
