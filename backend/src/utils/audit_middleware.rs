use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel, PrimaryKeyTrait, ModelTrait,
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use crate::services::audit_log_service::AuditLogService;

#[async_trait::async_trait]
pub trait AuditUpdateTrait<E, M, A>
where
    E: EntityTrait<Model = M>,
    M: ModelTrait + Serialize + DeserializeOwned + Sync + Send + Clone + IntoActiveModel<A>,
    A: ActiveModelTrait<Entity = E> + Send + Sync,
{
    async fn update_with_audit<C>(
        self,
        db: &C,
        table_name: &str,
        user_id: Option<i32>,
    ) -> Result<M, DbErr>
    where
        C: ConnectionTrait;

    async fn insert_with_audit<C>(
        self,
        db: &C,
        table_name: &str,
        user_id: Option<i32>,
    ) -> Result<M, DbErr>
    where
        C: ConnectionTrait;
}
