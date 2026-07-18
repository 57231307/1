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
use crate::models::dye_batch;
use crate::models::inventory_stock;
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
    ///
    /// 业务规则（审计报告 P0-F21）：
    /// - 返工必须走生产订单流程，不能直接修改原批次状态
    /// - 返工订单 order_type='rework'，original_batch_id 指向原 dye_batch
    /// - 返工成本归集到原缸号
    ///
    /// 产品 ID 来源：bulk_color_approval.product_id（若为 None 则返回错误）
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
    ///
    /// P0-F18：降级流程联动库存等级
    /// - 将关联库存的 grade 从"一等品"降为"二等品"或"二等品"降为"等外品"
    /// - 降级后质量状态自动降为"待检"（需重新质检）
    /// - downgraded 仍保持 delivery_blocking=true（不解除发货门禁）
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
    ///
    /// P0-F18：报废流程联动库存状态
    /// - 将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"
    /// - 报废原因追加到 bin_location 保留可追溯性
    /// - scrapped 保持 delivery_blocking=true（需重新生产或换缸）
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

        Ok(updated)
    }

    /// P0-F18：查找批色记录关联的库存记录
    ///
    /// 关联路径：bulk_color_approval.dye_batch_id → dye_batch.batch_no/color_no/dye_lot_no
    ///          → inventory_stock.batch_no/color_no/dye_lot_no
    ///
    /// 若 bulk_color_approval 的 batch_no/color_no/dye_lot_no 字段已填充，优先使用；
    /// 否则回退到加载 dye_batch 表获取。
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

    /// P0-F18：降级联动库存等级
    ///
    /// 等级降级规则：
    /// - 一等品 → 二等品
    /// - 二等品 → 等外品
    /// - 等外品 → 跳过（已是最低等级，无法继续降级）
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

    /// P0-F18：报废联动库存状态
    ///
    /// 将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"
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
