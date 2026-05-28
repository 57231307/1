#![allow(dead_code)]
use sea_orm::*;
use serde_json::Value;
use std::sync::Arc;
use crate::models::audit_log;
use chrono::Utc;
use serde::Serialize;

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
        resource_id: i32,
        action: &str,
        _old_data: Option<Value>,
        _new_data: Option<Value>,
        user_id: Option<i32>,
    ) -> Result<(), DbErr> {
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user_id),
            action: ActiveValue::Set(action.to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(resource_id)),
            ip_address: ActiveValue::Set(None),
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
    ) -> Result<<E as EntityTrait>::Model, DbErr>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + sea_orm::ActiveModelBehavior + Send + Sync,
        C: ConnectionTrait,
        <E as EntityTrait>::Model: Serialize + serde::de::DeserializeOwned + Sync + Send + Clone + IntoActiveModel<A>,
    {
        // 获取主键
        let pk_col = E::PrimaryKey::iter()
            .next()
            .ok_or_else(|| DbErr::Custom("Entity has no primary key".to_string()))?
            .into_column();
        
        let pk_val = active_model.get(pk_col);
        
        let pk_val_unwrapped = pk_val.into_value().unwrap_or(sea_orm::Value::Int(None));

        let record_id = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            id
        } else {
            0
        };

        let new_model = active_model.update(db).await?;
        
        // 记录审计日志
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user_id),
            action: ActiveValue::Set("UPDATE".to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(record_id)),
            ip_address: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Some(Utc::now())),
        };
        log.insert(db).await?;

        Ok(new_model)
    }
}
