//! 格式化工具函数

#[allow(dead_code)]
/// 格式化数字为带千分位的字符串
pub fn format_number(num: i64) -> String {
    let s = num.to_string();
    let is_negative = s.starts_with('-');
    let start_idx = if is_negative { 1 } else { 0 };
    
    let int_part = &s[start_idx..];
    
    let mut result = String::new();
    if is_negative {
        result.push('-');
    }
    
    let len = int_part.len();
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    
    result
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
