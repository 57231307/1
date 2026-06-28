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
/// - `Ok(String)`: 长度为 64 的小写 hex 字符串
/// - `Err(String)`: HMAC 初始化失败（密钥长度异常等）
///
/// # 设计变更（批次 7，2026-06-28）
/// - 原实现 `.expect("HMAC 初始化失败")` 在 spawn 任务内构成 panic 触发点
///   （omni_audit_service.rs:74 spawn 通过 crate::utils::hash::hmac_sha256_hex 调用），
///   若触发会导致审计引擎整个 spawn 任务死亡。
/// - 改为返回 Result，让调用方决定降级策略，消除 panic 风险。
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(data);
    let result = mac.finalize().into_bytes();
    Ok(hex::encode(result))
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
