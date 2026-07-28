//! 报价单 CRUD impl 子模块（quotation_ops/crud）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移 CRUD 相关方法。
//! 包含 create_draft / list / get_by_id + create_draft 私有 helpers。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

use crate::models::quotation_create_dto::{
    CreateQuotationDto, CreateQuotationItemDto, CreateQuotationTermDto,
};
use crate::models::sales_quotation::{
    self, ActiveModel as QuotationActive, Entity as QuotationEntity,
};
use crate::models::sales_quotation_item::{ActiveModel as ItemActive, Entity as ItemEntity};
use crate::models::sales_quotation_term::{ActiveModel as TermActive, Entity as TermEntity};
use crate::models::status::quotation as quotation_status;
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::pagination::paginate_with_total;

impl QuotationService {
    /// 创建报价单草稿
    pub async fn create_draft(
        &self,
        dto: CreateQuotationDto,
        user_id: i64,
    ) -> Result<sales_quotation::Model, ServiceError> {
        let quotation_no = self.generate_quotation_no().await?;
        let (subtotal, tax_amount, total_amount) = self.calculate_totals(&dto)?;
        self.validate_create(&dto)?;
        let txn = self.db.begin().await?;
        let now = Utc::now();
        let active = Self::build_new_quotation_active(
            &dto,
            quotation_no,
            subtotal,
            tax_amount,
            total_amount,
            user_id,
            now,
        );
        let result = active.insert(&txn).await?;
        Self::insert_new_quotation_items(&txn, &dto.items, result.id).await?;
        Self::insert_new_quotation_terms(&txn, dto.terms, result.id).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 构建新报价单主表 ActiveModel（纯函数，无 IO）
    pub(crate) fn build_new_quotation_active(
        dto: &CreateQuotationDto,
        quotation_no: String,
        subtotal: Decimal,
        tax_amount: Decimal,
        total_amount: Decimal,
        user_id: i64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> QuotationActive {
        QuotationActive {
            id: Default::default(),
            quotation_no: Set(quotation_no),
            customer_id: Set(dto.customer_id),
            sales_user_id: Set(dto.sales_user_id),
            quotation_date: Set(dto.quotation_date),
            valid_until: Set(dto.valid_until),
            currency: Set(dto.currency.clone()),
            exchange_rate: Set(dto.exchange_rate),
            base_currency: Set(dto.base_currency.clone()),
            price_terms: Set(dto.price_terms.clone()),
            incoterms_version: Set(dto.incoterms_version.clone()),
            incoterm_location: Set(dto.incoterm_location.clone()),
            tax_inclusive: Set(dto.tax_inclusive),
            tax_rate: Set(dto.tax_rate),
            moq: Set(dto.moq),
            lead_time_days: Set(dto.lead_time_days),
            customer_level: Set(dto.customer_level.clone()),
            subtotal: Set(subtotal),
            tax_amount: Set(tax_amount),
            total_amount: Set(total_amount),
            status: Set(quotation_status::DRAFT.to_string()),
            approval_instance_id: Set(None),
            approved_by: Set(None),
            approved_at: Set(None),
            rejection_reason: Set(None),
            converted_sales_order_id: Set(None),
            converted_at: Set(None),
            freight_cost: Set(None),
            insurance_cost: Set(None),
            duty_cost: Set(None),
            notes: Set(dto.notes.clone()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 批量插入报价单明细（insert_many，N 条合并为 1 次 INSERT）
    pub(crate) async fn insert_new_quotation_items(
        txn: &sea_orm::DatabaseTransaction,
        items: &[CreateQuotationItemDto],
        quotation_id: i64,
    ) -> Result<(), ServiceError> {
        let item_active_models: Vec<ItemActive> =
            items
                .iter()
                .enumerate()
                .map(|(idx, item_dto)| ItemActive {
                    id: Default::default(),
                    quotation_id: Set(quotation_id),
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
                    // 批次 87：金额计算补 round_dp(2) 精度归一化
                    amount: Set((item_dto.quantity * item_dto.unit_price).round_dp(2)),
                    amount_with_tax: Set(
                        (item_dto.quantity * item_dto.unit_price_with_tax).round_dp(2)
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
                })
                .collect();
        if !item_active_models.is_empty() {
            ItemEntity::insert_many(item_active_models)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 批量插入报价单贸易条款（仅当 terms 非空时执行 insert_many）
    pub(crate) async fn insert_new_quotation_terms(
        txn: &sea_orm::DatabaseTransaction,
        terms: Option<Vec<CreateQuotationTermDto>>,
        quotation_id: i64,
    ) -> Result<(), ServiceError> {
        let terms = match terms {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(()),
        };
        let term_active_models: Vec<TermActive> = terms
            .into_iter()
            .map(|term| TermActive {
                id: Default::default(),
                quotation_id: Set(quotation_id),
                term_type: Set(term.term_type),
                term_key: Set(term.term_key),
                term_value: Set(term.term_value),
                sequence: Set(term.sequence),
            })
            .collect();
        TermEntity::insert_many(term_active_models)
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 列表查询（分页 + 过滤）
    /// 批次 265：接入 paginate_with_total（已做 page-1 偏移）+ clamp(1,1000) 防 DoS
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

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i64) -> Result<sales_quotation::Model, ServiceError> {
        QuotationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(ServiceError::NotFound)
    }
}
