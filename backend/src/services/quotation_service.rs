//! 销售报价单服务层
//!
//! 基础 CRUD（create_draft / list / get_by_id / update / cancel）。
//! 完整业务功能（pricing/approval/convert）由 Week 2/3 子代理扩展。
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 4

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
use crate::models::quotation_update_dto::UpdateQuotationDto;
use crate::models::sales_quotation::{self, ActiveModel as QuotationActive, Entity as QuotationEntity};
use crate::models::sales_quotation_item::{
    self, ActiveModel as ItemActive, Entity as ItemEntity,
};
use crate::models::sales_quotation_term::{
    self, ActiveModel as TermActive, Entity as TermEntity,
};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("报价单不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 销售报价单服务
pub struct QuotationService {
    db: Arc<DatabaseConnection>,
}

impl QuotationService {
    /// 从数据库连接直接构造（与项目其他服务保持一致）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建报价单草稿
    pub async fn create_draft(
        &self,
        dto: CreateQuotationDto,
        user_id: i64,
    ) -> Result<sales_quotation::Model, ServiceError> {
        // 1. 生成 quotation_no（QT + YYYYMMDD + 4 位序号）
        let quotation_no = self.generate_quotation_no().await?;

        // 2. 计算金额
        let (subtotal, tax_amount, total_amount) = self.calculate_totals(&dto)?;

        // 3. 业务校验
        self.validate_create(&dto)?;

        // 4. 开始事务
        let txn = self.db.begin().await?;

        // 5. 插入主表
        let now = Utc::now();
        let active = QuotationActive {
            id: Default::default(),
            quotation_no: Set(quotation_no),
            customer_id: Set(dto.customer_id),
            sales_user_id: Set(dto.sales_user_id),
            quotation_date: Set(dto.quotation_date),
            valid_until: Set(dto.valid_until),
            currency: Set(dto.currency),
            exchange_rate: Set(dto.exchange_rate),
            base_currency: Set(dto.base_currency),
            price_terms: Set(dto.price_terms),
            incoterms_version: Set(dto.incoterms_version),
            incoterm_location: Set(dto.incoterm_location),
            tax_inclusive: Set(dto.tax_inclusive),
            tax_rate: Set(dto.tax_rate),
            moq: Set(dto.moq),
            lead_time_days: Set(dto.lead_time_days),
            customer_level: Set(dto.customer_level),
            subtotal: Set(subtotal),
            tax_amount: Set(tax_amount),
            total_amount: Set(total_amount),
            status: Set("draft".to_string()),
            approval_instance_id: Set(None),
            approved_by: Set(None),
            approved_at: Set(None),
            rejection_reason: Set(None),
            converted_sales_order_id: Set(None),
            converted_at: Set(None),
            notes: Set(dto.notes),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&txn).await?;

        // 6. 插入明细
        for (idx, item_dto) in dto.items.iter().enumerate() {
            let item = ItemActive {
                id: Default::default(),
                quotation_id: Set(result.id),
                product_id: Set(item_dto.product_id),
                color_id: Set(item_dto.color_id),
                color_code: Set(None),
                pantone_code: Set(None),
                cncs_code: Set(None),
                specification: Set(item_dto.specification.clone()),
                unit: Set(item_dto.unit.clone()),
                quantity: Set(item_dto.quantity),
                unit_price: Set(item_dto.unit_price),
                unit_price_with_tax: Set(item_dto.unit_price_with_tax),
                // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
                amount: Set((item_dto.quantity * item_dto.unit_price).round_dp(2)),
                amount_with_tax: Set(
                    (item_dto.quantity * item_dto.unit_price_with_tax).round_dp(2),
                ),
                tier_pricing: Set(item_dto
                    .tier_pricing
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())),
                discount_rate: Set(item_dto.discount_rate),
                discount_amount: Set(item_dto.discount_rate.map(|r| {
                    (item_dto.quantity * item_dto.unit_price * r / Decimal::from(100)).round_dp(2)
                })),
                notes: Set(item_dto.notes.clone()),
                sequence: Set(idx as i32),
            };
            item.insert(&txn).await?;
        }

        // 7. 插入贸易条款
        if let Some(terms) = dto.terms {
            for term in terms {
                let term_active = TermActive {
                    id: Default::default(),
                    quotation_id: Set(result.id),
                    term_type: Set(term.term_type),
                    term_key: Set(term.term_key),
                    term_value: Set(term.term_value),
                    sequence: Set(term.sequence),
                };
                term_active.insert(&txn).await?;
            }
        }

        // 8. 提交事务
        txn.commit().await?;

        Ok(result)
    }

    /// 列表查询（分页 + 过滤）
    pub async fn list(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        customer_id: Option<i64>,
        sales_user_id: Option<i64>,
        keyword: Option<String>,
    ) -> Result<(Vec<sales_quotation::Model>, u64), ServiceError> {
        let mut query = QuotationEntity::find();

        if let Some(s) = status {
            query = query.filter(sales_quotation::Column::Status.eq(s));
        }
        if let Some(c) = customer_id {
            query = query.filter(sales_quotation::Column::CustomerId.eq(c));
        }
        if let Some(u) = sales_user_id {
            query = query.filter(sales_quotation::Column::SalesUserId.eq(u));
        }
        if let Some(k) = keyword {
            let pattern = format!("%{}%", k);
            query = query.filter(sales_quotation::Column::QuotationNo.like(pattern));
        }

        let paginator = query
            .order_by_desc(sales_quotation::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i64) -> Result<sales_quotation::Model, ServiceError> {
        QuotationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(ServiceError::NotFound)
    }

    /// 更新报价单（仅 draft / rejected 状态可更新）
    ///
    /// 批次 85 v2 复审 P1-2 修复：状态门移入 txn + lock_exclusive，重算 update 移入 txn
    /// 原实现状态门在 txn 外查询（self.db），且 line 360 重算后 update 在 txn commit 后用 self.db，
    /// 存在 TOCTOU（并发 update/cancel 会基于过期状态通过检查）和非原子性（commit 后 update 失败导致状态不一致）
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateQuotationDto,
    ) -> Result<sales_quotation::Model, ServiceError> {
        let txn = self.db.begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if !["draft", "rejected"].contains(&existing.status.as_str()) {
            return Err(ServiceError::InvalidState);
        }

        let mut active: QuotationActive = existing.clone().into();
        if let Some(v) = dto.customer_id {
            active.customer_id = Set(v);
        }
        if let Some(v) = dto.sales_user_id {
            active.sales_user_id = Set(v);
        }
        if let Some(v) = dto.quotation_date {
            active.quotation_date = Set(v);
        }
        if let Some(v) = dto.valid_until {
            active.valid_until = Set(v);
        }
        if let Some(v) = dto.currency {
            active.currency = Set(v);
        }
        if let Some(v) = dto.exchange_rate {
            active.exchange_rate = Set(v);
        }
        if let Some(v) = dto.base_currency {
            active.base_currency = Set(v);
        }
        if let Some(v) = dto.price_terms {
            active.price_terms = Set(v);
        }
        if let Some(v) = dto.incoterms_version {
            active.incoterms_version = Set(Some(v));
        }
        if let Some(v) = dto.incoterm_location {
            active.incoterm_location = Set(Some(v));
        }
        if let Some(v) = dto.tax_inclusive {
            active.tax_inclusive = Set(v);
        }
        if let Some(v) = dto.tax_rate {
            active.tax_rate = Set(v);
        }
        if let Some(v) = dto.moq {
            active.moq = Set(Some(v));
        }
        if let Some(v) = dto.lead_time_days {
            active.lead_time_days = Set(Some(v));
        }
        if let Some(v) = dto.customer_level {
            active.customer_level = Set(Some(v));
        }
        if let Some(v) = dto.notes {
            active.notes = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;

        // 如果 dto.items 存在，全量替换明细
        if let Some(items) = dto.items {
            // 校验：非空
            if items.is_empty() {
                return Err(ServiceError::Validation("明细至少 1 条".to_string()));
            }
            // 删除旧明细
            ItemEntity::delete_many()
                .filter(sales_quotation_item::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            // 插入新明细
            for (idx, item_dto) in items.iter().enumerate() {
                let item_active = ItemActive {
                    id: Default::default(),
                    quotation_id: Set(id),
                    product_id: Set(item_dto.product_id),
                    color_id: Set(item_dto.color_id),
                    color_code: Set(None),
                    pantone_code: Set(None),
                    cncs_code: Set(None),
                    specification: Set(item_dto.specification.clone()),
                    unit: Set(item_dto.unit.clone()),
                    quantity: Set(item_dto.quantity),
                    unit_price: Set(item_dto.unit_price),
                    unit_price_with_tax: Set(item_dto.unit_price_with_tax),
                    // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
                    amount: Set((item_dto.quantity * item_dto.unit_price).round_dp(2)),
                    amount_with_tax: Set(
                        (item_dto.quantity * item_dto.unit_price_with_tax).round_dp(2),
                    ),
                    tier_pricing: Set(item_dto
                        .tier_pricing
                        .as_ref()
                        .and_then(|v| serde_json::from_value(v.clone()).ok())),
                    discount_rate: Set(item_dto.discount_rate),
                    discount_amount: Set(item_dto.discount_rate.map(|r| {
                        (item_dto.quantity * item_dto.unit_price * r / Decimal::from(100))
                            .round_dp(2)
                    })),
                    notes: Set(item_dto.notes.clone()),
                    sequence: Set(idx as i32),
                };
                item_active.insert(&txn).await?;
            }
        }

        // 如果 dto.terms 存在，全量替换条款
        if let Some(terms) = dto.terms {
            TermEntity::delete_many()
                .filter(sales_quotation_term::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            for term in terms {
                let term_active = TermActive {
                    id: Default::default(),
                    quotation_id: Set(id),
                    term_type: Set(term.term_type),
                    term_key: Set(term.term_key),
                    term_value: Set(term.term_value),
                    sequence: Set(term.sequence),
                };
                term_active.insert(&txn).await?;
            }
        }

        // 重算 subtotal/tax/total（在 txn 内查询和 update，保证原子性）
        let recalc_items: Vec<sales_quotation_item::Model> = ItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(id))
            .all(&txn)
            .await?;
        let subtotal: Decimal = recalc_items
            .iter()
            .map(|i| i.amount)
            .sum();
        let tax_amount = if updated.tax_inclusive {
            Decimal::ZERO
        } else {
            subtotal * updated.tax_rate / Decimal::from(100)
        };
        let total_amount = subtotal + tax_amount;

        let mut re_active: QuotationActive = updated.clone().into();
        re_active.subtotal = Set(subtotal);
        re_active.tax_amount = Set(tax_amount);
        re_active.total_amount = Set(total_amount);
        re_active.updated_at = Set(Utc::now());
        let final_model = re_active.update(&txn).await?;

        txn.commit().await?;
        Ok(final_model)
    }

    /// 取消报价单（任意非 converted 状态可取消）
    pub async fn cancel(
        &self,
        id: i64,
        user_id: i64,
    ) -> Result<sales_quotation::Model, ServiceError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询无行锁，且 update 也用裸连接，无事务保护。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更，update 一并纳入事务。
        let txn = (*self.db).begin().await?;
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if existing.status == "converted" {
            return Err(ServiceError::InvalidState);
        }
        if existing.status == "cancelled" {
            return Ok(existing);
        }

        let mut active: QuotationActive = existing.into();
        active.status = Set("cancelled".to_string());
        active.updated_at = Set(Utc::now());
        // 备注 created_by 引用仅作审计
        let _ = user_id;
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    // ----------------------------------------------------------------------
    // 私有辅助
    // ----------------------------------------------------------------------

    /// 生成报价单号：QT + YYYYMMDD + 4 位当日序号
    async fn generate_quotation_no(&self) -> Result<String, ServiceError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let pattern = format!("QT{}%", today);
        let count = QuotationEntity::find()
            .filter(sales_quotation::Column::QuotationNo.like(pattern))
            .count(&*self.db)
            .await?;
        Ok(format!("QT{}{:04}", today, count + 1))
    }

    /// 计算小计/税额/总金额
    fn calculate_totals(
        &self,
        dto: &CreateQuotationDto,
    ) -> Result<(Decimal, Decimal, Decimal), ServiceError> {
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let subtotal: Decimal = dto
            .items
            .iter()
            .map(|i: &CreateQuotationItemDto| (i.quantity * i.unit_price).round_dp(2))
            .sum::<Decimal>()
            .round_dp(2);

        let tax_amount = if dto.tax_inclusive {
            // 含税：报价单小计已含税，差额为 0
            Decimal::ZERO
        } else {
            // 不含税：税额 = 小计 * 税率
            (subtotal * dto.tax_rate / Decimal::from(100)).round_dp(2)
        };

        let total_amount = (subtotal + tax_amount).round_dp(2);
        Ok((subtotal, tax_amount, total_amount))
    }

    /// 业务校验
    fn validate_create(&self, dto: &CreateQuotationDto) -> Result<(), ServiceError> {
        if dto.items.is_empty() {
            return Err(ServiceError::Validation("明细至少 1 条".to_string()));
        }
        if dto.valid_until < dto.quotation_date {
            return Err(ServiceError::Validation(
                "有效期截止必须不早于报价日期".to_string(),
            ));
        }
        if !["FOB", "CIF", "EXW", "DDP", "DAP"].contains(&dto.price_terms.as_str()) {
            return Err(ServiceError::Validation(format!(
                "不支持的价格条款: {}",
                dto.price_terms
            )));
        }
        Ok(())
    }
}

// 抑制 unused warnings（Func/Expr 为后续可扩展点预留）
// 移除：占位函数（无业务引用）

