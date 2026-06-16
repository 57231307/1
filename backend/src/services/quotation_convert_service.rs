//! 销售报价单 → 销售订单 转换服务
//!
//! 业务功能：
//! - 校验报价单状态（仅 approved 可转）
//! - 事务化复制明细到 sales_order_items
//! - 创建 sales_orders 草稿
//! - 更新报价单状态为 converted，记录 converted_sales_order_id
//!
//! Week 2 任务 8 - 销售报价单模块
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 8

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::sync::Arc;

use crate::models::sales_order::{self, ActiveModel as OrderActive, Entity as OrderEntity};
use crate::models::sales_order_item::ActiveModel as OrderItemActive;
use crate::models::sales_quotation::{ActiveModel as QuotationActive, Entity as QuotationEntity};
use crate::models::sales_quotation_item::{self, Entity as QuotationItemEntity};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;

/// 转订单服务
pub struct QuotationConvertService {
    db: Arc<DatabaseConnection>,
}

impl QuotationConvertService {
    /// 从数据库连接直接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 将已审批的报价单转换为销售订单草稿
    pub async fn convert(
        &self,
        quotation_id: i64,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let txn = self.db.begin().await?;

        // 1. 校验报价单存在且状态为 approved
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if quotation.status != "approved" {
            return Err(AppError::business(format!(
                "报价单状态不允许转订单：{}（仅 approved 状态可转换）",
                quotation.status
            )));
        }

        // 2. 校验有效期
        if quotation.valid_until < Utc::now().date_naive() {
            // 标记为 expired
            let mut active: QuotationActive = quotation.clone().into();
            active.status = Set("expired".to_string());
            active.updated_at = Set(Utc::now());
            active.update(&txn).await?;
            return Err(AppError::business("报价单已过期，无法转订单"));
        }

        // 3. 加载报价单明细
        let items = QuotationItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(quotation_id))
            .all(&txn)
            .await?;

        // 4. 生成订单号
        let order_no = Self::generate_order_no_static(&txn).await?;

        // 5. 创建销售订单草稿
        let now = Utc::now();
        let new_order = OrderActive {
            id: Default::default(),
            order_no: Set(order_no),
            customer_id: Set(quotation.customer_id as i32),
            opportunity_id: Set(None),
            order_date: Set(now),
            required_date: Set(Utc::now() + chrono::Duration::days(30)),
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(quotation.subtotal),
            tax_amount: Set(quotation.tax_amount),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(quotation.total_amount),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(quotation.total_amount),
            shipping_address: Set(None),
            billing_address: Set(None),
            notes: Set(Some(format!(
                "[源自报价单 {}]\n{}",
                quotation.quotation_no,
                quotation.notes.clone().unwrap_or_default()
            ))),
            created_by: Set(Some(user_id)),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let order = new_order.insert(&txn).await?;

        // 6. 复制明细
        for item in &items {
            // 计算明细金额
            let subtotal = item.amount;
            let tax_amount = item.amount_with_tax - item.amount;
            let new_item = OrderItemActive {
                id: Default::default(),
                order_id: Set(order.id),
                product_id: Set(item.product_id as i32),
                quantity: Set(item.quantity),
                unit_price: Set(item.unit_price),
                discount_percent: Set(item.discount_rate.unwrap_or(Decimal::ZERO)),
                tax_percent: Set(quotation.tax_rate),
                subtotal: Set(subtotal),
                tax_amount: Set(tax_amount),
                discount_amount: Set(item.discount_amount.unwrap_or(Decimal::ZERO)),
                total_amount: Set(item.amount_with_tax),
                shipped_quantity: Set(Decimal::ZERO),
                notes: Set(item.notes.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                // 颜色字段：拼接所有色号
                color_no: Set(Self::compose_color_no(item)),
                color_name: Set(None),
                pantone_code: Set(item.pantone_code.clone()),
                grade_required: Set(None),
                quantity_meters: Set(item.quantity),
                quantity_kg: Set(Decimal::ZERO),
                gram_weight: Set(None),
                width: Set(None),
                batch_requirement: Set(None),
                dye_lot_requirement: Set(None),
                base_price: Set(Some(item.unit_price)),
                color_extra_cost: Set(Decimal::ZERO),
                grade_price_diff: Set(Decimal::ZERO),
                final_price: Set(Some(item.unit_price_with_tax)),
                shipped_quantity_meters: Set(Decimal::ZERO),
                shipped_quantity_kg: Set(Decimal::ZERO),
                paper_tube_weight: Set(None),
                is_net_weight: Set(None),
            };
            new_item.insert(&txn).await?;
        }

        // 7. 更新报价单状态为 converted
        let mut active: QuotationActive = quotation.into();
        active.status = Set("converted".to_string());
        active.converted_sales_order_id = Set(Some(order.id as i64));
        active.converted_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(&txn).await?;

        txn.commit().await?;

        Ok(order)
    }

    /// 拼接色号（color_code / pantone_code / cncs_code）
    fn compose_color_no(item: &sales_quotation_item::Model) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(c) = &item.color_code {
            if !c.is_empty() {
                parts.push(c.clone());
            }
        }
        if let Some(p) = &item.pantone_code {
            if !p.is_empty() {
                parts.push(format!("PANTONE:{}", p));
            }
        }
        if let Some(c) = &item.cncs_code {
            if !c.is_empty() {
                parts.push(format!("CNCS:{}", c));
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join("|")
        }
    }

    /// 生成销售订单号：SO + YYYYMMDD + 4 位当日序号
    async fn generate_order_no_static<C>(
        txn: &C,
    ) -> Result<String, AppError>
    where
        C: sea_orm::ConnectionTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let pattern = format!("SO{}%", today);
        let count = OrderEntity::find()
            .filter(sales_order::Column::OrderNo.like(pattern))
            .count(txn)
            .await?;
        Ok(format!("SO{}{:04}", today, count + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_color_no_empty() {
        let item = sales_quotation_item::Model {
            id: 1,
            quotation_id: 1,
            product_id: 1,
            color_id: None,
            color_code: None,
            pantone_code: None,
            cncs_code: None,
            specification: None,
            unit: "米".to_string(),
            quantity: Decimal::from(10),
            unit_price: Decimal::from(10),
            unit_price_with_tax: Decimal::from(11),
            amount: Decimal::from(100),
            amount_with_tax: Decimal::from(113),
            tier_pricing: None,
            discount_rate: None,
            discount_amount: None,
            notes: None,
            sequence: 0,
        };
        assert_eq!(QuotationConvertService::compose_color_no(&item), "-");
    }

    #[test]
    fn test_compose_color_no_with_pantone() {
        let item = sales_quotation_item::Model {
            id: 1,
            quotation_id: 1,
            product_id: 1,
            color_id: None,
            color_code: Some("RED-01".to_string()),
            pantone_code: Some("18-1664".to_string()),
            cncs_code: None,
            specification: None,
            unit: "米".to_string(),
            quantity: Decimal::from(10),
            unit_price: Decimal::from(10),
            unit_price_with_tax: Decimal::from(11),
            amount: Decimal::from(100),
            amount_with_tax: Decimal::from(113),
            tier_pricing: None,
            discount_rate: None,
            discount_amount: None,
            notes: None,
            sequence: 0,
        };
        let s = QuotationConvertService::compose_color_no(&item);
        assert!(s.contains("RED-01"));
        assert!(s.contains("PANTONE:18-1664"));
    }
}
