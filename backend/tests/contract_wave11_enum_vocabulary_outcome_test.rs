//! 枚举出参词表锁：审计严重级别与通知类型/优先级的 API/入库值必须来自唯一权威词表，
//! 禁止 Rust 枚举 Debug 形态（`format!("{:?}")`）或手工 `to_lowercase()` 归一外漏。
//!
//! ## 三处真源，任一漂移本文件即红
//! 1. 应用权威词表（运行时）：`Severity::as_str`（src/models/audit_log.rs）、
//!    `NotificationType::as_str` / `NotificationPriority::as_str`（src/models/notification.rs），
//!    变体全集经 `sea_orm::EnumIter`（通知两枚举）/显式列举（Severity 无 EnumIter）取得。
//! 2. 纸面词表解析对撞：
//!    - `audit_logs.severity` 取迁移列注释原文
//!      `migration/src/domain/production/m0026_extend_audit_log.rs` 的
//!      `'严重级别：INFO/WARN/ERROR/CRITICAL'`（该列无 CHECK，注释即 DB 侧唯一成文值域）；
//!    - 通知两枚举取 `src/models/notification.rs` 内 `#[sea_orm(string_value = "...")]`
//!      注解集合（DB 实际写入形态），与同文件 as_str 是**两处独立书写**，逐元素对撞。
//! 3. 活库目录读数（真 PostgreSQL `setup_test_db`）：
//!    `information_schema.columns` 的 column_default——notifications.priority 必须为
//!    m0014 成文的 'NORMAL'；audit_logs.severity 的 DEFAULT 取决于多支迁移落地顺序，
//!    存在时必须在词表内、缺省时打印留痕（不静默）；
//!    并查 `pg_constraint`——若上述列后续补了 CHECK，其 IN 列表必须与词表逐元素相等，
//!    当前无 CHECK 时该分支显式放行（放行条件本身打印留痕，不静默）。
//!
//! ## 行为活体证明（不是纸面比对）
//! - `build_payload_from_notification`（WebSocket 出参真实构造函数）逐变体调用，
//!   断言 `category == 变体.as_str()` 且 category 为大写权威 token（同时显式断言
//!   不等于 Debug 形态与 Debug 小写归一形态，钉死回归路径）；
//! - 源码静态锁：四个曾外漏 Debug 的站点文件（export_compliance_service.rs、
//!   permission_compliance_service.rs、notification_service.rs 两处）不得再出现
//!   对应枚举的 `format!("{:?}")` 表达式——一旦回退本锁当场红。
//!
//! ## 期望值来源（防恒真）
//! 测试内不手抄任何取值清单：合法集合一律来自运行时 as_str 枚举全集、迁移原文解析、
//! 注解解析与活库目录读数四方对撞；越界探针（Debug 形态 `Critical`、小写归一 `internal`）
//! 仅用于断言"错误形态确实不被出参函数产出"。

mod test_common;

use bingxi_backend::models::audit_log::Severity;
use bingxi_backend::models::notification::{
    Model, NotificationPriority, NotificationStatus, NotificationType,
};
use bingxi_backend::services::notification_service::build_payload_from_notification;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Iterable, Statement};
use std::collections::BTreeSet;
use test_common::setup_test_db;

const M0026_REL: &str = "migration/src/domain/production/m0026_extend_audit_log.rs";
const NOTIFICATION_MODEL_REL: &str = "src/models/notification.rs";
const EXPORT_COMPLIANCE_REL: &str = "src/services/export_compliance_service.rs";
const PERMISSION_COMPLIANCE_REL: &str = "src/services/permission_compliance_service.rs";
const NOTIFICATION_SERVICE_REL: &str = "src/services/notification_service.rs";

fn read_repo_file(rel: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败: {e}"))
}

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<sea_orm::Value>::new(),
    )
}

/// Severity 变体全集（该枚举未派生 EnumIter，显式列举；新增变体若漏补 as_str
/// 会先在 models/audit_log.rs 的穷尽 match 编译失败，此处只负责值集合对撞）
fn severity_all() -> Vec<Severity> {
    vec![
        Severity::Info,
        Severity::Warn,
        Severity::Error,
        Severity::Critical,
    ]
}

fn severity_vocab() -> BTreeSet<String> {
    severity_all()
        .iter()
        .map(|s| s.as_str().to_string())
        .collect()
}

/// 解析迁移原文中 audit_logs.severity 列注释成文的值域（`'严重级别：A/B/C/D'`）
fn parse_m0026_severity_comment() -> BTreeSet<String> {
    let text = read_repo_file(M0026_REL);
    let marker = "IS '严重级别：";
    let pos = text
        .find(marker)
        .unwrap_or_else(|| panic!("{M0026_REL} 未找到 severity 列注释标记 {marker}"));
    let rest = &text[pos + marker.len()..];
    let list = rest
        .split('\'')
        .next()
        .unwrap_or_else(|| panic!("{M0026_REL} severity 列注释引号形态异常"));
    list.split('/')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// 解析 models/notification.rs 中指定枚举块内的 `#[sea_orm(string_value = "X")]` 集合
fn parse_string_values(enum_name: &str) -> BTreeSet<String> {
    let text = read_repo_file(NOTIFICATION_MODEL_REL);
    let header = format!("pub enum {enum_name} {{");
    let start = text
        .find(&header)
        .unwrap_or_else(|| panic!("{NOTIFICATION_MODEL_REL} 未找到 {header}"));
    let body = &text[start + header.len()..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("枚举 {enum_name} 未找到结束大括号"));
    let block = &body[..end];
    block
        .lines()
        .filter_map(|line| {
            let key = "string_value = \"";
            line.find(key)
                .map(|i| &line[i + key.len()..])
                .and_then(|tail| tail.split('"').next())
                .map(|v| v.to_string())
        })
        .collect()
}

fn notification_type_vocab() -> BTreeSet<String> {
    NotificationType::iter()
        .map(|v| v.as_str().to_string())
        .collect()
}

fn notification_priority_vocab() -> BTreeSet<String> {
    NotificationPriority::iter()
        .map(|v| v.as_str().to_string())
        .collect()
}

/// 活库 information_schema 读数：列默认值 token（剥 `::character varying` 渲染并解引号）。
/// 返回 None 表示该列无 DEFAULT（audit_logs.severity 的 DEFAULT 取决于多支迁移的先后
/// 落地顺序，属 DB 侧非契约面；缺省时由调用点打印留痕，不静默吞掉）。
async fn column_default(db: &DatabaseConnection, table: &str, column: &str) -> Option<String> {
    let sql = format!(
        "SELECT column_default FROM information_schema.columns \
         WHERE table_name = '{table}' AND column_name = '{column}'"
    );
    let row = db
        .query_one_raw(pg_stmt(&sql))
        .await
        .unwrap_or_else(|e| panic!("读取 {table}.{column} 默认值失败: {e}\nSQL: {sql}"))
        .unwrap_or_else(|| {
            panic!("{table}.{column} 在 information_schema.columns 中无行\nSQL: {sql}")
        });
    let raw: Option<String> = row
        .try_get::<Option<String>>("", "column_default")
        .unwrap_or_else(|e| panic!("列 column_default 解码为文本失败: {e}\nSQL: {sql}"));
    let raw = match raw {
        Some(v) => v,
        None => return None,
    };
    let literal = raw
        .split("::")
        .next()
        .unwrap_or(raw.as_str())
        .trim()
        .to_string();
    Some(
        literal
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .unwrap_or_else(|| panic!("{table}.{column} DEFAULT 字面量形态异常: {raw}"))
            .to_string(),
    )
}

/// 活库 pg_constraint 读数：指定表/列上的 CHECK 定义原文列表（可能为空）
async fn check_constraints_on(db: &DatabaseConnection, table: &str, column: &str) -> Vec<String> {
    let sql = format!(
        "SELECT pg_get_constraintdef(c.oid) FROM pg_constraint c \
         WHERE c.contype = 'c' AND c.conrelid = '{table}'::regclass \
           AND pg_get_constraintdef(c.oid) ILIKE '%{column}%'"
    );
    let rows = db
        .query_all_raw(pg_stmt(&sql))
        .await
        .unwrap_or_else(|e| panic!("查询 {table}.{column} CHECK 失败: {e}"));
    rows.iter()
        .map(|r| {
            r.try_get::<String>("", "pg_get_constraintdef")
                .unwrap_or_else(|e| panic!("读取 CHECK 定义失败: {e}"))
        })
        .collect()
}

/// 从 CHECK 定义原文提取 IN 列表 token 集合（仅当存在 CHECK 时被调用）
fn parse_in_list(def: &str) -> Option<BTreeSet<String>> {
    let key = "IN (";
    let start = def.find(key)?;
    let rest = &def[start + key.len()..];
    let end = rest.find(')')?;
    Some(
        rest[..end]
            .split(',')
            .filter_map(|t| {
                let t = t.trim();
                t.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))
            })
            .map(|s| s.to_string())
            .collect(),
    )
}

#[test]
fn severity_vocab_equals_migration_comment() {
    let runtime = severity_vocab();
    let paper = parse_m0026_severity_comment();
    assert_eq!(
        runtime, paper,
        "Severity::as_str 集合与迁移列注释值域漂移: as_str={runtime:?} 注释={paper:?}"
    );
    // Display 与 as_str 必须逐字一致（出参链路混用两种调用形态）
    for s in severity_all() {
        assert_eq!(
            s.to_string(),
            s.as_str(),
            "Severity Display 与 as_str 漂移: {s:?}"
        );
    }
}

#[test]
fn notification_type_vocab_equals_sea_orm_string_values() {
    let runtime = notification_type_vocab();
    let orm = parse_string_values("NotificationType");
    assert_eq!(
        runtime, orm,
        "NotificationType::as_str 与 #[sea_orm(string_value)] 集合漂移: as_str={runtime:?} string_value={orm:?}"
    );
}

#[test]
fn notification_priority_vocab_equals_sea_orm_string_values() {
    let runtime = notification_priority_vocab();
    let orm = parse_string_values("NotificationPriority");
    assert_eq!(
        runtime, orm,
        "NotificationPriority::as_str 与 #[sea_orm(string_value)] 集合漂移: as_str={runtime:?} string_value={orm:?}"
    );
}

#[test]
fn ws_payload_category_uses_authority_vocab_not_debug() {
    let vocab = notification_type_vocab();
    for t in NotificationType::iter() {
        let model = Model {
            id: 1,
            user_id: 1,
            notification_type: t.clone(),
            title: "t".to_string(),
            content: "c".to_string(),
            priority: NotificationPriority::Normal,
            status: NotificationStatus::Unread,
            business_type: None,
            business_id: None,
            action_url: None,
            sender_id: None,
            sender_name: None,
            read_at: None,
            processed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            dedup_key: None,
        };
        let payload = build_payload_from_notification(&model);
        assert_eq!(
            payload.category,
            t.as_str(),
            "WS 出参 category 必须等于权威词表 token（变体 {t:?}）"
        );
        assert!(
            vocab.contains(&payload.category),
            "category 越出词表: {}",
            payload.category
        );
        let debug_form = format!("{t:?}");
        assert_ne!(
            payload.category, debug_form,
            "category 回退为 Debug 形态（{debug_form}），禁止"
        );
        assert_ne!(
            payload.category,
            debug_form.to_lowercase(),
            "category 回退为 Debug 小写归一形态，禁止"
        );
    }
}

#[test]
fn debug_leak_sites_stay_closed() {
    // 静态锁：四个曾外漏站点不得再出现对应枚举的 Debug 形态表达式
    let export = read_repo_file(EXPORT_COMPLIANCE_REL);
    assert!(
        !export.contains("format!(\"{:?}\", alert.severity)"),
        "{EXPORT_COMPLIANCE_REL} 重新出现 severity 的 Debug 形态外漏"
    );
    assert!(
        export.contains("\"severity\": alert.severity.as_str()"),
        "{EXPORT_COMPLIANCE_REL} 出参未走权威 as_str"
    );
    let permission = read_repo_file(PERMISSION_COMPLIANCE_REL);
    assert!(
        !permission.contains("format!(\"{:?}\", alert.severity)"),
        "{PERMISSION_COMPLIANCE_REL} 重新出现 severity 的 Debug 形态外漏"
    );
    assert!(
        permission.contains("\"severity\": alert.severity.as_str()"),
        "{PERMISSION_COMPLIANCE_REL} 出参未走权威 as_str"
    );
    let notify = read_repo_file(NOTIFICATION_SERVICE_REL);
    assert!(
        !notify.contains("format!(\"{:?}\", n.notification_type)"),
        "{NOTIFICATION_SERVICE_REL} 重新出现 notification_type 的 Debug 形态外漏"
    );
    assert!(
        !notify.contains("format!(\"{:?}\", notification.priority)"),
        "{NOTIFICATION_SERVICE_REL} 重新出现 priority 的 Debug 形态外漏"
    );
    assert!(
        notify.contains("category: n.notification_type.as_str()"),
        "{NOTIFICATION_SERVICE_REL} WS 出参未走权威 as_str"
    );
    assert!(
        notify.contains("\"priority\": notification.priority.as_str()"),
        "{NOTIFICATION_SERVICE_REL} webhook 出参未走权威 as_str"
    );
}

#[tokio::test]
async fn live_db_default_and_check_align_with_vocab() {
    let db = setup_test_db().await;

    // DEFAULT 读数：存在则必须落在词表内；缺省（多支迁移落地顺序所致）打印留痕不静默
    if let Some(sev_default) = column_default(&db, "audit_logs", "severity").await {
        assert!(
            severity_vocab().contains(&sev_default),
            "audit_logs.severity DEFAULT '{sev_default}' 不在 Severity::as_str 词表内"
        );
    } else {
        println!("留痕：audit_logs.severity 当前无 DEFAULT，DB 侧值域对撞由迁移列注释（纸面）承担");
    }
    let pri_default = column_default(&db, "notifications", "priority").await;
    assert_eq!(
        pri_default.as_deref(),
        Some("NORMAL"),
        "notifications.priority 的 DEFAULT 应为 m0014 成文的 'NORMAL'，实测 {pri_default:?}"
    );

    // CHECK 对撞：当前两列均无 CHECK；若未来补加，其 IN 列表必须与词表逐元素相等
    for (table, column, vocab, vocab_name) in [
        (
            "audit_logs",
            "severity",
            severity_vocab(),
            "Severity::as_str",
        ),
        (
            "notifications",
            "priority",
            notification_priority_vocab(),
            "NotificationPriority::as_str",
        ),
        (
            "notifications",
            "notification_type",
            notification_type_vocab(),
            "NotificationType::as_str",
        ),
    ] {
        let defs = check_constraints_on(&db, table, column).await;
        if defs.is_empty() {
            println!(
                "留痕：{table}.{column} 当前无 CHECK 约束，词表对撞由 DEFAULT + 注释/注解两源完成"
            );
        }
        for def in &defs {
            let allowed = parse_in_list(def)
                .unwrap_or_else(|| panic!("{table}.{column} CHECK 定义无法解析 IN 列表: {def}"));
            assert_eq!(
                allowed, vocab,
                "{table}.{column} 的 CHECK 允许值与 {vocab_name} 词表漂移: CHECK={allowed:?} 词表={vocab:?}"
            );
        }
    }
}
