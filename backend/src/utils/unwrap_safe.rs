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

/// 测试夹具：解析 Decimal 常量（等价于 from_f64_retain(x).expect("P9-1")，仅用于测试，生产代码严禁使用）
///
/// 注意：不使用 #[cfg(test)]，因为 #[macro_export] + #[cfg(test)] 组合导致集成测试
/// （tests/ 目录）无法通过 crate root 路径导入宏。保持无条件导出以确保可测试性。
#[macro_export]
macro_rules! dec {
    ($x:expr) => {
        rust_decimal::Decimal::from_f64_retain($x).expect("P9-1: 测试夹具 Decimal 解析失败")
    };
}

/// 测试夹具：解析 Decimal（支持字符串/整数/浮点，内部统一 to_string 后 FromStr 解析）
#[macro_export]
macro_rules! decs {
    ($x:expr) => {{
        use std::str::FromStr;
        rust_decimal::Decimal::from_str(&$x.to_string())
            .expect("P9-1: 测试夹具 Decimal 解析失败")
    }};
}

/// 测试夹具：解析日期（等价于 `NaiveDate::from_ymd_opt(y,m,d).expect("P9-1")`）
#[macro_export]
macro_rules! ymd {
    ($y:expr, $m:expr, $d:expr) => {
        chrono::NaiveDate::from_ymd_opt($y, $m, $d).expect("P9-1: 测试夹具日期解析失败")
    };
}

/// 测试夹具：解析 i64 常量（等价于 `from_str(x).expect("P9-1")`）
#[macro_export]
macro_rules! int {
    ($x:expr) => {
        $x.parse::<i64>().expect("P9-1: 测试夹具整数解析失败")
    };
}

/// 测试夹具：解析字符串常量（等价于 `String::from_str(x).expect("P9-1")`）
#[macro_export]
macro_rules! s {
    ($x:expr) => {
        String::from($x)
    };
}
