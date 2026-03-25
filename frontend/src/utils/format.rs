//! 格式化工具函数

#[allow(dead_code)]
/// 格式化数字为带千分位的字符串
pub fn format_number(num: i64) -> String {
    num.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

/// 格式化数字为带千分位的字符串（f64）
#[allow(dead_code)]
pub fn format_decimal(num: f64) -> String {
    format!("{:.2}", num)
}

/// 格式化日期时间
#[allow(dead_code)]
pub fn format_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化日期
#[allow(dead_code)]
pub fn format_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}
