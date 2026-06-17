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

/// 业务 helper：把 Option<T> 在业务已知非空的场景下取值，失败时记录中文日志
///
/// 仅适用于"业务上不可能为空"的字段（如配置加载后的全局变量、初始化后必填）。
/// 关键路径（请求处理、数据库事务）必须显式 match，不允许使用本函数。
pub fn must_some<T>(opt: Option<T>, ctx: &str) -> T {
    match opt {
        Some(v) => v,
        None => {
            tracing::error!(ctx = %ctx, "P9-1: 业务必填项为空");
            panic!("P9-1: 业务必填项为空: {ctx}")
        }
    }
}

/// 业务 helper：Result 在已知成功的场景下取值，失败时记录中文日志
pub fn must_ok<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> T {
    match r {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(ctx = %ctx, error = %e, "P9-1: 业务操作预期成功但失败");
            panic!("P9-1: {ctx}: {e}")
        }
    }
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

    #[test]
    fn test_must_some_ok() {
        let v = must_some(Some(1), "test ctx");
        assert_eq!(v, 1);
    }

    #[test]
    #[should_panic(expected = "P9-1")]
    fn test_must_some_panic() {
        let _: i32 = must_some(None, "test");
    }

    #[test]
    fn test_must_ok_ok() {
        let v: i32 = must_ok::<_, &str>(Ok(7), "test");
        assert_eq!(v, 7);
    }

    #[test]
    #[should_panic(expected = "P9-1")]
    fn test_must_ok_panic() {
        let _: i32 = must_ok::<_, &str>(Err("bad"), "test");
    }
}
