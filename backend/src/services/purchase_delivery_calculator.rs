//! 采购交期计算服务
//!
//! 根据供应商历史交货数据自动计算建议交货日期
#![allow(dead_code)]

use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, Statement, FromQueryResult, ConnectionTrait};
use std::sync::Arc;

use crate::utils::error::AppError;

/// 交期计算请求
#[derive(Debug, Clone)]
pub struct DeliveryCalculationRequest {
    pub supplier_id: i32,
    pub order_date: NaiveDate,
    pub items: Vec<OrderItemInfo>,
}

/// 订单物料信息
#[derive(Debug, Clone)]
pub struct OrderItemInfo {
    pub product_id: i32,
    pub quantity: Decimal,
}

/// 交期计算结果
#[derive(Debug, Clone)]
pub struct DeliveryCalculationResult {
    /// 建议交货日期
    pub suggested_date: NaiveDate,
    /// 供应商平均交货周期（天）
    pub avg_lead_time_days: i32,
    /// 最大生产周期（天）
    pub max_production_days: i32,
    /// 计算依据说明
    pub calculation_basis: String,
    /// 历史订单数量
    pub historical_orders: i64,
}

#[derive(Debug, FromQueryResult)]
struct AvgLeadTimeResult {
    avg_days: Option<i32>,
    order_count: Option<i64>,
}

/// 采购交期计算服务
pub struct PurchaseDeliveryCalculator {
    db: Arc<DatabaseConnection>,
}

impl PurchaseDeliveryCalculator {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 计算建议交货日期
    pub async fn calculate_delivery_date(
        &self,
        req: &DeliveryCalculationRequest,
    ) -> Result<DeliveryCalculationResult, AppError> {
        // 1. 获取供应商平均交货周期
        let (avg_lead_time, historical_orders) = self
            .get_supplier_avg_lead_time(req.supplier_id)
            .await?;

        // 2. 获取最大生产周期（简化实现，基于物料复杂度）
        let max_production_days = self
            .estimate_production_days(&req.items)
            .await?;

        // 3. 计算总准备时间
        let total_days = avg_lead_time + max_production_days;

        // 4. 考虑节假日，计算实际交货日期
        let suggested_date = self.add_business_days(req.order_date, total_days);

        let calculation_basis = if historical_orders > 0 {
            format!(
                "基于供应商历史{}笔订单数据，平均交货周期{}天",
                historical_orders, avg_lead_time
            )
        } else {
            format!(
                "供应商无历史交货数据，使用默认交货周期{}天",
                avg_lead_time
            )
        };

        Ok(DeliveryCalculationResult {
            suggested_date,
            avg_lead_time_days: avg_lead_time,
            max_production_days,
            calculation_basis,
            historical_orders,
        })
    }

    /// 获取供应商平均交货周期
    async fn get_supplier_avg_lead_time(
        &self,
        supplier_id: i32,
    ) -> Result<(i32, i64), AppError> {
        let result = self.db.query_one(
            Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"
                SELECT 
                    COALESCE(
                        AVG(
                            CASE 
                                WHEN actual_delivery_date IS NOT NULL AND order_date IS NOT NULL 
                                THEN actual_delivery_date - order_date 
                                ELSE NULL 
                            END
                        )::INTEGER,
                        7
                    ) as avg_days,
                    COUNT(*) as order_count
                FROM purchase_orders
                WHERE supplier_id = $1
                AND order_status IN ('COMPLETED', 'RECEIVED', 'PARTIALLY_RECEIVED')
                AND actual_delivery_date IS NOT NULL
                "#,
                vec![supplier_id.into()],
            ),
        )
        .await
        .map_err(|e| AppError::InternalError(format!("查询供应商交货周期失败: {}", e)))?;

        if let Some(row) = result {
            let avg_days: Option<i32> = row.try_get_by_index(0).ok();
            let order_count: Option<i64> = row.try_get_by_index(1).ok();
            Ok((avg_days.unwrap_or(7), order_count.unwrap_or(0)))
        } else {
            Ok((7, 0))
        }
    }

    /// 估算生产周期（简化实现）
    async fn estimate_production_days(
        &self,
        items: &[OrderItemInfo],
    ) -> Result<i32, AppError> {
        // 简化逻辑：根据物料数量估算
        // 实际项目中可以根据物料的BOM复杂度、工艺路线等计算
        let total_quantity: Decimal = items.iter().map(|i| i.quantity).sum();
        
        if total_quantity > Decimal::from(1000) {
            Ok(5)
        } else if total_quantity > Decimal::from(500) {
            Ok(3)
        } else if total_quantity > Decimal::from(100) {
            Ok(2)
        } else {
            Ok(1)
        }
    }

    /// 添加工作日（跳过周末）
    fn add_business_days(&self, start: NaiveDate, days: i32) -> NaiveDate {
        let mut current = start;
        let mut remaining = days;

        while remaining > 0 {
            current = current.succ_opt().unwrap_or(current);
            let weekday = current.weekday();
            if weekday != chrono::Weekday::Sat && weekday != chrono::Weekday::Sun {
                remaining -= 1;
            }
        }

        current
    }
}
