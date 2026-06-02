use chrono::{DateTime, FixedOffset, Utc};

/// 获取 UTC 固定偏移量（+00:00）
pub fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

/// 获取当前 UTC 时间（带固定偏移）
pub fn utc_now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&utc_offset())
}

/// 获取今天的开始时间（00:00:00）
pub fn today_start_utc() -> DateTime<Utc> {
    Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}
