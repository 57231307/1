//! 随机数工具模块
//!
//! 统一随机数生成函数，避免代码重复和不一致的随机数实现。
//!
//! 本模块只提供**非密码学**随机（4 位验证码、6 位编号这类"猜中无收益"的场景）：
//! 需要密码学安全的随机串（令牌、密钥类）不要用这里，应使用 OsRng 直接取字节。

use fastrand;

/// 生成 4 位随机数（0-9999）
pub fn random_4_digit() -> u16 {
    fastrand::u16(0..10000)
}

/// 生成指定长度的字母数字随机字符串（非密码学安全，用于验证码/编号）
// 后续接入 SchedulerRegistry/StateMachine 时会使用（保留作非密码学场景用）
#[allow(dead_code)]
pub fn random_alphanumeric(length: usize) -> String {
    (0..length)
        // fastrand::alphanumeric() 已直接返回 char，无需再 cast
        .map(|_| fastrand::alphanumeric())
        .collect()
}
