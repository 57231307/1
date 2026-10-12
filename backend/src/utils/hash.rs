//! 通用哈希工具
//!
//! 提供 HMAC-SHA256 签名封装，基于 `sha2` 与 `hmac` crate 实现，
//! 替代历史使用的 `ring` 库以减少依赖体积。

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// HMAC-SHA256 类型别名
type HmacSha256 = Hmac<Sha256>;

/// 计算 HMAC-SHA256 并以小写 hex 返回（key 密钥，data 待签名数据；
/// Ok(String) 为 64 字符小写 hex，Err(String) 为密钥长度不合法等初始化失败）
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(data);
    let result = mac.finalize().into_bytes();
    Ok(hex::encode(result))
}
