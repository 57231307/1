#![allow(dead_code)]

use crate::models::tenant::{self, Entity as Tenant, ActiveModel as TenantActiveModel};
use crate::models::tenant_user::{self, Entity as TenantUser};
use crate::models::tenant_config::{self, Entity as TenantConfig};
use crate::models::tenant_plan::{self, Entity as TenantPlan};
use sea_orm::*;
use std::sync::Arc;
use chrono::Utc;

pub struct TenantService {
    db: Arc<DatabaseConnection>,
}

impl TenantService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建租户
    pub async fn create_tenant(
        &self,
        code: &str,
        name: &str,
        description: Option<&str>,
        plan_id: Option<i32>,
    ) -> Result<tenant::Model, DbErr> {
        let now = Utc::now();
        let active_model = TenantActiveModel {
            code: Set(code.to_string()),
            name: Set(name.to_string()),
            description: Set(description.map(|s| s.to_string())),
            status: Set("PENDING".to_string()),
            plan_id: Set(plan_id),
            admin_user_id: Set(None),
            db_schema: Set(None),
            custom_domain: Set(None),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            expired_at: Set(None),
            ..Default::default()
        };

        let result = active_model.insert(self.db.as_ref()).await?;
        Ok(result)
    }

    /// 根据 ID 获取租户
    pub async fn get_tenant(&self, id: i32) -> Result<Option<tenant::Model>, DbErr> {
        Tenant::find_by_id(id).one(self.db.as_ref()).await
    }

    /// 根据编码获取租户
    pub async fn get_tenant_by_code(&self, code: &str) -> Result<Option<tenant::Model>, DbErr> {
        Tenant::find()
            .filter(tenant::Column::Code.eq(code))
            .one(self.db.as_ref())
            .await
    }

    /// 更新租户状态
    pub async fn update_tenant_status(
        &self,
        id: i32,
        status: &str,
    ) -> Result<tenant::Model, DbErr> {
        let tenant = Tenant::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(DbErr::Custom("租户不存在".to_string()))?;

        let mut active_model: TenantActiveModel = tenant.into();
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(self.db.as_ref()).await
    }

    /// 获取租户列表（分页）
    pub async fn list_tenants(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<tenant::Model>, u64), DbErr> {
        let paginator = Tenant::find()
            .filter(tenant::Column::IsDeleted.eq(false))
            .order_by_desc(tenant::Column::CreatedAt)
            .paginate(self.db.as_ref(), page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 添加用户到租户
    pub async fn add_user_to_tenant(
        &self,
        tenant_id: i32,
        user_id: i32,
        role: &str,
        is_primary: bool,
    ) -> Result<tenant_user::Model, DbErr> {
        let now = Utc::now();
        let active_model = tenant_user::ActiveModel {
            tenant_id: Set(tenant_id),
            user_id: Set(user_id),
            role_in_tenant: Set(role.to_string()),
            is_primary: Set(is_primary),
            joined_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        active_model.insert(self.db.as_ref()).await
    }

    /// 获取租户用户列表
    pub async fn get_tenant_users(
        &self,
        tenant_id: i32,
    ) -> Result<Vec<tenant_user::Model>, DbErr> {
        TenantUser::find()
            .filter(tenant_user::Column::TenantId.eq(tenant_id))
            .all(self.db.as_ref())
            .await
    }

    /// 获取租户配置
    pub async fn get_tenant_config(
        &self,
        tenant_id: i32,
        key: &str,
    ) -> Result<Option<String>, DbErr> {
        let config = TenantConfig::find()
            .filter(tenant_config::Column::TenantId.eq(tenant_id))
            .filter(tenant_config::Column::ConfigKey.eq(key))
            .one(self.db.as_ref())
            .await?;

        Ok(config.map(|c| c.config_value))
    }

    /// 设置租户配置
    pub async fn set_tenant_config(
        &self,
        tenant_id: i32,
        key: &str,
        value: &str,
        config_type: &str,
    ) -> Result<(), DbErr> {
        let existing = TenantConfig::find()
            .filter(tenant_config::Column::TenantId.eq(tenant_id))
            .filter(tenant_config::Column::ConfigKey.eq(key))
            .one(self.db.as_ref())
            .await?;

        let now = Utc::now();

        if let Some(config) = existing {
            let mut active_model: tenant_config::ActiveModel = config.into();
            active_model.config_value = Set(value.to_string());
            active_model.updated_at = Set(now);
            active_model.update(self.db.as_ref()).await?;
        } else {
            let active_model = tenant_config::ActiveModel {
                tenant_id: Set(tenant_id),
                config_key: Set(key.to_string()),
                config_value: Set(value.to_string()),
                config_type: Set(config_type.to_string()),
                description: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            active_model.insert(self.db.as_ref()).await?;
        }

        Ok(())
    }
}
