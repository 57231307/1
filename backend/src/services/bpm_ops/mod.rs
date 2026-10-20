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

/// 流程标识（process_key，即 `bpm_process_definition.code`）取值的唯一事实来源。
///
/// 四个业务发起点（报价 / 销售订单 / 采购订单 / 生产订单）构造
/// `StartProcessRequest` 与拼装 BPM 启动失败文案都必须取本模块常量，禁止再复制
/// 字面量；流程定义种子的 `code` 落库值同样须与本模块逐字符一致，否则
/// `start_process` 的 `Code.eq(process_key)` 查询恒不命中，表现为每次提交都
/// "审批流程启动失败"（与 `bpm_task` 状态词表大小写不同源的静默空转同一机理）。
pub mod process_key {
    /// 销售报价单审批
    pub const QUOTATION_APPROVAL: &str = "quotation_approval";
    /// 销售订单审批
    pub const SALES_ORDER_APPROVAL: &str = "sales_order_approval";
    /// 采购订单审批
    pub const PURCHASE_ORDER_APPROVAL: &str = "purchase_order_approval";
    /// 生产订单审批
    pub const PRODUCTION_ORDER_APPROVAL: &str = "production_order_approval";
}

/// BPM 启动失败对外文案的单源拼装（流程定义缺失/未启用族）。
///
/// 调用方：`bpm_ops::instance::start_process` 的缺定义分支（经
/// `AppError::not_found_displayable` 出参，404）与四个业务发起点的 catch 分支
/// （经 `AppError::business_displayable` 出参，400）。HTTP 出参 message 只准由
/// 本函数产出，禁止各点各写拼接变体（防文案漂移，契约锁按本函数句式的子串断言）。
///
/// 安全边界：process_key 是"系统设置-审批流程定义"页上管理员维护的公开配置标识，
/// 点名它正是文案目的（用户裁定），满足 `utils/error.rs` 的可外显条件；本函数
/// **不接收内部错误原文**，SQL / 引擎细节由调用方记 tracing 日志，绝不传入此处。
///
/// `document_state` 为发起点对单据处置结果的如实陈述（如"报价单未提交"、
/// "订单已退回草稿状态"）：调用方只准在**确认**对应处置真实发生后传入对应语句；
/// 补偿回滚失败或结果未知时必须传中性描述（如"单据当前状态请人工核对"），
/// 不得虚报单据已回退。
pub fn bpm_start_failure_message(process_key: &str, document_state: &str) -> String {
    format!(
        "审批流程启动失败：流程标识 {process_key} 未配置或未启用。{document_state}。请先在 系统设置-审批流程定义 中新建流程标识为 {process_key} 的流程并启用，然后重新提交"
    )
}

/// 内层失败文案已可外显（Displayable 族）时的转发拼装：保留真实原因，补流程标识
/// 上下文与单据处置语句，不重复"未配置或未启用"定性（原因各异，转发即如实）。
fn bpm_start_failure_relay(process_key: &str, document_state: &str, inner_reason: &str) -> String {
    format!(
        "审批流程启动失败（流程标识 {process_key}）：{inner_reason}。{document_state}。请管理员按原因项处理后重新提交"
    )
}

/// 内层为脱敏族（数据库/内部错误等）时的兜底引导：真实原文只进日志（调用方负责），
/// 出参不猜原因、不透原文，给 trace 定位指引与单据当前处置的如实陈述。
fn bpm_start_failure_system(process_key: &str, document_state: &str) -> String {
    format!(
        "审批流程启动失败：流程标识 {process_key} 的流程处理发生系统错误。{document_state}。请联系管理员结合响应中的 trace 号定位日志根因后重新提交"
    )
}

/// 业务发起点对 `start_process` 失败的统一出参构造（单源，按内层族别如实分支）。
///
/// 调用方：报价 / 销售订单 / 采购订单 / 生产订单四个发起点的 catch 分支。
/// 判责口径：出参责任在引擎真实失败的那一族，本函数不吞内层、也不把内层原文
/// 整串外推：
/// - 内层 NotFoundDisplayable（缺 ACTIVE 定义，出厂主因）→ 点名流程标识的
///   行动指引文案（bpm_start_failure_message）；
/// - 内层已是 Displayable 族（如节点配置错误的 validation_displayable）→
///   转发内层已声明安全的真实原因，补流程标识与单据处置上下文；
/// - 其余（数据库/内部错误等脱敏族）→ 不猜原因的外显形态，给固定引导句；
///   根因原文必须由调用方以 error 级日志全量落盘后方可返回。
///
/// 出参恒为 `business_displayable`：HTTP 400 与 code BUSINESS_ERROR 与四个
/// 发起点既有契约一致（状态码语义不变），唯一变化是 message 由固定句式
/// 换成分支后的如实文案。
///
/// `document_state` 约束同 [`bpm_start_failure_message`]：只准陈述已确认的
/// 真实处置结果，回滚失败或结果未知时用中性描述，不得虚报已回退。
pub fn bpm_start_failure_error(
    process_key: &str,
    document_state: &str,
    inner: &AppError,
) -> AppError {
    let message = match inner {
        AppError::NotFoundDisplayable(_) => bpm_start_failure_message(process_key, document_state),
        AppError::ValidationErrorDisplayable(reason)
        | AppError::BusinessErrorDisplayable(reason) => {
            bpm_start_failure_relay(process_key, document_state, reason)
        }
        _ => bpm_start_failure_system(process_key, document_state),
    };
    AppError::business_displayable(message)
}

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
