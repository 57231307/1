//! 出口退税（免抵退）核算 Service
//!
//! V15 P1 batch-08 缺陷 14：出口退税（免抵退）核算
//! 依据：财税[2012]39号 出口货物劳务增值税和消费税政策
//!
//! 真实业务：
//! - 登记出口报关单/外汇核销单/增值税发票
//! - 校验"单证齐全"（报关单+核销单+发票）
//! - 计算免抵退税额（免抵退办法）
//! - 生成退税申报表

use crate::models::export_customs_declaration::{
    self, Entity as CustomsEntity, Model as CustomsModel,
};
use crate::models::export_refund_declaration::{
    self, ActiveModel as RefundActiveModel, Entity as RefundEntity, Model as RefundModel,
};
use crate::models::foreign_exchange_verification::{self, Entity as FxEntity};
use crate::utils::data_scope::{DataScopeContext, apply_data_scope};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;

/// 创建出口报关单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCustomsDeclarationRequest {
    pub declaration_no: String,
    pub sales_order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub product_id: Option<i32>,
    pub export_date: chrono::NaiveDate,
    pub destination_country: Option<String>,
    pub currency_code: Option<String>,
    pub total_amount: Decimal,
    pub exchange_rate: Decimal,
    pub customs_code: Option<String>,
    pub remarks: Option<String>,
}

/// 创建外汇核销单请求
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code, reason = "预留：创建外汇核销单请求，待接入")]
pub struct CreateFxVerificationRequest {
    pub verification_no: String,
    pub customs_declaration_id: Option<i32>,
    pub sales_order_id: Option<i32>,
    pub verification_date: chrono::NaiveDate,
    pub foreign_currency_amount: Decimal,
    pub rmb_amount: Decimal,
    pub exchange_rate: Decimal,
    pub bank_code: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 免抵退税额计算参数
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefundCalculationInput {
    pub export_sales_amount: Decimal,
    pub refund_rate: Decimal,
    pub input_vat_amount: Decimal,
    pub carryforward_from_prev: Decimal,
}

/// 免抵退税额计算结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct RefundCalculationResult {
    /// 免抵退税额 = 出口销售额 × 退税率
    pub refundable_vat_amount: Decimal,
    /// 应退税额 = min(免抵退税额, 期初留抵 + 当期进项)
    pub actual_refund_amount: Decimal,
    /// 免抵税额 = 免抵退税额 - 应退税额
    pub exempt_vat_amount: Decimal,
    /// 结转下期留抵 = max(0, 期初留抵 + 当期进项 - 免抵退税额)
    pub carryforward_amount: Decimal,
}

pub struct ExportRefundService {
    db: Arc<DatabaseConnection>,
}

impl ExportRefundService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 单证核验状态唯一判定 token（models/export_customs_declaration.rs:39、
    /// models/foreign_exchange_verification.rs:36 词表中的 verified 项）。
    /// 核验端点（verify_documents_completeness）、documents_complete
    /// （orders_documents_complete）与退税申报金额聚合
    /// （verified_export_sales_amount）三者共用此常量，禁止任何一处另写第二套状态规则。
    const VERIFIED_STATUS: &'static str = "verified";

    /// 退税申报基数聚合：只计入 status=verified 报关单的人民币口径金额
    /// （total_amount × exchange_rate 逐单求和）。pending/cancelled 等非已核验
    /// 状态的行不进申报基数——与 [`Self::orders_documents_complete`] 的核验口径同源。
    pub fn verified_export_sales_amount(customs_list: &[CustomsModel]) -> Decimal {
        customs_list
            .iter()
            .filter(|c| c.status == Self::VERIFIED_STATUS)
            .map(|c| c.total_amount * c.exchange_rate)
            .sum()
    }

    /// 创建出口报关单
    pub async fn create_customs_declaration(
        &self,
        req: CreateCustomsDeclarationRequest,
        user_id: i32,
    ) -> Result<CustomsModel, AppError> {
        if req.total_amount < Decimal::ZERO {
            return Err(AppError::bad_request("报关金额不能为负"));
        }
        if req.exchange_rate <= Decimal::ZERO {
            return Err(AppError::bad_request("汇率必须大于 0"));
        }

        // 校验报关单号唯一性
        if CustomsEntity::find()
            .filter(export_customs_declaration::Column::DeclarationNo.eq(&req.declaration_no))
            .one(&*self.db)
            .await?
            .is_some()
        {
            return Err(AppError::business(format!(
                "报关单号 {} 已存在",
                req.declaration_no
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = export_customs_declaration::ActiveModel {
            declaration_no: Set(req.declaration_no),
            sales_order_id: Set(req.sales_order_id),
            customer_id: Set(req.customer_id),
            product_id: Set(req.product_id),
            export_date: Set(req.export_date),
            destination_country: Set(req.destination_country),
            currency_code: Set(req.currency_code),
            total_amount: Set(req.total_amount),
            exchange_rate: Set(req.exchange_rate),
            customs_code: Set(req.customs_code),
            status: Set("pending".to_string()),
            remarks: Set(req.remarks),
            // 建单人取服务端会话（由 handler 传入），请求体不承载身份
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("出口报关单创建失败: {}", e)))?;
        Ok(result)
    }

    /// 校验"单证齐全"（报关单+核销单）（业务规则：免抵退税申报要求报关单与核销单齐全）
    ///
    /// 与 `generate_refund_declaration` 的 `documents_complete` 共用同一判定源
    /// [`Self::orders_documents_complete`]（verified 报关单 + verified 核销单），
    /// 不在两处各写一套规则。
    pub async fn verify_documents_completeness(
        &self,
        sales_order_id: i32,
    ) -> Result<bool, AppError> {
        self.orders_documents_complete(&[sales_order_id]).await
    }

    /// 单证齐全统一判定源：给定销售订单集合，要求每个订单同时存在
    /// status=verified 的出口报关单与 status=verified 的外汇核销单。
    /// 空集合返回 false（无可判定对象不得虚报"齐全"）。
    async fn orders_documents_complete(&self, sales_order_ids: &[i32]) -> Result<bool, AppError> {
        if sales_order_ids.is_empty() {
            return Ok(false);
        }
        use std::collections::HashSet;

        let customs_verified: HashSet<i32> = CustomsEntity::find()
            .filter(
                export_customs_declaration::Column::SalesOrderId
                    .is_in(sales_order_ids.iter().copied()),
            )
            .filter(export_customs_declaration::Column::Status.eq(Self::VERIFIED_STATUS))
            .select_only()
            .column(export_customs_declaration::Column::SalesOrderId)
            .into_tuple::<Option<i32>>()
            .all(&*self.db)
            .await?
            .into_iter()
            .flatten()
            .collect();

        let fx_verified: HashSet<i32> = FxEntity::find()
            .filter(
                foreign_exchange_verification::Column::SalesOrderId
                    .is_in(sales_order_ids.iter().copied()),
            )
            .filter(foreign_exchange_verification::Column::Status.eq(Self::VERIFIED_STATUS))
            .select_only()
            .column(foreign_exchange_verification::Column::SalesOrderId)
            .into_tuple::<Option<i32>>()
            .all(&*self.db)
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(sales_order_ids
            .iter()
            .all(|id| customs_verified.contains(id) && fx_verified.contains(id)))
    }

    /// 计算免抵退税额（纯函数）
    /// 业务规则（财税[2012]39号 免抵退办法）：免抵退税额 = 出口销售额 × 退税率；应退税额 = min(免抵退税额, 期初留抵 + 当期进项)；免抵税额 = 免抵退税额 - 应退税额；结转下期 = max(0, 期初留抵 + 当期进项 - 免抵退税额)
    pub fn calculate_exempt_credit_refund(
        input: &RefundCalculationInput,
    ) -> RefundCalculationResult {
        // 免抵退税额 = 出口销售额 × 退税率
        let refundable_vat_amount = input.export_sales_amount * input.refund_rate;

        // 当期可抵扣进项税额 = 期初留抵 + 当期进项
        let available_input_vat = input.carryforward_from_prev + input.input_vat_amount;

        // 应退税额 = min(免抵退税额, 当期可抵扣进项税额)
        let actual_refund_amount = refundable_vat_amount.min(available_input_vat);

        // 免抵税额 = 免抵退税额 - 应退税额
        let exempt_vat_amount = refundable_vat_amount - actual_refund_amount;

        // 结转下期 = max(0, 当期可抵扣进项税额 - 免抵退税额)
        let carryforward_amount = if available_input_vat > refundable_vat_amount {
            available_input_vat - refundable_vat_amount
        } else {
            Decimal::ZERO
        };

        RefundCalculationResult {
            refundable_vat_amount,
            actual_refund_amount,
            exempt_vat_amount,
            carryforward_amount,
        }
    }

    /// 生成出口退税申报表
    pub async fn generate_refund_declaration(
        &self,
        period_year: i32,
        period_month: i32,
        refund_rate: Decimal,
        input_vat_amount: Decimal,
        carryforward_from_prev: Decimal,
        created_by: Option<i32>,
    ) -> Result<RefundModel, AppError> {
        // 汇总当期出口销售额：真实所属期间半开区间 [本月月初, 次月月初)。
        // 修复前只有 gte 下界无上界，"当期申报"会把该月之后所有报关数据聚合进来。
        // 期间非法拒绝文案仅回显用户自己提交的输入，走 business_displayable 外显
        if !(1..=12).contains(&period_month) {
            return Err(AppError::business_displayable(format!(
                "退税申报期间月份非法: {period_month}（须为 1-12）"
            )));
        }
        let period_start = chrono::NaiveDate::from_ymd_opt(period_year, period_month as u32, 1)
            .ok_or_else(|| {
                AppError::business_displayable(format!(
                    "退税申报期间非法: {period_year}-{period_month:02}"
                ))
            })?;
        let (next_year, next_month) = if period_month == 12 {
            (period_year + 1, 1)
        } else {
            (period_year, period_month + 1)
        };
        let period_end = chrono::NaiveDate::from_ymd_opt(next_year, next_month as u32, 1)
            .ok_or_else(|| {
                AppError::business_displayable(format!(
                    "退税申报期间次月月初非法: {next_year}-{next_month:02}"
                ))
            })?;

        let customs_list = CustomsEntity::find()
            .filter(export_customs_declaration::Column::ExportDate.gte(period_start))
            .filter(export_customs_declaration::Column::ExportDate.lt(period_end))
            .all(&*self.db)
            .await?;

        // 申报基数只计入 status=verified 的报关单（核验口径同源，见
        // [`Self::verified_export_sales_amount`]）；customs_list 保留全期间数据仅用于
        // documents_complete 的当期存在性/归属判定，未核验行不进金额基数。
        let export_sales_amount = Self::verified_export_sales_amount(&customs_list);

        let calc_input = RefundCalculationInput {
            export_sales_amount,
            refund_rate,
            input_vat_amount,
            carryforward_from_prev,
        };
        let calc = Self::calculate_exempt_credit_refund(&calc_input);

        let declaration_no = format!(
            "ERD-{:04}{:02}-{:05}",
            period_year,
            period_month,
            chrono::Utc::now().timestamp() % 100000
        );

        // documents_complete 与 verify_documents_completeness 统一判定源（修复前
        // `!customs_list.is_empty()` 是自造的第二套规则）：本期存在报关数据、且
        // 每条报关单都能归属到销售订单、且每个相关订单都有 verified 报关单 +
        // verified 核销单；无法归属（sales_order_id 为空）的报关单按核验口径
        // 判不齐，不虚报齐全。
        let period_order_ids: Vec<i32> = {
            let uniq: std::collections::HashSet<i32> = customs_list
                .iter()
                .filter_map(|c| c.sales_order_id)
                .collect();
            uniq.into_iter().collect()
        };
        let documents_complete = !customs_list.is_empty()
            && customs_list.iter().all(|c| c.sales_order_id.is_some())
            && self.orders_documents_complete(&period_order_ids).await?;

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = RefundActiveModel {
            declaration_no: Set(declaration_no),
            period_year: Set(period_year),
            period_month: Set(period_month),
            declaration_date: Set(now.date_naive()),
            export_sales_amount: Set(export_sales_amount),
            refundable_vat_amount: Set(calc.refundable_vat_amount),
            exempt_vat_amount: Set(calc.exempt_vat_amount),
            credit_vat_amount: Set(input_vat_amount),
            actual_refund_amount: Set(calc.actual_refund_amount),
            carryforward_amount: Set(calc.carryforward_amount),
            refund_rate: Set(refund_rate),
            documents_complete: Set(documents_complete),
            status: Set("draft".to_string()),
            remarks: Set(None),
            created_by: Set(created_by),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("出口退税申报表创建失败: {}", e)))?;
        Ok(result)
    }

    /// 查询出口退税申报表
    ///
    /// 行级数据权限在查询构建阶段下推：申报表无 department_id 归属列，`created_by`
    /// 是唯一归属列，故 owner 列与 dept 列同传 `created_by`（Dept 范围据此按可见
    /// 部门成员集合过滤，Self 仅本人，All 不加过滤）。过滤在 `.all()` 取数前生效，
    /// 返回列表本身即为范围内结果，调用方不得再对返回集做后置过滤或另发一次未加
    /// 范围的 count——归属人为 NULL 的历史行一律不进可见集，不放行成"无主即可读"。
    pub async fn list_refund_declarations(
        &self,
        period_year: Option<i32>,
        period_month: Option<i32>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<Vec<RefundModel>, AppError> {
        let mut query = RefundEntity::find();
        if let Some(y) = period_year {
            query = query.filter(export_refund_declaration::Column::PeriodYear.eq(y));
        }
        if let Some(m) = period_month {
            query = query.filter(export_refund_declaration::Column::PeriodMonth.eq(m));
        }
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                export_refund_declaration::Column::CreatedBy,
                export_refund_declaration::Column::CreatedBy,
            );
        }
        let list = query.all(&*self.db).await?;
        Ok(list)
    }
}
