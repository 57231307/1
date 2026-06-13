//! 通用哈希工具
//!
//! 提供 SHA256 摘要、HMAC-SHA256 等常用哈希算法封装。
//! 本模块基于 `sha2` 与 `hmac` crate 实现，替代历史使用的 `ring` 库，
//! 以减少不必要的依赖体积。

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

/// HMAC-SHA256 类型别名
type HmacSha256 = Hmac<Sha256>;

/// 计算 SHA256 摘要并以小写 hex 字符串返回
///
/// # 参数
/// - `data`: 待摘要的字节切片
///
/// # 返回
/// - 长度为 64 的小写 hex 字符串
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// 计算 HMAC-SHA256 并以小写 hex 字符串返回
///
/// # 参数
/// - `key`: HMAC 密钥字节切片
/// - `data`: 待签名的数据字节切片
///
/// # 返回
/// - 长度为 64 的小写 hex 字符串
///
/// # Panics
/// - 当 HMAC 内部初始化失败时（理论上不会发生，因为 SHA256 接受任意长度密钥）
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 初始化失败");
    mac.update(data);
    let result = mac.finalize().into_bytes();
    hex::encode(result)
}

/// 计算 HMAC-SHA256 并返回原始字节
///
/// # 参数
/// - `key`: HMAC 密钥字节切片
/// - `data`: 待签名的数据字节切片
///
/// # 返回
/// - 长度为 32 的字节数组
///
/// # Panics
/// - 当 HMAC 内部初始化失败时（理论上不会发生，因为 SHA256 接受任意长度密钥）
pub fn hmac_sha256_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 初始化失败");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// 计算多段数据拼接后的 SHA256 摘要并以小写 hex 字符串返回
///
/// 与 `sha256_hex` 等价，但支持按顺序拼接多段数据后统一摘要。
/// 常用于 `SHA256(a || b || c ...)` 这类组合摘要场景。
///
/// # 参数
/// - `parts`: 按顺序拼接的字节切片集合
///
/// # 返回
/// - 长度为 64 的小写 hex 字符串
pub fn sha256_hex_multi(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    let result = hasher.finalize();
    hex::encode(result)
}

