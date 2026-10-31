//! 报价单更新 impl 子模块（quotation_ops/update）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移 update 相关方法。
//! 包含 update + load_for_update / apply_field_updates / apply_price_terms_update
//! / replace_items / replace_terms / recalculate_totals_and_update。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::quotation_create_dto::{CreateQuotationItemDto, CreateQuotationTermDto};
use crate::models::quotation_update_dto::UpdateQuotationDto;
use crate::models::sales_quotation::{
    self, ActiveModel as QuotationActive, Entity as QuotationEntity,
};
use crate::models::sales_quotation_item::{self, ActiveModel as ItemActive, Entity as ItemEntity};
use crate::models::sales_quotation_term::{self, ActiveModel as TermActive, Entity as TermEntity};
use crate::models::status::quotation as quotation_status;
use crate::services::quotation_service::QuotationService;
use crate::utils::error::AppError;

impl QuotationService {
    /// 更新报价单（仅 draft / rejected 状态可更新）
    /// 批次 85 v2：状态门+重算移入 txn+lock_exclusive 防 TOCTOU；批次 94 用 update_with_audit 记审计
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateQuotationDto,
        user_id: i64,
    ) -> Result<sales_quotation::Model, AppError> {
        let txn = self.db.begin().await?;
        let existing = Self::load_for_update(&txn, id).await?;
        let mut active: QuotationActive = existing.clone().into();
        Self::apply_field_updates(&mut active, &dto, id)?;
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quotation",
            active,
            Some(user_id as i32),
        )
        .await?;
        if let Some(items) = dto.items {
            // 明细折算取本次写回后的口径，开关与税率改了、明细没改，行金额同样跟着换向
            Self::replace_items(&txn, id, items, updated.tax_inclusive, updated.tax_rate).await?;
        }
        if let Some(terms) = dto.terms {
            Self::replace_terms(&txn, id, terms).await?;
        }
        let final_model = Self::recalculate_totals_and_update(&txn, &updated).await?;
        txn.commit().await?;
        Ok(final_model)
    }

    /// 加载报价单并校验状态（draft / rejected 可更新），加 lock_exclusive 串行化并发状态变更
    pub(crate) async fn load_for_update(
        txn: &sea_orm::DatabaseTransaction,
        id: i64,
    ) -> Result<sales_quotation::Model, AppError> {
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if ![quotation_status::DRAFT, quotation_status::REJECTED]
            .contains(&existing.status.as_str())
        {
            // 状态门：仅草稿/驳回可改，前置状态未满足，归业务族
            return Err(AppError::business_displayable(
                "当前状态不允许此操作".to_string(),
            ));
        }
        Ok(existing)
    }

    /// 应用 DTO 标量字段更新到 ActiveModel（不含 items / terms）
    pub(crate) fn apply_field_updates(
        active: &mut QuotationActive,
        dto: &UpdateQuotationDto,
        id: i64,
    ) -> Result<(), AppError> {
        Self::apply_core_updates(active, dto);
        Self::apply_price_terms_update(active, &dto.price_terms, id)?;
        Self::apply_extended_updates(active, dto);
        active.updated_at = Set(Utc::now());
        Ok(())
    }

    fn apply_core_updates(active: &mut QuotationActive, dto: &UpdateQuotationDto) {
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
        if let Some(v) = &dto.currency {
            active.currency = Set(v.clone());
        }
        if let Some(v) = dto.exchange_rate {
            active.exchange_rate = Set(v);
        }
        if let Some(v) = &dto.base_currency {
            active.base_currency = Set(v.clone());
        }
    }

    fn apply_extended_updates(active: &mut QuotationActive, dto: &UpdateQuotationDto) {
        if let Some(v) = &dto.incoterms_version {
            active.incoterms_version = Set(Some(v.clone()));
        }
        if let Some(v) = &dto.incoterm_location {
            active.incoterm_location = Set(Some(v.clone()));
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
        if let Some(v) = &dto.customer_level {
            active.customer_level = Set(Some(v.clone()));
        }
        if let Some(v) = &dto.notes {
            active.notes = Set(Some(v.clone()));
        }
    }

    /// 校验并应用 price_terms 更新（批次 111 P1-2：更新时同样校验贸易术语合法性）
    pub(crate) fn apply_price_terms_update(
        active: &mut QuotationActive,
        price_terms: &Option<String>,
        id: i64,
    ) -> Result<(), AppError> {
        if let Some(v) = price_terms {
            let incoterm = Self::validate_price_terms(v)?;
            tracing::info!(
                quotation_id = id,
                incoterm_code = %v,
                incoterm_description = %incoterm.description(),
                "报价单贸易术语已更新"
            );
            active.price_terms = Set(v.clone());
        }
        Ok(())
    }

    /// 全量替换报价单明细（删除旧明细 + 批量插入新明细）
    ///
    /// 入参 tax_inclusive 与 tax_rate 取本张报价单**本次写回后**的含税标志与税率，
    /// 明细行的两枚单价与三枚金额由 items_to_active_models 交唯一折算入口出，
    /// 与建单腿 quotation_ops/crud.rs 的 insert_new_quotation_items 同式。
    pub(crate) async fn replace_items(
        txn: &sea_orm::DatabaseTransaction,
        id: i64,
        items: Vec<CreateQuotationItemDto>,
        tax_inclusive: bool,
        tax_rate: Decimal,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Err(AppError::validation_displayable(
                "明细至少 1 条".to_string(),
            ));
        }
        // 与创建同源的报价行单位一致性校验：更新明细同样必须跟随产品主数据交易单位
        Self::validate_item_units_against_products(txn, &items).await?;
        ItemEntity::delete_many()
            .filter(sales_quotation_item::Column::QuotationId.eq(id))
            .exec(txn)
            .await?;
        let item_active_models = Self::items_to_active_models(items, id, tax_inclusive, tax_rate)?;
        if !item_active_models.is_empty() {
            ItemEntity::insert_many(item_active_models)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 报价明细 DTO 转 ActiveModel，两枚单价与三枚金额一律走唯一折算入口
    ///
    /// 入参 items 是明细 DTO，quotation_id 是主表 id，tax_inclusive 与 tax_rate 是本单口径。
    /// 传给谁，出参交 replace_items 的 insert_many 落 sales_quotation_items 的
    /// unit_price、unit_price_with_tax、amount、amount_with_tax、discount_amount 列。
    /// 存什么存哪里，本函数不落库；含税标志为真时净额与不含税单价都是除法反算值，
    /// 为假时含税侧两列都是正算值，两条腿都不在调用点写任何税率公式。
    fn items_to_active_models(
        items: Vec<CreateQuotationItemDto>,
        quotation_id: i64,
        tax_inclusive: bool,
        tax_rate: Decimal,
    ) -> Result<Vec<ItemActive>, AppError> {
        let mut models: Vec<ItemActive> = Vec::with_capacity(items.len());
        for (idx, item_dto) in items.into_iter().enumerate() {
            let line = crate::services::quotation_ops::calc::compute_quotation_line(
                item_dto.quantity,
                item_dto.unit_price,
                item_dto.unit_price_with_tax,
                item_dto.discount_rate,
                tax_inclusive,
                Some(tax_rate),
            )
            .map_err(|e| AppError::validation_displayable(e.to_string()))?;
            models.push(ItemActive {
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
                unit_price: Set(line.unit_price),
                unit_price_with_tax: Set(line.unit_price_with_tax),
                amount: Set(line.net_amount),
                amount_with_tax: Set(line.gross_amount),
                tier_pricing: Set(item_dto
                    .tier_pricing
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())),
                discount_rate: Set(item_dto.discount_rate),
                discount_amount: Set(line.discount_amount),
                notes: Set(item_dto.notes.clone()),
                sequence: Set(idx as i32),
            });
        }
        Ok(models)
    }

    /// 全量替换报价单条款（删除旧条款 + 批量插入新条款）
    pub(crate) async fn replace_terms(
        txn: &sea_orm::DatabaseTransaction,
        id: i64,
        terms: Vec<CreateQuotationTermDto>,
    ) -> Result<(), AppError> {
        TermEntity::delete_many()
            .filter(sales_quotation_term::Column::QuotationId.eq(id))
            .exec(txn)
            .await?;
        if !terms.is_empty() {
            let term_active_models: Vec<TermActive> = terms
                .into_iter()
                .map(|term| TermActive {
                    id: Default::default(),
                    quotation_id: Set(id),
                    term_type: Set(term.term_type),
                    term_key: Set(term.term_key),
                    term_value: Set(term.term_value),
                    sequence: Set(term.sequence),
                })
                .collect();
            TermEntity::insert_many(term_active_models)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 重算 subtotal/tax/total 并更新主表（在 txn 内查询和 update，保证原子性）
    ///
    /// 入参 updated 是刚写回的报价单主表模型，明细取本事务内已落库的
    /// sales_quotation_items 的数量与两枚单价列。
    /// 每行都重新经 crate::services::quotation_ops::calc 的 compute_quotation_line
    /// 按**当前**含税标志与单据自身税率折算，折算结果与库里不同就同步回写明细行，
    /// 因此只翻动含税开关而不改明细时，明细的净额 与 含税 与 折扣同样跟着换向
    /// （此前只重算表头，明细仍停在旧口径，一张单上同时存在两套数）。
    /// 表头三值取逐行折算后的求和交 compute_quotation_header 做钩稽判定，
    /// 与建单腿同源，不做合计的直接换算。
    /// 出参三个值写回 sales_quotations 的 subtotal、tax_amount、total_amount 列，
    /// 行级四列写回 sales_quotation_items。
    pub(crate) async fn recalculate_totals_and_update(
        txn: &sea_orm::DatabaseTransaction,
        updated: &sales_quotation::Model,
    ) -> Result<sales_quotation::Model, AppError> {
        let recalc_items: Vec<sales_quotation_item::Model> = ItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(updated.id))
            .all(txn)
            .await?;

        let mut net_sum = Decimal::ZERO;
        let mut tax_sum = Decimal::ZERO;
        let mut gross_sum = Decimal::ZERO;
        for item in &recalc_items {
            let line = crate::services::quotation_ops::calc::compute_quotation_line(
                item.quantity,
                item.unit_price,
                item.unit_price_with_tax,
                item.discount_rate,
                updated.tax_inclusive,
                Some(updated.tax_rate),
            )
            .map_err(|e| AppError::validation_displayable(e.to_string()))?;

            if line.unit_price != item.unit_price
                || line.unit_price_with_tax != item.unit_price_with_tax
                || line.net_amount != item.amount
                || line.gross_amount != item.amount_with_tax
                || line.discount_amount != item.discount_amount
            {
                let mut item_active: sales_quotation_item::ActiveModel = item.clone().into();
                item_active.unit_price = Set(line.unit_price);
                item_active.unit_price_with_tax = Set(line.unit_price_with_tax);
                item_active.amount = Set(line.net_amount);
                item_active.amount_with_tax = Set(line.gross_amount);
                item_active.discount_amount = Set(line.discount_amount);
                item_active.update(txn).await?;
            }

            net_sum += line.net_amount;
            tax_sum += line.tax_amount;
            gross_sum += line.gross_amount;
        }

        let (subtotal, tax_amount, total_amount) =
            crate::services::quotation_ops::calc::compute_quotation_header(
                net_sum, tax_sum, gross_sum,
            )
            .map_err(|e| AppError::validation_displayable(e.to_string()))?;

        let mut re_active: QuotationActive = updated.clone().into();
        re_active.subtotal = Set(subtotal);
        re_active.tax_amount = Set(tax_amount);
        re_active.total_amount = Set(total_amount);
        re_active.updated_at = Set(Utc::now());
        let final_model = re_active.update(txn).await?;
        Ok(final_model)
    }
}
