use chrono::NaiveDate;
// 成本归集 Service
//
// 成本核算业务逻辑层

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;
use tracing::info;

use crate::models::cost_collection;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;
use sea_orm::ActiveValue::Set;

/// 更新成本归集请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateCostCollectionRequest {
    pub collection_date: Option<NaiveDate>,
    pub direct_material: Option<Decimal>,
    pub direct_labor: Option<Decimal>,
    pub manufacturing_overhead: Option<Decimal>,
    pub processing_fee: Option<Decimal>,
    pub dyeing_fee: Option<Decimal>,
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateCostCollectionRequest {
    pub collection_date: chrono::NaiveDate,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: Decimal,
    pub direct_labor: Decimal,
    pub manufacturing_overhead: Decimal,
    pub processing_fee: Decimal,
    pub dyeing_fee: Decimal,
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
}

/// 成本归集 Service
pub struct CostCollectionService {
    db: Arc<DatabaseConnection>,
}

impl CostCollectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建成本归集
    pub async fn create(
        &self,
        req: CreateCostCollectionRequest,
        user_id: i32,
    ) -> Result<cost_collection::Model, AppError> {
        info!(
            "创建成本归集：batch_no={:?}, color_no={:?}",
            req.batch_no, req.color_no
        );

        // 生成成本归集单编号
        let collection_no = self.generate_collection_no().await?;

        // 计算总成本
        let total_cost = req.direct_material
            + req.direct_labor
            + req.manufacturing_overhead
            + req.processing_fee
            + req.dyeing_fee;

        // 计算单位成本
        let unit_cost_meters = req.output_quantity_meters.as_ref().and_then(|q| {
            if q.is_zero() {
                None
            } else {
                Some(total_cost / *q)
            }
        });
        let unit_cost_kg = req.output_quantity_kg.as_ref().and_then(|q| {
            if q.is_zero() {
                None
            } else {
                Some(total_cost / *q)
            }
        });

        let active_model = cost_collection::ActiveModel {
            collection_no: sea_orm::Set(collection_no),
            collection_date: sea_orm::Set(req.collection_date),
            cost_object_type: sea_orm::Set(req.cost_object_type),
            cost_object_id: sea_orm::Set(req.cost_object_id),
            cost_object_no: sea_orm::Set(req.cost_object_no),
            batch_no: sea_orm::Set(req.batch_no),
            color_no: sea_orm::Set(req.color_no),
            workshop: sea_orm::Set(req.workshop),
            direct_material: sea_orm::Set(req.direct_material),
            direct_labor: sea_orm::Set(req.direct_labor),
            manufacturing_overhead: sea_orm::Set(req.manufacturing_overhead),
            processing_fee: sea_orm::Set(req.processing_fee),
            dyeing_fee: sea_orm::Set(req.dyeing_fee),
            total_cost: sea_orm::Set(total_cost),
            output_quantity_meters: sea_orm::Set(req.output_quantity_meters),
            output_quantity_kg: sea_orm::Set(req.output_quantity_kg),
            unit_cost_meters: sea_orm::Set(unit_cost_meters),
            unit_cost_kg: sea_orm::Set(unit_cost_kg),
            status: sea_orm::Set("draft".to_string()),
            created_by: sea_orm::Set(Some(user_id)),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("成本归集创建成功：no={}", result.collection_no);

        Ok(result)
    }

    /// 查询成本归集列表
    pub async fn get_list(
        &self,
        batch_no: Option<String>,
        color_no: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<cost_collection::Model>, u64), AppError> {
        info!("查询成本归集列表");

        let mut query = cost_collection::Entity::find();

        if let Some(batch) = batch_no {
            query = query.filter(cost_collection::Column::BatchNo.eq(batch));
        }

        if let Some(color) = color_no {
            query = query.filter(cost_collection::Column::ColorNo.eq(color));
        }

        let total = query.clone().count(&*self.db).await?;
        let collections = query
            .order_by(cost_collection::Column::CollectionDate, Order::Desc)
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        info!("成本归集列表查询成功，共 {} 条", total);
        Ok((collections, total))
    }

    /// 查询成本归集详情
    pub async fn get_by_id(&self, id: i32) -> Result<cost_collection::Model, AppError> {
        info!("查询成本归集详情 ID: {}", id);

        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("成本归集单不存在：{}", id)))?;

        Ok(collection)
    }

    /// 生成成本归集单编号
    async fn generate_collection_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "COST",
            cost_collection::Entity,
            cost_collection::Column::CollectionNo,
        )
        .await
    }

    pub async fn update(
        &self,
        id: i32,
        req: UpdateCostCollectionRequest,
        _user_id: i32,
    ) -> Result<cost_collection::Model, AppError> {
        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        let mut active_collection: cost_collection::ActiveModel = collection.clone().into();

        if let Some(date) = req.collection_date {
            active_collection.collection_date = Set(date);
        }
        if let Some(amt) = req.direct_material {
            active_collection.direct_material = Set(amt);
        }
        if let Some(amt) = req.direct_labor {
            active_collection.direct_labor = Set(amt);
        }
        if let Some(amt) = req.manufacturing_overhead {
            active_collection.manufacturing_overhead = Set(amt);
        }
        if let Some(amt) = req.processing_fee {
            active_collection.processing_fee = Set(amt);
        }
        if let Some(amt) = req.dyeing_fee {
            active_collection.dyeing_fee = Set(amt);
        }
        if let Some(amt) = req.output_quantity_meters {
            active_collection.output_quantity_meters = Set(Some(amt));
        }
        if let Some(amt) = req.output_quantity_kg {
            active_collection.output_quantity_kg = Set(Some(amt));
        }

        // Recalculate total cost
        let dm = req.direct_material.unwrap_or(collection.direct_material);
        let dl = req.direct_labor.unwrap_or(collection.direct_labor);
        let mo = req
            .manufacturing_overhead
            .unwrap_or(collection.manufacturing_overhead);
        let pf = req.processing_fee.unwrap_or(collection.processing_fee);
        let df = req.dyeing_fee.unwrap_or(collection.dyeing_fee);
        let total = dm + dl + mo + pf + df;
        active_collection.total_cost = Set(total);

        // Recalculate unit costs
        let meters = req
            .output_quantity_meters
            .or(collection.output_quantity_meters);
        if let Some(m) = meters {
            if m > Decimal::ZERO {
                active_collection.unit_cost_meters = Set(Some(total / m));
            }
        }
        let kg = req.output_quantity_kg.or(collection.output_quantity_kg);
        if let Some(k) = kg {
            if k > Decimal::ZERO {
                active_collection.unit_cost_kg = Set(Some(total / k));
            }
        }

        active_collection.updated_at = Set(chrono::Utc::now());

        let result = active_collection.update(&*self.db).await?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let _collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        cost_collection::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 获取成本分析汇总
    pub async fn get_cost_analysis_summary(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CostAnalysisSummary, AppError> {
        let mut query = cost_collection::Entity::find();

        if let Some(start) = start_date {
            query = query.filter(cost_collection::Column::CollectionDate.gte(start));
        }
        if let Some(end) = end_date {
            query = query.filter(cost_collection::Column::CollectionDate.lte(end));
        }

        let collections = query.all(&*self.db).await?;

        let mut total_direct_material = Decimal::ZERO;
        let mut total_direct_labor = Decimal::ZERO;
        let mut total_overhead = Decimal::ZERO;
        let mut total_processing = Decimal::ZERO;
        let mut total_dyeing = Decimal::ZERO;
        let mut total_cost = Decimal::ZERO;
        let mut total_output_meters = Decimal::ZERO;
        let mut total_output_kg = Decimal::ZERO;
        let mut record_count = 0;

        for c in &collections {
            total_direct_material += c.direct_material;
            total_direct_labor += c.direct_labor;
            total_overhead += c.manufacturing_overhead;
            total_processing += c.processing_fee;
            total_dyeing += c.dyeing_fee;
            total_cost += c.total_cost;
            if let Some(m) = c.output_quantity_meters {
                total_output_meters += m;
            }
            if let Some(k) = c.output_quantity_kg {
                total_output_kg += k;
            }
            record_count += 1;
        }

        let avg_unit_cost_meters = if total_output_meters > Decimal::ZERO {
            Some(total_cost / total_output_meters)
        } else {
            None
        };

        let avg_unit_cost_kg = if total_output_kg > Decimal::ZERO {
            Some(total_cost / total_output_kg)
        } else {
            None
        };

        Ok(CostAnalysisSummary {
            record_count,
            total_direct_material,
            total_direct_labor,
            total_overhead,
            total_processing,
            total_dyeing,
            total_cost,
            total_output_meters,
            total_output_kg,
            avg_unit_cost_meters,
            avg_unit_cost_kg,
            material_ratio: if total_cost > Decimal::ZERO {
                Some(total_direct_material / total_cost)
            } else {
                None
            },
            labor_ratio: if total_cost > Decimal::ZERO {
                Some(total_direct_labor / total_cost)
            } else {
                None
            },
            overhead_ratio: if total_cost > Decimal::ZERO {
                Some(total_overhead / total_cost)
            } else {
                None
            },
        })
    }

    /// 按批次获取成本分析
    pub async fn get_cost_by_batch(
        &self,
        batch_no: Option<String>,
    ) -> Result<Vec<BatchCostAnalysis>, AppError> {
        let mut query = cost_collection::Entity::find();

        if let Some(batch) = batch_no {
            query = query.filter(cost_collection::Column::BatchNo.eq(batch));
        }

        let collections = query
            .order_by(cost_collection::Column::CollectionDate, Order::Desc)
            .all(&*self.db)
            .await?;

        let mut result = Vec::new();
        for c in collections {
            result.push(BatchCostAnalysis {
                collection_no: c.collection_no,
                batch_no: c.batch_no,
                color_no: c.color_no,
                collection_date: Some(c.collection_date),
                direct_material: c.direct_material,
                direct_labor: c.direct_labor,
                manufacturing_overhead: c.manufacturing_overhead,
                processing_fee: c.processing_fee,
                dyeing_fee: c.dyeing_fee,
                total_cost: c.total_cost,
                output_quantity_meters: c.output_quantity_meters,
                output_quantity_kg: c.output_quantity_kg,
                unit_cost_meters: c.unit_cost_meters,
                unit_cost_kg: c.unit_cost_kg,
                status: c.status,
            });
        }

        Ok(result)
    }

    /// 审核成本归集
    pub async fn audit(
        &self,
        id: i32,
        approved: bool,
        _comment: Option<String>,
        _user_id: i32,
    ) -> Result<cost_collection::Model, AppError> {
        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("成本归集".to_string()))?;

        // 只有草稿状态才能审核
        if collection.status != "draft" {
            return Err(AppError::ValidationError(
                "只有草稿状态的成本归集才能审核".to_string(),
            ));
        }

        let status = if approved { "approved" } else { "rejected" };

        let active_model = cost_collection::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(collection.id),
            status: sea_orm::ActiveValue::Set(status.to_string()),
            ..Default::default()
        };

        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }
}

/// 成本分析汇总
#[derive(Debug, serde::Serialize)]
pub struct CostAnalysisSummary {
    pub record_count: i32,
    pub total_direct_material: Decimal,
    pub total_direct_labor: Decimal,
    pub total_overhead: Decimal,
    pub total_processing: Decimal,
    pub total_dyeing: Decimal,
    pub total_cost: Decimal,
    pub total_output_meters: Decimal,
    pub total_output_kg: Decimal,
    pub avg_unit_cost_meters: Option<Decimal>,
    pub avg_unit_cost_kg: Option<Decimal>,
    pub material_ratio: Option<Decimal>,
    pub labor_ratio: Option<Decimal>,
    pub overhead_ratio: Option<Decimal>,
}

/// 批次成本分析
#[derive(Debug, serde::Serialize)]
pub struct BatchCostAnalysis {
    pub collection_no: String,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub collection_date: Option<NaiveDate>,
    pub direct_material: Decimal,
    pub direct_labor: Decimal,
    pub manufacturing_overhead: Decimal,
    pub processing_fee: Decimal,
    pub dyeing_fee: Decimal,
    pub total_cost: Decimal,
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
    pub unit_cost_meters: Option<Decimal>,
    pub unit_cost_kg: Option<Decimal>,
    pub status: String,
}

/// 计算总成本
#[allow(dead_code)]
pub fn calculate_total_cost(
    direct_material: Decimal,
    direct_labor: Decimal,
    manufacturing_overhead: Decimal,
    processing_fee: Decimal,
    dyeing_fee: Decimal,
) -> Decimal {
    direct_material + direct_labor + manufacturing_overhead + processing_fee + dyeing_fee
}

/// 计算单位成本（米）
#[allow(dead_code)]
pub fn calculate_unit_cost_meters(
    total_cost: Decimal,
    output_meters: Option<Decimal>,
) -> Option<Decimal> {
    output_meters.and_then(|q| {
        if q.is_zero() {
            None
        } else {
            Some(total_cost / q)
        }
    })
}

/// 计算单位成本（公斤）
#[allow(dead_code)]
pub fn calculate_unit_cost_kg(total_cost: Decimal, output_kg: Option<Decimal>) -> Option<Decimal> {
    output_kg.and_then(|q| {
        if q.is_zero() {
            None
        } else {
            Some(total_cost / q)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_calculate_total_cost_basic() {
        let total = calculate_total_cost(
            Decimal::from(1000), // 直接材料
            Decimal::from(500),  // 直接人工
            Decimal::from(300),  // 制造费用
            Decimal::from(200),  // 加工费
            Decimal::from(100),  // 染费
        );
        assert_eq!(total, Decimal::from(2100));
    }

    #[test]
    fn test_calculate_total_cost_zero() {
        let total = calculate_total_cost(
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
        );
        assert_eq!(total, Decimal::ZERO);
    }

    #[test]
    fn test_calculate_total_cost_single_component() {
        let total = calculate_total_cost(
            Decimal::from(5000),
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
        );
        assert_eq!(total, Decimal::from(5000));
    }

    #[test]
    fn test_calculate_unit_cost_meters() {
        let total_cost = Decimal::from(10000);
        let output = Some(Decimal::from(500));

        let unit_cost = calculate_unit_cost_meters(total_cost, output);
        assert_eq!(unit_cost, Some(Decimal::from(20)));
    }

    #[test]
    fn test_calculate_unit_cost_meters_zero_output() {
        let total_cost = Decimal::from(10000);
        let output = Some(Decimal::ZERO);

        let unit_cost = calculate_unit_cost_meters(total_cost, output);
        assert_eq!(unit_cost, None);
    }

    #[test]
    fn test_calculate_unit_cost_meters_none_output() {
        let total_cost = Decimal::from(10000);

        let unit_cost = calculate_unit_cost_meters(total_cost, None);
        assert_eq!(unit_cost, None);
    }

    #[test]
    fn test_calculate_unit_cost_kg() {
        let total_cost = Decimal::from(10000);
        let output = Some(Decimal::from(200));

        let unit_cost = calculate_unit_cost_kg(total_cost, output);
        assert_eq!(unit_cost, Some(Decimal::from(50)));
    }

    #[test]
    fn test_calculate_unit_cost_kg_zero_output() {
        let total_cost = Decimal::from(10000);
        let output = Some(Decimal::ZERO);

        let unit_cost = calculate_unit_cost_kg(total_cost, output);
        assert_eq!(unit_cost, None);
    }

    #[test]
    fn test_calculate_unit_cost_kg_none_output() {
        let total_cost = Decimal::from(10000);

        let unit_cost = calculate_unit_cost_kg(total_cost, None);
        assert_eq!(unit_cost, None);
    }

    #[test]
    fn test_cost_calculation_workflow() {
        // 模拟完整的成本计算流程
        let direct_material = Decimal::from(5000);
        let direct_labor = Decimal::from(2000);
        let manufacturing_overhead = Decimal::from(1500);
        let processing_fee = Decimal::from(800);
        let dyeing_fee = Decimal::from(700);

        // 1. 计算总成本
        let total_cost = calculate_total_cost(
            direct_material,
            direct_labor,
            manufacturing_overhead,
            processing_fee,
            dyeing_fee,
        );
        assert_eq!(total_cost, Decimal::from(10000));

        // 2. 计算单位成本
        let output_meters = Some(Decimal::from(500));
        let output_kg = Some(Decimal::from(200));

        let unit_cost_meters = calculate_unit_cost_meters(total_cost, output_meters);
        let unit_cost_kg = calculate_unit_cost_kg(total_cost, output_kg);

        assert_eq!(unit_cost_meters, Some(Decimal::from(20)));
        assert_eq!(unit_cost_kg, Some(Decimal::from(50)));
    }

    #[test]
    fn test_cost_ratio_calculation() {
        let total_cost = Decimal::from(10000);
        let direct_material = Decimal::from(5000);
        let direct_labor = Decimal::from(2000);
        let manufacturing_overhead = Decimal::from(3000);

        // 计算各项占比
        let material_ratio = direct_material / total_cost;
        let labor_ratio = direct_labor / total_cost;
        let overhead_ratio = manufacturing_overhead / total_cost;

        assert_eq!(material_ratio, Decimal::try_from(0.5).unwrap());
        assert_eq!(labor_ratio, Decimal::try_from(0.2).unwrap());
        assert_eq!(overhead_ratio, Decimal::try_from(0.3).unwrap());

        // 验证占比之和为 1
        let total_ratio = material_ratio + labor_ratio + overhead_ratio;
        assert_eq!(total_ratio, Decimal::ONE);
    }

    #[test]
    fn test_cost_analysis_summary_fields() {
        let summary = CostAnalysisSummary {
            record_count: 10,
            total_direct_material: Decimal::from(50000),
            total_direct_labor: Decimal::from(20000),
            total_overhead: Decimal::from(15000),
            total_processing: Decimal::from(8000),
            total_dyeing: Decimal::from(7000),
            total_cost: Decimal::from(100000),
            total_output_meters: Decimal::from(5000),
            total_output_kg: Decimal::from(2000),
            avg_unit_cost_meters: Some(Decimal::from(20)),
            avg_unit_cost_kg: Some(Decimal::from(50)),
            material_ratio: Some(Decimal::try_from(0.5).unwrap()),
            labor_ratio: Some(Decimal::try_from(0.2).unwrap()),
            overhead_ratio: Some(Decimal::try_from(0.15).unwrap()),
        };

        assert_eq!(summary.record_count, 10);
        assert_eq!(summary.total_cost, Decimal::from(100000));
        assert!(summary.avg_unit_cost_meters.is_some());
    }
}
