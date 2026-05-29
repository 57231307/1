#![allow(dead_code)]

use crate::models::api_key::{self, ActiveModel as ApiKeyActiveModel, Entity as ApiKey};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub struct ApiKeyService {
    db: Arc<DatabaseConnection>,
}

impl ApiKeyService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成新的 API 密钥
    pub fn generate_api_key() -> String {
        let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();
        let key: String = (0..32)
            .map(|_| chars[fastrand::usize(..chars.len())])
            .collect();
        format!("bx_{}", key)
    }

    /// 哈希 API 密钥
    pub fn hash_api_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in result.iter() {
            hex.push_str(&format!("{:02x}", byte));
        }
        hex
    }

    /// 创建 API 密钥
    pub async fn create_api_key(
        &self,
        tenant_id: i32,
        name: &str,
        permissions: Option<&str>,
        rate_limit: i32,
        expires_days: Option<i64>,
    ) -> Result<(api_key::Model, String), AppError> {
        let plain_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&plain_key);
        let key_prefix = plain_key[..8].to_string();

        let expires_at = expires_days.map(|days| Utc::now() + chrono::Duration::days(days));
        let now = Utc::now();

        let active_model = ApiKeyActiveModel {
            tenant_id: Set(tenant_id),
            name: Set(name.to_string()),
            key_hash: Set(key_hash),
            key_prefix: Set(key_prefix),
            permissions: Set(permissions.map(|s| s.to_string())),
            rate_limit_per_minute: Set(rate_limit),
            last_used_at: Set(None),
            expires_at: Set(expires_at),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let model = active_model.insert(self.db.as_ref()).await?;
        Ok((model, plain_key))
    }

    /// 验证 API 密钥
    pub async fn validate_api_key(&self, key: &str) -> Result<Option<api_key::Model>, AppError> {
        let key_hash = Self::hash_api_key(key);

        let api_key = ApiKey::find()
            .filter(api_key::Column::KeyHash.eq(key_hash))
            .filter(api_key::Column::IsActive.eq(true))
            .one(self.db.as_ref())
            .await?;

        if let Some(ref key) = api_key {
            // 检查是否过期
            if let Some(expires_at) = key.expires_at {
                if expires_at < Utc::now() {
                    return Ok(None);
                }
            }

            // 更新最后使用时间
            let mut active_model: ApiKeyActiveModel = key.clone().into();
            active_model.last_used_at = Set(Some(Utc::now()));
            active_model.update(self.db.as_ref()).await?;
        }

        Ok(api_key)
    }

    /// 获取租户的 API 密钥列表
    pub async fn list_api_keys(&self, tenant_id: i32) -> Result<Vec<api_key::Model>, AppError> {
        ApiKey::find()
            .filter(api_key::Column::TenantId.eq(tenant_id))
            .filter(api_key::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 撤销 API 密钥
    pub async fn revoke_api_key(&self, id: i32) -> Result<(), AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or(AppError::BusinessError("API 密钥不存在".to_string()))?;

        let mut active_model: ApiKeyActiveModel = key.into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        Ok(())
    }
}
