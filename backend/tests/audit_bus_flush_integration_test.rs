//! 审计总线 flush 可验证性集成测试。
//!
//! 两段断：
//! 一，总线层灌 N 条导出类事件后 flush 断 audit_logs 行数严格等于 N 且 failed 等于 0。
//! 二，handler 层复刻 inventory_stock_handler 的 record_export_audit 事件形状，
//!    flush 后断库里存在 operation_type 为 EXPORT 且 resource_type 对应的行。
//!
//! 源码普查锁：断言 record_export_audit 构造事件时使用的资源名与路径字面量
//! 与测试中的常量一致，防止影子实现家族再次出现。
//!
//! 关停链路锁：断言优雅关停确实排空审计总线，且排空排在全部生产者停止之后，
//! 防止 flush 退化回"只有测试会调用"的影子实现形态。
//!
//! 连接归属：全局消费者不绑定首次 `AuditLogService::new` 传入的库连接，落库用的是每枚事件
//! 载荷自带的那枚连接，消费者仍只有进程级一份。因此同一测试二进制内多个用例各建各的连接池
//! 时，各自的事件落到各自的池，不会串到别个用例的连接上。
//! 执行形态仍按 db-integration 串行组：夹具 `setup_test_db` 每例 TRUNCATE 业务表，用例之间
//! 不得并发；本文件对全局累计计数的严格等号断言要求每例独占进程，本地逐例分次执行等价于
//! 该隔离形态。

mod test_common;

use bingxi_backend::models::audit_log::{self, Column, OperationType, Severity};
use bingxi_backend::services::audit_log_service::{self, AuditEvent, AuditLogService};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use std::sync::Arc;
use std::time::Duration;
use test_common::setup_test_db;

/// 与 record_export_audit 构造事件时使用的资源类型字面量。
const EXPORT_RESOURCE_TYPE: &str = "inventory_stock";
/// 与 record_export_audit 构造事件时使用的请求路径字面量。
const EXPORT_REQUEST_PATH: &str = "/api/v1/erp/inventory/stock/export";

/// 构造与 record_export_audit 形状一致的导出事件（不依赖审批令牌门，无 ctx）。
fn make_export_event(seq: u64) -> AuditEvent {
    AuditEvent {
        user_id: Some(1),
        username: Some(format!("auditbus_test_user_{seq}")),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some(EXPORT_RESOURCE_TYPE.to_string()),
        resource_id: None,
        resource_name: Some(format!("inventory_stock_export_{seq}.xlsx")),
        description: Some(format!(
            "用户 auditbus_test_user_{seq} 导出库存列表（共 1 条）"
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(EXPORT_REQUEST_PATH.to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": 1,
        })),
    }
}

/// 第一段：总线层灌 N 条导出类事件，flush 后断 audit_logs 行数严格等于 N 且 failed 等于 0。
#[tokio::test]
async fn bus_flush_inserts_exactly_n_export_events() {
    let db = Arc::new(setup_test_db().await);
    let svc = Arc::new(AuditLogService::new(db.clone()));

    let n: u64 = 5;
    for i in 0..n {
        let event = make_export_event(i);
        svc.clone().record_async(event, None);
    }

    let counts = audit_log_service::flush(Duration::from_secs(30)).await;
    assert_eq!(
        counts.inserted, n,
        "灌入 {n} 条导出事件，flush 后 inserted 必须严格等于 {n}"
    );
    assert_eq!(counts.failed, 0, "flush 后 failed 必须为 0，不允许放宽");
    assert_eq!(counts.panicked, 0, "flush 后 panicked 必须为 0");

    // 库侧断言行数严格等于 n（setup_test_db 已 TRUNCATE audit_logs，从零起算）
    let db_count: u64 = audit_log::Entity::find()
        .filter(Column::OperationType.eq(OperationType::Export.as_str()))
        .count(db.as_ref())
        .await
        .expect("统计 audit_logs EXPORT 行数失败");
    assert_eq!(
        db_count, n,
        "库中 operation_type=EXPORT 行数必须严格等于灌入数 {n}"
    );
}

/// 第二段：复刻 record_export_audit 形状的事件走总线，flush 后断库里存在
/// operation_type=EXPORT 且 resource_type 对应的行。
#[tokio::test]
async fn handler_export_shape_lands_audit_row_in_db() {
    let db = Arc::new(setup_test_db().await);
    let svc = Arc::new(AuditLogService::new(db.clone()));

    let event = make_export_event(99);
    svc.clone().record_async(event, None);

    let counts = audit_log_service::flush(Duration::from_secs(30)).await;
    assert_eq!(
        counts.inserted, 1,
        "单条导出事件 flush 后 inserted 必须为 1"
    );
    assert_eq!(counts.failed, 0, "flush 后 failed 必须为 0");

    // 断库里存在 operation_type=EXPORT 且 resource_type=inventory_stock 的行
    let row = audit_log::Entity::find()
        .filter(Column::OperationType.eq(OperationType::Export.as_str()))
        .filter(Column::ResourceType.eq(EXPORT_RESOURCE_TYPE))
        .one(db.as_ref())
        .await
        .expect("查询 audit_logs 单行失败");
    assert!(
        row.is_some(),
        "导出链路 flush 后库里必须存在 operation_type=EXPORT 且 resource_type={EXPORT_RESOURCE_TYPE} 的行"
    );

    // 进一步验证 request_path 字段与 record_export_audit 一致
    let found = row.unwrap();
    assert_eq!(
        found.request_path.as_deref(),
        Some(EXPORT_REQUEST_PATH),
        "落库行的 request_path 必须与 record_export_audit 使用的路径字面量一致"
    );
}

/// 源码普查锁：断言 record_export_audit 内使用的 resource_type 与 request_path
/// 字面量与测试常量保持同步。若 handler 侧字面量漂移而本测试未同步更新，此锁必红。
#[test]
fn record_export_audit_source_literals_locked() {
    let source = include_str!("../src/handlers/inventory_stock_handler.rs");

    // resource_type 字面量
    let rt_pattern = format!(r#"resource_type: Some("{EXPORT_RESOURCE_TYPE}".to_string())"#);
    assert!(
        source.contains(&rt_pattern),
        "record_export_audit 内 resource_type 字面量已漂移，期望 {EXPORT_RESOURCE_TYPE:?}，\
         必须同步更新本测试的 EXPORT_RESOURCE_TYPE 常量"
    );

    // request_path 字面量
    let path_pattern = format!(r#"request_path: Some("{EXPORT_REQUEST_PATH}".to_string())"#);
    assert!(
        source.contains(&path_pattern),
        "record_export_audit 内 request_path 字面量已漂移，期望 {EXPORT_REQUEST_PATH:?}，\
         必须同步更新本测试的 EXPORT_REQUEST_PATH 常量"
    );

    // operation_type 枚举变体
    assert!(
        source.contains("operation_type: OperationType::Export"),
        "record_export_audit 内 operation_type 不再是 OperationType::Export，\
         本测试的事件形状前提失效"
    );
}

/// 关停链路源码锁：优雅关停必须真实排空审计总线，且排空排在全部生产者停止之后。
///
/// 生产路径一旦不排空，队列里积压的审计事件就随进程退出永久丢失，而这种丢失
/// 不会让任何现有用例变红（事件是在退出那一刻才丢的）。因此把前提钉在源码上：
/// 删掉关停路径里的 flush 调用、或把它挪到生产者停止之前，本锁必红。
#[test]
fn shutdown_path_drains_audit_bus_after_producers_stop() {
    let source = include_str!("../src/main.rs");

    let body = fn_body(source, "async fn shutdown_resources").expect(
        "main.rs 里找不到 async fn shutdown_resources：关停函数已改名或去掉 async，\
         本锁的前提失效，需同步调整本测试与关停链路",
    );

    // 一，关停路径确实排空总线，而不是只在本测试里调用
    let flush_at = body
        .find("audit_log_service::flush(")
        .expect("关停路径未调用 audit_log_service::flush，审计总线积压会随进程退出丢失");

    // 二，排空必须在全部生产者停止之后：生产者还在入队时排空只能排到中途，
    // 之后新入队的事件仍会随进程退出丢失
    for producer_stop in [
        "shutdown_event_bus()",
        "shutdown_handles.shutdown()",
        "shutdown_main_background_tasks()",
        "shutdown_app_state_background_tasks()",
    ] {
        let stop_at = body.find(producer_stop).unwrap_or_else(|| {
            panic!("关停路径未调用 {producer_stop}：生产者停止点已改名或被删除，本锁的前提失效")
        });
        assert!(
            stop_at < flush_at,
            "{producer_stop} 被挪到了 audit_log_service::flush 之后：\
             排空时生产者仍在入队，积压事件仍会丢失"
        );
    }

    // 三，等待必须有界：必须用关停专用超时常量，不允许无界等待阻塞进程退出
    assert!(
        body.contains("SHUTDOWN_FLUSH_TIMEOUT"),
        "关停排空未使用 SHUTDOWN_FLUSH_TIMEOUT 有界超时，可能无界阻塞进程退出"
    );

    // 四，排空结果必须进关停日志：不允许把未排空与落库失败静默吞掉
    assert!(
        body.contains("counts.failed") && body.contains("counts.pending"),
        "关停日志未记录 flush 返回的 failed/pending 计数，排空不完整时会静默无痕迹"
    );
}

/// 按花括号配平从源码里取出指定函数签名的函数体（返回 `{` 与配对 `}` 之间的内容）。
///
/// 仅供源码普查锁使用：不做词法分析，字符串或注释里出现花括号会造成偏差，
/// 所以调用方的判据只取"标记是否存在"与"两个标记的先后"，不依赖精确行号或体边界。
fn fn_body<'a>(source: &'a str, signature: &str) -> Option<&'a str> {
    let head = source.find(signature)?;
    let start = head + source[head..].find('{')?;
    let mut depth = 0usize;
    let bytes = source.as_bytes();
    for idx in start..bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[start + 1..idx]);
                }
            }
            _ => {}
        }
    }
    None
}
