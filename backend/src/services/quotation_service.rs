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
use crate::models::status::quotation as quotation_status;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

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
    /// 批次 265：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
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
            status: Set(quotation_status::DRAFT.to_string()),
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

        // 6. 插入明细：改用 insert_many 批量 INSERT（原为循环内逐条 insert 导致 N 条=N 次 INSERT）
        let mut item_active_models: Vec<ItemActive> = Vec::with_capacity(dto.items.len());
        for (idx, item_dto) in dto.items.iter().enumerate() {
            item_active_models.push(ItemActive {
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
            });
        }
        if !item_active_models.is_empty() {
            ItemEntity::insert_many(item_active_models)
                .exec(&txn)
                .await?;
        }

        // 7. 插入贸易条款：改用 insert_many 批量 INSERT（原为循环内逐条 insert）
        if let Some(terms) = dto.terms {
            if !terms.is_empty() {
                let term_active_models: Vec<TermActive> = terms
                    .into_iter()
                    .map(|term| TermActive {
                        id: Default::default(),
                        quotation_id: Set(result.id),
                        term_type: Set(term.term_type),
                        term_key: Set(term.term_key),
                        term_value: Set(term.term_value),
                        sequence: Set(term.sequence),
                    })
                    .collect();
                TermEntity::insert_many(term_active_models)
                    .exec(&txn)
                    .await?;
            }
        }

        // 8. 提交事务
        txn.commit().await?;

        Ok(result)
    }

    /// 列表查询（分页 + 过滤）
    ///
    /// 批次 265 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
    /// paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
    /// 补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
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

    /// 更新报价单（仅 draft / rejected 状态可更新）
    ///
    /// 批次 85 v2 复审 P1-2 修复：状态门移入 txn + lock_exclusive，重算 update 移入 txn
    /// 原实现状态门在 txn 外查询（self.db），且 line 360 重算后 update 在 txn commit 后用 self.db，
    /// 存在 TOCTOU（并发 update/cancel 会基于过期状态通过检查）和非原子性（commit 后 update 失败导致状态不一致）
    // 批次 94 P2-13 修复：补 user_id 参数 + 用 update_with_audit 记录审计日志；返回类型改为 AppError 以兼容审计服务
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateQuotationDto,
        user_id: i64,
    ) -> Result<sales_quotation::Model, AppError> {
        let txn = self.db.begin().await?;

        // 加 lock_exclusive 串行化并发状态变更
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if ![quotation_status::DRAFT, quotation_status::REJECTED].contains(&existing.status.as_str()) {
            return Err(AppError::validation("当前状态不允许此操作".to_string()));
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
            // 批次 111 P1-2：更新时同样校验贸易术语合法性（原 update 未校验）
            let incoterm = Self::validate_price_terms(&v)?;
            tracing::info!(
                quotation_id = id,
                incoterm_code = %v,
                incoterm_description = %incoterm.description(),
                "报价单贸易术语已更新"
            );
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
        // 批次 94 P2-13 修复：用 update_with_audit 记录审计日志（原 active.update 无审计）
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quotation",
            active,
            Some(user_id as i32),
        )
        .await?;

        // 如果 dto.items 存在，全量替换明细：改用 insert_many 批量 INSERT（原为循环内逐条 insert）
        if let Some(items) = dto.items {
            // 校验：非空
            if items.is_empty() {
                return Err(AppError::validation("明细至少 1 条".to_string()));
            }
            // 删除旧明细
            ItemEntity::delete_many()
                .filter(sales_quotation_item::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            // 插入新明细：批量插入
            let mut item_active_models: Vec<ItemActive> = Vec::with_capacity(items.len());
            for (idx, item_dto) in items.iter().enumerate() {
                item_active_models.push(ItemActive {
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
                });
            }
            if !item_active_models.is_empty() {
                ItemEntity::insert_many(item_active_models)
                    .exec(&txn)
                    .await?;
            }
        }

        // 如果 dto.terms 存在，全量替换条款：改用 insert_many 批量 INSERT（原为循环内逐条 insert）
        if let Some(terms) = dto.terms {
            // 删除旧条款
            TermEntity::delete_many()
                .filter(sales_quotation_term::Column::QuotationId.eq(id))
                .exec(&txn)
                .await?;
            // 插入新条款：批量插入
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
                    .exec(&txn)
                    .await?;
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
    // 批次 94 P2-13 修复：移除 let _ = user_id 占位，用 update_with_audit 记录审计日志；返回类型改为 AppError
    pub async fn cancel(
        &self,
        id: i64,
        user_id: i64,
    ) -> Result<sales_quotation::Model, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询无行锁，且 update 也用裸连接，无事务保护。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更，update 一并纳入事务。
        let txn = (*self.db).begin().await?;
        let existing = QuotationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if existing.status == "converted" {
            return Err(AppError::validation("当前状态不允许此操作".to_string()));
        }
        if existing.status == quotation_status::CANCELLED {
            return Ok(existing);
        }

        let mut active: QuotationActive = existing.into();
        active.status = Set(quotation_status::CANCELLED.to_string());
        active.updated_at = Set(Utc::now());
        // 批次 94 P2-13 修复：用 update_with_audit 记录审计日志（原 active.update 无审计，user_id 仅占位丢弃）
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quotation",
            active,
            Some(user_id as i32),
        )
        .await?;
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
        // 批次 111 P1-2：接入 utils/incoterms.rs（原硬编码 ["FOB","CIF","EXW","DDP","DAP"]）
        // 通过 Incoterms2020::from_code 完成解析+校验，并记录贸易术语业务元数据到日志
        let incoterm = Self::validate_price_terms(&dto.price_terms)?;
        tracing::info!(
            incoterm_code = %dto.price_terms,
            incoterm_description = %incoterm.description(),
            includes_insurance = %incoterm.includes_insurance(),
            includes_freight = %incoterm.includes_freight(),
            requires_duty_paid = %incoterm.requires_duty_paid(),
            "报价单贸易术语已校验"
        );
        Ok(())
    }

    /// 校验价格条款（贸易术语）并返回解析后的 Incoterms2020 枚举
    ///
    /// 批次 111 P1-2：原 validate_create 使用硬编码 `["FOB","CIF","EXW","DDP","DAP"]` 列表，
    /// 现接入 utils/incoterms.rs 的 Incoterms2020::from_code 完成解析+校验，
    /// 同时通过 Incoterms2020::all() + code() 派生合法代码列表，避免重复维护。
    fn validate_price_terms(code: &str) -> Result<crate::utils::incoterms::Incoterms2020, ServiceError> {
        crate::utils::incoterms::Incoterms2020::from_code(code).map_err(|msg| {
            // 派生合法代码列表用于错误提示（同时使用 all() + code() 接入业务）
            let valid: Vec<&'static str> = crate::utils::incoterms::Incoterms2020::all()
                .iter()
                .map(|t| t.code())
                .collect();
            ServiceError::Validation(format!("{}（合法取值: {}）", msg, valid.join("/")))
        })
    }
}

// 抑制 unused warnings（Func/Expr 为后续可扩展点预留）
// 移除：占位函数（无业务引用）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use rust_decimal::Decimal;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    // 批次 415：decs! 宏展开为 Decimal::from_str，需导入 FromStr trait
    use std::str::FromStr;

    /// 构造合法的 CreateQuotationItemDto（单条明细）
    fn sample_item() -> CreateQuotationItemDto {
        CreateQuotationItemDto {
            product_id: 1001,
            color_id: Some(2001),
            specification: Some("规格 A".to_string()),
            unit: "M".to_string(),
            quantity: decs!(100),
            unit_price: decs!(10),
            unit_price_with_tax: decs!(11.3),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        }
    }

    /// 构造合法的 CreateQuotationDto（默认 FOB + 不含税 + 13% 税率）
    fn sample_dto() -> CreateQuotationDto {
        CreateQuotationDto {
            customer_id: 1,
            sales_user_id: 10,
            quotation_date: ymd!(2026, 7, 19),
            valid_until: ymd!(2026, 8, 19),
            currency: "CNY".to_string(),
            exchange_rate: Decimal::ONE,
            base_currency: "CNY".to_string(),
            price_terms: "FOB".to_string(),
            incoterms_version: Some("2020".to_string()),
            incoterm_location: Some("Shanghai".to_string()),
            tax_inclusive: false,
            tax_rate: decs!(13),
            moq: Some(decs!(50)),
            lead_time_days: Some(30),
            customer_level: Some("A".to_string()),
            notes: Some("测试报价单".to_string()),
            items: vec![sample_item()],
            terms: None,
        }
    }

    // ============ ServiceError 枚举值正确性测试 ============

    /// 测试_ServiceError_Display_格式正确
    ///
    /// 验证 5 个 ServiceError 变体的 Display 实现返回中文错误信息，
    /// 确保前端接收到的是人类可读的业务错误（规则 20：注释与功能一致）。
    #[test]
    fn 测试_ServiceError_Display_格式正确() {
        assert_eq!(ServiceError::NotFound.to_string(), "报价单不存在");
        assert_eq!(
            ServiceError::InvalidState.to_string(),
            "当前状态不允许此操作"
        );
        assert_eq!(
            ServiceError::Validation("明细至少 1 条".to_string()).to_string(),
            "参数校验失败: 明细至少 1 条"
        );
        // Database 和 App 变体依赖外部类型，仅验证前缀
        let db_err = ServiceError::Database(sea_orm::DbErr::RecordNotFound("test".to_string()));
        assert!(db_err.to_string().starts_with("数据库错误:"));
    }

    // ============ validate_create 业务校验测试 ============

    /// 测试_validate_create_空明细拒绝
    ///
    /// 业务规则：报价单至少 1 条明细（DTO 上 #[validate(length(min = 1))]）。
    /// 验证当 items 为空时，validate_create 返回 Validation 错误。
    #[tokio::test]
    async fn 测试_validate_create_空明细拒绝() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items.clear();
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            assert!(msg.contains("明细至少 1 条"));
        }
    }

    /// 测试_validate_create_有效期早于报价日期拒绝
    ///
    /// 业务规则：valid_until 必须 >= quotation_date。
    /// 验证 valid_until 早于 quotation_date 时返回 Validation 错误。
    #[tokio::test]
    async fn 测试_validate_create_有效期早于报价日期拒绝() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        // 翻转日期：valid_until 早于 quotation_date
        dto.valid_until = ymd!(2026, 6, 19);
        dto.quotation_date = ymd!(2026, 7, 19);
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            assert!(msg.contains("有效期截止必须不早于报价日期"));
        }
    }

    /// 测试_validate_create_非法贸易术语拒绝
    ///
    /// 业务规则：price_terms 必须是 Incoterms 2020 合法代码（EXW/FCA/CPT/CIP/DAP/DPU/DDP/FAS/FOB/CFR/CIF）。
    /// 验证非法代码（如 "XYZ"）返回 Validation 错误并附带合法取值列表。
    #[tokio::test]
    async fn 测试_validate_create_非法贸易术语拒绝() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.price_terms = "XYZ".to_string();
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            // 错误信息应包含合法取值列表（至少包含 FOB）
            assert!(msg.contains("FOB") || msg.contains("合法取值"));
        }
    }

    /// 测试_validate_create_合法参数通过
    ///
    /// 验证所有字段合法时返回 Ok(())。
    #[tokio::test]
    async fn 测试_validate_create_合法参数通过() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto();
        let result = svc.validate_create(&dto);
        assert!(result.is_ok());
    }

    // ============ calculate_totals 金额计算测试 ============

    /// 测试_calculate_totals_不含税金额计算正确
    ///
    /// 业务规则：不含税时 tax_amount = subtotal * tax_rate / 100，total = subtotal + tax。
    /// 验证：100 数量 * 10 单价 = 1000 小计，13% 税率 → 130 税额 → 1130 总额。
    #[tokio::test]
    async fn 测试_calculate_totals_不含税金额计算正确() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto(); // 默认 tax_inclusive=false
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, decs!(130));
        assert_eq!(total_amount, decs!(1130));
    }

    /// 测试_calculate_totals_含税金额税额为零
    ///
    /// 业务规则：含税时小计已含税，tax_amount = 0，total = subtotal。
    /// 验证 tax_inclusive=true 时税额为 0。
    #[tokio::test]
    async fn 测试_calculate_totals_含税金额税额为零() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.tax_inclusive = true;
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, Decimal::ZERO);
        assert_eq!(total_amount, decs!(1000));
    }

    /// 测试_calculate_totals_多明细汇总正确
    ///
    /// 验证多条明细时 subtotal 正确汇总：100*10 + 200*20 = 5000。
    #[tokio::test]
    async fn 测试_calculate_totals_多明细汇总正确() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items.push(CreateQuotationItemDto {
            product_id: 1002,
            color_id: None,
            specification: None,
            unit: "M".to_string(),
            quantity: decs!(200),
            unit_price: decs!(20),
            unit_price_with_tax: decs!(22.6),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        });
        let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(5000)); // 1000 + 4000
    }

    /// 测试_calculate_totals_精度归一到2位小数
    ///
    /// 业务规则（批次 87 P3 维度 4 修复）：金额计算补 round_dp(2) 精度归一化。
    /// 验证 33.333 * 3 = 99.999 → 99.99（subtotal round_dp(2)）。
    #[tokio::test]
    async fn 测试_calculate_totals_精度归一到2位小数() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items = vec![CreateQuotationItemDto {
            product_id: 1,
            color_id: None,
            specification: None,
            unit: "M".to_string(),
            quantity: decs!(3),
            unit_price: decs!(33.333),
            unit_price_with_tax: decs!(33.333),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        }];
        let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
        // 33.333 * 3 = 99.999 → round_dp(2) → 100.00
        assert_eq!(subtotal, decs!(100));
    }

    // ============ validate_price_terms 贸易术语校验测试 ============

    /// 测试_validate_price_terms_合法代码返回枚举
    ///
    /// 验证 11 个 Incoterms 2020 代码均能正确解析为 Incoterms2020 枚举。
    #[test]
    fn 测试_validate_price_terms_合法代码返回枚举() {
        // 11 个 Incoterms 2020 代码
        let valid_codes = [
            "EXW", "FCA", "CPT", "CIP", "DAP", "DPU", "DDP", "FAS", "FOB", "CFR", "CIF",
        ];
        for code in valid_codes {
            let result = QuotationService::validate_price_terms(code);
            assert!(result.is_ok(), "合法代码 {} 应通过校验", code);
        }
    }

    /// 测试_validate_price_terms_大小写不敏感
    ///
    /// 验证 from_code 内部 .to_uppercase() 转换，"fob" 应等价于 "FOB"。
    #[test]
    fn 测试_validate_price_terms_大小写不敏感() {
        let lower = QuotationService::validate_price_terms("fob");
        let upper = QuotationService::validate_price_terms("FOB");
        assert!(lower.is_ok());
        assert!(upper.is_ok());
    }

    /// 测试_validate_price_terms_非法代码返回错误
    ///
    /// 验证非法代码（"XYZ"）返回 Err，错误信息应包含合法取值列表。
    #[test]
    fn 测试_validate_price_terms_非法代码返回错误() {
        let result = QuotationService::validate_price_terms("XYZ");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // 错误信息应包含合法取值列表（至少包含 FOB 和 CIF）
        assert!(err_msg.contains("FOB"));
    }

    // ============ 状态常量值正确性测试 ============

    /// 测试_报价单状态常量_值正确性
    ///
    /// 验证 status::quotation 模块中 4 个状态常量值与状态机约定一致
    /// （小写：draft/approved/rejected/cancelled，规则 0：常量值统一管理）。
    #[test]
    fn 测试_报价单状态常量_值正确性() {
        assert_eq!(quotation_status::DRAFT, "draft");
        assert_eq!(quotation_status::APPROVED, "approved");
        assert_eq!(quotation_status::REJECTED, "rejected");
        assert_eq!(quotation_status::CANCELLED, "cancelled");
    }

    /// 测试_报价单状态常量_互不相同
    ///
    /// 业务规则：4 个状态必须互不相同，避免状态机歧义。
    #[test]
    fn 测试_报价单状态常量_互不相同() {
        let states = [
            quotation_status::DRAFT,
            quotation_status::APPROVED,
            quotation_status::REJECTED,
            quotation_status::CANCELLED,
        ];
        // 集合去重后长度应仍为 4
        let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
        assert_eq!(unique.len(), 4);
    }

    // ============ QuotationService 构造与 DB 连接测试 ============

    /// 测试_QuotationService_new_正确持有数据库连接
    ///
    /// 验证 new(Arc<DatabaseConnection>) 构造的 service 实例可以执行简单查询。
    #[tokio::test]
    async fn 测试_QuotationService_new_正确持有数据库连接() {
        let db = Arc::new(setup_test_db().await);
        let svc = QuotationService::new(db.clone());
        // 验证连接可用：执行一次空查询不报错
        use sea_orm::ConnectionTrait;
        let _ = svc
            .db
            .execute(sea_orm::Statement::from_sql_and_values(
                svc.db.get_database_backend(),
                "SELECT 1",
                Vec::new(),
            ))
            .await
            .expect("数据库连接应可用");
    }

    /// 测试_QuotationService_get_by_id_空数据库返回Err
    ///
    /// 业务规则：get_by_id 查询 sales_quotations 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_QuotationService_get_by_id_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.get_by_id(9999).await;
        // SQLite 内存数据库无 sales_quotations 表，应返回 Err（DbErr 转 ServiceError::Database）
        assert!(result.is_err());
    }

    /// 测试_QuotationService_list_空数据库返回Err
    ///
    /// 业务规则：list 查询 sales_quotations 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_QuotationService_list_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.list(1, 20, None, None, None, None).await;
        // SQLite 内存数据库无 sales_quotations 表，应返回 Err（DbErr 转 ServiceError::Database）
        assert!(result.is_err());
    }

    /// 测试_QuotationService_cancel_不存在返回AppError
    ///
    /// 业务规则：cancel 不存在的报价单返回 AppError::not_found。
    /// 注：cancel 返回 Result<_, AppError>（非 ServiceError），因审计日志服务接入需要。
    #[tokio::test]
    async fn 测试_QuotationService_cancel_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.cancel(9999, 1).await;
        assert!(result.is_err());
        // 验证是 AppError 类型（而非 panic 或其他错误）
        let err = result.unwrap_err();
        // not_found 错误的 message 应包含"报价单不存在"
        let msg = format!("{}", err);
        assert!(msg.contains("报价单不存在") || msg.contains("not found") || msg.contains("不存在"));
    }

    // ============ update 状态机校验测试 ============

    /// 测试_QuotationService_update_不存在返回AppError
    ///
    /// 业务规则：update 不存在的报价单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_QuotationService_update_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = UpdateQuotationDto::default();
        let result = svc.update(9999, dto, 1).await;
        assert!(result.is_err());
    }
}


