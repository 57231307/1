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

use sea_orm::{ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter};

use crate::models::notification::{NotificationPriority, NotificationType};
use crate::models::{bpm_process_instance, bpm_task, department, user, user_department};
use crate::services::notification_service::{CreateNotificationRequest, NotificationService};
use crate::utils::error::AppError;

/// 待办列表页路由（前端 `router/index.ts` BPMApproval 的完整路径）。
/// 通知的 `action_url` 必须指向真实存在的路由，否则用户点开通知会落空。
const BPM_APPROVAL_PATH: &str = "/bpm/approval";

/// 流程标识（process_key，即 `bpm_process_definition.code`）取值的唯一事实来源。
///
/// 五个业务发起点（报价 / 销售订单 / 采购订单 / 生产订单 / 敏感数据导出）构造
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
    /// 敏感数据导出审批（流程定义行由 services/bpm_definition_seed.rs 出厂种子登记）
    pub const EXPORT_APPROVAL: &str = "export_approval";
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
/// - 内层 NotFoundDisplayable（缺 ACTIVE 定义，出厂主因）时，出参点名流程标识的
///   行动指引文案（bpm_start_failure_message）；
/// - 内层已是 Displayable 族（如节点配置错误的 validation_displayable）时，
///   转发内层已声明安全的真实原因，补流程标识与单据处置上下文；
/// - 其余（数据库/内部错误等脱敏族）给固定引导句，不猜原因也不外透原文；
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

/// 申请人所在部门的识别结果（三态显式，杜绝把歧义当未配置静默放过）。
///
/// 调用方：`resolve_node_assignee` 的部门上级解析分支与 `start_process` 的部门落库。
/// 判定口径与 `data_permission_service::get_user_dept_scope_ids` 的兼容注释一致：
/// `user_departments` 关联优先（主部门唯一才算数），无关联记录时回落 `users.department_id` 单值。
pub(crate) enum ApplicantDept {
    /// 唯一确定到一个部门
    Unique(i32),
    /// 多部门且无唯一主部门标记（歧义，须人工配置消歧）
    Ambiguous,
    /// 完全未关联任何部门
    Unresolvable,
}

/// 识别用户所在部门（关联表优先、`users.department_id` 兼容回落），只查库不判定业务。
pub(crate) async fn applicant_department(
    txn: &DatabaseTransaction,
    user_id: i32,
) -> Result<ApplicantDept, AppError> {
    let rows = user_department::Entity::find()
        .filter(user_department::Column::UserId.eq(user_id))
        .all(txn)
        .await?;
    if rows.is_empty() {
        // 无关联表记录：兼容 users.department_id 单值路径（历史/未走 assign_user_departments 的用户）
        let dept = user::Entity::find_by_id(user_id)
            .one(txn)
            .await?
            .and_then(|u| u.department_id);
        return Ok(match dept {
            Some(d) => ApplicantDept::Unique(d),
            None => {
                tracing::warn!(user_id, "用户未关联任何部门，申请人部门识别为不可解析");
                ApplicantDept::Unresolvable
            }
        });
    }
    let primaries: Vec<&user_department::Model> = rows.iter().filter(|r| r.is_primary).collect();
    Ok(if primaries.len() == 1 {
        ApplicantDept::Unique(primaries[0].department_id)
    } else if primaries.is_empty() && rows.len() == 1 {
        ApplicantDept::Unique(rows[0].department_id)
    } else {
        tracing::warn!(
            user_id,
            dept_count = rows.len(),
            primary_count = primaries.len(),
            "用户存在多个部门关联且无唯一主部门标记，申请人部门识别为歧义"
        );
        ApplicantDept::Ambiguous
    })
}

/// 审批人解析这条链上引擎实际读取的流程节点配置键（`config.nodes[]` 的强类型视图）。
///
/// 字段名与节点 JSON 键逐字对齐，是本模块"读哪几个键、键缺失按什么算"的唯一事实来源；
/// 前端 `frontend/src/api/bpm-enhanced.ts` 的 `ProcessNode` 以此声明，契约门禁
/// `frontend/scripts/check-api-keys.mjs` 也把它当作后端真源。
///
/// 调用方：[`resolve_node_assignee`] 在入口经 [`NodeConfig::from_node`] 解析一次，
/// 解析结果交给 `resolve_user_assignee` 与 `resolve_dept_superior_assignee` 消费。
/// 存什么：`id` / `name` / `assignee_type` 存已按引擎缺省口径归一的文本；
/// `assignee_value` / `sensitive` 存 JSON 原文，因为"键缺失"与"键存在但类型不符"是两回事，
/// 类型判定必须留给调用方按 fail-closed 处理，本结构不做任何静默兜底。
pub(crate) struct NodeConfig {
    /// 节点标识，只用于拒绝文案括号内与日志落盘；非字符串或键缺失按 `unknown`
    pub(crate) id: String,
    /// 节点名称，拒绝文案里点名给用户看的业务名；非字符串或键缺失按空串
    pub(crate) name: String,
    /// 审批人类型，词表见 [`resolve_node_assignee`]；键缺失按 `user`（兼容存量定义）
    pub(crate) assignee_type: String,
    /// `assignee_value` 原文：`user` 类型须由它解析出正整数用户 ID
    pub(crate) assignee_value: Option<serde_json::Value>,
    /// `sensitive` 原文：`dept_superior` 类型据此决定是否再上溯一级部门
    pub(crate) sensitive: Option<serde_json::Value>,
}

impl NodeConfig {
    /// 从节点 JSON 取键。节点不是对象（历史脏数据）时每个键都读不到，
    /// 于是各自走缺失缺省值，与逐处字面量读取时代的判定完全一致。
    pub(crate) fn from_node(node: &serde_json::Value) -> Self {
        Self {
            id: node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            name: node
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            assignee_type: node
                .get("assignee_type")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string(),
            assignee_value: node.get("assignee_value").cloned(),
            sensitive: node.get("sensitive").cloned(),
        }
    }
}

/// 解析流程节点配置的审批人。
///
/// 节点键的读取与缺省口径集中在 [`NodeConfig`]，本函数只在入口解析一次再分派。
///
/// 引擎支持两种审批人类型（词表唯一事实来源）：
/// - `user`：节点 `assignee_value` 为具体用户 ID（缺省类型，兼容存量定义），须解析为正整数；
/// - `dept_superior`：按申请人所在部门的上一级部门负责人解析；节点带 `sensitive: true`
///   （敏感操作显式标记，缺省即 false，存量定义无需迁移）时再上一级，取该领导所在
///   部门的上一级部门负责人。
/// 全部路径 fail-closed：类型不认识、敏感标记非布尔、部门缺失或歧义、上级部门或
/// 其负责人未配置时，一律返回可外显校验错误并点名该去配置什么，绝不回落到某个默认人，
/// 也绝不放行成无人可审的孤儿工单（历史上处理人写 NULL 导致工单在按人过滤的待办里
/// 对任何人都不可见，正是本函数要消灭的形态）。内部标识（节点 id、实例 id、部门 id）
/// 只随日志落盘，出参文案只说业务名与该做的动作。
pub(crate) async fn resolve_node_assignee(
    node: &serde_json::Value,
    instance: &bpm_process_instance::Model,
    txn: &DatabaseTransaction,
) -> Result<Option<i32>, AppError> {
    let cfg = NodeConfig::from_node(node);
    let (node_id, node_name) = (cfg.id.as_str(), cfg.name.as_str());
    match cfg.assignee_type.as_str() {
        "user" => resolve_user_assignee(&cfg),
        "dept_superior" => resolve_dept_superior_assignee(&cfg, instance, txn).await,
        other => {
            tracing::warn!(
                instance_id = instance.id,
                node_id = %node_id,
                assignee_type = %other,
                "审批人类型不在引擎词表内，已拒绝（内部标识仅随本条日志落盘）"
            );
            Err(AppError::validation_displayable(format!(
                "审批流程节点 {node_name} 的审批人类型为 {other}，审批引擎只支持指定具体用户与按申请人部门上级负责人解析两种，请在流程定义中改为其中之一后重新提交"
            )))
        }
    }
}

/// `user` 类型分支：审批人取节点 `assignee_value` 里的数字用户 ID。
fn resolve_user_assignee(cfg: &NodeConfig) -> Result<Option<i32>, AppError> {
    let (node_name, node_id) = (cfg.name.as_str(), cfg.id.as_str());
    let label = format!("审批流程节点 {node_name}({node_id})");
    let parsed = match &cfg.assignee_value {
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

/// 读取节点敏感操作标记：缺省即非敏感（存量定义无需迁移）；值存在但非布尔属定义错误，
/// 按 fail-closed 拒绝，绝不静默当 false 吞掉配置笔误。
fn read_sensitive_flag(
    cfg: &NodeConfig,
    instance: &bpm_process_instance::Model,
) -> Result<bool, AppError> {
    let (node_id, node_name) = (cfg.id.as_str(), cfg.name.as_str());
    match &cfg.sensitive {
        None | Some(serde_json::Value::Bool(false)) => Ok(false),
        Some(serde_json::Value::Bool(true)) => Ok(true),
        Some(value) => {
            tracing::warn!(
                instance_id = instance.id,
                node_id = %node_id,
                value = %value,
                "敏感操作标记不是布尔值，已拒绝（内部标识仅随本条日志落盘）"
            );
            Err(AppError::validation_displayable(format!(
                "审批流程节点 {node_name} 的敏感操作标记必须是勾选或不勾选（布尔值），当前配置无法判定，请在流程定义中修正后重新提交"
            )))
        }
    }
}

/// `dept_superior` 类型分支：沿部门树上溯解析审批人。
///
/// 解析链：申请人所在部门（实例落库值优先，缺失时按 `applicant_department` 现算）
/// → 上一级部门的 `manager_id`；敏感操作节点（`sensitive: true`）再上溯一级，
/// 取该领导所在部门的上一级部门负责人。任一环缺失都以可外显校验错误指明去哪里配置。
async fn resolve_dept_superior_assignee(
    cfg: &NodeConfig,
    instance: &bpm_process_instance::Model,
    txn: &DatabaseTransaction,
) -> Result<Option<i32>, AppError> {
    let (node_id, node_name) = (cfg.id.as_str(), cfg.name.as_str());
    let sensitive = read_sensitive_flag(cfg, instance)?;
    let base_dept = match instance.initiator_department_id {
        Some(d) => d,
        None => match applicant_department(txn, instance.initiator_id).await? {
            ApplicantDept::Unique(d) => d,
            ApplicantDept::Ambiguous => {
                return Err(AppError::validation_displayable(format!(
                    "审批流程节点 {node_name} 按部门上级解析审批人，但申请人有多个部门且未标记主部门，无法确定从哪个部门上溯。请在 系统设置-部门管理 为申请人指定主部门后重新提交"
                )));
            }
            ApplicantDept::Unresolvable => {
                return Err(AppError::validation_displayable(format!(
                    "审批流程节点 {node_name} 按部门上级解析审批人，但申请人未关联任何部门。请在 系统设置-部门管理 为申请人配置部门归属后重新提交"
                )));
            }
        },
    };
    // 上溯层数：普通节点取申请人部门的上一级负责人；敏感节点取再上一级（该领导所在部门的上级负责人）
    let steps: u8 = if sensitive { 2 } else { 1 };
    let mut current_id = base_dept;
    let mut visited = std::collections::HashSet::new();
    visited.insert(current_id);
    for hop in 1..=steps {
        let dept = department::Entity::find_by_id(current_id)
            .one(txn)
            .await?
            .ok_or_else(|| {
                tracing::warn!(
                    instance_id = instance.id,
                    node_id = %node_id,
                    department_id = current_id,
                    "申请人部门链上的部门记录不存在，已拒绝（内部标识仅随本条日志落盘）"
                );
                AppError::validation_displayable(format!(
                    "审批流程节点 {node_name} 按部门上级解析审批人失败：申请人所在部门记录不存在或已被删除。请在 系统设置-部门管理 核对申请人的部门归属后重新提交"
                ))
            })?;
        let parent_id = dept.parent_id.ok_or_else(|| {
            tracing::warn!(
                instance_id = instance.id,
                node_id = %node_id,
                department_id = current_id,
                hop = hop,
                "部门链上没有更上一级，审批人解析不出（内部标识仅随本条日志落盘）"
            );
            AppError::validation_displayable(format!(
                "审批流程节点 {node_name} 按部门上级解析审批人失败：{}。请在 系统设置-部门管理 补全组织层级并配置负责人后重新提交",
                if sensitive && hop == 2 {
                    "该领导所在部门已是组织树顶层，没有再上一级部门可承接敏感操作审批"
                } else {
                    "申请人所在部门没有上级部门"
                }
            ))
        })?;
        if !visited.insert(parent_id) {
            tracing::warn!(
                instance_id = instance.id,
                node_id = %node_id,
                department_id = parent_id,
                "部门上下级存在成环配置，审批人解析不出（内部标识仅随本条日志落盘）"
            );
            return Err(AppError::validation_displayable(format!(
                "审批流程节点 {node_name} 按部门上级解析审批人失败：部门上下级配置成环，无法确定上一级负责人。请在 系统设置-部门管理 修正上下级关系后重新提交"
            )));
        }
        let parent = department::Entity::find_by_id(parent_id)
            .one(txn)
            .await?
            .ok_or_else(|| {
                tracing::warn!(
                    instance_id = instance.id,
                    node_id = %node_id,
                    department_id = parent_id,
                    "上级部门记录不存在，审批人解析不出（内部标识仅随本条日志落盘）"
                );
                AppError::validation_displayable(format!(
                    "审批流程节点 {node_name} 按部门上级解析审批人失败：上级部门记录不存在或已被删除。请在 系统设置-部门管理 核对组织层级后重新提交"
                ))
            })?;
        if hop == steps {
            let assignee_id = parent.manager_id.ok_or_else(|| {
                tracing::warn!(
                    instance_id = instance.id,
                    node_id = %node_id,
                    department_id = parent_id,
                    "目标上级部门未配置负责人，审批人解析不出（内部标识仅随本条日志落盘）"
                );
                AppError::validation_displayable(format!(
                    "审批流程节点 {node_name} 按部门上级解析审批人失败：{}。请在 系统设置-部门管理 为该部门指定负责人后重新提交",
                    if sensitive {
                        "该领导所在部门的上一级部门还没有配置负责人，敏感操作无人可终审"
                    } else {
                        "申请人所在部门的上一级部门还没有配置负责人"
                    }
                ))
            })?;
            tracing::info!(
                instance_id = instance.id,
                node_id = %node_id,
                applicant_department_id = base_dept,
                approver_department_id = parent_id,
                sensitive,
                assignee_id,
                "审批人已按申请人部门上级解析成功"
            );
            return Ok(Some(assignee_id));
        }
        current_id = parent_id;
    }
    // steps >= 1 且每层要么 Err 要么在 hop == steps 处 return，此处不可达
    Err(AppError::validation_displayable(format!(
        "审批流程节点 {node_name} 的部门上级审批人解析未达到可闭环的部门负责人，已拒绝。请在 系统设置-部门管理 核对组织层级后重新提交"
    )))
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
