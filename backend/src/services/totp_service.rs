use crate::utils::error::AppError;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set};
use totp_rs::{Algorithm, Secret, TOTP};
use std::sync::Arc;
use crate::models::user;

pub struct TotpService {
    db: Arc<DatabaseConnection>,
}

impl TotpService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 1. 为用户生成一个新的 TOTP Secret，并返回二维码 URL
    pub async fn generate_totp_secret(&self, user_id: i32, username: &str) -> Result<(String, String), AppError> {
        let secret = Secret::generate_secret().to_bytes().map_err(|e| AppError::InternalError(format!("TOTP密钥生成失败: {}", e)))?;
        
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            Some("Bingxi ERP".to_string()),
            username.to_string(),
        ).map_err(|e| AppError::InternalError(format!("TOTP 生成失败: {}", e)))?;

        let secret_base32 = totp.get_secret_base32();
        let qr_code = totp.get_qr_base64().map_err(|e| AppError::InternalError(format!("QR 生成失败: {}", e)))?;

        // 临时保存在数据库中，但不开启
        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound("User".to_string()))?;

        let mut active_user: user::ActiveModel = user.into();
        active_user.totp_secret = Set(Some(secret_base32.clone()));
        active_user.is_totp_enabled = Set(false); // 必须验证一次后才开启
        active_user.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((secret_base32, qr_code))
    }

    /// 2. 验证 TOTP 并在首次验证成功后正式开启
    pub async fn verify_and_enable(&self, user_id: i32, token: &str) -> Result<bool, AppError> {
        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound("User".to_string()))?;

        let secret_base32 = user.totp_secret.clone().ok_or_else(|| AppError::BadRequest("未生成 TOTP Secret".to_string()))?;
        
        let secret = Secret::Encoded(secret_base32).to_bytes().map_err(|e| AppError::InternalError(format!("TOTP密钥解析失败: {}", e)))?;
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            None,
            "".to_string(),
        ).map_err(|e| AppError::InternalError(format!("TOTP实例创建失败: {}", e)))?;

        let is_valid = match totp.check_current(token) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("TOTP 验证内部发生异常: {}", e);
                false
            }
        };

        if is_valid {
            // 验证通过，正式开启
            let mut active_user: user::ActiveModel = user.into();
            active_user.is_totp_enabled = Set(true);
            active_user.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 3. 日常登录验证 TOTP
    pub async fn verify_login_totp(&self, user_id: i32, token: &str) -> Result<bool, AppError> {
        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound("User".to_string()))?;

        if !user.is_totp_enabled {
            // 如果没开启 TOTP，则无需验证
            return Ok(true);
        }

        let secret_base32 = user.totp_secret.ok_or_else(|| AppError::InternalError("TOTP Secret未找到".to_string()))?;
        let secret = Secret::Encoded(secret_base32).to_bytes().map_err(|e| AppError::InternalError(format!("TOTP密钥解析失败: {}", e)))?;
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            None,
            "".to_string(),
        ).map_err(|e| AppError::InternalError(format!("TOTP实例创建失败: {}", e)))?;

        let is_valid = match totp.check_current(token) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("TOTP 验证内部发生异常: {}", e);
                false
            }
        };
        Ok(is_valid)
    }
}
