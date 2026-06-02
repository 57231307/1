#![allow(dead_code)]
use crate::models::audit_log;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::*;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

pub struct AuditLogService {
    db: Arc<DatabaseConnection>,
}

impl AuditLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 计算两个 JSON 对象的 Diff 并记录审计日志
    pub async fn log_change(
        &self,
        resource_type: &str,
        resource_id: &str,
        action: &str,
        old_data: Option<Value>,
        new_data: Option<Value>,
        user_id: Option<i32>,
        username: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        request_method: Option<String>,
        request_path: Option<String>,
        description: Option<String>,
    ) -> Result<(), AppError> {
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            tenant_id: ActiveValue::Set(Some(1)), // 默认租户
            user_id: ActiveValue::Set(user_id),
            username: ActiveValue::Set(username),
            action: ActiveValue::Set(action.to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(resource_id.to_string())),
            resource_name: ActiveValue::Set(None),
            description: ActiveValue::Set(description),
            ip_address: ActiveValue::Set(ip_address),
            user_agent: ActiveValue::Set(user_agent),
            request_method: ActiveValue::Set(request_method),
            request_path: ActiveValue::Set(request_path),
            request_body: ActiveValue::Set(None),
            response_status: ActiveValue::Set(None),
            duration_ms: ActiveValue::Set(None),
            old_value: ActiveValue::Set(old_data),
            new_value: ActiveValue::Set(new_data),
            created_at: ActiveValue::Set(Some(Utc::now())),
        };

        log.insert(self.db.as_ref()).await?;
        Ok(())
    }

    /// 作为 SeaORM 中间件，自动拦截并生成 Update 审计日志
    pub async fn update_with_audit<E, A, C>(
        db: &C,
        resource_type: &str,
        active_model: A,
        user_id: Option<i32>,
    ) -> Result<<E as EntityTrait>::Model, AppError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + sea_orm::ActiveModelBehavior + Send + Sync,
        C: ConnectionTrait,
        <E as EntityTrait>::Model:
            Serialize + serde::de::DeserializeOwned + Sync + Send + Clone + IntoActiveModel<A>,
    {
        // 获取主键
        let pk_col = E::PrimaryKey::iter()
            .next()
            .ok_or_else(|| AppError::BusinessError("Entity has no primary key".to_string()))?
            .into_column();

        let pk_val = active_model.get(pk_col);

        let pk_val_unwrapped = pk_val.into_value().unwrap_or(sea_orm::Value::Int(None));

        let record_id = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            id.to_string()
        } else {
            "0".to_string()
        };

        let new_model = active_model.update(db).await?;

        // 记录审计日志
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            tenant_id: ActiveValue::Set(Some(1)), // 默认租户
            user_id: ActiveValue::Set(user_id),
            username: ActiveValue::Set(None),
            action: ActiveValue::Set("UPDATE".to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(record_id)),
            resource_name: ActiveValue::Set(None),
            description: ActiveValue::Set(None),
            ip_address: ActiveValue::Set(None),
            user_agent: ActiveValue::Set(None),
            request_method: ActiveValue::Set(None),
            request_path: ActiveValue::Set(None),
            request_body: ActiveValue::Set(None),
            response_status: ActiveValue::Set(None),
            duration_ms: ActiveValue::Set(None),
            old_value: ActiveValue::Set(None),
            new_value: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Some(Utc::now())),
        };
        log.insert(db).await?;

        Ok(new_model)
    }
}
