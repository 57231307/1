//! Webhook 签名验证工具
//!
//! 使用 HMAC-SHA256 算法验证 Webhook 回调请求的真实性

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 计算 Webhook 出站签名（HMAC-SHA256）；P1-B 修复出站/入站统一算法避免长度扩展攻击，批次 117 P1-5 改返回 Result 避免 spawn 内 panic。payload 为请求体，secret 为 HMAC key，Ok(String) 为 64 字符小写 hex 摘要，Err(String) 为初始化失败
pub fn sign_webhook_payload(payload: &str, secret: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

/// 验证 Webhook 回调签名（payload 请求体，secret 密钥，signature hex 签名；Ok(true) 验证通过，Err(AppError) 验证失败）
pub fn verify_webhook_signature(
    payload: &str,
    secret: &str,
    signature: &str,
) -> Result<bool, crate::utils::error::AppError> {
    // P1-B 修复：复用 sign_webhook_payload 计算签名，确保出站/入站使用同一份算法
    // 批次 117 P1-5：sign_webhook_payload 返回 Result，签名计算失败时返回 401
    let computed =
        sign_webhook_payload(payload, secret).map_err(crate::utils::error::AppError::internal)?;

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
