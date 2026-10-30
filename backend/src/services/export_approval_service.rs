// 敏感数据导出审批 Service（生命周期改造版）
//
// 实现：
//   1. 创建审批请求：落库后立即发起 BPM 流程（process_key=export_approval），
//      审批办理与多级推进全部由 BPM 引擎承担，本表状态只由 BPM 结束事件与令牌生命周期驱动，
//      不再是游离的独立状态机。
//   2. BPM 终审通过（BpmProcessFinished approved=true）→ 生成一次性令牌：
//      明文只在签发瞬间与 token-once 首次查看时出现，库里只留 SHA256 哈希与前缀指纹。
//   3. 凭令牌下载：受理侧集中校验（哈希比对、归属人、资源类型、批准范围、重试窗口与次数）。
//   4. 导出成功 → 令牌消费（status=consumed，明文与哈希清空，禁复用）。
//   5. 未导出成功 → 允许在 5 分钟后重试，同一令牌未成功前提下最多重试 3 次；
//      窗口未到 / 次数用尽 / 已消费 / 已过期 四个态在错误文案与库内状态上均可区分。
//
// 兼容口径：历史行 token_hash IS NULL 且 download_token 有明文，校验侧回退明文比对；
// 迁移不 UPDATE 洗数据，老行按 token_expires_at 自然过期消亡。
//
// 设计依据与关联文件见下方各方法注释与文件头说明；令牌生命周期改造由迁移
// m_export_approval_lifecycle 登记（export_approval_request 补列、status CHECK 扩集、
// token_hash 部分唯一索引），export_approval 流程定义行由 services/bpm_definition_seed.rs
// 的出厂种子在初始化链路与启动链路写入 bpm_process_definition。
// 关联文件：models/export_approval_request.rs / handlers/export_approval_handler.rs /
//           services/bpm_service.rs（start_process/approve_task/cancel_instance）/
//           services/bpm_definition_seed.rs（出厂流程定义登记）/
//           services/event_bus_ops/listener.rs（BpmProcessFinished 分发）

use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::bpm_process_instance;
use crate::models::bpm_task;
use crate::models::dto::bpm_dto::{ApproveTaskRequest, StartProcessRequest};
use crate::models::export_approval_request::{
    ActiveModel as ExportApprovalActiveModel, ApprovalContext, ApprovalStatus, Column, Entity,
    ExportParams, Model, RiskLevel, sensitive_resources,
};
use crate::models::status::bpm_instance as instance_status;
use crate::models::status::bpm_task as bpm_task_status;
use crate::services::bpm_ops::process_key;
use crate::services::bpm_service::BpmService;
use crate::utils::error::AppError;

/// 令牌硬有效期（小时，自终审通过起算，重发不顺延）。
/// 口径说明：用户语义把旧的“5 分钟有效下载窗口”替换为“未成功后 5 分钟重试窗口 +
/// 最多 3 次重试”；每次重试需重发取回新明文，若硬有效期仍按分钟计，重试窗口一过
/// 令牌即过期，重试语义无法成立。裁定为签发日起 24 小时内有效（企业数据导出授权
/// 的通行时效：一次批准的凭据不应跨日悬置），超期落 expired 终态，需重新发起申请。
const TOKEN_LIFETIME_HOURS: i64 = 24;

/// 未导出成功后的重试窗口（距上次尝试至少 5 分钟才允许下一次）
pub const RETRY_WINDOW_MINUTES: i64 = 5;

/// 同一令牌未成功前提下的默认最大重试次数（首次尝试不计入重试）
pub const DEFAULT_MAX_RETRIES: i32 = 3;

/// BPM 业务类型登记值（start_process 的 business_type 与 listener 分发键逐字符一致）
pub const BPM_BUSINESS_TYPE_EXPORT_APPROVAL: &str = "export_approval";

/// 签发令牌的结果：明文只在返回体出现一次，调用方（签发处）负责一次性交付后丢弃
#[derive(Debug, Clone)]
pub struct IssuedToken {
    /// 完整明文（仅此一次可见）
    pub plaintext: String,
    /// SHA256 十六进制（落库列 token_hash）
    pub hash: String,
    /// 前缀指纹（落库列 token_fingerprint，供 UI 展示掩码令牌）
    pub fingerprint: String,
}

/// 令牌当前可重试状态（四个态可区分，供受理侧给出如实提示）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenRetryState {
    /// 允许本次下载尝试
    Eligible,
    /// 上次失败后未满 5 分钟窗口，remaining_secs 为还需等待秒数（至少 1）
    WindowNotReached { remaining_secs: i64 },
    /// 未成功前提下重试次数已用尽
    RetriesExhausted,
    /// 令牌已被成功导出消费
    Consumed,
    /// 令牌已过有效期
    Expired,
}

/// 创建审批请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateApprovalRequest {
    pub resource_type: String,
    pub export_params: serde_json::Value,
    pub estimated_rows: Option<i64>,
    pub file_format: Option<String>,
}

/// 审批操作 DTO
#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    /// 审批意见
    pub comments: Option<String>,
}

/// 审批请求列表 VO（items 为脱敏后的 JSON：明文令牌与哈希不外发）
#[derive(Debug, Serialize)]
pub struct ApprovalListVo {
    pub items: Vec<serde_json::Value>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListApprovalQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub resource_type: Option<String>,
    pub applicant_user_id: Option<i32>,
}

/// 敏感数据导出审批 Service
pub struct ExportApprovalService {
    db: Arc<DatabaseConnection>,
}

impl ExportApprovalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ========================================================================
    // 申请与 BPM 发起
    // ========================================================================

    /// 创建审批请求并发起 BPM 流程
    ///
    /// 风险等级按预估行数评估：< 1000 行 low（单节点审批）；1000 行及以上 medium/high/critical
    /// （双节点审批）。流程分支（一级直结束或进二级）由 BPM 按流程定义中的
    /// `${risk_level}` 条件边决定，本表不再自维护 current_approval_step 推进。
    ///
    /// 补偿口径：BPM 发起失败时删除刚落库的申请行并透传真实错误，
    /// 绝不留下无流程关联的孤儿 pending 申请（用户看不到审批入口、审批人也无从办理）。
    pub async fn create_request(
        &self,
        applicant_user_id: i32,
        applicant_username: String,
        applicant_ip: Option<String>,
        applicant_user_agent: Option<String>,
        req: CreateApprovalRequest,
    ) -> Result<Model, AppError> {
        let file_format = Self::validate_create_request_fields(&req)?;
        let (risk_level, approval_level) = Self::assess_risk_level(req.estimated_rows);
        let now = Utc::now();

        let active = Self::build_export_approval_active(
            applicant_user_id,
            applicant_username.clone(),
            applicant_ip,
            applicant_user_agent,
            &req,
            file_format,
            risk_level.clone(),
            approval_level,
            now,
        );
        let model = active.insert(&*self.db).await?;

        let bpm = BpmService::new(self.db.clone());
        let start_req = StartProcessRequest {
            process_key: process_key::EXPORT_APPROVAL.to_string(),
            business_type: BPM_BUSINESS_TYPE_EXPORT_APPROVAL.to_string(),
            business_id: model.id,
            title: format!("导出审批-{}-{}", req.resource_type, model.id),
            initiator_id: applicant_user_id,
            initiator_name: applicant_username,
            initiator_department_id: None,
            priority: None,
            form_data: None,
            variables: Some(serde_json::json!({
                "risk_level": risk_level,
                "estimated_rows": req.estimated_rows.unwrap_or(0),
                "resource_type": req.resource_type,
            })),
        };
        let started = match bpm.start_process(start_req).await {
            Ok(resp) => resp,
            Err(e) => {
                // 补偿：真实失败即回滚申请行，失败本身如实外抛（不吞、不重试造号）
                if let Err(del_err) = Entity::delete_by_id(model.id).exec(&*self.db).await {
                    tracing::error!(
                        approval_id = model.id,
                        error = %del_err,
                        "导出审批 BPM 发起失败的补偿删除未生效，申请行成为孤儿，需人工处置"
                    );
                }
                return Err(e);
            }
        };

        let mut active: ExportApprovalActiveModel = model.into();
        active.bpm_instance_id = Set(Some(started.instance_id));
        active.updated_at = Set(Utc::now().into());
        let saved = active.update(&*self.db).await?;
        Ok(saved)
    }

    /// 校验创建审批请求字段：资源类型必须敏感，文件格式仅支持 xlsx/pdf（返回规范化后的 file_format（小写））
    /// V15 主线审计 Low 修复：原实现允许 csv，与 .monkeycode/MEMORY.md 规则“成品导入/导出使用 .xlsx/.docx”冲突，
    /// 移除 csv 避免制度声明与代码行为不一致。
    fn validate_create_request_fields(req: &CreateApprovalRequest) -> Result<String, AppError> {
        if !sensitive_resources::is_sensitive(&req.resource_type) {
            return Err(AppError::validation(format!(
                "资源类型 {} 不在敏感资源清单内，无需二级审批",
                req.resource_type
            )));
        }

        let file_format = req
            .file_format
            .clone()
            .unwrap_or_else(|| "xlsx".to_string())
            .to_lowercase();
        if !matches!(file_format.as_str(), "xlsx" | "pdf") {
            return Err(AppError::validation(
                "file_format 仅支持 xlsx/pdf（与 .monkeycode/MEMORY.md 成品导出规则保持一致）",
            ));
        }
        Ok(file_format)
    }

    /// 评估风险等级和审批层级（审批层级：high/critical 必须二级审批；medium 默认二级；low 可一级）。
    /// estimated_rows 为 None 时按 0 行评估（low，一级即可）。
    fn assess_risk_level(estimated_rows: Option<i64>) -> (String, i32) {
        let risk = RiskLevel::from_row_count(estimated_rows.unwrap_or(0));
        let risk_level = risk.as_str().to_string();
        let approval_level = match risk {
            RiskLevel::Low => 1,
            RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical => 2,
        };
        (risk_level, approval_level)
    }

    /// 构建导出审批请求 ActiveModel（pending 状态）
    #[allow(
        clippy::too_many_arguments,
        reason = "申请行字段即建表列全集，拆结构反而丢失与 DDL 的逐列对照"
    )]
    fn build_export_approval_active(
        applicant_user_id: i32,
        applicant_username: String,
        applicant_ip: Option<String>,
        applicant_user_agent: Option<String>,
        req: &CreateApprovalRequest,
        file_format: String,
        risk_level: String,
        approval_level: i32,
        now: DateTime<Utc>,
    ) -> ExportApprovalActiveModel {
        let estimated_rows = req.estimated_rows.unwrap_or(0);
        ExportApprovalActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            applicant_user_id: Set(applicant_user_id),
            applicant_username: Set(applicant_username),
            approver_user_id: Set(None),
            approver_username: Set(None),
            resource_type: Set(req.resource_type.clone()),
            export_params: Set(Some(ExportParams(req.export_params.clone()))),
            estimated_rows: Set(Some(estimated_rows)),
            file_format: Set(file_format),
            status: Set(ApprovalStatus::Pending.as_str().to_string()),
            approval_level: Set(approval_level),
            approver_comments: Set(None),
            approved_at: Set(None),
            rejected_at: Set(None),
            download_token: Set(None),
            token_expires_at: Set(None),
            download_count: Set(0),
            max_downloads: Set(1),
            file_path: Set(None),
            file_size_bytes: Set(None),
            file_checksum: Set(None),
            applicant_ip: Set(applicant_ip),
            approver_ip: Set(None),
            applicant_user_agent: Set(applicant_user_agent),
            risk_level: Set(risk_level),
            context: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            completed_at: Set(None),
            token_hash: Set(None),
            token_fingerprint: Set(None),
            token_viewed: Set(false),
            retry_count: Set(0),
            last_attempt_at: Set(None),
            max_retries: Set(DEFAULT_MAX_RETRIES),
            token_consumed: Set(false),
            token_consumed_at: Set(None),
            bpm_instance_id: Set(None),
        }
    }

    // ========================================================================
    // 审批办理（BPM 代理入口，兼容既有前端端点）
    // ========================================================================

    /// 审批通过：代理到 BPM 引擎的当前待办任务。
    ///
    /// 本表状态不再由本方法直接推进——approve_task 会按流程定义推进节点，
    /// 终审完成时发布 BpmProcessFinished 事件，由 [`on_bpm_approved`] 统一置
    /// approved 并签发令牌。一级通过（流程未结束）时本表保持 pending 不变。
    pub async fn approve(
        &self,
        approval_id: i64,
        approver_user_id: i32,
        approver_username: String,
        req: ApproveRequest,
    ) -> Result<Model, AppError> {
        let task = self.find_pending_bpm_task(approval_id).await?;
        let bpm = BpmService::new(self.db.clone());
        bpm.approve_task(
            ApproveTaskRequest {
                task_id: task.id,
                handler_id: approver_user_id,
                handler_name: approver_username,
                action: crate::services::bpm_ops::task::APPROVE_ACTION.to_string(),
                approval_opinion: req.comments,
                attachment_urls: None,
            },
            Some(approver_user_id),
        )
        .await?;
        // 终审在同一请求内完成时，BpmProcessFinished 事件监听是异步的，直接回读会拿到
        // 尚未回写的 pending。这里按实例终态同步补一次终审回写，回写本身幂等，监听稍后
        // 重复投递会跳过；一级通过（实例未终结）仍保持 pending 不变。
        if self.instance_finished(approval_id).await? {
            self.on_bpm_approved(approval_id, approver_user_id).await?;
        }
        self.get_request(approval_id).await
    }

    /// 判定该申请对应的 BPM 实例是否已正常终结。
    ///
    /// 调用方：审批通过后决定是否同步补终审回写。
    /// 入参：申请 id。传给谁：无。存什么/存哪里：只读 bpm_process_instance，不写库。
    async fn instance_finished(&self, approval_id: i64) -> Result<bool, AppError> {
        let instance = bpm_process_instance::Entity::find()
            .filter(
                bpm_process_instance::Column::BusinessType.eq(BPM_BUSINESS_TYPE_EXPORT_APPROVAL),
            )
            .filter(bpm_process_instance::Column::BusinessId.eq(Some(approval_id)))
            .order_by_desc(bpm_process_instance::Column::Id)
            .one(self.db.as_ref())
            .await?;
        Ok(instance
            .and_then(|row| row.status)
            .is_some_and(|status| status == instance_status::COMPLETED))
    }

    /// 审批拒绝：代理到 BPM 引擎的当前待办任务（BPM 侧拒绝即终止实例，
    /// BpmProcessFinished approved=false 由 [`on_bpm_rejected`] 置本表 rejected）。
    pub async fn reject(
        &self,
        approval_id: i64,
        approver_user_id: i32,
        approver_username: String,
        req: ApproveRequest,
    ) -> Result<Model, AppError> {
        let task = self.find_pending_bpm_task(approval_id).await?;
        let bpm = BpmService::new(self.db.clone());
        bpm.approve_task(
            ApproveTaskRequest {
                task_id: task.id,
                handler_id: approver_user_id,
                handler_name: approver_username,
                action: crate::services::bpm_ops::task::REJECT_ACTION.to_string(),
                approval_opinion: req.comments,
                attachment_urls: None,
            },
            Some(approver_user_id),
        )
        .await?;
        self.get_request(approval_id).await
    }

    /// 申请人取消：校验归属后代理到 BPM 撤回实例（BPM 侧取消待办任务并终止实例，
    /// 本表状态由本方法置 cancelled——撤回不产生 approved=false 的“审批拒绝”语义，
    /// 也不走 BpmProcessFinished 回写，故在此直接落终态）。
    pub async fn cancel(
        &self,
        approval_id: i64,
        applicant_user_id: i32,
    ) -> Result<Model, AppError> {
        let model = self.get_request(approval_id).await?;

        if model.applicant_user_id != applicant_user_id {
            return Err(AppError::permission_denied("仅申请人本人可取消审批请求"));
        }

        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;
        if !status.is_pending_any() {
            return Err(AppError::business(format!(
                "审批请求状态为 {}，仅待审批状态可取消",
                model.status
            )));
        }

        if let Some(instance_id) = model.bpm_instance_id {
            let bpm = BpmService::new(self.db.clone());
            bpm.cancel_instance(
                instance_id,
                Some(applicant_user_id),
                Some("申请人取消导出申请".to_string()),
            )
            .await?;
        } else {
            // 存量兼容：无流程关联的老申请直接落 cancelled（历史数据无 BPM 实例可撤）
            tracing::warn!(
                approval_id,
                "导出审批无 BPM 实例关联（存量数据），取消仅落本表状态"
            );
        }

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Cancelled.as_str().to_string());
        active.updated_at = Set(now.into());
        active.completed_at = Set(Some(now.into()));
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按申请 ID 定位其 BPM 实例下的当前待办任务（审批代理入口的前置）
    async fn find_pending_bpm_task(&self, approval_id: i64) -> Result<bpm_task::Model, AppError> {
        let model = self.get_request(approval_id).await?;
        let instance_id = model.bpm_instance_id.ok_or_else(|| {
            AppError::business(format!(
                "导出审批 {approval_id} 未关联 BPM 流程实例（存量数据），无法在此办理，请通过 BPM 审批中心处理"
            ))
        })?;
        bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .filter(bpm_task::Column::Status.eq(bpm_task_status::PENDING))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!(
                    "导出审批 {approval_id} 的 BPM 实例没有待办任务，流程可能已终审完成或已终止"
                ))
            })
    }

    // ========================================================================
    // BPM 事件回写（本表状态机唯一权威驱动源）
    // ========================================================================

    /// BPM 终审通过回调（由 event_bus listener 的 BpmProcessFinished 分发调用）：
    /// 置 approved 并签发一次性令牌。
    ///
    /// 审批人姓名按 approver_user_id 从 users 表解析（事件只携带用户 ID）；
    /// 解析失败留错误日志并以空姓名落库（该列可空，不伪造姓名）。
    /// 幂等口径：已 approved（且令牌未消费）时直接返回现有指纹、不重发新令牌——
    /// 重发即作废旧令牌的语义留给显式的 [`rotate_token`]；事件重复投递不得产生两个可用令牌。
    /// 已进终态（rejected/cancelled/expired/consumed/exhausted）时如实拒绝回写并留错误日志。
    pub async fn on_bpm_approved(
        &self,
        approval_id: i64,
        approver_user_id: i32,
    ) -> Result<Model, AppError> {
        let approver_username = self.resolve_username(approver_user_id).await;
        let model = self.get_request(approval_id).await?;
        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;

        match status {
            ApprovalStatus::Approved => {
                tracing::info!(
                    approval_id,
                    "BPM 终审通过事件重复投递，申请已 approved，不重发令牌（幂等跳过）"
                );
                return Ok(model);
            }
            ApprovalStatus::Pending | ApprovalStatus::PendingL2 => {}
            other => {
                tracing::error!(
                    approval_id,
                    status = other.as_str(),
                    "BPM 终审通过事件回写被拒绝：申请已处于不可批准的状态"
                );
                return Err(AppError::business(format!(
                    "导出审批当前状态为 {}，不接受审批通过回写",
                    other.as_str()
                )));
            }
        }

        let issued = Self::issue_token();
        let now = Utc::now();
        let expires_at = now + Duration::hours(TOKEN_LIFETIME_HOURS);
        let target_level = model.approval_level;
        let existing_ctx = model.context.clone();

        let mut active: ExportApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Approved.as_str().to_string());
        active.approver_user_id = Set(Some(approver_user_id));
        active.approver_username = Set(Some(approver_username));
        active.approved_at = Set(Some(now.into()));
        // 明文暂存 download_token 列，仅供 token-once 首次查看；查看后置 NULL。
        active.download_token = Set(Some(issued.plaintext));
        active.token_hash = Set(Some(issued.hash));
        active.token_fingerprint = Set(Some(issued.fingerprint));
        active.token_viewed = Set(false);
        active.token_expires_at = Set(Some(expires_at.into()));
        active.updated_at = Set(now.into());
        active.completed_at = Set(None);
        active.context = Set(Some(write_current_approval_step(
            existing_ctx.as_ref(),
            target_level,
        )));

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按用户 ID 解析真实姓名（real_name 优先，空则 username）；查不到留错误日志返回空串
    /// （approver_username 列可空，不伪造、不占位）
    async fn resolve_username(&self, user_id: i32) -> String {
        use crate::models::user;
        match user::Entity::find_by_id(user_id).one(&*self.db).await {
            Ok(Some(u)) => u
                .real_name
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or(u.username),
            Ok(None) => {
                tracing::error!(user_id, "导出审批回写时用户不存在，审批人姓名按空落库");
                String::new()
            }
            Err(e) => {
                tracing::error!(user_id, error = %e, "导出审批回写解析审批人姓名失败，按空落库");
                String::new()
            }
        }
    }

    /// BPM 终审拒绝回调：置 rejected 终态。已是终态时幂等跳过。
    pub async fn on_bpm_rejected(
        &self,
        approval_id: i64,
        approver_user_id: i32,
    ) -> Result<Model, AppError> {
        let model = self.get_request(approval_id).await?;
        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;

        if status == ApprovalStatus::Rejected {
            return Ok(model);
        }
        if status.is_terminal() {
            tracing::error!(
                approval_id,
                status = status.as_str(),
                "BPM 终审拒绝事件回写被拒绝：申请已处于终态"
            );
            return Err(AppError::business(format!(
                "导出审批当前状态为 {}，不接受拒绝回写",
                status.as_str()
            )));
        }

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Rejected.as_str().to_string());
        active.approver_user_id = Set(Some(approver_user_id));
        active.rejected_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        active.completed_at = Set(Some(now.into()));
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 显式重发令牌：作废旧令牌明文与哈希、签发新令牌并置可再次查看一次。
    /// 仅申请人本人可对 approved（未消费）状态发起；重发后旧明文立即失效。
    ///
    /// 口径（与一次性凭据形态配套）：
    /// - 硬有效期不顺延（token_expires_at 保持终审时刻起算的 24 小时，防靠重发永生）；
    /// - 累计重试计数与最近尝试时刻不清零（重试额度按申请生命周期记，换明文不刷新额度）；
    /// - 已过硬有效期或已用尽额度（exhausted/consumed/expired）一律拒绝重发。
    pub async fn rotate_token(
        &self,
        approval_id: i64,
        applicant_user_id: i32,
    ) -> Result<IssuedToken, AppError> {
        let model = self.get_request(approval_id).await?;
        if model.applicant_user_id != applicant_user_id {
            return Err(AppError::permission_denied("仅申请人本人可重发导出令牌"));
        }
        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;
        if status != ApprovalStatus::Approved {
            return Err(AppError::business(format!(
                "审批请求状态为 {}，仅已批准且未消费的令牌可重发（用尽或过期后请重新发起导出申请）",
                status.as_str()
            )));
        }
        if let Some(expires_at) = model.token_expires_at {
            if Utc::now() > expires_at {
                return Err(AppError::business(
                    "令牌已超过签发日起 24 小时的硬有效期，重发不再放行，请重新发起导出申请"
                        .to_string(),
                ));
            }
        }
        if model.retry_count >= model.max_retries {
            return Err(AppError::business(
                "未成功重试次数已用尽，重发不再放行，请重新发起导出申请".to_string(),
            ));
        }

        let issued = Self::issue_token();
        let now = Utc::now();

        let mut active: ExportApprovalActiveModel = model.into();
        active.download_token = Set(Some(issued.plaintext.clone()));
        active.token_hash = Set(Some(issued.hash.clone()));
        active.token_fingerprint = Set(Some(issued.fingerprint.clone()));
        active.token_viewed = Set(false);
        active.updated_at = Set(now.into());

        active.update(&*self.db).await?;
        Ok(issued)
    }

    // ========================================================================
    // 令牌查看与校验
    // ========================================================================

    /// 一次性查看令牌明文（签发后仅可取走一次）。
    ///
    /// 第一次调用：返回完整明文并把 `token_viewed` 置真、清空明文列——此后任何接口
    /// 都不再出现该明文。第二次起调用：返回 `AlreadyViewed` 并只携带指纹，
    /// 提示走 [`rotate_token`] 重发。仅申请人本人可取；admin 同样不可代取（令牌即凭据，
    /// 代取等于代持凭据）。
    pub async fn view_token_once(
        &self,
        approval_id: i64,
        caller_user_id: i32,
    ) -> Result<TokenViewOutcome, AppError> {
        let model = self.get_request(approval_id).await?;
        if model.applicant_user_id != caller_user_id {
            return Err(AppError::permission_denied(
                "导出令牌仅申请人本人可查看，管理员也不可代取",
            ));
        }
        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;
        if status != ApprovalStatus::Approved {
            return Err(AppError::business(format!(
                "审批请求状态为 {}，仅已批准的申请可查看令牌",
                status.as_str()
            )));
        }

        if model.token_viewed || model.token_consumed {
            return Ok(TokenViewOutcome::AlreadyViewed {
                fingerprint: model.token_fingerprint.clone().unwrap_or_default(),
            });
        }

        let plaintext = model.download_token.clone().ok_or_else(|| {
            // viewed=false 但明文已空属数据异常（比如老令牌过期清理后的行），如实报出
            AppError::business(
                "令牌明文不可用（可能已过有效期被清理），请重发令牌后再查看".to_string(),
            )
        })?;

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.into();
        active.token_viewed = Set(true);
        active.download_token = Set(None);
        active.updated_at = Set(now.into());
        active.update(&*self.db).await?;

        Ok(TokenViewOutcome::Token(plaintext))
    }

    /// 校验下载令牌（哈希优先，历史明文回退），并返回申请行供 handler 记录消费。
    ///
    /// 校验链（全部 fail-closed）：
    /// 1. 按 SHA256(呈现令牌) 查 token_hash；查不到时回退查 download_token 明文列
    ///    （历史行兼容，明文比对用恒时比较，见 utils 侧说明）；两者都查不到 → 拒绝。
    /// 2. 状态必须为 approved。
    /// 3. 有效期（token_expires_at）未到；已过则落 expired 终态并拒绝。
    /// 4. 未被成功导出消费（token_consumed=false 且 download_count < max_downloads）。
    pub async fn verify_download_token(&self, token: &str) -> Result<Model, AppError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(AppError::permission_denied("下载令牌无效"));
        }

        let hash = Self::sha256_hex(token.as_bytes());
        let model = Entity::find()
            .filter(Column::TokenHash.eq(&hash))
            .one(&*self.db)
            .await?;
        let model = match model {
            Some(m) => m,
            None => {
                // 历史行回退：token_hash IS NULL 且明文列仍在
                Entity::find()
                    .filter(Column::DownloadToken.eq(token))
                    .filter(Column::TokenHash.is_null())
                    .one(&*self.db)
                    .await?
                    .ok_or_else(|| AppError::permission_denied("下载令牌无效"))?
            }
        };

        self.assert_token_consumable(&model).await
    }

    /// 对已定位的申请行做可消费性校验（供哈希路径与受理路径共用）
    async fn assert_token_consumable(&self, model: &Model) -> Result<Model, AppError> {
        let status = ApprovalStatus::parse_status(&model.status).ok_or_else(|| {
            AppError::internal(format!("导出审批状态值不可解析: {}", model.status))
        })?;

        if status == ApprovalStatus::Consumed || model.token_consumed {
            return Err(AppError::permission_denied(format!(
                "导出令牌已被使用（状态 {}），一次性令牌禁止复用",
                status.as_str()
            )));
        }
        if status != ApprovalStatus::Approved {
            return Err(AppError::permission_denied(format!(
                "审批状态为 {}，不可下载",
                status.as_str()
            )));
        }

        if let Some(expires_at) = model.token_expires_at {
            if Utc::now() > expires_at {
                let mut active: ExportApprovalActiveModel = model.clone().into();
                let now = Utc::now();
                active.status = Set(ApprovalStatus::Expired.as_str().to_string());
                active.updated_at = Set(now.into());
                active.completed_at = Set(Some(now.into()));
                active.update(&*self.db).await?;
                return Err(AppError::permission_denied("下载令牌已过期"));
            }
        }

        if model.download_count >= model.max_downloads {
            return Err(AppError::permission_denied(format!(
                "下载次数已达上限 {}",
                model.max_downloads
            )));
        }

        Ok(model.clone())
    }

    /// 组装本次导出请求实际携带的范围参数，形状与申请落库的 export_params 同形。
    ///
    /// 功能：把导出端点真实读取的筛选项收敛成一个对象，本次没带的维度不进对象。
    /// 调用方：各敏感资源导出 handler 的受理腿。
    /// 入参：项名与该项取值，None 表示本次请求未携带这一维。
    /// 传给谁：交 enforce_export_download_secured 的 requested_params，
    /// 由 validate_scope_against_approval 与批准时的参数做逐项比对。
    /// 存什么：不落库，只参与受理门的比对。
    /// 存哪里：不适用。
    ///
    /// 取值保持原形态，数字仍是数字、布尔仍是布尔，不做字符串化，否则与批准
    /// 参数的逐字相等比对会自己造出差集。数字 0 与布尔 false 是有效取值，必须
    /// 进对象，不按空值丢掉。
    pub fn scope_params(items: &[(&str, Option<serde_json::Value>)]) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for (key, value) in items {
            if let Some(v) = value {
                obj.insert((*key).to_string(), v.clone());
            }
        }
        serde_json::Value::Object(obj)
    }

    /// 敏感导出 fail-closed 受理门（单一入口），资源导出 handler 的受理腿依次校验：
    /// 令牌非空 → 令牌哈希校验 → 调用者必须是被批准的申请人本人（防令牌被转发复用）
    /// → 资源类型匹配 → 实际请求范围与批准范围逐项一致（防超范围导出）
    /// → 重试状态门与尝试登记。
    ///
    /// 尝试登记口径：每次通过校验进入导出即记一次尝试（last_attempt_at=now；
    /// 若此前已有尝试则 retry_count+1）。导出成功由 handler 随后调 [`record_download`]
    /// 置 consumed；导出未成功时 handler 不会调用 record_download，该尝试自然成为
    /// 一次失败，计入重试额度。窗口未到 / 次数用尽 / 已消费 / 已过期 四态分别给出
    /// 可区分文案，次数用尽同时落 exhausted 终态。
    ///
    /// requested_params 由调用方经 scope_params 组装，传 None 等于关掉范围核验，
    /// 只允许确实不存在任何数据范围维度的端点使用。
    #[allow(
        clippy::too_many_arguments,
        reason = "受理门即校验要素全集：令牌/资源/调用者/请求范围，拆参会丢契约对照"
    )]
    pub async fn enforce_export_download_secured(
        &self,
        token: Option<&str>,
        expected_resource: &str,
        caller_user_id: i32,
        requested_params: Option<serde_json::Value>,
    ) -> Result<Model, AppError> {
        let token = token.unwrap_or("").trim();
        if token.is_empty() {
            return Err(AppError::permission_denied(
                "敏感资源导出需先获得导出审批令牌",
            ));
        }

        let model = self.verify_download_token(token).await?;

        if model.applicant_user_id != caller_user_id {
            return Err(AppError::permission_denied(
                "导出令牌只能由被批准的申请人本人使用",
            ));
        }

        if model.resource_type != expected_resource {
            return Err(AppError::permission_denied(format!(
                "导出令牌资源类型不匹配：期望 {}，实际 {}",
                expected_resource, model.resource_type
            )));
        }

        if let Some(requested) = requested_params {
            self.validate_scope_against_approval(&model, &requested)?;
        }

        self.gate_and_register_attempt(&model).await
    }

    /// 范围比对：请求参数必须是批准参数集合的子集，且同名维度取值逐字相等。
    ///
    /// 超范围（请求里有批准外的键）与改范围（同名键值不同）一律点名差集拒绝；
    /// 批准里有而请求里没有的维度视为放大了数据范围，同样拒绝并提示缺哪个维度。
    fn validate_scope_against_approval(
        &self,
        model: &Model,
        requested: &serde_json::Value,
    ) -> Result<(), AppError> {
        let approved_obj = match model.export_params.as_ref() {
            Some(ExportParams(v)) if v.is_object() => v.as_object().cloned().unwrap_or_default(),
            _ => serde_json::Map::new(),
        };
        let requested_obj = match requested {
            v if v.is_object() => v.as_object().cloned().unwrap_or_default(),
            _ => {
                return Err(AppError::validation(
                    "导出请求参数必须是对象形状（与申请时 export_params 同形）",
                ));
            }
        };

        let extra: Vec<&String> = requested_obj
            .keys()
            .filter(|k| !approved_obj.contains_key(*k))
            .collect();
        let missing: Vec<&String> = approved_obj
            .keys()
            .filter(|k| !requested_obj.contains_key(*k))
            .collect();
        let mismatched: Vec<&String> = requested_obj
            .iter()
            .filter(|(k, v)| match approved_obj.get(*k) {
                Some(av) => av != *v,
                None => false,
            })
            .map(|(k, _)| k)
            .collect();

        if extra.is_empty() && missing.is_empty() && mismatched.is_empty() {
            return Ok(());
        }

        let mut hints: Vec<String> = Vec::new();
        if !extra.is_empty() {
            hints.push(format!("新增了未批准的维度 {:?}", extra));
        }
        if !missing.is_empty() {
            hints.push(format!("丢失了批准时的维度 {:?}（扩大数据范围）", missing));
        }
        if !mismatched.is_empty() {
            hints.push(format!("改变了批准时的取值 {:?}", mismatched));
        }
        Err(AppError::permission_denied(format!(
            "导出范围超出批准范围：{}",
            hints.join("；")
        )))
    }

    /// 重试状态门 + 尝试登记（enforce_export_download_secured 受理门的收口环节）
    async fn gate_and_register_attempt(&self, model: &Model) -> Result<Model, AppError> {
        match Self::retry_state_of(model, Utc::now()) {
            TokenRetryState::Eligible => {}
            TokenRetryState::WindowNotReached { remaining_secs } => {
                return Err(AppError::business(format!(
                    "上次导出未成功，请在 {remaining_secs} 秒后重试（重试窗口 5 分钟）"
                )));
            }
            TokenRetryState::RetriesExhausted => {
                // 落终态后拒绝（幂等：已是 exhausted 时 update 同样值）
                if ApprovalStatus::parse_status(&model.status) != Some(ApprovalStatus::Exhausted) {
                    let mut active: ExportApprovalActiveModel = model.clone().into();
                    let now = Utc::now();
                    active.status = Set(ApprovalStatus::Exhausted.as_str().to_string());
                    active.updated_at = Set(now.into());
                    active.completed_at = Set(Some(now.into()));
                    active.update(&*self.db).await?;
                }
                return Err(AppError::business(
                    "该导出令牌的未成功重试次数已用尽，本次申请不再允许导出，请重新发起导出申请",
                ));
            }
            TokenRetryState::Consumed => {
                return Err(AppError::permission_denied("导出令牌已被使用，禁止复用"));
            }
            TokenRetryState::Expired => {
                return Err(AppError::permission_denied("下载令牌已过期"));
            }
        }

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.clone().into();
        // 已有尝试史则本次属重试，计入额度；首次尝试只登记时刻
        if model.last_attempt_at.is_some() {
            active.retry_count = Set(model.retry_count + 1);
        }
        active.last_attempt_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        let registered = active.update(&*self.db).await?;
        Ok(registered)
    }

    /// 判定令牌重试状态（四态可区分）。
    ///
    /// 无 self、时钟显式入参：活体与纯逻辑用例都可注入确定时刻钉边界，
    /// 不依赖真等 5 分钟（5 分钟整点按“窗口已过、允许重试”确定收敛）。
    pub fn retry_state_of(model: &Model, now: DateTime<Utc>) -> TokenRetryState {
        let status = match ApprovalStatus::parse_status(&model.status) {
            Some(s) => s,
            None => return TokenRetryState::Eligible,
        };
        if status == ApprovalStatus::Consumed || model.token_consumed {
            return TokenRetryState::Consumed;
        }
        if status == ApprovalStatus::Exhausted {
            return TokenRetryState::RetriesExhausted;
        }
        if status == ApprovalStatus::Expired {
            return TokenRetryState::Expired;
        }
        if let Some(expires_at) = model.token_expires_at {
            if now > expires_at {
                return TokenRetryState::Expired;
            }
        }

        match model.last_attempt_at {
            None => TokenRetryState::Eligible,
            Some(last) => {
                // 额度判定先行：已用完重试次数则不再给“等待”类提示
                if model.retry_count >= model.max_retries {
                    return TokenRetryState::RetriesExhausted;
                }
                let window_secs = RETRY_WINDOW_MINUTES * 60;
                let elapsed_secs = now
                    .signed_duration_since(last.with_timezone(&Utc))
                    .num_seconds();
                if elapsed_secs < window_secs {
                    let remaining = (window_secs - elapsed_secs).max(1);
                    return TokenRetryState::WindowNotReached {
                        remaining_secs: remaining,
                    };
                }
                TokenRetryState::Eligible
            }
        }
    }

    // ========================================================================
    // 消费与留痕
    // ========================================================================

    /// 记录文件导出成功（handler 在生成并交付文件后调用；调用点必须 `.await?` 透传）。
    ///
    /// 成功后令牌进入一次性终态：status=consumed、token_consumed=true、
    /// 清空明文列与哈希列（哈希不再需要，明文在 view-once 后本就已空）。
    pub async fn record_download(
        &self,
        approval_id: i64,
        file_path: String,
        file_size_bytes: i64,
        file_checksum: String,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        let now = Utc::now();
        let new_count = model.download_count + 1;
        if new_count > model.max_downloads {
            return Err(AppError::business(format!(
                "导出次数已超出批准上限（{}），拒绝登记",
                model.max_downloads
            )));
        }
        let consumes_token = new_count >= model.max_downloads;

        let mut active: ExportApprovalActiveModel = model.into();
        active.download_count = Set(new_count);
        active.file_path = Set(Some(file_path));
        active.file_size_bytes = Set(Some(file_size_bytes));
        active.file_checksum = Set(Some(file_checksum));
        active.updated_at = Set(now.into());
        if consumes_token {
            // 一次性消费终态：状态、消费标记、消费时刻落库，明文与哈希清空（哈希不再需要）
            active.status = Set(ApprovalStatus::Consumed.as_str().to_string());
            active.token_consumed = Set(true);
            active.token_consumed_at = Set(Some(now.into()));
            active.completed_at = Set(Some(now.into()));
            active.download_token = Set(None);
            active.token_hash = Set(None);
        }

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 显式登记导出失败，导出腿任一步失败都走这一条。
    ///
    /// 功能：把失败原因留痕进申请的 context，并即时按重试额度判定是否落 exhausted 终态。
    /// 调用方：run_export_leg（各资源导出 handler 经它执行导出腿）。
    /// 入参：approval_id 是被受理的申请行主键，reason 是失败原因的原文。
    /// 传给谁：写导出审批表本行，不外发。
    /// 存什么：retry_count 加一、last_attempt_at 置当前、context.last_failure 记时刻与原因，
    /// 达到 max_retries 时另置 status 与 completed_at。
    /// 存哪里：export_approval_request 本行。
    pub async fn record_export_failure(
        &self,
        approval_id: i64,
        reason: &str,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        let now = Utc::now();
        let new_retry = model.retry_count + 1;
        let exhausts = new_retry >= model.max_retries;
        let existing_ctx = model.context.clone();

        let mut active: ExportApprovalActiveModel = model.into();
        active.retry_count = Set(new_retry);
        active.last_attempt_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        if exhausts {
            active.status = Set(ApprovalStatus::Exhausted.as_str().to_string());
            active.completed_at = Set(Some(now.into()));
        }
        // 失败原因写进 context 留痕（不外显给列表接口）
        let mut ctx = match existing_ctx {
            Some(ApprovalContext(v)) => v,
            None => serde_json::Value::Object(serde_json::Map::new()),
        };
        if !ctx.is_object() {
            ctx = serde_json::Value::Object(serde_json::Map::new());
        }
        if let Some(obj) = ctx.as_object_mut() {
            obj.insert(
                "last_failure".to_string(),
                serde_json::json!({ "at": now.to_rfc3339(), "reason": reason }),
            );
        }
        active.context = Set(Some(ApprovalContext(ctx)));

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 执行导出腿并在任一步失败时登记一次失败留痕，受理门之后的取数与成文件都走这一条。
    ///
    /// 功能：把受理门之后到登记消费之前的一段执行体统一交给失败记账，失败不改变错误原文。
    /// 调用方：各敏感资源导出 handler，在 enforce_export_download_secured 放行之后。
    /// 入参：approval_id 是刚被放行的申请行主键；leg 是该段执行体，产出 Result。
    /// 传给谁：成功时把执行体原值交回 handler 继续成文件；失败时把原因原文交
    /// record_export_failure 记账，随后把同一个错误原样回给调用方。
    /// 存什么：成功不落库；失败时由 record_export_failure 落重试计数与失败原因留痕。
    /// 存哪里：export_approval_request 本行。
    ///
    /// 登记消费之后的成文件步骤不在本段内，那一阶段令牌已进入消费终态，
    /// 再记一次失败会把已消费的令牌改成重试额度耗尽，两类终态不得混写。
    /// 记账本身失败时不掩盖导出错误，只把记账错误落日志。
    pub async fn run_export_leg<F, T>(&self, approval_id: i64, leg: F) -> Result<T, AppError>
    where
        F: std::future::Future<Output = Result<T, AppError>>,
    {
        match leg.await {
            Ok(value) => Ok(value),
            Err(err) => {
                if let Err(record_err) = self
                    .record_export_failure(approval_id, &err.to_string())
                    .await
                {
                    tracing::warn!(
                        "导出失败留痕登记未成功 申请 {} 导出错误 {} 登记错误 {}",
                        approval_id,
                        err,
                        record_err
                    );
                }
                Err(err)
            }
        }
    }

    // ========================================================================
    // 查询（脱敏出参）
    // ========================================================================

    /// 审批请求列表查询（出参脱敏：明文令牌与哈希不外发）
    pub async fn list_requests(&self, q: ListApprovalQuery) -> Result<ApprovalListVo, AppError> {
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = Entity::find();
        if let Some(s) = &q.status {
            select = select.filter(Column::Status.eq(s));
        }
        if let Some(rt) = &q.resource_type {
            select = select.filter(Column::ResourceType.eq(rt));
        }
        if let Some(uid) = q.applicant_user_id {
            select = select.filter(Column::ApplicantUserId.eq(uid));
        }

        let total = select.clone().count(&*self.db).await?;
        let paginator = select
            .order_by_desc(Column::CreatedAt)
            .paginate(&*self.db, page_size);
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok(ApprovalListVo {
            items: items.iter().map(masked_json).collect(),
            total,
            page,
            page_size,
        })
    }

    /// 审批请求详情（出参脱敏）
    pub async fn get_request(&self, id: i64) -> Result<Model, AppError> {
        Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("导出审批请求不存在: id={}", id)))
    }

    /// 详情出参的脱敏 JSON（handler 对外序列化统一走这里）
    pub fn detail_masked(&self, model: &Model) -> serde_json::Value {
        masked_json(model)
    }

    /// 当前用户的待审批任务（BPM 化后在途审批态为 pending，存量行的 pending_l2
    /// 一并纳入可见面）：
    /// - admin：所有在途申请；
    /// - 普通用户：申请人非本人的在途申请（具体可办理性由 BPM 工单归属裁决，
    ///   本表不再维护 approver 指派态）。
    pub async fn list_pending_for_user(
        &self,
        user_id: i32,
        is_admin: bool,
        q: ListApprovalQuery,
    ) -> Result<ApprovalListVo, AppError> {
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = Entity::find().filter(Column::Status.is_in([
            ApprovalStatus::Pending.as_str(),
            ApprovalStatus::PendingL2.as_str(),
        ]));

        if !is_admin {
            use sea_orm::sea_query::Expr;
            select = select.filter(
                Condition::all().add(Expr::cust_with_values("applicant_user_id <> $1", [user_id])),
            );
        }

        let total = select.clone().count(&*self.db).await?;
        let paginator = select
            .order_by_desc(Column::CreatedAt)
            .paginate(&*self.db, page_size);
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok(ApprovalListVo {
            items: items.iter().map(masked_json).collect(),
            total,
            page,
            page_size,
        })
    }

    /// 清理过期 token（定时任务调用）（将已过期但仍为 approved 状态的请求标记为 expired）
    #[allow(dead_code, reason = "预留定时任务接入点：过期 token 后台清理")]
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, AppError> {
        let now = Utc::now();
        let expired = Entity::find()
            .filter(Column::Status.eq(ApprovalStatus::Approved.as_str()))
            .filter(Column::TokenExpiresAt.lt(now))
            .all(&*self.db)
            .await?;

        let mut count = 0u64;
        for model in expired {
            let mut active: ExportApprovalActiveModel = model.into();
            active.status = Set(ApprovalStatus::Expired.as_str().to_string());
            active.updated_at = Set(now.into());
            active.completed_at = Set(Some(now.into()));
            active.update(&*self.db).await?;
            count += 1;
        }

        Ok(count)
    }

    // ========================================================================
    // 工具
    // ========================================================================

    /// 签发一次性令牌：明文仅存在于返回体，调用方负责一次性交付。
    /// pub 供审计工具与活体用例复算哈希比对（纯函数，无库副作用）。
    pub fn issue_token() -> IssuedToken {
        let plaintext = format!("exp_{}", Uuid::new_v4().simple());
        let hash = Self::sha256_hex(plaintext.as_bytes());
        // 指纹取明文前 12 字符（exp_ + 8 位十六进制前缀），足以让用户辨认但不构成可用凭据
        let fingerprint: String = plaintext.chars().take(12).collect();
        IssuedToken {
            plaintext,
            hash,
            fingerprint,
        }
    }

    /// SHA256 十六进制摘要（小写）。pub 供校验侧与用例复算同源比对（纯函数）。
    pub fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut out = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(out, "{b:02x}");
        }
        out
    }
}

/// view_token_once 的两种结局：首次取走返回明文；已取过只回指纹
#[derive(Debug, Clone)]
pub enum TokenViewOutcome {
    /// 首次查看：完整明文（调用方序列化返回后不得再留存）
    Token(String),
    /// 已查看过：仅指纹提示
    AlreadyViewed { fingerprint: String },
}

/// 出参脱敏：明文令牌与哈希一律不外发，只保留指纹与 viewed 标记
fn masked_json(model: &Model) -> serde_json::Value {
    let mut v = serde_json::to_value(model)
        .unwrap_or_else(|e| serde_json::json!({"serialization_error": e.to_string()}));
    if let Some(obj) = v.as_object_mut() {
        obj.insert("download_token".to_string(), serde_json::Value::Null);
        obj.insert("token_hash".to_string(), serde_json::Value::Null);
    }
    v
}

/// 写入 current_approval_step 到 context（历史字段，仅供存量行读取兼容与事件回写标记）
fn write_current_approval_step(existing: Option<&ApprovalContext>, step: i32) -> ApprovalContext {
    let mut value = match existing {
        Some(ctx) => ctx.0.clone(),
        None => serde_json::Value::Object(serde_json::Map::new()),
    };
    if !value.is_object() {
        value = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(obj) = value.as_object_mut() {
        obj.insert("current_approval_step".to_string(), serde_json::json!(step));
    }
    ApprovalContext(value)
}
