use sea_orm::*;
use serde_json::Value;
use std::sync::Arc;
use crate::models::audit_log;
use chrono::Utc;
use serde::{Serialize, de::DeserializeOwned};

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
        table_name: &str,
        record_id: i32,
        action: &str,
        old_data: Option<Value>,
        new_data: Option<Value>,
        user_id: Option<i32>,
    ) -> Result<(), DbErr> {
        let mut changed_fields = serde_json::Map::new();

        if let (Some(old), Some(new)) = (&old_data, &new_data) {
            if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
                for (k, v_new) in new_obj {
                    if let Some(v_old) = old_obj.get(k) {
                        if v_old != v_new && k != "updated_at" && k != "created_at" {
                            changed_fields.insert(k.clone(), serde_json::json!({
                                "from": v_old,
                                "to": v_new
                            }));
                        }
                    }
                }
            }
        }

        let changed_fields_val = if changed_fields.is_empty() && action == "UPDATE" {
            return Ok(()); // 没有实际变更，不记录
        } else {
            Some(Value::Object(changed_fields))
        };

        let log = audit_log::ActiveModel {
            table_name: ActiveValue::Set(table_name.to_string()),
            record_id: ActiveValue::Set(record_id),
            action: ActiveValue::Set(action.to_string()),
            old_data: ActiveValue::Set(old_data),
            new_data: ActiveValue::Set(new_data),
            changed_fields: ActiveValue::Set(changed_fields_val),
            user_id: ActiveValue::Set(user_id),
            created_at: ActiveValue::Set(Utc::now()),
            ..Default::default()
        };

        log.insert(self.db.as_ref()).await?;
        Ok(())
    }

    /// 作为 SeaORM 中间件，自动拦截并生成 Update 审计日志
    pub async fn update_with_audit<E, A, C>(
        db: &C,
        table_name: &str,
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
        let pk_col = E::PrimaryKey::iter().next().unwrap().into_column();
        let pk_val = active_model.get(pk_col.clone());
        
        let pk_val_unwrapped = pk_val.into_value().unwrap_or(sea_orm::Value::Int(None));

        let record_id = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            id
        } else {
            0
        };

        // 获取旧数据
        use sea_orm::sea_query::IntoCondition;
        let old_data = E::find()
            .filter(sea_orm::sea_query::Expr::col(pk_col).eq(pk_val_unwrapped))
            .one(db)
            .await?;
            
        let old_json = old_data.as_ref().map(|d| serde_json::to_value(d).unwrap());

        // 执行更新
        let new_model = active_model.update(db).await?;
        let new_json = Some(serde_json::to_value(&new_model).unwrap());

        // 记录审计日志
        let mut changed_fields = serde_json::Map::new();

        if let (Some(old), Some(new)) = (&old_json, &new_json) {
            if let (Some(old_obj), Some(new_obj)) = (old.as_object(), new.as_object()) {
                for (k, v_new) in new_obj {
                    if let Some(v_old) = old_obj.get(k) {
                        if v_old != v_new && k != "updated_at" && k != "created_at" {
                            changed_fields.insert(k.clone(), serde_json::json!({
                                "from": v_old,
                                "to": v_new
                            }));
                        }
                    }
                }
            }
        }

        if !changed_fields.is_empty() {
            let log = audit_log::ActiveModel {
                id: ActiveValue::NotSet,
                table_name: ActiveValue::Set(table_name.to_string()),
                record_id: ActiveValue::Set(record_id),
                action: ActiveValue::Set("UPDATE".to_string()),
                old_data: ActiveValue::Set(old_json),
                new_data: ActiveValue::Set(new_json),
                changed_fields: ActiveValue::Set(Some(Value::Object(changed_fields))),
                user_id: ActiveValue::Set(user_id),
                created_at: ActiveValue::Set(Utc::now()),
                ..Default::default()
            };
            log.insert(db).await?;
        }

        Ok(new_model)
    }
}
