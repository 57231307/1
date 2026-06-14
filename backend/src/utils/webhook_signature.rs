//! Webhook 签名验证工具
//!
//! 使用 HMAC-SHA256 算法验证 Webhook 回调请求的真实性

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 验证 Webhook 回调签名
///
/// # 参数
/// - `payload`: 请求体原始内容
/// - `secret`: Webhook 密钥
/// - `signature`: 请求中携带的签名（hex 编码）
///
/// # 返回
/// - `Ok(true)`: 签名验证通过
/// - `Err(AppError)`: 签名验证失败
pub fn verify_webhook_signature(
    payload: &str,
    secret: &str,
    signature: &str,
) -> Result<bool, crate::utils::error::AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| crate::utils::error::AppError::internal(format!("HMAC 初始化失败：{}", e)))?;
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let computed = hex::encode(result.into_bytes());

    // 常量时间比较，防止时序攻击
    use subtle::ConstantTimeEq;
    let sig_bytes = hex::decode(signature)
        .map_err(|_| crate::utils::error::AppError::unauthorized("无效的签名格式"))?;
    let computed_bytes = hex::decode(&computed)
        .map_err(|_| crate::utils::error::AppError::internal("签名计算异常"))?;

    if sig_bytes.ct_eq(&computed_bytes).unwrap_u8() == 1 {
        Ok(true)
    } else {
        Err(crate::utils::error::AppError::unauthorized(
            "Webhook 签名验证失败",
        ))
    }
}

/// 生成 Webhook 签名（用于发送 Webhook 时）
pub fn generate_webhook_signature(
    payload: &str,
    secret: &str,
) -> Result<String, crate::utils::error::AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| crate::utils::error::AppError::internal(format!("HMAC 初始化失败：{}", e)))?;
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}
