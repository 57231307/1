//! 通用哈希工具
//!
//! 提供 SHA256 摘要、HMAC-SHA256 等常用哈希算法封装。
//! 本模块基于 `sha2` 与 `hmac` crate 实现，替代历史使用的 `ring` 库，
//! 以减少不必要的依赖体积。

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

/// HMAC-SHA256 类型别名
type HmacSha256 = Hmac<Sha256>;

/// 计算 SHA256 摘要并以小写 hex 字符串返回（data 待摘要字节切片，返回 64 字符小写 hex）
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// 计算 HMAC-SHA256 并以小写 hex 返回（key 密钥，data 待签名数据；Ok(String) 64 字符 hex，Err(String) 初始化失败。批次 7 改返回 Result 消除 spawn 内 panic 风险）
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(data);
    let result = mac.finalize().into_bytes();
    Ok(hex::encode(result))
}
