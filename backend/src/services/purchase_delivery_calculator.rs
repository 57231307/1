//! 采购交期计算服务
//!
//! 根据供应商历史交货数据自动计算建议交货日期

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Statement,
};
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
        let (avg_lead_time, historical_orders) =
            self.get_supplier_avg_lead_time(req.supplier_id).await?;

        // 2. 获取最大生产周期（简化实现，基于物料复杂度）
        let max_production_days = self.estimate_production_days(&req.items).await?;

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
            format!("供应商无历史交货数据，使用默认交货周期{}天", avg_lead_time)
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
    async fn get_supplier_avg_lead_time(&self, supplier_id: i32) -> Result<(i32, i64), AppError> {
        let result: Option<sea_orm::QueryResult> = self
            .db
            .query_one(Statement::from_sql_and_values(
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
            ))
            .await
            .map_err(|e| AppError::internal(format!("查询供应商交货周期失败: {}", e)))?;

        if let Some(row) = result {
            let avg_days: Option<i32> = row.try_get_by_index::<i32>(0).ok();
            let order_count: Option<i64> = row.try_get_by_index::<i64>(1).ok();
            // 缺记录时默认 7 天交货 + 0 单（业务接受估算）
            Ok((avg_days.unwrap_or(7), order_count.unwrap_or_default()))
        } else {
            Ok((7, 0))
        }
    }

    /// 估算生产周期
    async fn estimate_production_days(&self, items: &[OrderItemInfo]) -> Result<i32, AppError> {
        use crate::models::bom::{Column as BomColumn, Entity as BomEntity};

        let mut total_days = 0;

        // v15 批次 42 修复：批量查询所有产品的 BOM 数量，避免循环内逐个 count（N+1 查询）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let bom_counts: std::collections::HashMap<i32, usize> = if product_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            BomEntity::find()
                .filter(BomColumn::ProductId.is_in(product_ids))
                .all(&*self.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .fold(std::collections::HashMap::new(), |mut acc, bom| {
                    *acc.entry(bom.product_id).or_insert(0) += 1;
                    acc
                })
        };

        for item in items {
            // 从批量查询结果获取 BOM 数量（缺失时默认 0，与原 unwrap_or_default 语义一致）
            let bom_count = bom_counts.get(&item.product_id).copied().unwrap_or(0);

            // 基础生产天数
            let base_days = if bom_count > 0 {
                // 有BOM的产品，根据BOM数量增加复杂度
                let bom_complexity = std::cmp::min(bom_count as i32, 5);
                2 + bom_complexity
            } else {
                // 无BOM的产品，根据数量估算
                if item.quantity > Decimal::from(1000) {
                    5
                } else if item.quantity > Decimal::from(500) {
                    3
                } else if item.quantity > Decimal::from(100) {
                    2
                } else {
                    1
                }
            };

            total_days = std::cmp::max(total_days, base_days);
        }

        // 批量生产效率提升（多产品并行生产）
        if items.len() > 1 {
            total_days = (total_days as f64 * 1.2) as i32; // 增加20%的时间
        }

        Ok(total_days.max(1))
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
