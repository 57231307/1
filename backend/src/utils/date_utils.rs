use chrono::{DateTime, FixedOffset, Utc};

/// 获取 UTC 固定偏移量（+00:00）；east_opt(0) 永远返回 Some 为数学不变量（|0| <= 86400）
pub fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap_or_else(|| {
        tracing::error!("FixedOffset::east_opt(0) 失败（理论不可达），使用 west_opt(0) 兜底");
        // 理论不可达：west_opt(0) 也永远返回 Some（|0| <= 86400），等价于 UTC +00:00
        FixedOffset::west_opt(0).expect("理论不可达：west_opt(0) 永远合法（|0| <= 86400）")
    })
}

/// 获取当前 UTC 时间（带固定偏移）
pub fn utc_now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&utc_offset())
}

/// 获取今天的开始时间（00:00:00）；L-14 修复消除 expect 改 unwrap_or_else+日志，and_hms_opt(0,0,0) 永远合法为数学不变量
pub fn today_start_utc() -> DateTime<Utc> {
    Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| {
            tracing::error!("and_hms_opt(0, 0, 0) 失败（理论不可达），使用默认时间兜底");
            // 理论不可达：(0,0,0) 永远合法，兜底用 NaiveDateTime 默认值（Unix 纪元）
            chrono::NaiveDateTime::default()
        })
        .and_utc()
}
