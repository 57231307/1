//! 大货批色审批服务（V15 P0-F15/F16/F17 创建）
//!
//! 业务流程：剪样 → 发送客户 → 客户批色确认 → 通过/拒绝/返工/降级/报废
//!
//! 8 态状态机：
//!   pending → sampled → sent_to_customer → approved / rejected / rework
//!                                                       ↓
//!                                                 downgraded / scrapped
//!
//! 状态转换规则（共 9 条合法边）：
//!   1. pending → sampled（剪大货样，P0-F16）
//!   2. sampled → sent_to_customer（发送客户批色）
//!   3. sent_to_customer → approved（客户批色确认通过，P0-F17）
//!   4. sent_to_customer → rejected（客户拒绝，终态）
//!   5. sent_to_customer → rework（客户要求返工）
//!   6. rework → sampled（返工后重新剪样）
//!   7. approved → downgraded（降级处理，终态）
//!   8. approved → scrapped（报废处理，终态）
//!   9. pending/sampled → scrapped（直接报废，终态）
//!
//! 关联任务：P0-F15（建表）/ P0-F16（剪大货样）/ P0-F17（客户批色确认）
//! P0-F19 ship_order 校验由 services/so/delivery.rs 调用 validate_bulk_color_approval()

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::bulk_color_approval::{self, ActiveModel, Entity};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum BulkColorApprovalError {
    #[error("批色记录不存在")]
    NotFound,
    #[error("销售订单不存在")]
    SalesOrderNotFound,
    #[error("染色批次不存在")]
    DyeBatchNotFound,
    #[error("客户不存在")]
    CustomerNotFound,
    #[error("当前状态 {0} 不允许此操作")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 批色状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// 待剪样
    Pending,
    /// 已剪样
    Sampled,
    /// 已发送客户批色
    SentToCustomer,
    /// 客户批准（解除发货门禁）
    Approved,
    /// 客户拒绝（终态）
    Rejected,
    /// 客户要求返工
    Rework,
    /// 降级处理（终态）
    Downgraded,
    /// 报废（终态）
    Scrapped,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sampled => "sampled",
            Self::SentToCustomer => "sent_to_customer",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Rework => "rework",
            Self::Downgraded => "downgraded",
            Self::Scrapped => "scrapped",
        }
    }

    /// 是否为终态（终态不可再转换）
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Approved | Self::Rejected | Self::Downgraded | Self::Scrapped
        )
    }

    /// 是否解除发货门禁
    ///
    /// 业务规则：仅 approved 状态解除门禁；其它状态保持 blocking=true
    /// 注意：downgraded/scrapped 虽为终态但仍阻断发货（需重新安排生产或换缸）
    pub fn unblocks_delivery(&self) -> bool {
        matches!(self, Self::Approved)
    }
}

/// ApprovalStatus 解析错误
#[derive(Debug, Clone)]
pub struct ApprovalStatusParseError(pub String);

impl std::fmt::Display for ApprovalStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApprovalStatus 解析失败: {}", self.0)
    }
}

impl std::error::Error for ApprovalStatusParseError {}

impl FromStr for ApprovalStatus {
    type Err = ApprovalStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "sampled" => Ok(Self::Sampled),
            "sent_to_customer" => Ok(Self::SentToCustomer),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "rework" => Ok(Self::Rework),
            "downgraded" => Ok(Self::Downgraded),
            "scrapped" => Ok(Self::Scrapped),
            _ => Err(ApprovalStatusParseError(s.to_string())),
        }
    }
}

/// 批色记录查询参数
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ListBulkColorApprovalQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sales_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub customer_id: Option<i64>,
    pub approval_status: Option<String>,
    pub from_date: Option<chrono::DateTime<Utc>>,
    pub to_date: Option<chrono::DateTime<Utc>>,
}

/// 创建参数（封装以避免 clippy::too_many_arguments）
#[derive(Debug, Clone)]
pub struct CreateBulkColorApprovalParams {
    pub sales_order_id: i32,
    pub dye_batch_id: i32,
    pub customer_id: i64,
    pub production_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub sample_type: Option<String>,
    pub remark: Option<String>,
}

/// 剪大货样参数（P0-F16）
#[derive(Debug, Clone)]
pub struct CutSampleParams {
    pub sample_length_m: Decimal,
    pub sample_piece_id: Option<i64>,
    pub attachment_url: Option<String>,
    pub delta_e_value: Option<Decimal>,
    pub operator_id: i32,
}

/// 大货批色审批服务
pub struct BulkColorApprovalService {
    db: Arc<DatabaseConnection>,
}

impl BulkColorApprovalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 创建批色记录（初始状态 pending，delivery_blocking=true）
    pub async fn create(
        &self,
        params: CreateBulkColorApprovalParams,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        // 校验样布类型
        let sample_type = params
            .sample_type
            .as_deref()
            .unwrap_or("cut_sample");
        if !matches!(sample_type, "cut_sample" | "lab_sample") {
            return Err(BulkColorApprovalError::Validation(format!(
                "sample_type 必须为 cut_sample 或 lab_sample，实际: {}",
                sample_type
            )));
        }

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            sales_order_id: Set(params.sales_order_id),
            dye_batch_id: Set(params.dye_batch_id),
            customer_id: Set(params.customer_id),
            production_order_id: Set(params.production_order_id),
            product_id: Set(params.product_id),
            color_no: Set(params.color_no),
            dye_lot_no: Set(params.dye_lot_no),
            batch_no: Set(params.batch_no),
            sample_type: Set(sample_type.to_string()),
            sample_piece_id: Set(None),
            sample_length_m: Set(None),
            approval_status: Set(ApprovalStatus::Pending.as_str().to_string()),
            approver_id: Set(None),
            approval_date: Set(None),
            sent_to_customer_at: Set(None),
            customer_feedback: Set(None),
            delta_e_value: Set(None),
            reject_reason: Set(None),
            delivery_blocking: Set(true),
            attachment_url: Set(None),
            remark: Set(params.remark),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 列表查询（带分页与过滤）
    pub async fn list(
        &self,
        query: ListBulkColorApprovalQuery,
    ) -> Result<(Vec<bulk_color_approval::Model>, u64), BulkColorApprovalError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut cond = Condition::all();
        if let Some(v) = query.sales_order_id {
            cond = cond.add(bulk_color_approval::Column::SalesOrderId.eq(v));
        }
        if let Some(v) = query.dye_batch_id {
            cond = cond.add(bulk_color_approval::Column::DyeBatchId.eq(v));
        }
        if let Some(v) = query.customer_id {
            cond = cond.add(bulk_color_approval::Column::CustomerId.eq(v));
        }
        if let Some(v) = &query.approval_status {
            cond = cond.add(bulk_color_approval::Column::ApprovalStatus.eq(v));
        }
        if let Some(v) = query.from_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.gte(v));
        }
        if let Some(v) = query.to_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.lte(v));
        }

        let paginator = Entity::find()
            .filter(cond)
            .order_by_desc(bulk_color_approval::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let rows = paginator.fetch_page(page - 1).await?;
        Ok((rows, total))
    }

    /// 获取详情
    pub async fn get(&self, id: i64) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)
    }

    /// P0-F16：剪大货样
    ///
    /// 状态转换：pending → sampled 或 rework → sampled
    /// 业务：从大货中剪取一段样布用于客户批色
    pub async fn cut_sample(
        &self,
        id: i64,
        params: CutSampleParams,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        if params.sample_length_m <= Decimal::ZERO {
            return Err(BulkColorApprovalError::Validation(
                "样布长度必须 > 0".to_string(),
            ));
        }

        let txn = self.db.begin().await?;
        let model = Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)?;

        let current = ApprovalStatus::from_str(&model.approval_status)
            .map_err(|e| BulkColorApprovalError::InvalidState(e.0))?;

        // 仅 pending / rework 允许剪样
        if !matches!(current, ApprovalStatus::Pending | ApprovalStatus::Rework) {
            return Err(BulkColorApprovalError::InvalidState(format!(
                "剪样仅允许 pending/rework 状态，当前为 {}",
                current.as_str()
            )));
        }

        let mut active: ActiveModel = model.into();
        active.sample_length_m = Set(Some(params.sample_length_m));
        active.sample_piece_id = Set(params.sample_piece_id);
        if let Some(url) = params.attachment_url {
            active.attachment_url = Set(Some(url));
        }
        if let Some(de) = params.delta_e_value {
            active.delta_e_value = Set(Some(de));
        }
        active.approval_status = Set(ApprovalStatus::Sampled.as_str().to_string());
        active.updated_at = Set(Utc::now());

        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 发送客户批色
    ///
    /// 状态转换：sampled → sent_to_customer
    pub async fn send_to_customer(
        &self,
        id: i64,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        let txn = self.db.begin().await?;
        let model = Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)?;

        let current = ApprovalStatus::from_str(&model.approval_status)
            .map_err(|e| BulkColorApprovalError::InvalidState(e.0))?;

        if current != ApprovalStatus::Sampled {
            return Err(BulkColorApprovalError::InvalidState(format!(
                "发送客户仅允许 sampled 状态，当前为 {}",
                current.as_str()
            )));
        }

        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.approval_status = Set(ApprovalStatus::SentToCustomer.as_str().to_string());
        active.sent_to_customer_at = Set(Some(now));
        active.updated_at = Set(now);

        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// P0-F17：客户批色确认
    ///
    /// 状态转换：sent_to_customer → approved / rejected / rework
    pub async fn customer_approve(
        &self,
        id: i64,
        approver_id: i32,
        feedback: Option<String>,
        delta_e_value: Option<Decimal>,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        self.transition_to(
            id,
            ApprovalStatus::Approved,
            Some(approver_id),
            feedback,
            delta_e_value,
            None,
            true, // approved → delivery_blocking=false
        )
        .await
    }

    pub async fn customer_reject(
        &self,
        id: i64,
        approver_id: i32,
        reject_reason: String,
        feedback: Option<String>,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        self.transition_to(
            id,
            ApprovalStatus::Rejected,
            Some(approver_id),
            feedback,
            None,
            Some(reject_reason),
            false,
        )
        .await
    }

    pub async fn customer_rework(
        &self,
        id: i64,
        approver_id: i32,
        reject_reason: String,
        feedback: Option<String>,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        self.transition_to(
            id,
            ApprovalStatus::Rework,
            Some(approver_id),
            feedback,
            None,
            Some(reject_reason),
            false,
        )
        .await
    }

    /// approved → downgraded（终态）
    pub async fn downgrade(
        &self,
        id: i64,
        reject_reason: String,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        self.transition_to(
            id,
            ApprovalStatus::Downgraded,
            None,
            None,
            None,
            Some(reject_reason),
            false, // downgraded 仍保持 blocking=true
        )
        .await
    }

    /// approved → scrapped 或 pending/sampled → scrapped（终态）
    pub async fn scrap(
        &self,
        id: i64,
        reject_reason: String,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        let txn = self.db.begin().await?;
        let model = Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)?;

        let current = ApprovalStatus::from_str(&model.approval_status)
            .map_err(|e| BulkColorApprovalError::InvalidState(e.0))?;

        // 报废允许的来源：approved / pending / sampled
        if !matches!(
            current,
            ApprovalStatus::Approved | ApprovalStatus::Pending | ApprovalStatus::Sampled
        ) {
            return Err(BulkColorApprovalError::InvalidState(format!(
                "报废仅允许 approved/pending/sampled 状态，当前为 {}",
                current.as_str()
            )));
        }

        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.approval_status = Set(ApprovalStatus::Scrapped.as_str().to_string());
        active.reject_reason = Set(Some(reject_reason));
        active.updated_at = Set(now);
        // scrapped 保持 delivery_blocking=true（需重新生产或换缸）

        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 通用状态转换：sent_to_customer → approved/rejected/rework 或 approved → downgraded
    ///
    /// 源状态校验：仅 sent_to_customer 允许进入 approved/rejected/rework；
    /// downgraded 必须从 approved 进入（专用方法调用）。
    async fn transition_to(
        &self,
        id: i64,
        target: ApprovalStatus,
        approver_id: Option<i32>,
        feedback: Option<String>,
        delta_e_value: Option<Decimal>,
        reject_reason: Option<String>,
        unblock_delivery: bool,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        let txn = self.db.begin().await?;
        let model = Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)?;

        let current = ApprovalStatus::from_str(&model.approval_status)
            .map_err(|e| BulkColorApprovalError::InvalidState(e.0))?;

        // 仅 sent_to_customer 允许进入 approved/rejected/rework
        // downgraded 由专用 downgrade 方法处理（此处仅处理其余 3 个分支）
        if target != ApprovalStatus::Downgraded && current != ApprovalStatus::SentToCustomer {
            return Err(BulkColorApprovalError::InvalidState(format!(
                "状态 {} 仅允许从 sent_to_customer 进入，当前为 {}",
                target.as_str(),
                current.as_str()
            )));
        }
        if target == ApprovalStatus::Downgraded && current != ApprovalStatus::Approved {
            return Err(BulkColorApprovalError::InvalidState(format!(
                "downgraded 仅允许从 approved 进入，当前为 {}",
                current.as_str()
            )));
        }

        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.approval_status = Set(target.as_str().to_string());
        active.approver_id = Set(approver_id);
        active.approval_date = Set(Some(now));
        active.customer_feedback = Set(feedback);
        if let Some(de) = delta_e_value {
            active.delta_e_value = Set(Some(de));
        }
        active.reject_reason = Set(reject_reason);
        active.delivery_blocking = Set(!unblock_delivery);
        active.updated_at = Set(now);

        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }
}

/// P0-F19：发货前校验大货批色门禁
///
/// 业务规则：发货销售订单关联的所有 bulk_color_approval 记录必须全部为 approved 状态
/// 否则阻止发货（delivery_blocking=true 阻断）
///
/// 调用位置：services/so/delivery.rs ship_order 方法，事务开启前
///
/// 参数 db 接受 `&Arc<DatabaseConnection>` 以避免调用方 `&*arc` 显式 deref（clippy::deref_arg）
pub async fn validate_bulk_color_approval(
    db: &std::sync::Arc<DatabaseConnection>,
    sales_order_id: i32,
) -> Result<(), BulkColorApprovalError> {
    use crate::models::sales_order::Entity as SalesOrderEntity;
    let conn: &DatabaseConnection = db.as_ref();

    // 1. 校验销售订单是否存在
    let _order = SalesOrderEntity::find_by_id(sales_order_id)
        .one(conn)
        .await?
        .ok_or(BulkColorApprovalError::SalesOrderNotFound)?;

    // 2. 查询该订单关联的所有批色记录
    let approvals = Entity::find()
        .filter(bulk_color_approval::Column::SalesOrderId.eq(sales_order_id))
        .all(conn)
        .await?;

    // 3. 校验所有记录必须为 approved 状态（解除发货门禁）
    let blockers: Vec<&bulk_color_approval::Model> = approvals
        .iter()
        .filter(|a| {
            ApprovalStatus::from_str(&a.approval_status)
                .map(|s| !s.unblocks_delivery())
                .unwrap_or(true)
        })
        .collect();

    if !blockers.is_empty() {
        let details: Vec<String> = blockers
            .iter()
            .map(|b| {
                format!(
                    "id={} status={} blocking={}",
                    b.id, b.approval_status, b.delivery_blocking
                )
            })
            .collect();
        return Err(BulkColorApprovalError::InvalidState(format!(
            "销售订单 {} 关联 {} 条批色记录未通过审批，无法发货：{}",
            sales_order_id,
            blockers.len(),
            details.join("; ")
        )));
    }

    Ok(())
}
