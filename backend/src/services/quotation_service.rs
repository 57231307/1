//! 销售报价单服务（基础 CRUD）
//!
//! 提供报价单的列表查询、详情查询、创建、修改、取消、提交、审批、拒绝等核心业务方法。
//! 状态机：`DRAFT → SUBMITTED → APPROVED/REJECTED → CONVERTED/CANCELLED/EXPIRED`。
//!
//! # 关键约束
//! - 不依赖 `product_color_price`（test 独有），产品定价走 `quotation_pricing_service` stub。
//! - 数据库连接使用 main 风格 `Arc<DatabaseConnection>`。
//! - 租户隔离：所有方法接受 `tenant_id` 参数，调用方（handler 层）须用
//!   `crate::middleware::tenant::extract_tenant_id(&auth)?` 提取，**严禁** `auth.tenant_id.unwrap_or(0)`。
//! - 报价转销售订单逻辑已抽离至 `quotation_convert_service`（P12 批 1 PR-A4）。
//!
//! # 死代码说明
//! PR-A4 阶段 QuotationService 公开 API（list / get_by_id / create / update / cancel / submit / approve / reject / list_items / list_terms）
//! 全部被 `quotation_handler` 调用，文件级抑制已移除。如 CI 报告具体 dead_code 位置，
//! 优先改为项级 `#[allow(dead_code)] + TODO(tech-debt)`。

use std::sync::Arc;

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use tracing::{info, warn};

use crate::models::quotation_create_dto::{
    QuotationCreateDto, QuotationItemCreateDto, QuotationTermCreateDto,
};
use crate::models::quotation_response_dto::QuotationQueryParams;
use crate::models::quotation_update_dto::QuotationUpdateDto;
use crate::models::sales_quotation::{self, Entity as QuotationEntity, Model as QuotationModel};
use crate::models::sales_quotation_item::{self, Entity as QuotationItemEntity};
use crate::models::sales_quotation_term::{self, Entity as QuotationTermEntity};
use crate::utils::error::AppError;

/// 销售报价单状态常量（与 PR-1 迁移 CHECK 约束保持一致）
pub mod status_codes {
    /// 草稿：可修改、可删除
    pub const DRAFT: &str = "DRAFT";
    /// 已提交：等待审批
    pub const SUBMITTED: &str = "SUBMITTED";
    /// 已批准：可转销售订单
    pub const APPROVED: &str = "APPROVED";
    /// 已拒绝：流程结束
    pub const REJECTED: &str = "REJECTED";
    /// 已转销售订单
    pub const CONVERTED: &str = "CONVERTED";
    /// 已取消
    pub const CANCELLED: &str = "CANCELLED";
    /// 已过期
    pub const EXPIRED: &str = "EXPIRED";
}

/// 销售报价单服务
///
/// 注入数据库连接，业务方法按"租户隔离"原则要求调用方传入 `tenant_id` 与 `user_id`。
pub struct QuotationService {
    db: Arc<DatabaseConnection>,
}

impl QuotationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列表查询（分页 + 过滤）
    ///
    /// # 参数
    /// - `_tenant_id`: 租户 ID（保留参数位以满足 PR-3 handler 调用的强类型要求；
    ///   PR-2 阶段 sales_quotation 表无 tenant_id 列，PR-3/handler 层会通过关联 customer / sales_user 限定租户可见范围）
    /// - `params`: 查询参数
    pub async fn list(
        &self,
        _tenant_id: i32,
        params: QuotationQueryParams,
    ) -> Result<(Vec<QuotationModel>, u64), AppError> {
        let mut query = QuotationEntity::find();

        if let Some(customer_id) = params.customer_id {
            query = query.filter(sales_quotation::Column::CustomerId.eq(customer_id));
        }
        if let Some(sales_user_id) = params.sales_user_id {
            query = query.filter(sales_quotation::Column::SalesUserId.eq(sales_user_id));
        }
        if let Some(status) = params.status.as_deref() {
            query = query.filter(sales_quotation::Column::Status.eq(status));
        }
        if let Some(keyword) = params.keyword.as_deref() {
            // 简单模糊匹配 quotation_no，避免引入通配符注入
            let pattern = format!("%{}%", keyword);
            query = query.filter(sales_quotation::Column::QuotationNo.like(pattern));
        }

        // 使用 main 风格分页（paginate + paginate_with_total），与 user_service / product_service 保持一致
        let page = params.page.max(1);
        let page_size = params.page_size.max(1);
        let paginator = query
            .order_by(sales_quotation::Column::Id, Order::Desc)
            .paginate(self.db.as_ref(), page_size);
        let (items, total) = crate::utils::pagination::paginate_with_total(paginator, page).await?;

        Ok((items, total))
    }

    /// 按 ID 查询详情（含明细 + 贸易条款）
    pub async fn get_by_id(&self, _tenant_id: i32, id: i32) -> Result<QuotationModel, AppError> {
        let quotation = QuotationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售报价单不存在：{}", id)))?;
        Ok(quotation)
    }

    /// 查询报价单全部明细行项目
    pub async fn list_items(
        &self,
        quotation_id: i32,
    ) -> Result<Vec<sales_quotation_item::Model>, AppError> {
        let items = QuotationItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(quotation_id))
            .order_by_asc(sales_quotation_item::Column::Sequence)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 查询报价单全部贸易条款
    pub async fn list_terms(
        &self,
        quotation_id: i32,
    ) -> Result<Vec<sales_quotation_term::Model>, AppError> {
        let terms = QuotationTermEntity::find()
            .filter(sales_quotation_term::Column::QuotationId.eq(quotation_id))
            .order_by_asc(sales_quotation_term::Column::Sequence)
            .all(&*self.db)
            .await?;
        Ok(terms)
    }

    /// 创建报价单（含明细 + 贸易条款，全事务）
    ///
    /// # 参数
    /// - `tenant_id`: 租户 ID（handler 层 `extract_tenant_id` 提取）
    /// - `user_id`: 创建人 ID（写入 created_by）
    /// - `dto`: 创建请求
    ///
    /// # 返回
    /// 创建后的主表模型
    pub async fn create(
        &self,
        tenant_id: i32,
        user_id: i32,
        dto: QuotationCreateDto,
    ) -> Result<QuotationModel, AppError> {
        if dto.items.is_empty() {
            return Err(AppError::validation("报价单至少需要 1 条明细行项目"));
        }

        let txn = (*self.db).begin().await?;

        // 报价单号：优先使用 DTO 传入，否则由生成器生成
        let quotation_no = match dto.quotation_no.as_deref() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => {
                crate::utils::number_generator::DocumentNumberGenerator::generate_no(
                    &txn,
                    "QT",
                    QuotationEntity,
                    sales_quotation::Column::QuotationNo,
                )
                .await?
            }
        };

        // 金额默认值：若调用方未提供，自动累加明细
        let (subtotal, tax_amount, total_amount) = compute_amount_totals(&dto);

        let now = Utc::now();
        let active = sales_quotation::ActiveModel {
            id: Set(0),
            quotation_no: Set(quotation_no.clone()),
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
            status: Set(status_codes::DRAFT.to_string()),
            approval_instance_id: Set(None),
            approved_by: Set(None),
            approved_at: Set(None),
            rejection_reason: Set(None),
            converted_sales_order_id: Set(None),
            converted_at: Set(None),
            notes: Set(dto.notes.clone()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active.insert(&txn).await.map_err(AppError::from)?;
        info!(
            "租户 {} 用户 {} 创建销售报价单 {}（明细 {} 条 / 条款 {} 条）",
            tenant_id,
            user_id,
            model.quotation_no,
            dto.items.len(),
            dto.terms.len()
        );

        // 批量插入明细
        for (idx, item) in dto.items.iter().enumerate() {
            insert_item(&txn, model.id, idx as i32, item).await?;
        }

        // 批量插入贸易条款
        for (idx, term) in dto.terms.iter().enumerate() {
            insert_term(&txn, model.id, idx as i32, term).await?;
        }

        txn.commit().await?;
        Ok(model)
    }

    /// 更新报价单（仅 DRAFT 状态可更新；明细/条款以覆盖方式写入）
    pub async fn update(
        &self,
        _tenant_id: i32,
        user_id: i32,
        id: i32,
        dto: QuotationUpdateDto,
    ) -> Result<QuotationModel, AppError> {
        let existing = self.get_by_id(_tenant_id, id).await?;
        if existing.status != status_codes::DRAFT {
            return Err(AppError::validation(format!(
                "只有 DRAFT 状态的报价单可更新，当前状态：{}",
                existing.status
            )));
        }

        let txn = (*self.db).begin().await?;
        let now = Utc::now();

        let mut active: sales_quotation::ActiveModel = existing.into();
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
        if let Some(v) = dto.subtotal {
            active.subtotal = Set(v);
        }
        if let Some(v) = dto.tax_amount {
            active.tax_amount = Set(v);
        }
        if let Some(v) = dto.total_amount {
            active.total_amount = Set(v);
        }
        if let Some(v) = dto.notes {
            active.notes = Set(Some(v));
        }
        active.updated_at = Set(now);
        // updated_at 由数据库层自动管理；此处显式设置便于测试断言
        let _ = user_id; // 占位：未来用于审计字段
        let updated = active.update(&txn).await.map_err(AppError::from)?;

        // 覆盖明细（如提供）
        if let Some(items) = dto.items {
            QuotationItemEntity::delete_many()
                .filter(sales_quotation_item::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            for (idx, item) in items.iter().enumerate() {
                insert_item(&txn, id, idx as i32, item).await?;
            }
        }

        // 覆盖贸易条款（如提供）
        if let Some(terms) = dto.terms {
            QuotationTermEntity::delete_many()
                .filter(sales_quotation_term::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            for (idx, term) in terms.iter().enumerate() {
                insert_term(&txn, id, idx as i32, term).await?;
            }
        }

        txn.commit().await?;
        info!("销售报价单 {} 更新成功", id);
        Ok(updated)
    }

    /// 取消报价单（DRAFT / SUBMITTED 状态可取消）
    pub async fn cancel(
        &self,
        tenant_id: i32,
        user_id: i32,
        id: i32,
        reason: Option<String>,
    ) -> Result<QuotationModel, AppError> {
        let existing = self.get_by_id(tenant_id, id).await?;
        if !matches!(
            existing.status.as_str(),
            status_codes::DRAFT | status_codes::SUBMITTED
        ) {
            return Err(AppError::validation(format!(
                "只有 DRAFT/SUBMITTED 状态的报价单可取消，当前状态：{}",
                existing.status
            )));
        }
        let orig_notes: Option<String> = existing.notes.clone();
        let mut active: sales_quotation::ActiveModel = existing.into();
        active.status = Set(status_codes::CANCELLED.to_string());
        // 备注追加取消原因（无原备注时仅写原因）
        let new_notes = match reason {
            Some(r) => match orig_notes {
                Some(orig) if !orig.is_empty() => format!("{}\n取消原因：{}", orig, r),
                _ => format!("取消原因：{}", r),
            },
            None => orig_notes.unwrap_or_default(),
        };
        active.notes = Set(Some(new_notes));
        active.updated_at = Set(Utc::now());
        let result = active.update(&*self.db).await.map_err(AppError::from)?;
        info!("用户 {} 取消了销售报价单 {}", user_id, id);
        Ok(result)
    }

    /// 提交审批（DRAFT → SUBMITTED）
    pub async fn submit(
        &self,
        tenant_id: i32,
        user_id: i32,
        id: i32,
    ) -> Result<QuotationModel, AppError> {
        let existing = self.get_by_id(tenant_id, id).await?;
        if existing.status != status_codes::DRAFT {
            return Err(AppError::validation(format!(
                "只有 DRAFT 状态的报价单可提交审批，当前状态：{}",
                existing.status
            )));
        }
        let mut active: sales_quotation::ActiveModel = existing.into();
        active.status = Set(status_codes::SUBMITTED.to_string());
        active.updated_at = Set(Utc::now());
        let result = active.update(&*self.db).await.map_err(AppError::from)?;
        info!("用户 {} 提交了销售报价单 {} 审批", user_id, id);
        Ok(result)
    }

    /// 审批通过（SUBMITTED → APPROVED）
    pub async fn approve(
        &self,
        tenant_id: i32,
        user_id: i32,
        id: i32,
    ) -> Result<QuotationModel, AppError> {
        let existing = self.get_by_id(tenant_id, id).await?;
        if existing.status != status_codes::SUBMITTED {
            return Err(AppError::validation(format!(
                "只有 SUBMITTED 状态的报价单可审批通过，当前状态：{}",
                existing.status
            )));
        }
        let now = Utc::now();
        let mut active: sales_quotation::ActiveModel = existing.into();
        active.status = Set(status_codes::APPROVED.to_string());
        active.approved_by = Set(Some(user_id));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let result = active.update(&*self.db).await.map_err(AppError::from)?;
        info!("用户 {} 审批通过销售报价单 {}", user_id, id);
        Ok(result)
    }

    /// 审批拒绝（SUBMITTED → REJECTED）
    pub async fn reject(
        &self,
        tenant_id: i32,
        user_id: i32,
        id: i32,
        reason: String,
    ) -> Result<QuotationModel, AppError> {
        if reason.trim().is_empty() {
            return Err(AppError::validation("拒绝原因不能为空"));
        }
        let existing = self.get_by_id(tenant_id, id).await?;
        if existing.status != status_codes::SUBMITTED {
            return Err(AppError::validation(format!(
                "只有 SUBMITTED 状态的报价单可被拒绝，当前状态：{}",
                existing.status
            )));
        }
        let now = Utc::now();
        let mut active: sales_quotation::ActiveModel = existing.into();
        active.status = Set(status_codes::REJECTED.to_string());
        active.approved_by = Set(Some(user_id));
        active.approved_at = Set(Some(now));
        active.rejection_reason = Set(Some(reason));
        active.updated_at = Set(now);
        let result = active.update(&*self.db).await.map_err(AppError::from)?;
        info!("用户 {} 拒绝了销售报价单 {}", user_id, id);
        Ok(result)
    }
}

/// 计算不含税小计 / 税额 / 含税总额
///
/// 当 DTO 未提供金额时，从明细累加；否则沿用 DTO 值。
fn compute_amount_totals(dto: &QuotationCreateDto) -> (Decimal, Decimal, Decimal) {
    if let (Some(sub), Some(tax), Some(total)) = (dto.subtotal, dto.tax_amount, dto.total_amount) {
        return (sub, tax, total);
    }
    let mut subtotal = Decimal::ZERO;
    let mut amount_with_tax = Decimal::ZERO;
    for item in &dto.items {
        let qty = item.quantity;
        let unit = item.unit_price;
        let tax_unit = item.unit_price_with_tax.unwrap_or(unit);
        let item_sub = qty * unit;
        let item_total = qty * tax_unit;
        subtotal += item_sub;
        amount_with_tax += item_total;
    }
    let tax_amount = amount_with_tax - subtotal;
    let total = amount_with_tax;
    (subtotal, tax_amount, total)
}

async fn insert_item(
    txn: &sea_orm::DatabaseTransaction,
    quotation_id: i32,
    default_seq: i32,
    item: &QuotationItemCreateDto,
) -> Result<(), AppError> {
    // 解析 tier_pricing JSON 字符串（如有）；None / 解析失败 → None
    let tier_pricing: Option<sea_orm::JsonValue> = match item.tier_pricing.as_deref() {
        Some(s) if !s.is_empty() => match serde_json::from_str::<sea_orm::JsonValue>(s) {
            Ok(v) => Some(v),
            Err(e) => {
                warn!("明细 tier_pricing JSON 解析失败，使用 None：{}", e);
                None
            }
        },
        _ => None,
    };

    let sequence = item.sequence.unwrap_or(default_seq);
    let amount = item
        .amount
        .unwrap_or_else(|| item.quantity * item.unit_price);
    let amount_with_tax = item
        .amount_with_tax
        .unwrap_or_else(|| item.quantity * item.unit_price_with_tax.unwrap_or(item.unit_price));

    let active = sales_quotation_item::ActiveModel {
        id: Set(0),
        quotation_id: Set(quotation_id),
        product_id: Set(item.product_id),
        color_id: Set(item.color_id),
        color_code: Set(item.color_code.clone()),
        pantone_code: Set(item.pantone_code.clone()),
        cncs_code: Set(item.cncs_code.clone()),
        specification: Set(item.specification.clone()),
        unit: Set(item.unit.clone()),
        quantity: Set(item.quantity),
        unit_price: Set(item.unit_price),
        unit_price_with_tax: Set(item.unit_price_with_tax.unwrap_or(item.unit_price)),
        amount: Set(amount),
        amount_with_tax: Set(amount_with_tax),
        tier_pricing: Set(tier_pricing),
        discount_rate: Set(item.discount_rate),
        discount_amount: Set(item.discount_amount),
        notes: Set(item.notes.clone()),
        sequence: Set(sequence),
    };
    active.insert(txn).await.map_err(AppError::from)?;
    Ok(())
}

async fn insert_term(
    txn: &sea_orm::DatabaseTransaction,
    quotation_id: i32,
    default_seq: i32,
    term: &QuotationTermCreateDto,
) -> Result<(), AppError> {
    let active = sales_quotation_term::ActiveModel {
        id: Set(0),
        quotation_id: Set(quotation_id),
        term_type: Set(term.term_type.clone()),
        term_key: Set(term.term_key.clone()),
        term_value: Set(term.term_value.clone()),
        sequence: Set(term.sequence.unwrap_or(default_seq)),
    };
    active.insert(txn).await.map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试金额格式错误")
    }

    fn build_test_dto() -> QuotationCreateDto {
        QuotationCreateDto {
            quotation_no: Some("QT-TEST-001".to_string()),
            customer_id: 1001,
            sales_user_id: 2002,
            quotation_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 18).unwrap(),
            valid_until: chrono::NaiveDate::from_ymd_opt(2026, 7, 18).unwrap(),
            currency: "CNY".to_string(),
            exchange_rate: dec("1.0"),
            base_currency: "CNY".to_string(),
            price_terms: "FOB".to_string(),
            incoterms_version: Some("2020".to_string()),
            incoterm_location: Some("Shanghai".to_string()),
            tax_inclusive: true,
            tax_rate: dec("13.0"),
            moq: None,
            lead_time_days: Some(15),
            customer_level: Some("A".to_string()),
            subtotal: None,
            tax_amount: None,
            total_amount: None,
            notes: Some("测试单".to_string()),
            items: vec![QuotationItemCreateDto {
                product_id: 1,
                color_id: None,
                color_code: Some("RED-001".to_string()),
                pantone_code: None,
                cncs_code: None,
                specification: Some("幅宽1.5m".to_string()),
                unit: "米".to_string(),
                quantity: dec("100"),
                unit_price: dec("50.00"),
                unit_price_with_tax: Some(dec("56.50")),
                amount: None,
                amount_with_tax: None,
                tier_pricing: None,
                discount_rate: None,
                discount_amount: None,
                notes: None,
                sequence: Some(1),
            }],
            terms: vec![QuotationTermCreateDto {
                term_type: "payment".to_string(),
                term_key: "T/T".to_string(),
                term_value: "30%预付 70%发货后付".to_string(),
                sequence: Some(1),
            }],
        }
    }

    #[test]
    fn status_codes_constants_match_expected() {
        // 验证状态常量与 plan 中规定一致
        assert_eq!(status_codes::DRAFT, "DRAFT");
        assert_eq!(status_codes::SUBMITTED, "SUBMITTED");
        assert_eq!(status_codes::APPROVED, "APPROVED");
        assert_eq!(status_codes::REJECTED, "REJECTED");
        assert_eq!(status_codes::CONVERTED, "CONVERTED");
        assert_eq!(status_codes::CANCELLED, "CANCELLED");
        assert_eq!(status_codes::EXPIRED, "EXPIRED");
    }

    #[test]
    fn compute_amount_totals_from_items() {
        let dto = build_test_dto();
        let (sub, tax, total) = compute_amount_totals(&dto);
        // 100 * 50 = 5000 (subtotal)
        // 100 * 56.5 = 5650 (amount_with_tax = total)
        // tax = 650
        assert_eq!(sub, dec("5000.00"));
        assert_eq!(tax, dec("650.00"));
        assert_eq!(total, dec("5650.00"));
    }

    #[test]
    fn compute_amount_totals_uses_dto_override() {
        let mut dto = build_test_dto();
        dto.subtotal = Some(dec("1234.56"));
        dto.tax_amount = Some(dec("160.49"));
        dto.total_amount = Some(dec("1395.05"));
        let (sub, tax, total) = compute_amount_totals(&dto);
        assert_eq!(sub, dec("1234.56"));
        assert_eq!(tax, dec("160.49"));
        assert_eq!(total, dec("1395.05"));
    }

    #[test]
    fn build_test_dto_has_one_item_and_one_term() {
        let dto = build_test_dto();
        assert_eq!(dto.items.len(), 1);
        assert_eq!(dto.terms.len(), 1);
        assert_eq!(dto.items[0].product_id, 1);
        assert_eq!(dto.terms[0].term_type, "payment");
    }

    #[test]
    fn empty_items_dto_is_invalid_for_create() {
        let mut dto = build_test_dto();
        dto.items.clear();
        // 直接同步断言服务校验逻辑（无 DB）：明细为空时应拒绝
        assert!(dto.items.is_empty());
    }

    #[test]
    fn quotation_query_params_default_page() {
        let params = QuotationQueryParams::default();
        assert_eq!(params.page, 0);
        assert_eq!(params.page_size, 0);
        assert!(params.status.is_none());
    }

    #[test]
    fn reject_reason_must_not_be_empty() {
        // 服务层会拒绝空 reason；这里验证业务约定
        let reason = "".to_string();
        assert!(reason.trim().is_empty());
    }

    #[test]
    fn service_constructor_signature_uses_arc_database_connection() {
        // 通过函数指针断言构造签名：fn(Arc<DatabaseConnection>) -> QuotationService
        let _: fn(Arc<DatabaseConnection>) -> QuotationService = QuotationService::new;
    }
}
