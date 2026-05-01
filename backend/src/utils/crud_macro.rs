/// 通用 CRUD Handler 生成宏
/// 用于减少各个实体基础增删改查路由的模板代码
#[macro_export]
macro_rules! define_crud_handlers {
    (
        $service_ty:ty,
        $create_req:ty,
        $update_req:ty,
        $query_params:ty,
        $id_ty:ty
    ) => {
        use crate::utils::app_state::AppState;
        use crate::utils::error::AppError;
        use crate::utils::response::ApiResponse;
        use axum::{
            extract::{Path, Query, State},
            Json,
        };
        use serde_json::Value as JsonValue;
        use validator::Validate;

        pub async fn list(
            State(state): State<AppState>,
            Query(params): Query<$query_params>,
        ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
            if let Err(e) = params.validate() {
                return Err(AppError::ValidationError(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let result = service.list(params).await?;
            Ok(Json(ApiResponse::success(
                serde_json::to_value(result).map_err(AppError::from)?,
            )))
        }

        pub async fn get(
            State(state): State<AppState>,
            Path(id): Path<$id_ty>,
        ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
            let service = <$service_ty>::new(state.db.clone());
            let item = service.get(id).await?;
            Ok(Json(ApiResponse::success(
                serde_json::to_value(item).map_err(AppError::from)?,
            )))
        }

        pub async fn create(
            State(state): State<AppState>,
            Json(req): Json<$create_req>,
        ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
            if let Err(e) = req.validate() {
                return Err(AppError::ValidationError(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.create(req).await?;
            Ok(Json(ApiResponse::success_with_message(
                serde_json::to_value(item).map_err(AppError::from)?,
                "创建成功",
            )))
        }

        pub async fn update(
            State(state): State<AppState>,
            Path(id): Path<$id_ty>,
            Json(req): Json<$update_req>,
        ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
            if let Err(e) = req.validate() {
                return Err(AppError::ValidationError(e.to_string()));
            }
            let service = <$service_ty>::new(state.db.clone());
            let item = service.update(id, req).await?;
            Ok(Json(ApiResponse::success_with_message(
                serde_json::to_value(item).map_err(AppError::from)?,
                "更新成功",
            )))
        }

        pub async fn delete(
            State(state): State<AppState>,
            Path(id): Path<$id_ty>,
        ) -> Result<Json<ApiResponse<()>>, AppError> {
            let service = <$service_ty>::new(state.db.clone());
            service.delete(id).await?;
            Ok(Json(ApiResponse::success_with_message((), "删除成功")))
        }
    };
}
