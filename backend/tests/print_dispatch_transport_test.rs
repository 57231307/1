//! 打印派发传输层与编排状态机的集成/单元测试。
//!
//! 分两层：
//! 1. 不需活库的纯单元：传输工厂对缺失连接字段/未知后端的可见失败、system_command
//!    的确定性执行（无害 `echo`/`true`/非零退出）、IPP 编码器对固定字节夹具的校验；
//! 2. 需真实 PostgreSQL（`#[ignore]`，经 `TEST_DATABASE_URL` 注入）的编排回归：以注入式
//!    `MockTransport` 驱动 `dispatch_task_with_transport`，断言各终态（成功/传输错误/超时/
//!    panic）在 `print_tasks` 行上的 status / external_job_id / error_message / 时间戳落库，
//!    以及乐观锁不重复推进；并以真实 `dispatch_task` 走数据库目标解析（system_command 端到端、
//!    目标缺失/禁用失败可见）。
//!
//! 运行受库测试：
//! `TEST_DATABASE_URL=postgres://user:pass@host:5432/bingxi_test cargo test --test \
//!  print_dispatch_transport_test -- --ignored`

mod test_common;

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use test_common::setup_test_db;

use bingxi_backend::config::settings::PrintConfig;
use bingxi_backend::models::print_target::{ActiveModel as TargetActive, Entity as PrintTarget};
use bingxi_backend::models::print_task::{ActiveModel as TaskActive, Entity as PrintTask};
use bingxi_backend::models::status::print::print_task_status as status;
use bingxi_backend::services::print_dispatch::transport::{
    DispatchError, KIND_CUPS, KIND_SYSTEM_COMMAND, PrintJob, PrintTransport, ResolvedTarget,
    build_transport,
};
use bingxi_backend::services::print_dispatch::transports::system_command::SystemCommandTransport;
use bingxi_backend::services::print_dispatch::{dispatch_task, dispatch_task_with_transport};

// ============================================================================
// 注入式 Mock 传输：确定性复现 成功 / 失败 / 超时 / panic 四类传输结局
// ============================================================================

#[derive(Clone)]
enum MockBehavior {
    /// 成功返回给定外部作业号
    Success(String),
    /// 失败返回给定原因
    Fail(String),
    /// 先睡眠再返回（用于触发编排层超时）
    SleepThen(Duration, String),
    /// 在 send 内 panic（用于验证编排层捕获 panic）
    Panic,
}

struct MockTransport {
    behavior: MockBehavior,
}

#[async_trait]
impl PrintTransport for MockTransport {
    async fn send(&self, _job: &PrintJob, _bytes: &[u8]) -> Result<String, DispatchError> {
        match &self.behavior {
            MockBehavior::Success(id) => Ok(id.clone()),
            MockBehavior::Fail(reason) => Err(DispatchError::Transport {
                detail: reason.clone(),
            }),
            MockBehavior::SleepThen(d, id) => {
                tokio::time::sleep(*d).await;
                Ok(id.clone())
            }
            MockBehavior::Panic => panic!("mock transport panic"),
        }
    }
}

/// 构造一个供注入的单台目标上下文（Mock 不读取其连接字段）。
fn mock_target() -> ResolvedTarget {
    ResolvedTarget {
        target_code: "mock-printer".to_string(),
        kind: KIND_SYSTEM_COMMAND.to_string(),
        queue_name: None,
        host: None,
        port: None,
        endpoint_url: None,
        command_template: Some("true".to_string()),
    }
}

fn test_config(timeout_secs: u64, keep_failed: bool) -> PrintConfig {
    PrintConfig {
        dispatch_timeout_secs: timeout_secs,
        keep_failed_temp: keep_failed,
        temp_dir_root: None,
    }
}

// ============================================================================
// 不需活库：传输工厂的可见失败
// ============================================================================

/// cups 目标缺 queue_name 时工厂必须失败可见，不构造空队列传输。
#[test]
fn build_transport_rejects_cups_without_queue() {
    let target = ResolvedTarget {
        target_code: "cups-x".to_string(),
        kind: KIND_CUPS.to_string(),
        queue_name: None,
        host: None,
        port: None,
        endpoint_url: None,
        command_template: None,
    };
    assert!(
        matches!(
            build_transport(&target),
            Err(DispatchError::MissingTargetField { .. })
        ),
        "cups 缺 queue_name 必须报缺失连接字段"
    );
}

/// 词表外后端类型必须被判为不支持。
#[test]
fn build_transport_rejects_unknown_kind() {
    let target = ResolvedTarget {
        target_code: "weird".to_string(),
        kind: "fax".to_string(),
        queue_name: None,
        host: None,
        port: None,
        endpoint_url: None,
        command_template: Some("true".to_string()),
    };
    assert!(
        matches!(
            build_transport(&target),
            Err(DispatchError::UnsupportedBackend { .. })
        ),
        "未知 kind 必须报不支持后端"
    );
}

/// network 目标缺 host 时工厂必须失败可见。
#[test]
fn build_transport_rejects_network_without_host() {
    let target = ResolvedTarget {
        target_code: "net-x".to_string(),
        kind: "network".to_string(),
        queue_name: None,
        host: None,
        port: Some(9100),
        endpoint_url: None,
        command_template: None,
    };
    assert!(
        matches!(
            build_transport(&target),
            Err(DispatchError::MissingTargetField { .. })
        ),
        "network 缺 host 必须报缺失连接字段"
    );
}

// ============================================================================
// 不需活库：system_command 传输的确定性执行（无硬件/网络）
// ============================================================================

fn job_with_file(job_code: &str, file_path: PathBuf) -> PrintJob {
    PrintJob {
        target: ResolvedTarget {
            target_code: "sys".to_string(),
            kind: KIND_SYSTEM_COMMAND.to_string(),
            queue_name: None,
            host: None,
            port: None,
            endpoint_url: None,
            command_template: None,
        },
        copies: 1,
        file_path,
        job_code: job_code.to_string(),
    }
}

/// `echo` 模板把 stdout 作为外部作业号返回（仅 unix，生产派发运行于 Linux）。
#[cfg(unix)]
#[tokio::test]
async fn system_command_capture_stdout_is_external_id() {
    let t = SystemCommandTransport::new("echo SYSID-987".to_string());
    let job = job_with_file("J1", PathBuf::from("/tmp/does-not-need-to-exist.docx"));
    let id = t.send(&job, b"x").await.expect("echo 应成功");
    assert_eq!(id, "SYSID-987");
}

/// 非零退出必须失败可见并带上原因（不吞 stderr）。
#[cfg(unix)]
#[tokio::test]
async fn system_command_nonzero_exit_is_error() {
    let t = SystemCommandTransport::new("echo boom 1>&2; exit 7".to_string());
    let job = job_with_file("J2", PathBuf::from("/tmp/whatever.docx"));
    let err = t.send(&job, b"x").await.expect_err("非零退出必须失败");
    let msg = err.to_string();
    assert!(
        msg.contains("boom") || msg.contains("非零退出"),
        "实际={msg}"
    );
}

/// 命令成功但无 stdout（无回执通道）时合成可关联任务号，不谎称真实作业号。
#[cfg(unix)]
#[tokio::test]
async fn system_command_silent_success_synthesizes_id() {
    let t = SystemCommandTransport::new("true".to_string());
    let job = job_with_file("J3", PathBuf::from("/tmp/x.docx"));
    let id = t.send(&job, b"x").await.expect("true 应成功");
    assert!(id.starts_with("syscmd-"), "应合成 syscmd- 前缀，实际={id}");
}

// ============================================================================
// 需真实 PostgreSQL（#[ignore]）：编排状态机 + 乐观锁 + 数据库目标解析
// ============================================================================

async fn seed_task(
    db: &sea_orm::DatabaseConnection,
    job_code: &str,
    target_name: &str,
    init_status: &str,
    preexisting_external: Option<&str>,
) -> i32 {
    let am = TaskActive {
        job_code: Set(job_code.to_string()),
        requested_by: Set(1),
        requested_by_name: Set("派发测试发起人".to_string()),
        doc_type: Set("test_doc".to_string()),
        doc_id: Set(1_i64),
        target_kind: Set("printer".to_string()),
        target_name: Set(target_name.to_string()),
        source_module: Set("test".to_string()),
        backend_kind: Set(KIND_SYSTEM_COMMAND.to_string()),
        status: Set(init_status.to_string()),
        external_job_id: Set(preexisting_external.map(|s| s.to_string())),
        ..Default::default()
    };
    let model = am.insert(db).await.expect("插入打印任务失败");
    model.id
}

async fn seed_target(
    db: &sea_orm::DatabaseConnection,
    target_code: &str,
    kind: &str,
    command_template: Option<&str>,
    enabled: bool,
    pool_members: Option<Vec<String>>,
) -> i32 {
    let mut am = TargetActive {
        target_code: Set(target_code.to_string()),
        display_name: Set(target_code.to_string()),
        kind: Set(kind.to_string()),
        enabled: Set(enabled),
        ..Default::default()
    };
    if let Some(ct) = command_template {
        am.command_template = Set(Some(ct.to_string()));
    }
    if let Some(members) = pool_members {
        am.is_pool = Set(true);
        am.pool_members = Set(serde_json::Value::Array(
            members.into_iter().map(serde_json::Value::String).collect(),
        ));
    }
    let model = am.insert(db).await.expect("插入打印目标失败");
    model.id
}

async fn reload_task(
    db: &sea_orm::DatabaseConnection,
    job_id: i32,
) -> bingxi_backend::models::print_task::Model {
    PrintTask::find_by_id(job_id)
        .one(db)
        .await
        .expect("回读打印任务失败")
        .expect("打印任务应存在")
}

/// 成功：注入恒成功 Mock → 行进入 succeeded，external_job_id 落库，派发/完成时间被置。
#[tokio::test]
#[ignore]
async fn dispatch_success_sets_succeeded_and_external_id() {
    let db = setup_test_db().await;
    let job_id = seed_task(&db, "JOB-OK", "mock-printer", status::PENDING, None).await;

    let transport = Box::new(MockTransport {
        behavior: MockBehavior::Success("MOCK-OK".to_string()),
    });
    dispatch_task_with_transport(
        &db,
        job_id,
        b"data",
        &test_config(5, false),
        transport,
        mock_target(),
    )
    .await
    .expect("成功派发应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::SUCCEEDED, "应进入 succeeded");
    assert_eq!(row.external_job_id.as_deref(), Some("MOCK-OK"));
    assert!(row.error_message.is_none(), "成功不应有 error_message");
    assert!(row.dispatched_at.is_some(), "应记录 dispatched_at");
    assert!(row.completed_at.is_some(), "应记录 completed_at");
}

/// 传输失败：Mock 返回 Err → 行进入 failed 且 error_message 非空（满足 CHECK）。
#[tokio::test]
#[ignore]
async fn dispatch_transport_error_sets_failed_with_message() {
    let db = setup_test_db().await;
    let job_id = seed_task(&db, "JOB-ERR", "mock-printer", status::PENDING, None).await;

    let transport = Box::new(MockTransport {
        behavior: MockBehavior::Fail("设备离线".to_string()),
    });
    dispatch_task_with_transport(
        &db,
        job_id,
        b"data",
        &test_config(5, false),
        transport,
        mock_target(),
    )
    .await
    .expect("失败终态已落库，应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::FAILED, "应进入 failed");
    assert!(
        row.error_message
            .as_deref()
            .is_some_and(|m| m.contains("设备离线")),
        "error_message 应携带传输原因，实际={:?}",
        row.error_message
    );
    assert!(row.completed_at.is_some(), "失败终态应记录 completed_at");
}

/// 超时：Mock 睡眠超过配置超时 → failed 且文案「派发超时」。
#[tokio::test]
#[ignore]
async fn dispatch_timeout_sets_failed_with_timeout_message() {
    let db = setup_test_db().await;
    let job_id = seed_task(&db, "JOB-TIMEOUT", "mock-printer", status::PENDING, None).await;

    // Mock 睡 1.5s，超时设 1s → 编排层应先于 send 完成而判超时。
    let transport = Box::new(MockTransport {
        behavior: MockBehavior::SleepThen(Duration::from_millis(1500), "too-late".to_string()),
    });
    dispatch_task_with_transport(
        &db,
        job_id,
        b"data",
        &test_config(1, false),
        transport,
        mock_target(),
    )
    .await
    .expect("超时终态已落库，应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::FAILED);
    assert_eq!(
        row.error_message.as_deref(),
        Some("派发超时"),
        "超时文案必须逐字为「派发超时」，实际={:?}",
        row.error_message
    );
    assert_eq!(row.external_job_id, None, "超时不得写入迟到的作业号");
}

/// panic：Mock 内 panic → 编排层捕获 JoinError → failed 且 error_message 非空（不静默停留 in-flight）。
#[tokio::test]
#[ignore]
async fn dispatch_transport_panic_is_caught_and_failed() {
    let db = setup_test_db().await;
    let job_id = seed_task(&db, "JOB-PANIC", "mock-printer", status::PENDING, None).await;

    let transport = Box::new(MockTransport {
        behavior: MockBehavior::Panic,
    });
    dispatch_task_with_transport(
        &db,
        job_id,
        b"data",
        &test_config(5, false),
        transport,
        mock_target(),
    )
    .await
    .expect("panic 已被捕获并落终态，应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(
        row.status,
        status::FAILED,
        "panic 必须转入 failed，不得停留 in-flight"
    );
    assert!(
        row.error_message
            .as_deref()
            .is_some_and(|m| m.contains("异常")),
        "error_message 应说明派发异常，实际={:?}",
        row.error_message
    );
}

/// 乐观锁：行已处于 succeeded（非 pending）→ 不重复推进，原 external_job_id 保持不变。
#[tokio::test]
#[ignore]
async fn dispatch_honors_optimistic_lock_on_terminal_row() {
    let db = setup_test_db().await;
    let job_id = seed_task(
        &db,
        "JOB-ALREADY-DONE",
        "mock-printer",
        status::SUCCEEDED,
        Some("ORIGINAL-123"),
    )
    .await;

    let transport = Box::new(MockTransport {
        behavior: MockBehavior::Success("SHOULD-NOT-OVERWRITE".to_string()),
    });
    dispatch_task_with_transport(
        &db,
        job_id,
        b"data",
        &test_config(5, false),
        transport,
        mock_target(),
    )
    .await
    .expect("让位路径应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::SUCCEEDED, "已是终态不应被改写状态");
    assert_eq!(
        row.external_job_id.as_deref(),
        Some("ORIGINAL-123"),
        "已是终态不应被重复回写作业号"
    );
}

/// 真实 dispatch_task 走数据库解析：system_command echo 目标端到端成功。
#[cfg(unix)]
#[tokio::test]
#[ignore]
async fn dispatch_task_resolves_target_from_db_and_succeeds() {
    let db = setup_test_db().await;
    seed_target(
        &db,
        "sys-echo",
        KIND_SYSTEM_COMMAND,
        Some("echo DB-SYS-OK"),
        true,
        None,
    )
    .await;
    let job_id = seed_task(&db, "JOB-DBOK", "sys-echo", status::PENDING, None).await;

    dispatch_task(&db, job_id, b"docbytes", &test_config(10, false))
        .await
        .expect("派发应完成并落终态");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::SUCCEEDED, "实际状态={}", row.status);
    assert_eq!(row.external_job_id.as_deref(), Some("DB-SYS-OK"));
}

/// 目标缺失：target_name 不在 print_targets → failed，文案逐字「打印目标未配置或未启用: <name>」。
#[tokio::test]
#[ignore]
async fn dispatch_task_fails_visibly_when_target_missing() {
    let db = setup_test_db().await;
    let job_id = seed_task(&db, "JOB-NOTARGET", "ghost-printer", status::PENDING, None).await;

    dispatch_task(&db, job_id, b"docbytes", &test_config(10, false))
        .await
        .expect("目标缺失已落 failed 终态，应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::FAILED);
    assert_eq!(
        row.error_message.as_deref(),
        Some("打印目标未配置或未启用: ghost-printer"),
        "目标缺失文案必须逐字匹配，实际={:?}",
        row.error_message
    );
    assert!(
        row.dispatched_at.is_none(),
        "在 queued 阶段即失败，未进入 dispatched"
    );
}

/// 目标被禁用：enabled=false 的行同样判为未启用，失败可见。
#[tokio::test]
#[ignore]
async fn dispatch_task_fails_visibly_when_target_disabled() {
    let db = setup_test_db().await;
    seed_target(
        &db,
        "sys-disabled",
        KIND_SYSTEM_COMMAND,
        Some("true"),
        false,
        None,
    )
    .await;
    let job_id = seed_task(&db, "JOB-DISABLED", "sys-disabled", status::PENDING, None).await;

    dispatch_task(&db, job_id, b"docbytes", &test_config(10, false))
        .await
        .expect("禁用目标已落 failed 终态，应返回 Ok");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::FAILED);
    assert_eq!(
        row.error_message.as_deref(),
        Some("打印目标未配置或未启用: sys-disabled")
    );
}

/// 打印机池：成员编码数组首个被选中并轮换取下一个；解析成员行后成功派发。
#[cfg(unix)]
#[tokio::test]
#[ignore]
async fn dispatch_task_resolves_pool_member_and_rotates() {
    let db = setup_test_db().await;
    // 两台成员目标，echo 各自的可识别作业号
    seed_target(
        &db,
        "pool-a",
        KIND_SYSTEM_COMMAND,
        Some("echo POOL-A"),
        true,
        None,
    )
    .await;
    seed_target(
        &db,
        "pool-b",
        KIND_SYSTEM_COMMAND,
        Some("echo POOL-B"),
        true,
        None,
    )
    .await;
    seed_target(
        &db,
        "the-pool",
        KIND_SYSTEM_COMMAND,
        None,
        true,
        Some(vec!["pool-a".to_string(), "pool-b".to_string()]),
    )
    .await;
    let job_id = seed_task(&db, "JOB-POOL", "the-pool", status::PENDING, None).await;

    dispatch_task(&db, job_id, b"docbytes", &test_config(10, false))
        .await
        .expect("池派发应完成");

    let row = reload_task(&db, job_id).await;
    assert_eq!(row.status, status::SUCCEEDED, "实际状态={}", row.status);
    // 首轮应取 members[0] = pool-a → echo POOL-A
    assert_eq!(
        row.external_job_id.as_deref(),
        Some("POOL-A"),
        "池首轮应命中首个成员 pool-a，实际={:?}",
        row.external_job_id
    );

    // 轮换游标：再建一个指向同一池的任务，应轮到 pool-b
    let job2 = seed_task(&db, "JOB-POOL2", "the-pool", status::PENDING, None).await;
    dispatch_task(&db, job2, b"docbytes", &test_config(10, false))
        .await
        .expect("第二次池派发应完成");
    let row2 = reload_task(&db, job2).await;
    assert_eq!(row2.status, status::SUCCEEDED);
    assert_eq!(
        row2.external_job_id.as_deref(),
        Some("POOL-B"),
        "第二轮应轮到 pool-b（轮换游标已前移），实际={:?}",
        row2.external_job_id
    );
}
