//! BPM 监控统计聚合列的解码契约锁（`GET /bpm/monitor/stats` 的服务层真身）。
//!
//! 钉住的不变量（`bpm_ops/monitor.rs::get_monitor_stats`）：
//! 1. 平均处理时长聚合列的类型对应关系：PostgreSQL 的 `AVG` 作用于 numeric 表达式
//!    （`EXTRACT(EPOCH FROM interval)` 在 PG14+ 返回 numeric）产出列即 numeric，
//!    SQL 侧必须显式 `::float8`，与 Rust 解码类型 `Option<f64>` 逐字符对上，
//!    否则在 sqlx 解码期即报 `mismatched types` 整个端点 500。
//! 2. 出参线形态：`avg_process_duration_minutes` 是时长度量统计（非金额/比率），
//!    前端（`frontend/src/types/bpm.ts`）按 `number | null` 消费、页面以 `Math.round`
//!    展示，因此序列化必须是 JSON number，无样本时是 JSON null——不许漂成字符串，
//!    也不许把 NULL 兜底成 0.0（那会把 "无已完成实例" 与 "平均耗时为 0" 混为一谈）。
//! 3. 空表场景：聚合零行时 SQL 返回单行 NULL，接口必须成功返回且 avg 为 None，
//!    而不是 unwrap 崩溃或解码失败。
//!
//! 活库用例依赖 `test_common::setup_test_db()`（TEST_DATABASE_URL 指向已迁移
//! PostgreSQL，属 CI 活库串行组），本地无库时该组整体不执行，判据由 CI 兑付；
//! 静态形态锁不依赖活库，常规 CI 必跑。

mod test_common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use bingxi_backend::models::status::bpm_instance as instance_status;
use bingxi_backend::models::{bpm_process_definition, bpm_process_instance};
use bingxi_backend::services::bpm_service::BpmService;
use chrono::{TimeZone, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::Value;

// ---------------------------------------------------------------------------
// 1) 静态形态锁：SQL 产出列型与 Rust 解码型必须成对出现，缺一不可
// ---------------------------------------------------------------------------

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

#[test]
fn monitor_avg_column_casts_float8_and_decodes_f64_in_pair() {
    let src = read_rel("src/services/bpm_ops/monitor.rs").replace('\r', "");
    assert!(
        src.contains("AVG(EXTRACT(EPOCH FROM (completed_at - started_at)) / 60)::float8"),
        "平均时长聚合列必须在 SQL 侧显式 ::float8，与解码类型对齐（AVG 过 numeric 产出 numeric）"
    );
    assert!(
        src.contains("into_tuple::<Option<f64>>()"),
        "解码端必须按 Option<f64> 取该列（float8 可空列）；若改列语义须两侧同步改，本锁逼显式处理"
    );
}

// ---------------------------------------------------------------------------
// 2) 活库行为锁：空表返回明确 null；有样本时解码为 number 且数值正确
// ---------------------------------------------------------------------------

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间，None 不可达")
}

async fn seed_definition(db: &DatabaseConnection) -> i32 {
    let now = Utc::now();
    bpm_process_definition::ActiveModel {
        name: Set("监控统计锁流程".to_string()),
        code: Set(format!("itest-monitor-{}", unique_tag())),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
    .id
}

/// 种一条实例行：状态取写入方词表常量；`completed_at` 为 None 即进行中样本。
async fn seed_instance(
    db: &DatabaseConnection,
    definition_id: i32,
    status: &str,
    started_at: Option<chrono::DateTime<Utc>>,
    completed_at: Option<chrono::DateTime<Utc>>,
) {
    let now = Utc::now();
    bpm_process_instance::ActiveModel {
        instance_no: Set(format!("BPI-IT-{}", unique_tag())),
        process_definition_id: Set(definition_id),
        business_type: Set("itest".to_string()),
        applicant_id: Set(1),
        title: Set("监控统计锁实例".to_string()),
        status: Set(Some(status.to_string())),
        initiator_id: Set(1),
        initiator_name: Set("itest".to_string()),
        started_at: Set(started_at),
        completed_at: Set(completed_at),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 夹具已 TRUNCATE 业务表：全表聚合在活库串行组内即本用例自种样本的口径。
#[tokio::test]
async fn monitor_stats_empty_tables_return_explicit_null_avg() {
    let db = Arc::new(test_common::setup_test_db().await);
    let svc = BpmService::new(db.clone());

    let stats = svc
        .get_monitor_stats()
        .await
        .expect("空表时监控统计必须成功返回，聚合零行解 NULL 不允许变成解码错误或 panic");

    assert_eq!(stats.total_instances, 0);
    assert_eq!(stats.completed_instances, 0);
    assert_eq!(stats.total_tasks, 0);
    assert_eq!(
        stats.avg_process_duration_minutes, None,
        "无已完成样本时平均时长必须是 None（明确的无数据语义），禁止兜底成 0.0"
    );

    let json = serde_json::to_value(&stats).unwrap();
    assert_eq!(
        json["avg_process_duration_minutes"],
        Value::Null,
        "出参线形态锁：空样本必须序列化为 JSON null，与前端 `number | null` 声明一致"
    );
}

#[tokio::test]
async fn monitor_stats_avg_decodes_seeded_durations_as_json_number() {
    let db = Arc::new(test_common::setup_test_db().await);
    let definition_id = seed_definition(&db).await;

    // 两个已完成样本：30 分钟与 90 分钟，均值恰为 60.0；
    // 一个进行中样本（completed_at 为空）不进 AVG 分母，钉住过滤口径。
    let base = Utc.with_ymd_and_hms(2026, 1, 10, 0, 0, 0).unwrap();
    seed_instance(
        &db,
        definition_id,
        instance_status::COMPLETED,
        Some(base),
        Some(base + chrono::Duration::minutes(30)),
    )
    .await;
    seed_instance(
        &db,
        definition_id,
        instance_status::COMPLETED,
        Some(base + chrono::Duration::days(1)),
        Some(base + chrono::Duration::days(1) + chrono::Duration::minutes(90)),
    )
    .await;
    seed_instance(
        &db,
        definition_id,
        instance_status::PROCESSING,
        Some(base + chrono::Duration::days(2)),
        None,
    )
    .await;

    let svc = BpmService::new(db.clone());
    let stats = svc
        .get_monitor_stats()
        .await
        .expect("AVG(numeric)::float8 列必须可按 Option<f64> 解码");

    assert_eq!(stats.total_instances, 3);
    assert_eq!(stats.completed_instances, 2);
    assert_eq!(stats.processing_instances, 1);
    let avg = stats
        .avg_process_duration_minutes
        .expect("有两个已完成样本，平均时长必须解出实值而非 None");
    assert!(
        (avg - 60.0).abs() < 1e-9,
        "平均处理时长应为 (30+90)/2=60 分钟，实际 {avg}"
    );

    let json = serde_json::to_value(&stats).unwrap();
    assert!(
        json["avg_process_duration_minutes"].is_number(),
        "出参线形态锁：平均时长必须是 JSON number（前端 Math.round 直接消费），实际 {json}"
    );
}
