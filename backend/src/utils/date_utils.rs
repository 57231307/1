use chrono::{DateTime, FixedOffset, Utc};

/// 获取 UTC 固定偏移量（+00:00）
///
/// 批次 117 P1-5 修复：原 `.unwrap()` 改为 `expect` + 不变量注释。
/// `FixedOffset::east_opt(0)` 在秒数 |0| <= 86_400 时永远返回 Some，
/// 0 永远合法，此处为数学不变量，expect 不会触发 panic。
pub fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).expect("不变量：east_opt(0) 永远合法（|0| <= 86400）")
}

/// 获取当前 UTC 时间（带固定偏移）
pub fn utc_now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&utc_offset())
}

/// 获取今天的开始时间（00:00:00）
///
/// 批次 117 P1-5 修复：原 `.unwrap()` 改为 `expect` + 不变量注释。
/// `and_hms_opt(0, 0, 0)` 在 0<=h<24 && 0<=m<60 && 0<=s<60 时永远合法，
/// (0,0,0) 永远合法，此处为数学不变量，expect 不会触发 panic。
pub fn today_start_utc() -> DateTime<Utc> {
    Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("不变量：and_hms_opt(0, 0, 0) 永远合法（0<24, 0<60, 0<60）")
        .and_utc()
}
