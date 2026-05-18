//! MRP物料需求计算引擎
//!
//! 基于BOM和库存数据计算物料需求

#![allow(dead_code)]

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::bom_item::Entity as BomItemEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Entity as MrpResultEntity, Model as MrpResultModel,
};
use crate::utils::error::AppError;

/// 物料需求计算结果
#[derive(Debug, Clone)]
pub struct MaterialRequirement {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub source_type: String,
    pub source_id: Option<i32>,
}

/// 产能负荷信息
#[derive(Debug, Clone)]
pub struct CapacityLoad {
    pub work_center_id: i32,
    pub work_center_name: String,
    pub date: NaiveDate,
    pub planned_hours: Decimal,
    pub available_hours: Decimal,
    pub utilization_rate: Decimal,
}

/// MRP计算引擎
pub struct MrpEngineService {
    db: Arc<DatabaseConnection>,
}

impl MrpEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 计算物料需求
    pub async fn calculate_requirement(
        &self,
        product_id: i32,
        required_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
    ) -> Result<MaterialRequirement, AppError> {
        // 查询当前库存
        let stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(product_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let available_quantity = stock.map(|s| s.quantity_available).unwrap_or(Decimal::ZERO);
        let shortage_quantity = if required_quantity > available_quantity {
            required_quantity - available_quantity
        } else {
            Decimal::ZERO
        };

        Ok(MaterialRequirement {
            product_id,
            required_quantity,
            required_date,
            available_quantity,
            shortage_quantity,
            source_type,
            source_id,
        })
    }

    /// 基于BOM展开计算子物料需求
    pub async fn explode_bom(
        &self,
        _product_id: i32,
        parent_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let mut requirements = Vec::new();

        // 查询BOM明细
        let bom_items = BomItemEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        for item in bom_items {
            // 计算子物料需求数量（考虑损耗率）
            let quantity = parent_quantity * item.quantity;
            let quantity_with_scrap = if let Some(scrap_rate) = item.scrap_rate {
                quantity * (Decimal::ONE + scrap_rate)
            } else {
                quantity
            };

            // 查询子物料库存
            let stock = InventoryStockEntity::find()
                .filter(crate::models::inventory_stock::Column::ProductId.eq(item.material_id))
                .one(&*self.db)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let available_quantity = stock.map(|s| s.quantity_available).unwrap_or(Decimal::ZERO);
            let shortage_quantity = if quantity_with_scrap > available_quantity {
                quantity_with_scrap - available_quantity
            } else {
                Decimal::ZERO
            };

            requirements.push(MaterialRequirement {
                product_id: item.material_id,
                required_quantity: quantity_with_scrap,
                required_date: required_date - Duration::days(7), // 提前7天需求
                available_quantity,
                shortage_quantity,
                source_type: source_type.clone(),
                source_id,
            });
        }

        Ok(requirements)
    }

    /// 执行MRP计算并保存结果
    pub async fn run_mrp_calculation(
        &self,
        product_id: i32,
        required_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let mut results = Vec::new();

        // 计算主产品需求
        let main_req = self.calculate_requirement(
            product_id,
            required_quantity,
            required_date,
            source_type.clone(),
            source_id,
        )
        .await?;

        // 保存主产品MRP结果
        let calculation_no = format!("MRP{}", Utc::now().timestamp());
        let active_model = MrpResultActiveModel {
            calculation_no: Set(calculation_no.clone()),
            product_id: Set(main_req.product_id),
            required_quantity: Set(main_req.required_quantity),
            required_date: Set(Some(main_req.required_date)),
            source_type: Set(main_req.source_type),
            source_id: Set(main_req.source_id),
            planned_order_quantity: Set(Some(main_req.shortage_quantity)),
            planned_order_date: Set(Some(main_req.required_date - Duration::days(14))),
            status: Set("PLANNED".to_string()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let main_result = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        results.push(main_result);

        // 展开BOM计算子物料需求
        let sub_requirements = self.explode_bom(
            product_id,
            required_quantity,
            required_date,
            source_type,
            source_id,
        )
        .await?;

        for req in sub_requirements {
            if req.shortage_quantity > Decimal::ZERO {
                let sub_active_model = MrpResultActiveModel {
                    calculation_no: Set(format!("{}-{}", calculation_no, req.product_id)),
                    product_id: Set(req.product_id),
                    required_quantity: Set(req.required_quantity),
                    required_date: Set(Some(req.required_date)),
                    source_type: Set(req.source_type),
                    source_id: Set(req.source_id),
                    planned_order_quantity: Set(Some(req.shortage_quantity)),
                    planned_order_date: Set(Some(req.required_date - Duration::days(14))),
                    status: Set("PLANNED".to_string()),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                };

                let sub_result = sub_active_model
                    .insert(&*self.db)
                    .await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                results.push(sub_result);
            }
        }

        Ok(results)
    }

    /// 获取缺料预警列表
    pub async fn get_shortage_alerts(
        &self,
        days_ahead: i64,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let alert_date = Utc::now().date_naive() + Duration::days(days_ahead);

        let mrp_results = MrpResultEntity::find()
            .filter(crate::models::mrp_result::Column::RequiredDate.lte(alert_date))
            .filter(crate::models::mrp_result::Column::Status.eq("PLANNED"))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut alerts = Vec::new();
        for result in mrp_results {
            let stock = InventoryStockEntity::find()
                .filter(crate::models::inventory_stock::Column::ProductId.eq(result.product_id))
                .one(&*self.db)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let available = stock.map(|s| s.quantity_available).unwrap_or(Decimal::ZERO);
            let shortage = if result.required_quantity > available {
                result.required_quantity - available
            } else {
                Decimal::ZERO
            };

            if shortage > Decimal::ZERO {
                alerts.push(MaterialRequirement {
                    product_id: result.product_id,
                    required_quantity: result.required_quantity,
                    required_date: result.required_date.unwrap_or(alert_date),
                    available_quantity: available,
                    shortage_quantity: shortage,
                    source_type: result.source_type,
                    source_id: result.source_id,
                });
            }
        }

        Ok(alerts)
    }
}
