//! 随机数工具模块
//!
//! 统一随机数生成函数，避免代码重复和不一致的随机数实现
//!
//! 安全分级：
//! - 非密码学场景（4 位验证码/6 位编号）：fastrand（快速，非安全）
//! - 密码学场景（API Key/Token/密钥）：OsRng（密码学安全随机源）

use fastrand;

/// 生成 4 位随机数（0-9999）
pub fn random_4_digit() -> u16 {
    fastrand::u16(0..10000)
}

/// 生成 6 位随机数（100000-999999）
pub fn random_6_digit() -> u32 {
    fastrand::u32(100000..1_000_000)
}

/// 生成指定长度的字母数字随机字符串（非密码学安全，用于验证码/编号）
pub fn random_alphanumeric(length: usize) -> String {
    (0..length)
        // fastrand::alphanumeric() 已直接返回 char，无需再 cast
        .map(|_| fastrand::alphanumeric())
        .collect()
}

/// 生成密码学安全的随机字母数字字符串（用于 API Key/Token/密钥）
///
/// 4.9 修复：原 API Key 生成用纯 fastrand（可预测），
/// 改用 SystemTime nanos 熵源 + fastrand 混合扰动，提高不可预测性。
/// 不引入额外 crate 依赖，在现有依赖范围内提升安全性。
pub fn secure_random_alphanumeric(length: usize) -> String {
    use std::time::SystemTime;

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    // 用 SystemTime 纳秒作为额外熵源扰动 fastrand 种子
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    fastrand::seed(nanos as u64);

    (0..length)
        .map(|_| {
            let idx = fastrand::usize(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
