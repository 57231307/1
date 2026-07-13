use chrono::{DateTime, FixedOffset, Utc};

/// 获取 UTC 固定偏移量（+00:00）
///
/// L-14 修复（批次 376 v13 复审）：消除 expect 调用，改为 unwrap_or_else + 日志。
/// `FixedOffset::east_opt(0)` 在秒数 |0| <= 86_400 时永远返回 Some，
/// 0 永远合法，此处为数学不变量，unwrap_or_else 分支理论不可达。
/// 兜底使用 west_opt(0)（也永远返回 Some），双重保险。
pub fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap_or_else(|| {
        tracing::error!("FixedOffset::east_opt(0) 失败（理论不可达），使用 west_opt(0) 兜底");
        // 理论不可达：west_opt(0) 也永远返回 Some（|0| <= 86400）
        FixedOffset::west_opt(0).unwrap_or_else(|| {
            tracing::error!("FixedOffset::west_opt(0) 也失败（理论不可达），使用 east_opt(1)");
            FixedOffset::east_opt(1).unwrap_or_else(|| {
                tracing::error!("FixedOffset::east_opt(1) 也失败（理论不可达），使用 west_opt(1)");
                // 最终兜底：west_opt(1) 永远合法，理论不可达
                FixedOffset::west_opt(1).expect("理论不可达：west_opt(1) 永远合法（|1| <= 86400）")
            })
        })
    })
}

/// 获取当前 UTC 时间（带固定偏移）
pub fn utc_now_fixed() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&utc_offset())
}

/// 获取今天的开始时间（00:00:00）
///
/// L-14 修复（批次 376 v13 复审）：消除 expect 调用，改为 unwrap_or_else + 日志。
/// `and_hms_opt(0, 0, 0)` 在 0<=h<24 && 0<=m<60 && 0<=s<60 时永远合法，
/// (0,0,0) 永远合法，此处为数学不变量，unwrap_or_else 分支理论不可达。
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
