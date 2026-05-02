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
use sea_orm::ActiveValue::Set;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;

/// 创建成本归集请求
#[derive(Debug, Clone)]
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
            created_by: sea_orm::Set(user_id),
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
            .offset(page - 1)
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
            &self.db,
            "COST",
            cost_collection::Entity,
            cost_collection::Column::CollectionNo,
        )
        .await
    }

    pub async fn update(&self, id: i32, req: UpdateCostCollectionRequest, _user_id: i32) -> Result<cost_collection::Model, AppError> {
        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        let mut active_collection: cost_collection::ActiveModel = collection.clone().into();
        
        if let Some(date) = req.collection_date { active_collection.collection_date = Set(date); }
        if let Some(amt) = req.direct_material { active_collection.direct_material = Set(amt); }
        if let Some(amt) = req.direct_labor { active_collection.direct_labor = Set(amt); }
        if let Some(amt) = req.manufacturing_overhead { active_collection.manufacturing_overhead = Set(amt); }
        if let Some(amt) = req.processing_fee { active_collection.processing_fee = Set(amt); }
        if let Some(amt) = req.dyeing_fee { active_collection.dyeing_fee = Set(amt); }
        if let Some(amt) = req.output_quantity_meters { active_collection.output_quantity_meters = Set(Some(amt)); }
        if let Some(amt) = req.output_quantity_kg { active_collection.output_quantity_kg = Set(Some(amt)); }
        
        // Recalculate total cost
        let dm = req.direct_material.unwrap_or(collection.direct_material);
        let dl = req.direct_labor.unwrap_or(collection.direct_labor);
        let mo = req.manufacturing_overhead.unwrap_or(collection.manufacturing_overhead);
        let pf = req.processing_fee.unwrap_or(collection.processing_fee);
        let df = req.dyeing_fee.unwrap_or(collection.dyeing_fee);
        let total = dm + dl + mo + pf + df;
        active_collection.total_cost = Set(total);

        // Recalculate unit costs
        let meters = req.output_quantity_meters.or(collection.output_quantity_meters);
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

        let result = active_collection.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let _collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        cost_collection::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
