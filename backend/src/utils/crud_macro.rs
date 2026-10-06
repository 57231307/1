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

/// 通用单号生成函数宏（减少 generate_*_no 模板代码；无 txn 变体用 &*self.db 依赖 UNIQUE 约束去重，带 txn 变体用调用方传入的 &DatabaseTransaction，P1 5-10 改调 generate_no_with_txn 避免 savepoint 上 advisory_xact_lock 提前释放；批次 346 改 $entity 为 path metavariable 消除 clippy 警告）
#[macro_export]
macro_rules! impl_generate_no {
    ($fn_name:ident, $prefix:expr, $entity:path, $column:expr) => {
        pub async fn $fn_name(&self) -> Result<String, $crate::utils::error::AppError> {
            $crate::utils::number_generator::DocumentNumberGenerator::generate_no(
                &*self.db, $prefix, $entity, $column,
            )
            .await
        }
    };
    ($fn_name:ident, $prefix:expr, $entity:path, $column:expr, $conn:ident) => {
        pub async fn $fn_name(
            &self,
            $conn: &sea_orm::DatabaseTransaction,
        ) -> Result<String, $crate::utils::error::AppError> {
            // P1 5-10 修复（批次 60）：调用 generate_no_with_txn 直接在传入 txn 上获取
            // advisory_xact_lock，避免在 savepoint 上加锁导致锁提前释放
            $crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                $conn, $prefix, $entity, $column,
            )
            .await
        }
    };
}

/// 通用 CRUD Handler 生成宏（减少增删改查路由模板代码；要求 Service 实现 list/get/create/update/delete，update/delete 注入 user_id 审计；另有 define_tuple_crud_handlers! 变体适用于返回元组与 Option 的 Service）
///
/// 出参类型形参（批次 476 契约收紧）：`$list_resp_ty` 为 list 的**真实**载荷类型
/// （各域 service 一律返回 `PaginatedResponse<T>`，序列化后顶层键为
/// `items/total/page/page_size`，见 utils/response.rs:39-45），`$item_resp_ty` 为
/// get/create/update 的真实单对象类型（各域 service 一律返回实体 `Model`）。
/// 原实现把 typed 结果经 `serde_json::to_value` 塌成 `serde_json::Value`，出参形状
/// 在类型层消失，前端与门禁（check-api-envelope）均无法静态验证分页信封键名与行类型，
/// 只能记「未分类」；现类型化贯穿到出参，删除这次有损往返。
/// 信封形状（`code/data/message/total` 键）、`ApiResponse::success` /
/// `success_with_message` 选用、状态码与 `AppError` 错误通道语义均不变。
#[macro_export]
macro_rules! define_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty,
        $list_resp_ty:ty,
        $item_resp_ty:ty
    ) => {
        #[allow(dead_code, reason = "宏生成的 handler 可能未被所有调用方使用")]
        pub async fn list(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Query(params): axum::extract::Query<$query_params>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$list_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&params) {
                return Err($crate::utils::error::AppError::from(e));
            }
            let service = <$service_ty>::new(state.db.clone());
            let result = service.list(params).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                result,
            )))
        }

        pub async fn get(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let item = service.get(id).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                item,
            )))
        }

        pub async fn create(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::Json(req): axum::Json<$create_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::from(e));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(req, auth.user_id).await?;

            // V15 P2 B19-P2-1：创建操作补审计日志（与 update/delete 对齐）
            {
                use std::sync::Arc;
                use $crate::models::audit_log::{OperationType, Severity};
                use $crate::services::audit_log_service::AuditEvent;
                let audit_svc = Arc::new(
                    $crate::services::audit_log_service::AuditLogService::new(state.db.clone()),
                );
                let event = AuditEvent {
                    user_id: Some(auth.user_id),
                    username: None,
                    operation_type: OperationType::Create,
                    severity: Severity::Info,
                    resource_type: Some(stringify!($service_ty).to_string()),
                    resource_id: None,
                    resource_name: None,
                    description: Some(format!("创建记录 user_id={}", auth.user_id)),
                    request_method: Some("POST".to_string()),
                    request_path: None,
                    before_snapshot: None,
                    after_snapshot: Some(serde_json::to_value(&item).unwrap_or_default()),
                };
                audit_svc.record_async(event, None);
            }

            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    item,
                    $crate::utils::messages::biz_msg::CREATE_OK,
                ),
            ))
        }

        pub async fn update(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
            axum::Json(req): axum::Json<$update_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::from(e));
            }
            let service = <$service_ty>::new(state.db.clone());
            // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
            let item = service.update(id, auth.user_id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    item,
                    $crate::utils::messages::biz_msg::UPDATE_OK,
                ),
            ))
        }

        pub async fn delete(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<()>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
            service.delete(id, auth.user_id).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    (),
                    $crate::utils::messages::biz_msg::DELETE_OK,
                ),
            ))
        }
    };
}

/// 返回 Option 详情与 user_id 建单形态的 CRUD Handler 生成宏（与 define_crud_handlers! 仅接口形态不同：
/// list 的 Service 方法返回统一分页信封 `PaginatedResponse<T>`，get_by_id 返回 `Option<T>`（None 归 404），
/// create 接收 user_id；适用于报表订阅/邮件模板/公告等）
///
/// 出参类型形参与 define_crud_handlers! 同源同义：`$list_resp_ty` = service.list 的真实返回类型
/// （必须是 `utils::response::PaginatedResponse<行类型>`，分页默认值与 clamp 只允许留在 service，
/// 宏内不得复制该逻辑以免出现第二套口径；信封顶层键恒为 items/total/page/page_size），
/// `$item_resp_ty` = get/create/update 的真实单对象类型（各域 service 一律返回实体 `Model`）。
/// 出参在类型层保持具体，`check-api-envelope` 判定器可静态比对形状；
/// 禁止以 `serde_json::Value` 作为这两个形参的实参塌形。
#[macro_export]
macro_rules! define_tuple_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty,
        $list_resp_ty:ty,
        $item_resp_ty:ty,
        $not_found_msg:expr
    ) => {
        #[allow(dead_code, reason = "宏生成的 handler 可能未被所有调用方使用")]
        /// 列表查询
        pub async fn list(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Query(params): axum::extract::Query<$query_params>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$list_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let result = service.list(params).await?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                result,
            )))
        }

        /// 详情查询（自动处理未找到场景）
        pub async fn get(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            let item = service
                .get_by_id(id)
                .await?
                .ok_or_else(|| $crate::utils::error::AppError::not_found($not_found_msg))?;
            Ok(axum::Json($crate::utils::response::ApiResponse::success(
                item,
            )))
        }

        /// 创建（自动注入 user_id）
        pub async fn create(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            auth: $crate::middleware::auth_context::AuthContext,
            axum::Json(req): axum::Json<$create_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::from(e));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(auth.user_id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    item,
                    $crate::utils::messages::biz_msg::CREATE_OK,
                ),
            ))
        }

        /// 更新
        pub async fn update(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
            axum::Json(req): axum::Json<$update_req>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<$item_resp_ty>>,
            $crate::utils::error::AppError,
        > {
            if let Err(e) = validator::Validate::validate(&req) {
                return Err($crate::utils::error::AppError::from(e));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.update(id, req).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    item,
                    $crate::utils::messages::biz_msg::UPDATE_OK,
                ),
            ))
        }

        /// 删除
        pub async fn delete(
            axum::extract::State(state): axum::extract::State<$crate::container::AppState>,
            _auth: $crate::middleware::auth_context::AuthContext,
            axum::extract::Path(id): axum::extract::Path<$id_ty>,
        ) -> Result<
            axum::Json<$crate::utils::response::ApiResponse<()>>,
            $crate::utils::error::AppError,
        > {
            let service = <$service_ty>::new(state.db.clone());
            service.delete(id).await?;
            Ok(axum::Json(
                $crate::utils::response::ApiResponse::success_with_message(
                    (),
                    $crate::utils::messages::biz_msg::DELETE_OK,
                ),
            ))
        }
    };
}
