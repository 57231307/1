//! BPM 任务状态「写入点 ↔ 查询/过滤点 ↔ 库层默认值」同源性回归，以及审批动作取值域校验
//! （确定性文本+常量比对，不依赖活库，CI 常规可跑）。
//!
//! 锁定缺陷（历史：BPM 审批中心整块不可用）：
//! - 写入点 `bpm_ops/instance.rs`、`bpm_ops/task.rs` 以 `status::bpm_task` 小写词表
//!   （pending/completed/rejected/cancelled）落库；
//! - 查询点 `handlers/bpm_handler.rs` 的 get_pending_tasks / get_completed_tasks 曾硬编码
//!   大写 "PENDING" / "COMPLETED" 作 Status.eq 过滤值 ⇒ Status.eq 恒不命中任何行，
//!   待办/已办列表对任何用户（含 admin）永远为空，同意/拒绝/审批链按钮永不渲染；
//! - 库层 `bpm_task.status` 建表默认值为大写 'PENDING'（m0001），绕过 service 的行落进
//!   词表外死状态，同样永不被待办查询命中（v15 尾部已收敛默认值并回填历史行）。
//!
//! 另有审批动作取值域缺陷：approve_task 用 `action == "reject"` 判拒绝、
//! update_task_status 用 `action == "approve"` 判完成，两处独立比较使任何越界动作
//! （transfer/delegate/拼写错误）在一次调用内「既按同意推进流程、又按拒绝写任务状态」。

use std::fs;
use std::path::PathBuf;

use bingxi_backend::models::status::bpm_task as task_status;
use bingxi_backend::services::bpm_ops::task::{
    ALL_APPROVE_ACTIONS, APPROVE_ACTION, REJECT_ACTION, validate_approve_action,
};

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 剔除以 `//`（含 `///` 文档注释）起始的注释行，只保留代码内容。
/// 供负向裸字面量守卫使用：守卫本意是禁止「代码」里的裸动作/状态字面量，
/// 文档注释里以反引号示例提及 `action == "reject"` 等是正当用途，不应被误报。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 基准：bpm_task 词表为小写（本测试其余断言全部以它的常量值为源，词表变则一并暴露）
#[test]
fn bpm_task_word_list_is_lowercase() {
    assert_eq!(task_status::PENDING, "pending");
    assert_eq!(task_status::COMPLETED, "completed");
    assert_eq!(task_status::REJECTED, "rejected");
    assert_eq!(task_status::CANCELLED, "cancelled");
}

/// 待办/已办列表的过滤值必须以 `status::bpm_task` 常量为源，不得残留裸字面量。
/// 这是「审批通过后列表能不能查到」的直接判据：过滤值与写入值不同源即结构性查不到。
#[test]
fn pending_and_completed_handlers_filter_with_word_list_constants() {
    let h = read_rel("src/handlers/bpm_handler.rs");
    assert!(
        h.contains("status: Some(task_status::PENDING.to_string())"),
        "get_pending_tasks 的 Status 过滤值必须以 bpm_task::PENDING 为源"
    );
    assert!(
        h.contains("status: Some(task_status::COMPLETED.to_string())"),
        "get_completed_tasks 的 Status 过滤值必须以 bpm_task::COMPLETED 为源"
    );
    for lit in [
        r#"Some("PENDING""#,
        r#"Some("COMPLETED""#,
        r#"Some("REJECTED""#,
        r#"Some("CANCELLED""#,
    ] {
        assert!(
            !h.contains(lit),
            "bpm_handler 不得残留大写任务状态裸字面量过滤 {lit}"
        );
    }
}

/// 大小写必须严格同源：查询点用的常量值与写入点用的常量值是同一个符号，
/// 任何一侧改用 to_uppercase/eq_ignore_ascii_case 兜底都会把该缺陷变隐性，故一并封死。
#[test]
fn write_and_read_sides_share_one_constant_and_no_case_folding() {
    for (label, src) in [
        (
            "src/services/bpm_ops/instance.rs",
            read_rel("src/services/bpm_ops/instance.rs"),
        ),
        (
            "src/services/bpm_ops/task.rs",
            read_rel("src/services/bpm_ops/task.rs"),
        ),
        (
            "src/services/bpm_ops/monitor.rs",
            read_rel("src/services/bpm_ops/monitor.rs"),
        ),
        (
            "src/handlers/bpm_handler.rs",
            read_rel("src/handlers/bpm_handler.rs"),
        ),
    ] {
        assert!(
            src.contains("use crate::models::status::bpm_task as task_status")
                || src.contains("use crate::models::status::bpm_task"),
            "{label} 必须引用 bpm_task 词表模块"
        );
        for fold in ["to_uppercase", "to_lowercase", "eq_ignore_ascii_case"] {
            assert!(
                !src.contains(fold),
                "{label} 禁止用 {fold} 把两套大小写词表糊起来（会把显性缺陷变隐性）"
            );
        }
    }
    // 写入点与监控统计点也必须是常量而非字面量
    let instance = read_rel("src/services/bpm_ops/instance.rs");
    assert!(
        instance.contains("task_status::PENDING.to_string()"),
        "建任务写入点必须以 bpm_task::PENDING 为源"
    );
    let monitor = read_rel("src/services/bpm_ops/monitor.rs");
    assert!(
        monitor.contains("Column::Status.eq(task_status::PENDING)"),
        "监控待办统计过滤点必须以 bpm_task::PENDING 为源"
    );
}

/// 库层收口：bpm_task.status 的默认值必须落在词表内，且历史大写行有回填
/// （无 CHECK 约束列，故只需默认值收敛 + UPDATE 回填两步）。
#[test]
fn migration_converges_bpm_task_status_default_and_backfills_uppercase_rows() {
    // 迁移 crate 在 backend/migration（CARGO_MANIFEST_DIR = backend），故不带 `../`
    let sql = read_rel("migration/src/domain/v15/mod.rs");
    assert!(
        sql.contains(r#"ALTER TABLE "bpm_task" ALTER COLUMN "status" SET DEFAULT 'pending';"#),
        "v15 须把 bpm_task.status 默认值收敛到词表值 'pending'"
    );
    assert!(
        sql.contains(r#"UPDATE "bpm_task" SET "status" = 'pending' WHERE "status" = 'PENDING';"#),
        "v15 须回填历史大写 'PENDING' 行，否则存量库的待办仍查不到"
    );
    // 建表默认值（m0001）为历史事实，只作对照，不再被词表认可
    let ddl = read_rel("migration/src/domain/system/m0001_initial_schema.rs");
    assert!(
        ddl.contains(r#""status" VARCHAR(20) NOT NULL DEFAULT 'PENDING'"#),
        "前置事实：m0001 建表默认值越界（大写），v15 的收敛正是针对它"
    );
}

/// 审批动作取值域：只认 approve / reject，大小写敏感，越界值必须被显式拒绝。
#[test]
fn approve_action_domain_is_validated_and_case_sensitive() {
    assert_eq!(APPROVE_ACTION, "approve");
    assert_eq!(REJECT_ACTION, "reject");
    assert_eq!(ALL_APPROVE_ACTIONS, ["approve", "reject"].as_slice());

    assert!(validate_approve_action("approve").is_ok());
    assert!(validate_approve_action("reject").is_ok());
    // 越界动作：曾被两处独立比较静默接受（一边当同意推进、一边当拒绝写状态）
    for bad in ["transfer", "delegate", "APPROVE", "Reject", "", "approve "] {
        assert!(
            validate_approve_action(bad).is_err(),
            "非法审批动作 {bad:?} 必须被拒绝（不得用 to_lowercase 兜底）"
        );
    }
}

/// approve_task 的两处动作比较必须引用同一对常量，杜绝再次出现独立字面量比较。
#[test]
fn approve_task_compares_action_with_single_source_constants() {
    let src = read_rel("src/services/bpm_ops/task.rs");
    let code = code_only(&src);
    assert!(
        src.contains("validate_approve_action(&req.action)?"),
        "approve_task 入口必须校验动作取值域"
    );
    assert!(
        src.contains("if req.action == REJECT_ACTION"),
        "拒绝分支必须以 REJECT_ACTION 为源"
    );
    assert!(
        src.contains("if req.action == APPROVE_ACTION"),
        "任务状态写入分支必须以 APPROVE_ACTION 为源"
    );
    for lit in [r#"action == "reject""#, r#"action == "approve""#] {
        assert!(
            !code.contains(lit),
            "不得残留动作裸字面量比较 {lit}（两处独立比较即状态错写之源）"
        );
    }
}
