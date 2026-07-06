use crate::models::user;
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use totp_rs::{Algorithm, Secret, TOTP};

crate::define_service!(TotpService);

impl TotpService {
    /// 1. 为用户生成一个新的 TOTP Secret，并返回二维码 URL
    pub async fn generate_totp_secret(
        &self,
        user_id: i32,
        username: &str,
    ) -> Result<(String, String), AppError> {
        let secret = Secret::generate_secret()
            .to_bytes()
            .map_err(|e| AppError::internal(format!("TOTP密钥生成失败: {}", e)))?;

        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            Some("Bingxi ERP".to_string()),
            username.to_string(),
        )
        .map_err(|e| AppError::internal(format!("TOTP 生成失败: {}", e)))?;

        let secret_base32 = totp.get_secret_base32();
        let qr_code = totp
            .get_qr_base64()
            .map_err(|e| AppError::internal(format!("QR 生成失败: {}", e)))?;

        // 临时保存在数据库中，但不开启
        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("User"))?;

        let mut active_user: user::ActiveModel = user.into();
        active_user.totp_secret = Set(Some(secret_base32.clone()));
        active_user.is_totp_enabled = Set(false); // 必须验证一次后才开启
        active_user.update(&*self.db).await?;

        Ok((secret_base32, qr_code))
    }

    /// 2. 验证 TOTP 并在首次验证成功后正式开启
    pub async fn verify_and_enable(&self, user_id: i32, token: &str) -> Result<bool, AppError> {
        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("User"))?;

        let secret_base32 = user
            .totp_secret
            .clone()
            .ok_or_else(|| AppError::bad_request("未生成 TOTP Secret"))?;

        let secret = Secret::Encoded(secret_base32)
            .to_bytes()
            .map_err(|e| AppError::internal(format!("TOTP密钥解析失败: {}", e)))?;
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            None,
            "".to_string(),
        )
        .map_err(|e| AppError::internal(format!("TOTP实例创建失败: {}", e)))?;

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
            active_user.update(&*self.db).await?;
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
            .ok_or_else(|| AppError::not_found("User"))?;

        if !user.is_totp_enabled {
            // 如果没开启 TOTP，则无需验证
            return Ok(true);
        }

        let secret_base32 = user
            .totp_secret
            .ok_or_else(|| AppError::internal("TOTP Secret未找到"))?;
        let secret = Secret::Encoded(secret_base32)
            .to_bytes()
            .map_err(|e| AppError::internal(format!("TOTP密钥解析失败: {}", e)))?;
        let totp = TOTP::new(
            Algorithm::SHA256,
            6,
            1, // skew: 1 = ±30秒的时间窗口容差
            30,
            secret,
            None,
            "".to_string(),
        )
        .map_err(|e| AppError::internal(format!("TOTP实例创建失败: {}", e)))?;

        let is_valid = match totp.check_current(token) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("TOTP 验证内部发生异常: {}", e);
                false
            }
        };
        Ok(is_valid)
    }

    /// 4. 生成 2FA 恢复码（v11 批次 141 新增）
    ///
    /// 生成 10 个 8 字符的恢复码（base32 编码，去除易混淆字符 0/O/1/I），
    /// 明文返回给前端展示，哈希（argon2）后存入 user.totp_recovery_codes（JSON 数组）。
    ///
    /// 安全要求：
    /// - 使用 `OsRng` 密码学安全随机源
    /// - 明文仅在生成时返回一次，后续无法获取
    /// - 哈希使用 argon2id（与密码哈希同算法）
    /// - 每次生成会覆盖旧恢复码
    pub async fn generate_recovery_codes(
        &self,
        user_id: i32,
    ) -> Result<Vec<String>, AppError> {
        use argon2::password_hash::rand_core::{OsRng, RngCore};
        use argon2::password_hash::{PasswordHasher, SaltString};
        use argon2::{Algorithm as ArgonAlgorithm, Argon2, Params, Version};

        // 恢复码字符集（去除易混淆字符 0/O/1/I/L）
        const CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
        const CODE_LEN: usize = 8;
        const CODE_COUNT: usize = 10;

        // 生成 10 个 8 字符随机恢复码（OsRng = 操作系统 CSPRNG）
        let mut rng = OsRng;
        let mut codes: Vec<String> = Vec::with_capacity(CODE_COUNT);
        for _ in 0..CODE_COUNT {
            let mut buf = [0u8; CODE_LEN];
            rng.fill_bytes(&mut buf);
            let code: String = buf
                .iter()
                .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
                .collect();
            codes.push(code);
        }

        // 对每个恢复码进行 argon2id 哈希
        let argon2 = Argon2::new(
            ArgonAlgorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 3, 4, None)
                .map_err(|e| AppError::internal(format!("Argon2 参数错误: {}", e)))?,
        );
        let mut hashed_codes: Vec<String> = Vec::with_capacity(CODE_COUNT);
        for code in &codes {
            let salt = SaltString::generate(&mut OsRng);
            let hash = argon2
                .hash_password(code.as_bytes(), &salt)
                .map_err(|e| AppError::internal(format!("恢复码哈希失败: {}", e)))?
                .to_string();
            hashed_codes.push(hash);
        }

        // 序列化为 JSON 存入 user.totp_recovery_codes
        let json = serde_json::to_string(&hashed_codes)
            .map_err(|e| AppError::internal(format!("序列化恢复码失败: {}", e)))?;

        let user_model = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("User"))?;
        let mut active_user: user::ActiveModel = user_model.into();
        active_user.totp_recovery_codes = Set(Some(json));
        active_user.updated_at = Set(chrono::Utc::now());
        active_user.update(&*self.db).await?;

        // 返回明文恢复码（仅此一次返回）
        Ok(codes)
    }

    /// 5. 使用恢复码登录（v11 批次 141 新增，预留接口）
    ///
    /// 验证恢复码是否匹配，匹配成功后消耗该恢复码（从列表中删除）。
    /// 返回 true 表示验证成功，false 表示恢复码无效。
    pub async fn verify_recovery_code(
        &self,
        user_id: i32,
        code: &str,
    ) -> Result<bool, AppError> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        use argon2::Argon2;

        let user_model = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("User"))?;

        let json = user_model
            .totp_recovery_codes
            .clone()
            .ok_or_else(|| AppError::bad_request("未生成恢复码"))?;

        let mut hashed_codes: Vec<String> = serde_json::from_str(&json)
            .map_err(|e| AppError::internal(format!("恢复码格式错误: {}", e)))?;

        if hashed_codes.is_empty() {
            return Ok(false);
        }

        let argon2 = Argon2::default();
        let mut matched_index: Option<usize> = None;
        for (i, hash_str) in hashed_codes.iter().enumerate() {
            if let Ok(parsed) = PasswordHash::new(hash_str) {
                if argon2.verify_password(code.as_bytes(), &parsed).is_ok() {
                    matched_index = Some(i);
                    break;
                }
            }
        }

        if let Some(i) = matched_index {
            // 消耗该恢复码（从列表中删除）
            hashed_codes.remove(i);
            let new_json = serde_json::to_string(&hashed_codes)
                .map_err(|e| AppError::internal(format!("序列化恢复码失败: {}", e)))?;
            let mut active_user: user::ActiveModel = user_model.into();
            active_user.totp_recovery_codes = Set(Some(new_json));
            active_user.updated_at = Set(chrono::Utc::now());
            active_user.update(&*self.db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
