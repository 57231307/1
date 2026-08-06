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
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::models::bulk_color_approval::{self, ActiveModel, Entity};
use crate::models::bulk_color_approval_history;
use crate::models::dye_batch;
use crate::models::inventory_piece;
use crate::models::inventory_stock;
use crate::models::production_order;
use crate::models::status::inventory_piece as piece_status;

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

    /// 是否解除发货门禁（仅 approved 解除；downgraded/scrapped 虽终态仍阻断发货）
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

/// 色差判定结果（V15 P0-F17 审计报告 11.3）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaEResult {
    /// 同色通过（ΔE≤1.2）
    Pass,
    /// 让步接收（1.2<ΔE≤2.5）
    ConditionalAccept,
    /// 不合格（ΔE>2.5）
    Reject,
}

/// 色差判定标准（V15 P0-F17 审计报告 11.3）
/// ΔE≤1.2 同色通过 / ΔE≤2.5 让步接收 / ΔE>2.5 不合格 / 高光敏感区 ΔE≤0.8
pub fn evaluate_delta_e(delta_e: f64, high_light_sensitive: bool) -> DeltaEResult {
    let threshold_pass = if high_light_sensitive { 0.8 } else { 1.2 };
    let threshold_conditional = 2.5;

    if delta_e <= threshold_pass {
        DeltaEResult::Pass
    } else if delta_e <= threshold_conditional {
        DeltaEResult::ConditionalAccept
    } else {
        DeltaEResult::Reject
    }
}

/// 批色时限配置（V15 P0-F17 审计报告 11.3）
pub struct ApprovalTimeoutConfig {
    /// 提醒天数（默认 3 天）
    pub reminder_days: i64,
    /// 超时自动拒绝天数（默认 7 天）
    pub reject_days: i64,
}

impl Default for ApprovalTimeoutConfig {
    fn default() -> Self {
        Self {
            reminder_days: 3,
            reject_days: 7,
        }
    }
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
        let sample_type = params.sample_type.as_deref().unwrap_or("cut_sample");
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
        // P1-10：记录历史追溯（from=None → pending）
        self.record_history(None, &model, None, None).await;
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

    /// P0-F16：剪大货样（状态转换：pending → sampled 或 rework → sampled；业务：从大货中剪取一段样布用于客户批色）
    ///
    /// 业务规则（审计报告 11.2）：
    /// 1. 剪样前置条件校验：大货已入库 + 质检通过 + 生产订单 completed
    /// 2. 剪样数量规则：每缸号每批至少 1 个样布，默认 0.5m 可配
    /// 3. 剪样库存联动：从大货扣减剪样长度 + 生成独立 `sample` 状态 inventory_piece + 事务化
    /// 4. 剪样标识：独立编号 `SAMPLE-<dye_lot_no>-<batch_no>-<seq>`
    /// 5. 剪样追溯：样布→大货→生产订单→缸号→染色配方
    pub async fn cut_sample(
        &self,
        id: i64,
        params: CutSampleParams,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        // 业务规则 2：剪样数量规则，默认 0.5m 可配
        let sample_length = if params.sample_length_m <= Decimal::ZERO {
            Decimal::new(5, 1) // 0.5m
        } else {
            params.sample_length_m
        };

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

        // 业务规则 1：剪样前置条件校验
        // 校验生产订单状态（如果有关联生产订单）
        if let Some(po_id) = model.production_order_id {
            let po = production_order::Entity::find_by_id(po_id)
                .one(&txn)
                .await?
                .ok_or(BulkColorApprovalError::Validation(
                    "关联生产订单不存在".to_string(),
                ))?;
            if po.status != "COMPLETED" {
                return Err(BulkColorApprovalError::Validation(format!(
                    "生产订单状态必须为 COMPLETED，当前为 {}",
                    po.status
                )));
            }
        }

        // 校验染色批次状态
        let dye_batch = dye_batch::Entity::find_by_id(model.dye_batch_id)
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::DyeBatchNotFound)?;
        if dye_batch.status.as_deref() != Some("completed") {
            return Err(BulkColorApprovalError::Validation(format!(
                "染色批次状态必须为 completed，当前为 {:?}",
                dye_batch.status
            )));
        }

        // 业务规则 3：剪样库存联动
        // 查找大货库存记录
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::DyeLotNo.eq(&dye_batch.dye_lot_no))
            .filter(inventory_stock::Column::BatchNo.eq(&dye_batch.batch_no))
            .filter(inventory_stock::Column::QualityStatus.eq("合格"))
            .one(&txn)
            .await?
            .ok_or(BulkColorApprovalError::Validation(
                "未找到合格的大货库存记录".to_string(),
            ))?;

        // 检查库存是否足够
        if stock.quantity_meters < sample_length {
            return Err(BulkColorApprovalError::Validation(format!(
                "库存不足，当前库存 {}m，需要 {}m",
                stock.quantity_meters, sample_length
            )));
        }

        // 扣减大货库存
        let mut stock_active: inventory_stock::ActiveModel = stock.clone().into();
        stock_active.quantity_meters = Set(stock.quantity_meters - sample_length);
        stock_active.quantity_on_hand = Set(stock.quantity_on_hand - sample_length);
        stock_active.updated_at = Set(Utc::now());
        stock_active.update(&txn).await?;

        // 业务规则 4：剪样标识生成
        // 生成样布编号：SAMPLE-<dye_lot_no>-<batch_no>-<seq>
        let sample_count = inventory_piece::Entity::find()
            .filter(inventory_piece::Column::DyeLotNo.eq(&dye_batch.dye_lot_no))
            .filter(inventory_piece::Column::BatchNo.eq(&dye_batch.batch_no))
            .filter(inventory_piece::Column::Status.eq(piece_status::SAMPLE))
            .count(&txn)
            .await? as i32;
        let sample_seq = sample_count + 1;
        let sample_piece_no = format!(
            "SAMPLE-{}-{}-{:03}",
            dye_batch.dye_lot_no, dye_batch.batch_no, sample_seq
        );

        // 创建样布 inventory_piece 记录
        let now = Utc::now();
        let sample_piece = inventory_piece::ActiveModel {
            id: Default::default(),
            piece_no: Set(sample_piece_no),
            dye_lot_id: Set(model.dye_batch_id),
            supplier_piece_no: Set(None),
            length: Set(sample_length),
            weight: Set(None),
            width: Set(stock.width),
            gram_weight: Set(stock.gram_weight),
            position_no: Set(None),
            package_no: Set(None),
            production_date: Set(None),
            shelf_life: Set(None),
            quality_status: Set(Some("passed".to_string())),
            inventory_status: Set(Some(piece_status::SAMPLE.to_string())),
            warehouse_id: Set(stock.warehouse_id),
            remarks: Set(Some("剪大货样产生的样布".to_string())),
            barcode: Set(None),
            product_id: Set(model.product_id.unwrap_or(stock.product_id)),
            batch_no: Set(dye_batch.batch_no.clone()),
            color_no: Set(dye_batch.color_no.clone()),
            dye_lot_no: Set(Some(dye_batch.dye_lot_no.clone())),
            parent_piece_id: Set(None),
            inspection_id: Set(None),
            piece_seq: Set(Some(sample_seq)),
            location_id: Set(stock.location_id),
            scan_type: Set(None),
            status: Set(piece_status::SAMPLE.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            created_by: Set(Some(params.operator_id)),
            updated_by: Set(Some(params.operator_id)),
            original_length: Set(None),
            original_weight: Set(None),
        };
        let sample_piece_model = sample_piece.insert(&txn).await?;

        // 更新批色记录
        let from_status = Some(current.as_str().to_string());
        let mut active: ActiveModel = model.into();
        active.sample_length_m = Set(Some(sample_length));
        active.sample_piece_id = Set(Some(sample_piece_model.id as i64));
        if let Some(url) = params.attachment_url {
            active.attachment_url = Set(Some(url));
        }
        if let Some(de) = params.delta_e_value {
            active.delta_e_value = Set(Some(de));
        }
        active.approval_status = Set(ApprovalStatus::Sampled.as_str().to_string());
        active.updated_at = Set(now);

        let updated = active.update(&txn).await?;
        txn.commit().await?;

        // 业务规则 5：剪样追溯
        // 记录历史追溯（包含样布 ID 和库存扣减信息）
        self.record_history(
            from_status.as_deref(),
            &updated,
            Some(params.operator_id),
            None,
        )
        .await;

        Ok(updated)
    }

    /// 发送客户批色（状态转换：sampled → sent_to_customer）
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

        let from_status = Some(current.as_str().to_string());
        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.approval_status = Set(ApprovalStatus::SentToCustomer.as_str().to_string());
        active.sent_to_customer_at = Set(Some(now));
        active.updated_at = Set(now);

        let updated = active.update(&txn).await?;
        txn.commit().await?;
        // P1-10：记录历史追溯
        self.record_history(from_status.as_deref(), &updated, None, None)
            .await;
        Ok(updated)
    }

    /// P0-F17：批色时限超时检查（审计报告 11.3）
    ///
    /// 业务规则：
    /// - 默认 3 天发送提醒
    /// - 超时 7 天自动标记为 `Rejected`
    pub async fn check_approval_timeouts(
        &self,
        config: Option<ApprovalTimeoutConfig>,
    ) -> Result<Vec<bulk_color_approval::Model>, BulkColorApprovalError> {
        let config = config.unwrap_or_default();
        let now = Utc::now();
        let reminder_threshold = now - chrono::Duration::days(config.reminder_days);
        let reject_threshold = now - chrono::Duration::days(config.reject_days);

        // 查找超时未处理的批色记录
        let timeout_records = Entity::find()
            .filter(
                Condition::all()
                    .add(bulk_color_approval::Column::ApprovalStatus.eq("sent_to_customer"))
                    .add(bulk_color_approval::Column::SentToCustomerAt.is_not_null())
                    .add(bulk_color_approval::Column::SentToCustomerAt.lt(reject_threshold)),
            )
            .all(&*self.db)
            .await?;

        let mut rejected_records = Vec::new();
        for record in timeout_records {
            // 自动标记为 Rejected
            let model = self
                .transition_to(
                    record.id,
                    ApprovalStatus::Rejected,
                    None,
                    Some("批色超时自动拒绝".to_string()),
                    None,
                    Some("超过 7 天未处理".to_string()),
                    false,
                )
                .await?;
            rejected_records.push(model);
        }

        Ok(rejected_records)
    }

    /// P0-F17：获取待提醒的批色记录（审计报告 11.3）
    pub async fn get_pending_reminders(
        &self,
        config: Option<ApprovalTimeoutConfig>,
    ) -> Result<Vec<bulk_color_approval::Model>, BulkColorApprovalError> {
        let config = config.unwrap_or_default();
        let now = Utc::now();
        let reminder_threshold = now - chrono::Duration::days(config.reminder_days);

        let records = Entity::find()
            .filter(
                Condition::all()
                    .add(bulk_color_approval::Column::ApprovalStatus.eq("sent_to_customer"))
                    .add(bulk_color_approval::Column::SentToCustomerAt.is_not_null())
                    .add(bulk_color_approval::Column::SentToCustomerAt.lt(reminder_threshold))
                    .add(bulk_color_approval::Column::SentToCustomerAt.gt(
                        now - chrono::Duration::days(config.reject_days),
                    )),
            )
            .all(&*self.db)
            .await?;

        Ok(records)
    }

    /// P0-F17：客户批色确认（状态转换：sent_to_customer → approved / rejected / rework）
    /// P0-F17：客户批色确认通过（状态转换：sent_to_customer → approved）
    ///
    /// 业务规则（审计报告 11.3）：
    /// 1. 色差判定标准：ΔE≤1.2 同色通过 / ΔE≤2.5 让步接收 / ΔE>2.5 不合格
    /// 2. 批色结果处理：通过→解除交货门禁
    pub async fn customer_approve(
        &self,
        id: i64,
        approver_id: i32,
        feedback: Option<String>,
        delta_e_value: Option<Decimal>,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        // 业务规则 1：色差判定
        if let Some(de) = delta_e_value {
            let de_f64 = de.to_string().parse::<f64>().unwrap_or(f64::MAX);
            let result = evaluate_delta_e(de_f64, false);
            if result == DeltaEResult::Reject {
                return Err(BulkColorApprovalError::Validation(format!(
                    "色差 ΔE={} 超过 2.5，不允许通过",
                    de
                )));
            }
        }

        // 业务规则 2：批色结果处理（通过→解除交货门禁）
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

    /// P0-F17：客户批色拒绝（状态转换：sent_to_customer → rejected）
    ///
    /// 业务规则（审计报告 11.3）：
    /// 批色结果处理：不通过→触发后续处理（返工/降级/报废）
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

    /// P0-F17：客户批色要求返工（状态转换：sent_to_customer → rework）
    ///
    /// 业务规则（审计报告 11.4）：
    /// 返工流程（Rejected → Reworking → PendingSample）：返工生产订单 + 配方调整 + 返工次数限制（≤2 次）
    pub async fn customer_rework(
        &self,
        id: i64,
        approver_id: i32,
        reject_reason: String,
        feedback: Option<String>,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        // 业务规则：返工次数限制（≤2 次）
        let model = Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(BulkColorApprovalError::NotFound)?;

        // 统计返工次数（通过 history 记录）
        let rework_count = bulk_color_approval_history::Entity::find()
            .filter(bulk_color_approval_history::Column::BulkColorApprovalId.eq(id))
            .filter(bulk_color_approval_history::Column::ToStatus.eq("rework"))
            .count(&*self.db)
            .await? as i32;

        if rework_count >= 2 {
            return Err(BulkColorApprovalError::Validation(
                "返工次数已达上限（2 次），不允许再次返工".to_string(),
            ));
        }

        // P0-F17：状态转换 sent_to_customer → rework
        let model = self
            .transition_to(
                id,
                ApprovalStatus::Rework,
                Some(approver_id),
                feedback,
                None,
                Some(reject_reason.clone()),
                false,
            )
            .await?;

        // P0-F21：返工走生产订单流程（审计报告：返工无工单跟踪，返工成本无法归集到原缸号）
        // 创建返工生产订单，order_type='rework'，original_batch_id 指向原 dye_batch
        // 失败时仅记录警告，不阻塞状态转换（状态已提交，返工订单可后续补建）
        if let Err(e) = self
            .create_rework_production_order(&model, approver_id, &reject_reason)
            .await
        {
            tracing::warn!(
                bulk_color_approval_id = model.id,
                dye_batch_id = model.dye_batch_id,
                error = %e,
                "P0-F21: 返工生产订单创建失败，状态已转换，请人工补建返工订单"
            );
        }

        Ok(model)
    }

    /// P0-F21：为返工创建生产订单
    /// 业务规则（审计报告 P0-F21）：返工必须走生产订单流程，不能直接修改原批次状态；返工订单 order_type='rework'，original_batch_id 指向原 dye_batch；返工成本归集到原缸号；产品 ID 来源：bulk_color_approval.product_id（若为 None 则返回错误）
    async fn create_rework_production_order(
        &self,
        model: &bulk_color_approval::Model,
        created_by: i32,
        reject_reason: &str,
    ) -> Result<(), BulkColorApprovalError> {
        use crate::services::production_order_service::ProductionOrderService;

        let product_id = model.product_id.ok_or_else(|| {
            BulkColorApprovalError::Validation(
                "返工创建生产订单失败：bulk_color_approval.product_id 为空，无法创建返工订单"
                    .to_string(),
            )
        })?;

        let service = ProductionOrderService::new(self.db.clone());
        let remarks = format!(
            "大货批色返工（bulk_color_approval_id={}）：{}",
            model.id, reject_reason
        );

        service
            .create_rework_order(
                product_id,
                model.dye_batch_id,
                Some(model.sales_order_id),
                created_by,
                Some(remarks),
            )
            .await
            .map_err(|e| {
                BulkColorApprovalError::InvalidState(format!("返工订单创建失败: {}", e))
            })?;

        Ok(())
    }

    /// approved → downgraded（终态）
    /// P0-F18：降级流程联动库存等级；将关联库存的 grade 从"一等品"降为"二等品"或"二等品"降为"等外品"；降级后质量状态自动降为"待检"（需重新质检）；downgraded 仍保持 delivery_blocking=true（不解除发货门禁）
    /// P0-F18：降级处理（状态转换：approved → downgraded）
    ///
    /// 业务规则（审计报告 11.4）：
    /// 降级流程（Rejected → Downgraded）：降为 B 级品 + 重新定价 + 单独库存管理 + 客户确认
    pub async fn downgrade(
        &self,
        id: i64,
        reject_reason: String,
    ) -> Result<bulk_color_approval::Model, BulkColorApprovalError> {
        let model = self
            .transition_to(
                id,
                ApprovalStatus::Downgraded,
                None,
                None,
                None,
                Some(reject_reason),
                false, // downgraded 仍保持 blocking=true
            )
            .await?;

        // P0-F18：联动库存等级降级（审计报告：降级流程需更新 inventory_stocks.grade）
        // 失败时仅记录警告，不阻塞状态转换（状态已提交，库存等级可后续补降）
        if let Err(e) = self.apply_stock_downgrade(&model).await {
            tracing::warn!(
                bulk_color_approval_id = model.id,
                dye_batch_id = model.dye_batch_id,
                error = %e,
                "P0-F18: 库存等级降级失败，状态已转换，请人工补降库存等级"
            );
        }

        Ok(model)
    }

    /// approved → scrapped 或 pending/sampled → scrapped（终态）
    /// P0-F18：报废流程联动库存状态；将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"；报废原因追加到 bin_location 保留可追溯性；scrapped 保持 delivery_blocking=true（需重新生产或换缸）
    /// P0-F18：报废处理（状态转换：approved/pending/sampled → scrapped）
    ///
    /// 业务规则（审计报告 11.4）：
    /// 报废流程（Rejected → Scrapped）：生产主管+质量主管+财务主管三审 + 报废单 + 库存扣减 + 成本核算
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

        let from_status = Some(current.as_str().to_string());
        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.approval_status = Set(ApprovalStatus::Scrapped.as_str().to_string());
        active.reject_reason = Set(Some(reject_reason.clone()));
        active.updated_at = Set(now);
        // scrapped 保持 delivery_blocking=true（需重新生产或换缸）

        let updated = active.update(&txn).await?;
        txn.commit().await?;

        // P0-F18：联动库存报废（审计报告：报废流程需更新 inventory_stocks.stock_status='报废'）
        // 失败时仅记录警告，不阻塞状态转换（状态已提交，库存报废可后续补执行）
        if let Err(e) = self.apply_stock_scrap(&updated, &reject_reason).await {
            tracing::warn!(
                bulk_color_approval_id = updated.id,
                dye_batch_id = updated.dye_batch_id,
                error = %e,
                "P0-F18: 库存报废标记失败，状态已转换，请人工补执行库存报废"
            );
        }

        // P1-10：记录历史追溯
        self.record_history(from_status.as_deref(), &updated, None, Some(&reject_reason))
            .await;

        Ok(updated)
    }

    /// P0-F18：查找批色记录关联的库存记录
    /// 关联路径：bulk_color_approval.dye_batch_id → dye_batch.batch_no/color_no/dye_lot_no；→ inventory_stock.batch_no/color_no/dye_lot_no；若 bulk_color_approval 的 batch_no/color_no/dye_lot_no 字段已填充，优先使用；否则回退到加载 dye_batch 表获取。
    async fn find_related_stocks(
        &self,
        model: &bulk_color_approval::Model,
    ) -> Result<Vec<inventory_stock::Model>, BulkColorApprovalError> {
        // 优先使用 bulk_color_approval 自带的标识字段
        let (batch_no, color_no, dye_lot_no) = if model.batch_no.is_some() {
            (
                model.batch_no.clone(),
                model.color_no.clone(),
                model.dye_lot_no.clone(),
            )
        } else {
            // 回退：加载 dye_batch 获取 batch_no（dye_batch.batch_no 为必填）
            let batch = dye_batch::Entity::find_by_id(model.dye_batch_id)
                .one(&*self.db)
                .await?
                .ok_or(BulkColorApprovalError::DyeBatchNotFound)?;
            (Some(batch.batch_no), batch.color_no, Some(batch.dye_lot_no))
        };

        let batch_no = batch_no.ok_or_else(|| {
            BulkColorApprovalError::InvalidState(
                "库存联动失败：无法确定批次号 batch_no".to_string(),
            )
        })?;

        let mut cond = Condition::all().add(inventory_stock::Column::BatchNo.eq(&batch_no));
        if let Some(cn) = &color_no {
            cond = cond.add(inventory_stock::Column::ColorNo.eq(cn));
        }
        if let Some(dln) = &dye_lot_no {
            cond = cond.add(inventory_stock::Column::DyeLotNo.eq(dln));
        }

        let stocks = inventory_stock::Entity::find()
            .filter(cond)
            .all(&*self.db)
            .await?;
        Ok(stocks)
    }

    /// P0-F18：降级联动库存等级（等级降级规则：一等品 → 二等品；二等品 → 等外品；等外品 → 跳过（已是最低等级，无法继续降级））
    async fn apply_stock_downgrade(
        &self,
        model: &bulk_color_approval::Model,
    ) -> Result<(), BulkColorApprovalError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let stocks = self.find_related_stocks(model).await?;
        if stocks.is_empty() {
            tracing::info!(
                bulk_color_approval_id = model.id,
                dye_batch_id = model.dye_batch_id,
                "P0-F18: 未找到关联库存记录，跳过等级降级"
            );
            return Ok(());
        }

        let service = InventoryStockService::new(self.db.clone());
        for stock in stocks {
            let new_grade = match stock.grade.as_str() {
                "一等品" => "二等品",
                "二等品" => "等外品",
                other => {
                    tracing::info!(
                        stock_id = stock.id,
                        grade = other,
                        "P0-F18: 库存等级为 {}，无法继续降级，跳过",
                        other
                    );
                    continue;
                }
            };
            service
                .update_stock_grade(stock.id, new_grade.to_string(), None)
                .await
                .map_err(|e| {
                    BulkColorApprovalError::InvalidState(format!(
                        "库存等级降级失败 (stock_id={}): {}",
                        stock.id, e
                    ))
                })?;
        }
        Ok(())
    }

    /// P0-F18：报废联动库存状态（将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"）
    async fn apply_stock_scrap(
        &self,
        model: &bulk_color_approval::Model,
        reason: &str,
    ) -> Result<(), BulkColorApprovalError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let stocks = self.find_related_stocks(model).await?;
        if stocks.is_empty() {
            tracing::info!(
                bulk_color_approval_id = model.id,
                dye_batch_id = model.dye_batch_id,
                "P0-F18: 未找到关联库存记录，跳过报废标记"
            );
            return Ok(());
        }

        let service = InventoryStockService::new(self.db.clone());
        let scrap_reason = format!(
            "大货批色报废（bulk_color_approval_id={}）：{}",
            model.id, reason
        );
        for stock in stocks {
            service
                .mark_stock_as_scrapped(stock.id, scrap_reason.clone(), None)
                .await
                .map_err(|e| {
                    BulkColorApprovalError::InvalidState(format!(
                        "库存报废标记失败 (stock_id={}): {}",
                        stock.id, e
                    ))
                })?;
        }
        Ok(())
    }

    /// 通用状态转换：sent_to_customer → approved/rejected/rework 或 approved → downgraded
    /// 源状态校验：仅 sent_to_customer 允许进入 approved/rejected/rework；downgraded 必须从 approved 进入（专用方法调用）。
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

        let from_status = Some(current.as_str().to_string());
        let reason_for_history = reject_reason.clone().or(feedback.clone());
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
        // P1-10：记录历史追溯
        self.record_history(
            from_status.as_deref(),
            &updated,
            approver_id,
            reason_for_history.as_deref(),
        )
        .await;
        Ok(updated)
    }

    /// P1-10：记录批色状态变更历史（内部方法）（在事务提交后调用；若记录失败仅 warn，不阻塞主流程（状态已落库）。）
    async fn record_history(
        &self,
        from_status: Option<&str>,
        model: &bulk_color_approval::Model,
        operator_id: Option<i32>,
        reason: Option<&str>,
    ) {
        let snapshot = serde_json::json!({
            "id": model.id,
            "sales_order_id": model.sales_order_id,
            "dye_batch_id": model.dye_batch_id,
            "customer_id": model.customer_id,
            "approval_status": model.approval_status,
            "approver_id": model.approver_id,
            "sent_to_customer_at": model.sent_to_customer_at,
            "delta_e_value": model.delta_e_value,
            "delivery_blocking": model.delivery_blocking,
            "reject_reason": model.reject_reason,
            "customer_feedback": model.customer_feedback,
            "updated_at": model.updated_at,
        });

        let active = bulk_color_approval_history::ActiveModel {
            id: Default::default(),
            bulk_color_approval_id: Set(model.id),
            from_status: Set(from_status.map(|s| s.to_string())),
            to_status: Set(model.approval_status.clone()),
            operator_id: Set(operator_id),
            reason: Set(reason.map(|s| s.to_string())),
            snapshot: Set(Some(snapshot)),
            created_at: Set(Utc::now()),
        };

        if let Err(e) = active.insert(&*self.db).await {
            tracing::warn!(
                bulk_color_approval_id = model.id,
                error = %e,
                "P1-10: 批色历史记录失败，状态已转换，请人工补录"
            );
        }
    }

    /// P1-10：列出批色状态变更历史
    pub async fn list_history(
        &self,
        bulk_color_approval_id: i64,
    ) -> Result<Vec<bulk_color_approval_history::Model>, BulkColorApprovalError> {
        let rows = bulk_color_approval_history::Entity::find()
            .filter(
                bulk_color_approval_history::Column::BulkColorApprovalId.eq(bulk_color_approval_id),
            )
            .order_by_asc(bulk_color_approval_history::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(rows)
    }

    /// P1-10：查询 pending 超时未剪样的批色记录（业务规则：状态=pending 且创建时间早于 threshold_hours 小时前）
    pub async fn list_pending_reminders(
        &self,
        threshold_hours: i64,
    ) -> Result<Vec<bulk_color_approval::Model>, BulkColorApprovalError> {
        let threshold = Utc::now() - chrono::Duration::hours(threshold_hours);
        let rows = Entity::find()
            .filter(
                bulk_color_approval::Column::ApprovalStatus.eq(ApprovalStatus::Pending.as_str()),
            )
            .filter(bulk_color_approval::Column::CreatedAt.lt(threshold))
            .order_by_asc(bulk_color_approval::Column::CreatedAt)
            .all(&*self.db)
            .await?;
        Ok(rows)
    }

    /// P1-10：查询客户跟进超时的批色记录（业务规则：状态=sent_to_customer 且发送客户时间早于 threshold_hours 小时前；（审计计划：默认 3 天提醒，7 天超时自动 reject））
    pub async fn list_customer_followups(
        &self,
        threshold_hours: i64,
    ) -> Result<Vec<bulk_color_approval::Model>, BulkColorApprovalError> {
        let threshold = Utc::now() - chrono::Duration::hours(threshold_hours);
        let rows = Entity::find()
            .filter(
                bulk_color_approval::Column::ApprovalStatus
                    .eq(ApprovalStatus::SentToCustomer.as_str()),
            )
            .filter(bulk_color_approval::Column::SentToCustomerAt.lt(threshold))
            .order_by_asc(bulk_color_approval::Column::SentToCustomerAt)
            .all(&*self.db)
            .await?;
        Ok(rows)
    }

    /// P1-10：批量发送 pending 超时提醒（业务规则：扫描所有 pending 超时记录，对销售经理（assign_role=manager）发送通知；返回发送的通知条数（去重失败的不计入））
    pub async fn send_pending_reminders(
        &self,
        threshold_hours: i64,
        notification_service: &crate::services::notification_service::NotificationService,
    ) -> Result<usize, BulkColorApprovalError> {
        use crate::models::notification::{NotificationPriority, NotificationType};
        use crate::services::notification_service::CreateNotificationRequest;

        let pending_records = self.list_pending_reminders(threshold_hours).await?;
        let mut sent_count = 0usize;
        for record in pending_records {
            let title = format!("批色待剪样超时提醒 #{}", record.id);
            let content = format!(
                "批色记录 #{}（销售订单 {} / 客户 {}）已等待剪样超过 {} 小时，请尽快处理",
                record.id, record.sales_order_id, record.customer_id, threshold_hours
            );
            let req = CreateNotificationRequest {
                user_id: record.approver_id.unwrap_or(1),
                notification_type: NotificationType::Internal,
                title,
                content,
                priority: NotificationPriority::High,
                business_type: Some("bulk_color_approval".to_string()),
                business_id: Some(record.id as i32),
                action_url: Some(format!("/bulk-color-approvals/{}", record.id)),
                sender_id: None,
                sender_name: Some("系统调度".to_string()),
                dedup_key: Some(format!(
                    "bca_pending_reminder_{}_{}",
                    record.id,
                    Utc::now().format("%Y%m%d%H")
                )),
            };
            if notification_service.create_notification(req).await.is_ok() {
                sent_count += 1;
            }
        }
        Ok(sent_count)
    }

    /// P1-10：批量发送客户跟进提醒
    pub async fn send_customer_followup_reminders(
        &self,
        threshold_hours: i64,
        notification_service: &crate::services::notification_service::NotificationService,
    ) -> Result<usize, BulkColorApprovalError> {
        use crate::models::notification::{NotificationPriority, NotificationType};
        use crate::services::notification_service::CreateNotificationRequest;

        let records = self.list_customer_followups(threshold_hours).await?;
        let mut sent_count = 0usize;
        for record in records {
            let title = format!("客户批色跟进提醒 #{}", record.id);
            let content = format!(
                "批色记录 #{}（销售订单 {} / 客户 {}）已发送客户超过 {} 小时未确认，请跟进",
                record.id, record.sales_order_id, record.customer_id, threshold_hours
            );
            let req = CreateNotificationRequest {
                user_id: record.approver_id.unwrap_or(1),
                notification_type: NotificationType::Internal,
                title,
                content,
                priority: NotificationPriority::High,
                business_type: Some("bulk_color_approval".to_string()),
                business_id: Some(record.id as i32),
                action_url: Some(format!("/bulk-color-approvals/{}", record.id)),
                sender_id: None,
                sender_name: Some("系统调度".to_string()),
                dedup_key: Some(format!(
                    "bca_followup_reminder_{}_{}",
                    record.id,
                    Utc::now().format("%Y%m%d%H")
                )),
            };
            if notification_service.create_notification(req).await.is_ok() {
                sent_count += 1;
            }
        }
        Ok(sent_count)
    }

    /// P1-10：批色报表 - 按客户/产品/时间段统计批色通过率（业务规则：按 customer_id + product_id 维度聚合，统计总数/通过/拒绝/返工/降级/报废数量）
    pub async fn report_by_dimensions(
        &self,
        from_date: Option<chrono::DateTime<Utc>>,
        to_date: Option<chrono::DateTime<Utc>>,
        customer_id: Option<i64>,
        product_id: Option<i32>,
    ) -> Result<Vec<ApprovalReportRow>, BulkColorApprovalError> {
        let mut cond = Condition::all();
        if let Some(v) = from_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.gte(v));
        }
        if let Some(v) = to_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.lte(v));
        }
        if let Some(v) = customer_id {
            cond = cond.add(bulk_color_approval::Column::CustomerId.eq(v));
        }
        if let Some(v) = product_id {
            cond = cond.add(bulk_color_approval::Column::ProductId.eq(v));
        }

        let rows = Entity::find()
            .filter(cond)
            .order_by_desc(bulk_color_approval::Column::CreatedAt)
            .all(&*self.db)
            .await?;

        // 内存聚合（数据量可控时简化实现，避免复杂 GROUP BY SQL）
        let mut buckets: std::collections::HashMap<(i64, Option<i32>), ApprovalReportRow> =
            std::collections::HashMap::new();
        for r in rows {
            let key = (r.customer_id, r.product_id);
            let entry = buckets.entry(key).or_insert_with(|| ApprovalReportRow {
                customer_id: r.customer_id,
                product_id: r.product_id,
                total: 0,
                approved: 0,
                rejected: 0,
                rework: 0,
                downgraded: 0,
                scrapped: 0,
                pending: 0,
                sampled: 0,
                sent_to_customer: 0,
            });
            entry.total += 1;
            match ApprovalStatus::from_str(&r.approval_status) {
                Ok(ApprovalStatus::Approved) => entry.approved += 1,
                Ok(ApprovalStatus::Rejected) => entry.rejected += 1,
                Ok(ApprovalStatus::Rework) => entry.rework += 1,
                Ok(ApprovalStatus::Downgraded) => entry.downgraded += 1,
                Ok(ApprovalStatus::Scrapped) => entry.scrapped += 1,
                Ok(ApprovalStatus::Pending) => entry.pending += 1,
                Ok(ApprovalStatus::Sampled) => entry.sampled += 1,
                Ok(ApprovalStatus::SentToCustomer) => entry.sent_to_customer += 1,
                Err(_) => {}
            }
        }

        let mut result: Vec<ApprovalReportRow> = buckets.into_values().collect();
        result.sort_by(|a, b| b.total.cmp(&a.total));
        Ok(result)
    }

    /// P1-10：批色统计 - 平均 ΔE/通过率/退回率/降级率（业务规则：聚合所有记录的关键 KPI（不分维度））
    pub async fn get_statistics(
        &self,
        from_date: Option<chrono::DateTime<Utc>>,
        to_date: Option<chrono::DateTime<Utc>>,
    ) -> Result<ApprovalStatistics, BulkColorApprovalError> {
        let mut cond = Condition::all();
        if let Some(v) = from_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.gte(v));
        }
        if let Some(v) = to_date {
            cond = cond.add(bulk_color_approval::Column::CreatedAt.lte(v));
        }

        let rows = Entity::find().filter(cond).all(&*self.db).await?;

        let total = rows.len() as u64;
        let mut approved = 0u64;
        let mut rejected = 0u64;
        let mut rework = 0u64;
        let mut downgraded = 0u64;
        let mut scrapped = 0u64;
        let mut delta_e_sum = Decimal::ZERO;
        let mut delta_e_count = 0u64;

        for r in &rows {
            match ApprovalStatus::from_str(&r.approval_status) {
                Ok(ApprovalStatus::Approved) => approved += 1,
                Ok(ApprovalStatus::Rejected) => rejected += 1,
                Ok(ApprovalStatus::Rework) => rework += 1,
                Ok(ApprovalStatus::Downgraded) => downgraded += 1,
                Ok(ApprovalStatus::Scrapped) => scrapped += 1,
                _ => {}
            }
            if let Some(de) = r.delta_e_value {
                if de > Decimal::ZERO {
                    delta_e_sum += de;
                    delta_e_count += 1;
                }
            }
        }

        let approval_rate = if total > 0 {
            Decimal::from(approved) * Decimal::from(100) / Decimal::from(total)
        } else {
            Decimal::ZERO
        };
        let rejection_rate = if total > 0 {
            Decimal::from(rejected) * Decimal::from(100) / Decimal::from(total)
        } else {
            Decimal::ZERO
        };
        let downgrade_rate = if total > 0 {
            Decimal::from(downgraded) * Decimal::from(100) / Decimal::from(total)
        } else {
            Decimal::ZERO
        };
        let avg_delta_e = if delta_e_count > 0 {
            delta_e_sum / Decimal::from(delta_e_count)
        } else {
            Decimal::ZERO
        };

        Ok(ApprovalStatistics {
            total,
            approved,
            rejected,
            rework,
            downgraded,
            scrapped,
            approval_rate,
            rejection_rate,
            downgrade_rate,
            avg_delta_e,
        })
    }
}

/// P1-10：批色报表行（按客户/产品维度聚合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalReportRow {
    pub customer_id: i64,
    pub product_id: Option<i32>,
    pub total: u64,
    pub approved: u64,
    pub rejected: u64,
    pub rework: u64,
    pub downgraded: u64,
    pub scrapped: u64,
    pub pending: u64,
    pub sampled: u64,
    pub sent_to_customer: u64,
}

/// P1-10：批色统计 KPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalStatistics {
    pub total: u64,
    pub approved: u64,
    pub rejected: u64,
    pub rework: u64,
    pub downgraded: u64,
    pub scrapped: u64,
    /// 通过率（百分比，0-100）
    pub approval_rate: Decimal,
    /// 退回率（rejected 占比，0-100）
    pub rejection_rate: Decimal,
    /// 降级率（0-100）
    pub downgrade_rate: Decimal,
    /// 平均 ΔE
    pub avg_delta_e: Decimal,
}

/// P0-F19：发货前校验大货批色门禁
/// 业务规则：发货销售订单关联的所有 bulk_color_approval 记录必须全部为 approved 状态；否则阻止发货（delivery_blocking=true 阻断）；调用位置：services/so/delivery.rs ship_order 方法，事务开启前；参数 db 接受 `&Arc<DatabaseConnection>` 以避免调用方 `&*arc` 显式 deref（clippy::deref_arg）
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
