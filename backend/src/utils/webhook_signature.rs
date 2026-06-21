//! Webhook 签名验证工具
//!
//! 使用 HMAC-SHA256 算法验证 Webhook 回调请求的真实性

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 计算 Webhook 出站签名（HMAC-SHA256）
///
/// P1-B 修复：出站与入站使用同一份 HMAC-SHA256 实现，
/// 避免旧实现 `SHA256(body || secret)` 的长度扩展攻击风险。
///
/// # 参数
/// - `payload`: 请求体原始内容
/// - `secret`: Webhook 密钥（作为 HMAC key）
///
/// # 返回
/// - hex 编码的 HMAC-SHA256 摘要（64 字符小写）
pub fn sign_webhook_payload(payload: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC 接受任意长度密钥，初始化不会失败");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

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
    // P1-B 修复：复用 sign_webhook_payload 计算签名，确保出站/入站使用同一份算法
    let computed = sign_webhook_payload(payload, secret);

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
