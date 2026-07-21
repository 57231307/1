//! 8D 质量管理流程服务（V15 P0-F20 Batch 480 创建）
//!
//! 业务流程：D0 准备 → D1 团队 → D2 描述问题 → D3 临时措施 → D4 根因 → D5 永久措施
//!           → D6 实施验证 → D7 预防措施 → D8 团队表彰 → 关闭
//!
//! 11 态状态机：
//!   not_started → d0_plan → d1_team → d2_problem → d3_interim → d4_root_cause
//!               → d5_permanent → d6_verify → d7_prevent → d8_recognize → closed
//!
//! 状态转换规则（10 条合法边）：
//!   1. not_started → d0_plan（启动 8D，填 D0 准备信息）
//!   2. d0_plan → d1_team（填 D1 团队成员）
//!   3. d1_team → d2_problem（填 D2 问题描述）
//!   4. d2_problem → d3_interim（填 D3 临时措施）
//!   5. d3_interim → d4_root_cause（填 D4 根因方法/详细/总结）
//!   6. d4_root_cause → d5_permanent（填 D5 永久措施/责任人/计划完成日期）
//!   7. d5_permanent → d6_verify（填 D6 验证结果）
//!   8. d6_verify → d7_prevent（填 D7 预防措施）
//!   9. d7_prevent → d8_recognize（填 D8 闭环总结）
//!   10. d8_recognize → closed（关闭，记录 closed_by + closed_at）
//!
//! 关联任务：P0-F20（建表 + 11 态状态机 + 8D 流程推进）
//! 关联文件：models/quality_8d_report.rs / models/quality_8d_dto.rs /
//!          handlers/quality_8d_handler.rs / routes/quality_8d.rs

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::quality_8d_report::{self, ActiveModel, Entity};
use crate::models::quality_8d_dto::{
    AdvanceStepPayload, CloseEightDRequest, ListEightDQuery, StartEightDRequest,
};
use crate::models::quality_issue;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误
#[derive(Debug, Error)]
pub enum EightDError {
    #[error("8D 报告不存在")]
    NotFound,
    #[error("质量异常不存在")]
    QualityIssueNotFound,
    #[error("该质量异常已存在 8D 报告（一对一约束）")]
    AlreadyExists,
    #[error("当前状态 {current} 不允许此操作（期望 {expected}）")]
    InvalidState {
        current: String,
        expected: &'static str,
    },
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// paginate_with_total 返回 AppError，透传所需
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 11 态状态机枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EightDStatus {
    NotStarted,
    D0Plan,
    D1Team,
    D2Problem,
    D3Interim,
    D4RootCause,
    D5Permanent,
    D6Verify,
    D7Prevent,
    D8Recognize,
    Closed,
}

impl EightDStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::D0Plan => "d0_plan",
            Self::D1Team => "d1_team",
            Self::D2Problem => "d2_problem",
            Self::D3Interim => "d3_interim",
            Self::D4RootCause => "d4_root_cause",
            Self::D5Permanent => "d5_permanent",
            Self::D6Verify => "d6_verify",
            Self::D7Prevent => "d7_prevent",
            Self::D8Recognize => "d8_recognize",
            Self::Closed => "closed",
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed)
    }
}

impl std::fmt::Display for EightDStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// EightDStatus 解析错误
#[derive(Debug, Clone)]
pub struct EightDStatusParseError(pub String);

impl std::fmt::Display for EightDStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EightDStatus 解析失败: {}", self.0)
    }
}

impl std::error::Error for EightDStatusParseError {}

impl FromStr for EightDStatus {
    type Err = EightDStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_started" => Ok(Self::NotStarted),
            "d0_plan" => Ok(Self::D0Plan),
            "d1_team" => Ok(Self::D1Team),
            "d2_problem" => Ok(Self::D2Problem),
            "d3_interim" => Ok(Self::D3Interim),
            "d4_root_cause" => Ok(Self::D4RootCause),
            "d5_permanent" => Ok(Self::D5Permanent),
            "d6_verify" => Ok(Self::D6Verify),
            "d7_prevent" => Ok(Self::D7Prevent),
            "d8_recognize" => Ok(Self::D8Recognize),
            "closed" => Ok(Self::Closed),
            _ => Err(EightDStatusParseError(s.to_string())),
        }
    }
}

/// 8D 质量管理流程服务
pub struct QualityEightDService {
    db: Arc<DatabaseConnection>,
}

impl QualityEightDService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 启动 8D 流程（not_started → d0_plan）
    ///
    /// 业务规则：
    /// 1. quality_issue 必须存在
    /// 2. 该 quality_issue 不能已有 8D 报告（一对一约束）
    /// 3. 创建后立即推进到 d0_plan 状态（避免空报告）
    pub async fn start_8d(
        &self,
        req: StartEightDRequest,
    ) -> Result<quality_8d_report::Model, EightDError> {
        // 校验 quality_issue 存在
        let issue_exists = quality_issue::Entity::find_by_id(req.quality_issue_id)
            .one(&*self.db)
            .await?
            .is_some();
        if !issue_exists {
            return Err(EightDError::QualityIssueNotFound);
        }

        // 校验未存在 8D 报告（UNIQUE 索引会兜底，但提前检查给出友好错误）
        let existing = Entity::find()
            .filter(quality_8d_report::Column::QualityIssueId.eq(req.quality_issue_id))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(EightDError::AlreadyExists);
        }

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            quality_issue_id: Set(req.quality_issue_id),
            status: Set(EightDStatus::D0Plan.as_str().to_string()),
            d0_date: Set(Some(now)),
            d0_prepared_by: Set(Some(req.prepared_by)),
            d0_plan: Set(req.plan),
            d1_date: Set(None),
            d1_team_members: Set(None),
            d2_date: Set(None),
            d2_problem_description: Set(None),
            d3_date: Set(None),
            d3_interim_action: Set(None),
            d4_date: Set(None),
            d4_root_cause_method: Set(None),
            d4_root_cause_detail: Set(None),
            d4_root_cause_summary: Set(None),
            d5_date: Set(None),
            d5_permanent_action: Set(None),
            d5_action_owner: Set(None),
            d5_due_date: Set(None),
            d5_completed_at: Set(None),
            d6_date: Set(None),
            d6_verification_result: Set(None),
            d7_date: Set(None),
            d7_prevention_action: Set(None),
            d8_date: Set(None),
            d8_closure_summary: Set(None),
            closed_at: Set(None),
            closed_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 推进到下一 D 阶段（10 条合法边的 2-9 条，第 1 条由 start_8d 完成，第 10 条由 close_8d 完成）
    ///
    /// 校验规则：报告存在 + 当前 status 与 payload 期望源状态匹配 + 必填字段非空。
    pub async fn advance(
        &self,
        report_id: i64,
        payload: AdvanceStepPayload,
    ) -> Result<quality_8d_report::Model, EightDError> {
        let txn = (*self.db).begin().await?;
        let now = Utc::now();
        let (mut active, current) = Self::load_active_for_advance(&txn, report_id, now).await?;
        // 校验 + 应用 payload：先尝试 D0-D3 转换，再尝试 D4-D8 转换；均不匹配则报状态错误
        let next_status = Self::apply_d0_to_d3_step(&mut active, now, &current, &payload)
            .or_else(|| Self::apply_d4_to_d8_step(&mut active, now, &current, &payload))
            .ok_or_else(|| EightDError::InvalidState {
                current: current.to_string(),
                expected: expected_source_status_for_payload(&payload),
            })?;
        active.status = Set(next_status.as_str().to_string());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 加载 8D 报告 ActiveModel 并解析当前状态（lock_exclusive 防并发推进；写入 updated_at）
    async fn load_active_for_advance(
        txn: &sea_orm::DatabaseTransaction,
        report_id: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(ActiveModel, EightDStatus), EightDError> {
        let existing = Entity::find_by_id(report_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or(EightDError::NotFound)?;
        let current = EightDStatus::from_str(&existing.status)
            .map_err(|_| EightDError::Validation(format!("非法 status: {}", existing.status)))?;
        let mut active: ActiveModel = existing.into();
        active.updated_at = Set(now);
        Ok((active, current))
    }

    /// 应用 D0→D1、D1→D2、D2→D3 状态转换；匹配则写入字段并返回下一状态，不匹配返回 None
    fn apply_d0_to_d3_step(
        active: &mut ActiveModel,
        now: chrono::DateTime<Utc>,
        current: &EightDStatus,
        payload: &AdvanceStepPayload,
    ) -> Option<EightDStatus> {
        match (current, payload) {
            (EightDStatus::D0Plan, AdvanceStepPayload::D1Team { team_members }) => {
                active.d1_date = Set(Some(now));
                active.d1_team_members = Set(Some(team_members.clone()));
                Some(EightDStatus::D1Team)
            }
            (EightDStatus::D1Team, AdvanceStepPayload::D2Problem { problem_description }) => {
                active.d2_date = Set(Some(now));
                active.d2_problem_description = Set(Some(problem_description.clone()));
                Some(EightDStatus::D2Problem)
            }
            (EightDStatus::D2Problem, AdvanceStepPayload::D3Interim { interim_action }) => {
                active.d3_date = Set(Some(now));
                active.d3_interim_action = Set(Some(interim_action.clone()));
                Some(EightDStatus::D3Interim)
            }
            _ => None,
        }
    }

    /// 应用 D3→D4 ... D7→D8 状态转换；匹配则写入字段并返回下一状态，不匹配返回 None
    fn apply_d4_to_d8_step(
        active: &mut ActiveModel,
        now: chrono::DateTime<Utc>,
        current: &EightDStatus,
        payload: &AdvanceStepPayload,
    ) -> Option<EightDStatus> {
        match (current, payload) {
            (EightDStatus::D3Interim, AdvanceStepPayload::D4RootCause { method, detail, summary }) => {
                active.d4_date = Set(Some(now));
                active.d4_root_cause_method = Set(Some(method.as_str().to_string()));
                active.d4_root_cause_detail = Set(Some(detail.clone()));
                active.d4_root_cause_summary = Set(Some(summary.clone()));
                Some(EightDStatus::D4RootCause)
            }
            (EightDStatus::D4RootCause, AdvanceStepPayload::D5Permanent { permanent_action, action_owner, due_date }) => {
                active.d5_date = Set(Some(now));
                active.d5_permanent_action = Set(Some(permanent_action.clone()));
                active.d5_action_owner = Set(Some(action_owner.clone()));
                active.d5_due_date = Set(Some(*due_date));
                Some(EightDStatus::D5Permanent)
            }
            (EightDStatus::D5Permanent, AdvanceStepPayload::D6Verify { verification_result }) => {
                active.d6_date = Set(Some(now));
                active.d6_verification_result = Set(Some(verification_result.clone()));
                // D6 验证通过即视为 D5 永久措施已完成
                active.d5_completed_at = Set(Some(now));
                Some(EightDStatus::D6Verify)
            }
            (EightDStatus::D6Verify, AdvanceStepPayload::D7Prevent { prevention_action }) => {
                active.d7_date = Set(Some(now));
                active.d7_prevention_action = Set(Some(prevention_action.clone()));
                Some(EightDStatus::D7Prevent)
            }
            (EightDStatus::D7Prevent, AdvanceStepPayload::D8Recognize { closure_summary }) => {
                active.d8_date = Set(Some(now));
                active.d8_closure_summary = Set(Some(closure_summary.clone()));
                Some(EightDStatus::D8Recognize)
            }
            _ => None,
        }
    }

    /// 关闭 8D 流程（d8_recognize → closed）
    pub async fn close_8d(
        &self,
        report_id: i64,
        req: CloseEightDRequest,
    ) -> Result<quality_8d_report::Model, EightDError> {
        let txn = (*self.db).begin().await?;

        let existing = Entity::find_by_id(report_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(EightDError::NotFound)?;

        let current = EightDStatus::from_str(&existing.status)
            .map_err(|_| EightDError::Validation(format!("非法 status: {}", existing.status)))?;

        if current != EightDStatus::D8Recognize {
            return Err(EightDError::InvalidState {
                current: current.to_string(),
                expected: "d8_recognize",
            });
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.status = Set(EightDStatus::Closed.as_str().to_string());
        active.closed_at = Set(Some(now));
        active.closed_by = Set(Some(req.closed_by));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, report_id: i64) -> Result<quality_8d_report::Model, EightDError> {
        Entity::find_by_id(report_id)
            .one(&*self.db)
            .await?
            .ok_or(EightDError::NotFound)
    }

    /// 按 quality_issue_id 查询（一对一，最多返回一条）
    pub async fn get_by_quality_issue(
        &self,
        quality_issue_id: i64,
    ) -> Result<Option<quality_8d_report::Model>, EightDError> {
        let model = Entity::find()
            .filter(quality_8d_report::Column::QualityIssueId.eq(quality_issue_id))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 列表查询（带分页与过滤）
    pub async fn list(
        &self,
        query: ListEightDQuery,
    ) -> Result<(Vec<quality_8d_report::Model>, u64), EightDError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = Entity::find();
        if let Some(v) = query.quality_issue_id {
            select = select.filter(quality_8d_report::Column::QualityIssueId.eq(v));
        }
        if let Some(v) = query.status {
            // 校验 status 合法性
            if EightDStatus::from_str(&v).is_err() {
                return Err(EightDError::Validation(format!(
                    "非法 status: {}，合法值：not_started/d0_plan/d1_team/d2_problem/d3_interim/d4_root_cause/d5_permanent/d6_verify/d7_prevent/d8_recognize/closed",
                    v
                )));
            }
            select = select.filter(quality_8d_report::Column::Status.eq(v));
        }

        let paginator = select
            .order_by_desc(quality_8d_report::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}

/// 给定 payload，返回期望的源状态（用于错误信息）
fn expected_source_status_for_payload(payload: &AdvanceStepPayload) -> &'static str {
    match payload {
        AdvanceStepPayload::D1Team { .. } => "d0_plan",
        AdvanceStepPayload::D2Problem { .. } => "d1_team",
        AdvanceStepPayload::D3Interim { .. } => "d2_problem",
        AdvanceStepPayload::D4RootCause { .. } => "d3_interim",
        AdvanceStepPayload::D5Permanent { .. } => "d4_root_cause",
        AdvanceStepPayload::D6Verify { .. } => "d5_permanent",
        AdvanceStepPayload::D7Prevent { .. } => "d6_verify",
        AdvanceStepPayload::D8Recognize { .. } => "d7_prevent",
    }
}
