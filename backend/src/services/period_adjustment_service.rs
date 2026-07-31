//! 期末调整 Service（V15 P2 B05-P2-10）
//!
//! 实现期末权责发生制调整机制：暂估 / 摊销 / 预提。
//! 状态机：draft(草稿) → confirmed(已确认，生成凭证) → reversed(已冲销，红字凭证) / cancelled(已取消)
//!
//! 业务规则：
//! - confirm：draft → confirmed，生成调整凭证（借 debit_subject / 贷 credit_subject），回写 voucher_id
//! - reverse：confirmed → reversed，生成红字冲销凭证（借贷对调），回写 reverse_voucher_id（暂估类下月初冲销）
//! - cancel：draft → cancelled
//! - close_period 注入 confirm_pending：结账前批量确认本期间 draft 调整，确保调整凭证计入本期试算平衡

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::period_adjustment_record::{
    self, ActiveModel as AdjustmentActiveModel, Entity as AdjustmentEntity,
    Model as AdjustmentModel,
};
use crate::utils::error::AppError;

/// 期末调整状态机常量
pub mod period_adjustment_status {
    pub const DRAFT: &str = "draft";
    pub const CONFIRMED: &str = "confirmed";
    pub const REVERSED: &str = "reversed";
    pub const CANCELLED: &str = "cancelled";
}

/// 期末调整类型常量
pub mod period_adjustment_type {
    /// 暂估（已收货/已受益未取得发票，暂估入账，下月初红字冲销）
    pub const ESTIMATE: &str = "estimate";
    /// 摊销（待摊费用按受益期分摊，如保险费/租金）
    pub const AMORTIZATION: &str = "amortization";
    /// 预提（已发生未支付的费用预提入账，如利息/水电）
    pub const PROVISION: &str = "provision";
}

/// 创建期末调整请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePeriodAdjustmentRequest {
    pub adjustment_type: String,
    pub period: String,
    pub description: String,
    pub debit_subject_code: String,
    pub debit_subject_name: String,
    pub credit_subject_code: String,
    pub credit_subject_name: String,
    pub amount: Decimal,
    pub source_type: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 期末调整查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct PeriodAdjustmentQuery {
    pub adjustment_type: Option<String>,
    pub period: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 期末调整 Service
pub struct PeriodAdjustmentService {
    db: Arc<DatabaseConnection>,
}

impl PeriodAdjustmentService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成调整单号：PA-YYYYMMDDHHMMSS-NNN
    fn generate_adjustment_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("PA-{}-{:03}", timestamp, random)
    }

    /// 校验调整类型合法
    fn validate_adjustment_type(t: &str) -> Result<(), AppError> {
        match t {
            period_adjustment_type::ESTIMATE
            | period_adjustment_type::AMORTIZATION
            | period_adjustment_type::PROVISION => Ok(()),
            other => Err(AppError::business(format!(
                "无效的调整类型：{}（合法值：estimate/amortization/provision）",
                other
            ))),
        }
    }

    /// 创建期末调整记录（draft 状态）
    pub async fn create(
        &self,
        req: CreatePeriodAdjustmentRequest,
    ) -> Result<AdjustmentModel, AppError> {
        Self::validate_adjustment_type(&req.adjustment_type)?;
        if req.period.trim().is_empty() {
            return Err(AppError::business("会计期间(period)不能为空"));
        }
        if req.amount <= Decimal::ZERO {
            return Err(AppError::business("调整金额必须大于 0"));
        }
        if req.debit_subject_code.trim().is_empty() || req.credit_subject_code.trim().is_empty() {
            return Err(AppError::business("借贷科目编码不能为空"));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = AdjustmentActiveModel {
            id: Default::default(),
            adjustment_no: Set(Self::generate_adjustment_no()),
            adjustment_type: Set(req.adjustment_type),
            period: Set(req.period),
            description: Set(req.description),
            debit_subject_code: Set(req.debit_subject_code),
            debit_subject_name: Set(req.debit_subject_name),
            credit_subject_code: Set(req.credit_subject_code),
            credit_subject_name: Set(req.credit_subject_name),
            amount: Set(req.amount),
            source_type: Set(req.source_type),
            source_bill_id: Set(req.source_bill_id),
            source_bill_no: Set(req.source_bill_no),
            voucher_id: Set(None),
            reverse_voucher_id: Set(None),
            status: Set(period_adjustment_status::DRAFT.to_string()),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            reversed_by: Set(None),
            reversed_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("期末调整记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 确认期末调整（draft → confirmed），生成调整凭证并回写 voucher_id
    pub async fn confirm(&self, id: i32, user_id: i32) -> Result<AdjustmentModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != period_adjustment_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();

        // 生成调整凭证（借 debit_subject / 贷 credit_subject）
        let voucher_id = self.create_adjustment_voucher(&model, user_id, false).await;

        let mut active: AdjustmentActiveModel = model.into();
        active.status = Set(period_adjustment_status::CONFIRMED.to_string());
        active.voucher_id = Set(voucher_id);
        active.confirmed_by = Set(Some(user_id));
        active.confirmed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 红字冲销（confirmed → reversed），生成红字冲销凭证（借贷对调）并回写 reverse_voucher_id
    /// 典型场景：暂估类调整下月初红字冲销
    pub async fn reverse(&self, id: i32, user_id: i32) -> Result<AdjustmentModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != period_adjustment_status::CONFIRMED {
            return Err(AppError::business(format!(
                "仅已确认(confirmed)状态可冲销，当前状态: {}",
                model.status
            )));
        }
        let now = crate::utils::date_utils::utc_now_fixed();

        // 生成红字冲销凭证（借贷对调：借 credit_subject / 贷 debit_subject）
        let reverse_voucher_id = self.create_adjustment_voucher(&model, user_id, true).await;

        let mut active: AdjustmentActiveModel = model.into();
        active.status = Set(period_adjustment_status::REVERSED.to_string());
        active.reverse_voucher_id = Set(reverse_voucher_id);
        active.reversed_by = Set(Some(user_id));
        active.reversed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消期末调整（draft → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<AdjustmentModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != period_adjustment_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可取消，当前状态: {}",
                model.status
            )));
        }
        let mut active: AdjustmentActiveModel = model.into();
        active.status = Set(period_adjustment_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 生成调整凭证（is_reverse=true 时借贷对调生成红字冲销凭证）。
    /// 失败仅 warn 返回 None（不阻断状态流转，与 B05-P2-8/B05-P2-9 语义一致）。
    async fn create_adjustment_voucher(
        &self,
        model: &AdjustmentModel,
        user_id: i32,
        is_reverse: bool,
    ) -> Option<i32> {
        use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest};

        let amount = model.amount;
        if amount <= Decimal::ZERO {
            return None;
        }
        // 红字冲销：借贷对调
        let (debit_code, debit_name, credit_code, credit_name) = if is_reverse {
            (
                model.credit_subject_code.clone(),
                model.credit_subject_name.clone(),
                model.debit_subject_code.clone(),
                model.debit_subject_name.clone(),
            )
        } else {
            (
                model.debit_subject_code.clone(),
                model.debit_subject_name.clone(),
                model.credit_subject_code.clone(),
                model.credit_subject_name.clone(),
            )
        };
        let summary = if is_reverse {
            format!(
                "红字冲销 期末调整 {} ({})",
                model.adjustment_no, model.description
            )
        } else {
            format!(
                "期末调整确认 {} ({})",
                model.adjustment_no, model.description
            )
        };
        let source_module = if is_reverse {
            "period_adjustment_reverse"
        } else {
            "period_adjustment_record"
        };

        let voucher_service =
            crate::services::voucher_service::VoucherService::new(self.db.clone());
        let voucher_date = chrono::Utc::now().date_naive();
        let req = CreateVoucherRequest {
            voucher_type: "transfer".to_string(),
            voucher_date,
            source_type: Some("period_adjustment".to_string()),
            source_module: Some(source_module.to_string()),
            source_bill_id: Some(model.id),
            source_bill_no: Some(model.adjustment_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                VoucherItemRequest {
                    line_no: None,
                    subject_code: Some(debit_code),
                    subject_name: Some(debit_name),
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
                VoucherItemRequest {
                    line_no: None,
                    subject_code: Some(credit_code),
                    subject_name: Some(credit_name),
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
            ],
        };
        match voucher_service.create_and_post(req, user_id).await {
            Ok(voucher) => {
                tracing::info!(
                    adjustment_id = model.id,
                    adjustment_no = %model.adjustment_no,
                    voucher_id = voucher.id,
                    is_reverse,
                    "期末调整凭证生成成功（B05-P2-10）"
                );
                Some(voucher.id)
            }
            Err(e) => {
                tracing::warn!(
                    adjustment_id = model.id,
                    adjustment_no = %model.adjustment_no,
                    is_reverse,
                    error = %e,
                    "期末调整凭证生成失败（不阻断状态流转，B05-P2-10）"
                );
                None
            }
        }
    }

    /// V15 P2 B05-P2-10：结账前批量确认指定期间的 draft 期末调整（close_period 调用，单条失败仅 warn）。
    pub async fn confirm_pending(&self, period: &str, user_id: i32) -> Result<u64, AppError> {
        let pending = AdjustmentEntity::find()
            .filter(period_adjustment_record::Column::Period.eq(period))
            .filter(period_adjustment_record::Column::Status.eq(period_adjustment_status::DRAFT))
            .filter(period_adjustment_record::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await?;

        let total = pending.len() as u64;
        let mut confirmed = 0u64;
        for adj in pending {
            match self.confirm(adj.id, user_id).await {
                Ok(_) => confirmed += 1,
                Err(e) => tracing::warn!(
                    adjustment_id = adj.id,
                    adjustment_no = %adj.adjustment_no,
                    period = %period,
                    error = %e,
                    "期末调整自动确认失败（不阻断结账，B05-P2-10）"
                ),
            }
        }
        tracing::info!(
            period = %period,
            total,
            confirmed,
            "期末调整批量确认完成（B05-P2-10）"
        );
        Ok(confirmed)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<AdjustmentModel, AppError> {
        AdjustmentEntity::find_by_id(id)
            .filter(period_adjustment_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("期末调整记录 {} 不存在", id)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: PeriodAdjustmentQuery,
    ) -> Result<(Vec<AdjustmentModel>, u64), AppError> {
        let mut q =
            AdjustmentEntity::find().filter(period_adjustment_record::Column::IsDeleted.eq(false));
        if let Some(v) = query.adjustment_type {
            q = q.filter(period_adjustment_record::Column::AdjustmentType.eq(v));
        }
        if let Some(v) = query.period {
            q = q.filter(period_adjustment_record::Column::Period.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(period_adjustment_record::Column::Status.eq(v));
        }
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(period_adjustment_record::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
