//! P9-1 unwrap 清理统一工具
//!
//! 提供业务化的宏与函数，把散落在各处的 `.unwrap()` / `.expect(...)` 调用
//! 集中到此模块，使关键路径上的错误处理更显式、更易排查。
//!
//! 设计原则：
//! 1. 业务关键路径的 unwrap 必须改写为 Result 风格，本文件不为其提供 helper。
//! 2. 测试夹具（已知合法常量）使用 `dec!`/`decs!`/`date!`/`ymd!` 等宏替代 `.unwrap()`，
//!    宏定义在编译期展开为内部 expect，业务上等同于"立即失败+友好中文提示"。
//! 3. 所有 helper/宏命名遵循"中文含义 + 英文短名"约定，名称不超过 9 个英文字符。

/// 测试夹具：解析 Decimal 常量（等价于 `from_f64_retain(x).expect("P9-1")`）
///
/// 仅用于 `#[cfg(test)]` 模块。生产代码严禁使用。
#[cfg(test)]
#[macro_export]
macro_rules! dec {
    ($x:expr) => {
        rust_decimal::Decimal::from_f64_retain($x).expect("P9-1: 测试夹具 Decimal 解析失败")
    };
}

/// 测试夹具：解析 Decimal 字符串（等价于 `Decimal::from_str(s).expect("P9-1")`）
///
/// 调用方需自行 `use std::str::FromStr;`。
#[cfg(test)]
#[macro_export]
macro_rules! decs {
    ($x:expr) => {
        rust_decimal::Decimal::from_str($x).expect("P9-1: 测试夹具 Decimal 字符串解析失败")
    };
}

/// 测试夹具：解析日期（等价于 `NaiveDate::from_ymd_opt(y,m,d).expect("P9-1")`）
#[cfg(test)]
#[macro_export]
macro_rules! ymd {
    ($y:expr, $m:expr, $d:expr) => {
        chrono::NaiveDate::from_ymd_opt($y, $m, $d).expect("P9-1: 测试夹具日期解析失败")
    };
}

/// 测试夹具：解析 i64 常量（等价于 `from_str(x).expect("P9-1")`）
#[cfg(test)]
#[macro_export]
macro_rules! int {
    ($x:expr) => {
        $x.parse::<i64>().expect("P9-1: 测试夹具整数解析失败")
    };
}

/// 测试夹具：解析字符串常量（等价于 `String::from_str(x).expect("P9-1")`）
#[cfg(test)]
#[macro_export]
macro_rules! s {
    ($x:expr) => {
        String::from($x)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dec_macro() {
        // P9-1: 用宏替代散落的 expect，验证宏行为
        let v = dec!(1000.0);
        assert_eq!(v.to_string(), "1000");
    }

    #[test]
    fn test_int_macro() {
        let v: i64 = int!("42");
        assert_eq!(v, 42);
    }

    #[test]
    fn test_s_macro() {
        let v: String = s!("hello-p9-1");
        assert_eq!(v, "hello-p9-1");
    }
}
