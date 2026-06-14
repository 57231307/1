//! 随机数工具模块
//!
//! 统一随机数生成函数，避免代码重复和不一致的随机数实现
use fastrand;

/// 生成 4 位随机数（0-9999）
pub fn random_4_digit() -> u16 {
    fastrand::u16(0..10000)
}

/// 生成 6 位随机数（100000-999999）
pub fn random_6_digit() -> u32 {
    fastrand::u32(100000..1_000_000)
}

/// 生成指定长度的字母数字随机字符串
pub fn random_alphanumeric(length: usize) -> String {
    (0..length)
        // fastrand::alphanumeric() 已直接返回 char，无需再 cast
        .map(|_| fastrand::alphanumeric())
        .collect()
}
