//! 成本归集 Service
//!
//! 成本核算业务逻辑层

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, PaginatorTrait, Order,
};
use std::sync::Arc;
use tracing::info;
use chrono::Datelike;

use crate::models::cost_collection;
use crate::utils::error::AppError;
use rust_decimal::Decimal;

/// 创建成本归集请求
#[derive(Debug, Clone)]
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
        info!("创建成本归集：batch_no={:?}, color_no={:?}", req.batch_no, req.color_no);

        // 生成成本归集单编号
        let collection_no = self.generate_collection_no(req.collection_date)?;

        // 计算总成本
        let total_cost = req.direct_material + req.direct_labor + 
                        req.manufacturing_overhead + req.processing_fee + req.dyeing_fee;

        // 计算单位成本
        let unit_cost_meters = req.output_quantity_meters.as_ref()
            .and_then(|q| if q.is_zero() { None } else { Some(total_cost / *q) });
        let unit_cost_kg = req.output_quantity_kg.as_ref()
            .and_then(|q| if q.is_zero() { None } else { Some(total_cost / *q) });

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
    #[allow(dead_code)]
    pub async fn get_by_id(&self, id: i32) -> Result<cost_collection::Model, AppError> {
        info!("查询成本归集详情 ID: {}", id);

        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("成本归集单不存在：{}", id)))?;

        Ok(collection)
    }

    /// 生成成本归集单编号
    fn generate_collection_no(
        &self,
        collection_date: chrono::NaiveDate,
    ) -> Result<String, AppError> {
        let year_month = format!("{:04}{:02}", collection_date.year(), collection_date.month());
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        Ok(format!("COST{}{:04}", year_month, timestamp % 10000))
    }
}
