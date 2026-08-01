//! 存货跌价准备服务
//! V15 P2 B08-16：季节性降价/呆滞面料/过期化学品跌价准备计提

use crate::models::inventory_write_down::{ActiveModel, Entity as Iwd, Model};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

pub struct InventoryWriteDownService {
    db: Arc<DatabaseConnection>,
}

impl InventoryWriteDownService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        let mut query = Iwd::find();
        if let Some(product_id) = params.product_id {
            query =
                query.filter(crate::models::inventory_write_down::Column::ProductId.eq(product_id));
        }
        if let Some(write_down_type) = params.write_down_type {
            query = query.filter(
                crate::models::inventory_write_down::Column::WriteDownType.eq(write_down_type),
            );
        }
        let paginator = query
            .order_by_desc(crate::models::inventory_write_down::Column::Period)
            .paginate(&*self.db, params.page_size.unwrap_or(20));
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(params.page.unwrap_or(0)).await?;
        Ok((items, total))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Model, AppError> {
        Iwd::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("跌价准备记录 {} 不存在", id)))
    }

    pub async fn create(&self, data: CreateWriteDownReq) -> Result<Model, AppError> {
        let active = ActiveModel {
            product_id: Set(data.product_id),
            write_down_type: Set(data.write_down_type),
            original_cost: Set(data.original_cost),
            net_realizable_value: Set(data.net_realizable_value),
            write_down_amount: Set(data.original_cost - data.net_realizable_value),
            reason: Set(data.reason),
            period: Set(data.period),
            status: Set("draft".to_string()),
            created_by: Set(data.created_by),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    pub async fn confirm(&self, id: i32, confirmed_by: i32) -> Result<Model, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: ActiveModel = model.into();
        active.status = Set("confirmed".to_string());
        active.confirmed_by = Set(Some(confirmed_by));
        active.confirmed_at = Set(Some(chrono::Utc::now()));
        let model = active.update(&*self.db).await?;
        Ok(model)
    }
}

pub struct ListParams {
    pub product_id: Option<i32>,
    pub write_down_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub struct CreateWriteDownReq {
    pub product_id: i32,
    pub write_down_type: String,
    pub original_cost: Decimal,
    pub net_realizable_value: Decimal,
    pub reason: Option<String>,
    pub period: chrono::NaiveDate,
    pub created_by: i32,
}
