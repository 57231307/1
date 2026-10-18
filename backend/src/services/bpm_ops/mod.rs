//! BPM 工作流服务业务实现子模块（bpm_ops）
//!
//! 批次 D10 拆分：从原 `bpm_service.rs`（1060 行）拆出 `BpmService` impl 业务方法块。
//! - `instance`：流程实例生命周期（start_process / cancel_instance / 业务查询 / 审批链 / 业务关联）
//! - `task`：任务审批流（approve_task + 推进/拒绝/完成 + 查询/转办/催办）
//! - `monitor`：流程监控统计（实例/任务统计 + 待处理任务列表 + 实例列表）
//!
//! `BpmService` struct 定义与 `new` 构造函数、纯函数（`evaluate_bpm_condition` /
//! `resolve_first_task_node`）保留在 facade `bpm_service` 中，impl 业务方法块分散到本子模块，
//! Rust 允许同一 crate 多文件多 impl 块。
//! `db` 字段声明为 `pub(crate)` 以便本子模块访问；跨 facade 调用的纯函数亦为 `pub(crate)`。

pub mod instance;
pub mod monitor;
pub mod task;

use sea_orm::DatabaseConnection;

use crate::models::notification::{NotificationPriority, NotificationType};
use crate::models::{bpm_process_instance, bpm_task};
use crate::services::notification_service::{CreateNotificationRequest, NotificationService};
use crate::utils::error::AppError;

/// 待办列表页路由（前端 `router/index.ts` BPMApproval 的完整路径）。
/// 通知的 `action_url` 必须指向真实存在的路由，否则用户点开通知会落空。
const BPM_APPROVAL_PATH: &str = "/bpm/approval";

/// 解析流程节点配置的审批人。
///
/// 引擎当前只认「具体用户 ID」这一种审批人。历史上 `assignee_value` 非数字或节点属于
/// `role`/`department`/`dynamic` 类型时，代码直接把处理人写成 NULL 继续建单，而待办查询
/// 按处理人过滤，于是工单落库后对任何人都不可见——表现为「提交成功了，但没人能审」。
/// 现改为 fail-visible：配置不被引擎支持或未配审批人时拒绝发起，点名节点，绝不静默建孤儿单。
pub(crate) fn resolve_node_assignee(node: &serde_json::Value) -> Result<Option<i32>, AppError> {
    let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let node_name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let label = format!("审批流程节点 {node_name}({node_id})");

    let assignee_type = node
        .get("assignee_type")
        .and_then(|v| v.as_str())
        .unwrap_or("user");
    if assignee_type != "user" {
        return Err(AppError::validation_displayable(format!(
            "{label} 的审批人类型为 {assignee_type}，当前审批引擎只支持指定具体用户，请在流程定义中把审批人改为具体用户"
        )));
    }

    let raw = node.get("assignee_value");
    let parsed = match raw {
        Some(serde_json::Value::String(s)) => s.trim().parse::<i32>().ok(),
        Some(v) => v.as_i64().and_then(|n| i32::try_from(n).ok()),
        None => None,
    };

    match parsed {
        Some(id) if id > 0 => Ok(Some(id)),
        Some(_) => Err(AppError::validation_displayable(format!(
            "{label} 的审批人用户 ID 非法，须为正整数"
        ))),
        None => Err(AppError::validation_displayable(format!(
            "{label} 未配置审批人，工单生成后将无人可处理，请在流程定义中指定审批用户"
        ))),
    }
}

/// 工单落库后通知其处理人（best-effort，失败只 warn 不回滚业务）。
///
/// 通知对象只取工单上已解析出的真实处理人，不猜角色、不兜底塞某个固定用户；
/// 带 `dedup_key` 防同一工单在重试时重复轰炸。
pub(crate) async fn notify_task_pending(
    db: &std::sync::Arc<DatabaseConnection>,
    task: &bpm_task::Model,
    instance: &bpm_process_instance::Model,
    reason: &str,
) {
    let Some(assignee_id) = task.actual_handler_id else {
        tracing::warn!(
            task_id = task.id,
            task_no = %task.task_no,
            "工单无已解析处理人，跳过通知（应由流程定义的审批人配置修正触发）"
        );
        return;
    };
    let business_id = instance.business_id.unwrap_or(0);
    let service = NotificationService::new(db.clone());
    if let Err(e) = service
        .create_notification(CreateNotificationRequest {
            user_id: assignee_id,
            notification_type: NotificationType::Internal,
            title: "有新的待办工单".to_string(),
            content: format!(
                "{}：工单 {}（{}）等待您处理",
                reason, task.task_no, task.node_name
            ),
            priority: NotificationPriority::Normal,
            business_type: Some("BPM".to_string()),
            business_id: Some(i64::from(task.instance_id)),
            action_url: Some(BPM_APPROVAL_PATH.to_string()),
            sender_id: None,
            sender_name: Some("系统".to_string()),
            dedup_key: Some(format!("bpm_task:{}:pending", task.id)),
        })
        .await
    {
        tracing::warn!(
            task_id = task.id,
            assignee_id = assignee_id,
            error = %e,
            "待办工单通知发送失败（best-effort，不影响审批主流程）"
        );
    }
    tracing::info!(
        task_id = task.id,
        assignee_id = assignee_id,
        business_type = %instance.business_type,
        business_id = business_id,
        "待办工单通知已派发"
    );
}
