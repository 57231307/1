use crate::models::purchase_price;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// 采购价格查询参数
#[derive(Debug, Clone, Default)]
pub struct PurchasePriceQueryParams {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建采购价格请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePurchasePriceInput {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: rust_decimal::Decimal,
    pub currency: String,
    pub min_order_qty: Option<rust_decimal::Decimal>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

pub struct PurchasePriceService {
    db: Arc<DatabaseConnection>,
}

impl PurchasePriceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取采购价格列表
    pub async fn get_prices_list(
        &self,
        params: PurchasePriceQueryParams,
    ) -> Result<(Vec<purchase_price::Model>, u64), AppError> {
        let mut query = purchase_price::Entity::find();

        if let Some(product_id) = params.product_id {
            query = query.filter(purchase_price::Column::ProductId.eq(product_id));
        }

        if let Some(supplier_id) = params.supplier_id {
            query = query.filter(purchase_price::Column::SupplierId.eq(supplier_id));
        }

        if let Some(status) = &params.status {
            query = query.filter(purchase_price::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let prices = query
            .order_by(purchase_price::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((prices, total))
    }

    /// 创建采购价格
    pub async fn create_price(
        &self,
        req: CreatePurchasePriceInput,
        user_id: i32,
    ) -> Result<purchase_price::Model, AppError> {
        info!(
            "用户 {} 正在创建采购价格，产品 ID: {}, 供应商 ID: {}",
            user_id, req.product_id, req.supplier_id
        );

        let active_price = purchase_price::ActiveModel {
            product_id: Set(req.product_id),
            supplier_id: Set(req.supplier_id),
            price: Set(req.price),
            currency: Set(req.currency),
            min_order_qty: Set(req.min_order_qty.unwrap_or_default()),
            effective_date: Set(req
                .effective_date
                .parse()
                .map_err(|e| AppError::ValidationError(format!("日期格式错误：{}", e)))?),
            expiry_date: Set(req.expiry_date.and_then(|d| d.parse().ok())),
            status: Set("pending".to_string()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let price = active_price.insert(&*self.db).await?;
        info!("采购价格创建成功，ID: {}", price.id);
        Ok(price)
    }

    /// 获取采购价格
    pub async fn get_price(&self, id: i32) -> Result<purchase_price::Model, AppError> {
        info!("查询采购价格，ID: {}", id);

        let price = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("采购价格 {} 未找到", id)))?;

        Ok(price)
    }

    /// 批准采购价格
    pub async fn approve_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在批准采购价格，ID: {}", user_id, id);

        let mut price: purchase_price::ActiveModel = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("采购价格 {} 未找到", id)))?
            .into();

        price.status = Set("approved".to_string());
        price.approved_by = Set(Some(user_id));
        price.save(&*self.db).await?;

        info!("采购价格批准成功，ID: {}", id);
        Ok(())
    }

    /// 获取价格历史
    pub async fn get_price_history(
        &self,
        material_id: i32,
    ) -> Result<Vec<purchase_price::Model>, AppError> {
        info!("查询物料 {} 的价格历史", material_id);

        let history = purchase_price::Entity::find()
            .filter(purchase_price::Column::ProductId.eq(material_id))
            .order_by(purchase_price::Column::EffectiveDate, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(history)
    }

    pub async fn update_price(
        &self,
        id: i32,
        price: Decimal,
        expiry_date: Option<String>,
        status: Option<String>,
    ) -> Result<(), AppError> {
        info!("更新采购价格，ID: {}", id);

        let mut price_model: purchase_price::ActiveModel = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("采购价格 {} 未找到", id)))?
            .into();

        price_model.price = Set(price);
        if let Some(ed) = expiry_date {
            price_model.expiry_date =
                Set(Some(ed.parse().map_err(|e| {
                    AppError::ValidationError(format!("日期格式错误：{}", e))
                })?));
        }
        if let Some(s) = status {
            price_model.status = Set(s);
        }

        price_model.save(&*self.db).await?;
        info!("采购价格更新成功，ID: {}", id);
        Ok(())
    }

    pub async fn delete_price(&self, id: i32) -> Result<(), AppError> {
        info!("删除采购价格，ID: {}", id);

        let price = purchase_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("采购价格 {} 未找到", id)))?;

        price.delete(&*self.db).await?;
        info!("采购价格删除成功，ID: {}", id);
        Ok(())
    }
}
