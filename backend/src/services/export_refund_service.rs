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
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
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
    pub created_by: Option<i32>,
}

/// 创建外汇核销单请求
#[derive(Debug, Clone, Deserialize)]
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

    /// 创建出口报关单
    pub async fn create_customs_declaration(
        &self,
        req: CreateCustomsDeclarationRequest,
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
            created_by: Set(req.created_by),
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
    pub async fn verify_documents_completeness(
        &self,
        sales_order_id: i32,
    ) -> Result<bool, AppError> {
        let customs_count = CustomsEntity::find()
            .filter(export_customs_declaration::Column::SalesOrderId.eq(sales_order_id))
            .filter(export_customs_declaration::Column::Status.eq("verified"))
            .count(&*self.db)
            .await?;

        let fx_count = FxEntity::find()
            .filter(foreign_exchange_verification::Column::SalesOrderId.eq(sales_order_id))
            .filter(foreign_exchange_verification::Column::Status.eq("verified"))
            .count(&*self.db)
            .await?;

        Ok(customs_count > 0 && fx_count > 0)
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
        // 汇总当期出口销售额
        let customs_list = CustomsEntity::find()
            .filter(
                export_customs_declaration::Column::ExportDate.gte(
                    chrono::NaiveDate::from_ymd_opt(period_year, period_month as u32, 1)
                        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()),
                ),
            )
            .all(&*self.db)
            .await?;

        let export_sales_amount: Decimal = customs_list
            .iter()
            .map(|c| c.total_amount * c.exchange_rate)
            .sum();

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
            documents_complete: Set(!customs_list.is_empty()),
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
    pub async fn list_refund_declarations(
        &self,
        period_year: Option<i32>,
        period_month: Option<i32>,
    ) -> Result<Vec<RefundModel>, AppError> {
        let mut query = RefundEntity::find();
        if let Some(y) = period_year {
            query = query.filter(export_refund_declaration::Column::PeriodYear.eq(y));
        }
        if let Some(m) = period_month {
            query = query.filter(export_refund_declaration::Column::PeriodMonth.eq(m));
        }
        let list = query.all(&*self.db).await?;
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_calculate_exempt_credit_refund_normal() {
        let input = RefundCalculationInput {
            export_sales_amount: Decimal::new(1000000, 0), // 100万
            refund_rate: Decimal::new(13, 2),              // 13%
            input_vat_amount: Decimal::new(80000, 0),      // 8万
            carryforward_from_prev: Decimal::ZERO,
        };
        let result = ExportRefundService::calculate_exempt_credit_refund(&input);
        // 免抵退税额 = 100万 × 13% = 13万
        assert_eq!(result.refundable_vat_amount, Decimal::new(130000, 0));
        // 应退税额 = min(13万, 8万) = 8万
        assert_eq!(result.actual_refund_amount, Decimal::new(80000, 0));
        // 免抵税额 = 13万 - 8万 = 5万
        assert_eq!(result.exempt_vat_amount, Decimal::new(50000, 0));
        // 结转下期 = 0
        assert_eq!(result.carryforward_amount, Decimal::ZERO);
    }

    #[test]
    fn test_calculate_exempt_credit_refund_with_carryforward() {
        let input = RefundCalculationInput {
            export_sales_amount: Decimal::new(100000, 0),   // 10万
            refund_rate: Decimal::new(13, 2),               // 13%
            input_vat_amount: Decimal::new(50000, 0),       // 5万
            carryforward_from_prev: Decimal::new(30000, 0), // 3万
        };
        let result = ExportRefundService::calculate_exempt_credit_refund(&input);
        // 免抵退税额 = 10万 × 13% = 1.3万
        assert_eq!(result.refundable_vat_amount, Decimal::new(13000, 0));
        // 当期可抵扣 = 3万 + 5万 = 8万
        // 应退税额 = min(1.3万, 8万) = 1.3万
        assert_eq!(result.actual_refund_amount, Decimal::new(13000, 0));
        // 免抵税额 = 1.3万 - 1.3万 = 0
        assert_eq!(result.exempt_vat_amount, Decimal::ZERO);
        // 结转下期 = 8万 - 1.3万 = 6.7万
        assert_eq!(result.carryforward_amount, Decimal::new(67000, 0));
    }
}
