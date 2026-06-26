use crate::models::sales_price;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct SalesPriceQueryParams {
    pub product_id: Option<i32>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSalesPriceInput {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Decimal,
    pub currency: Option<String>,
    pub min_order_qty: Option<Decimal>,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSalesPriceInput {
    pub product_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Option<Decimal>,
    pub currency: Option<String>,
    pub min_order_qty: Option<Decimal>,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
}

pub struct SalesPriceService {
    db: Arc<DatabaseConnection>,
}

impl SalesPriceService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_prices_list(
        &self,
        params: SalesPriceQueryParams,
    ) -> Result<(Vec<sales_price::Model>, u64), AppError> {
        let mut query = sales_price::Entity::find();

        if let Some(product_id) = params.product_id {
            query = query.filter(sales_price::Column::ProductId.eq(product_id));
        }

        if let Some(customer_type) = params.customer_type {
            query = query.filter(sales_price::Column::CustomerType.eq(customer_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(sales_price::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let prices = query
            .order_by(sales_price::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((prices, total))
    }

    pub async fn create_price(
        &self,
        req: CreateSalesPriceInput,
        user_id: i32,
    ) -> Result<sales_price::Model, AppError> {
        info!(
            "用户 {} 正在创建销售价格，产品 ID: {}",
            user_id, req.product_id
        );

        let active_price = sales_price::ActiveModel {
            product_id: Set(req.product_id),
            customer_id: Set(req.customer_id),
            customer_type: Set(req.customer_type),
            price: Set(req.price),
            currency: Set(req.currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            min_order_qty: Set(req.min_order_qty.unwrap_or_default()),
            effective_date: Set(req
                .effective_date
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string())
                .parse()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?),
            expiry_date: Set(req.expiry_date.and_then(|d| d.parse().ok())),
            status: Set("pending".to_string()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let price = active_price.insert(&*self.db).await?;
        info!("销售价格创建成功，ID: {}", price.id);
        Ok(price)
    }

    pub async fn get_price(&self, id: i32) -> Result<sales_price::Model, AppError> {
        info!("查询销售价格，ID: {}", id);

        let price = sales_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        Ok(price)
    }

    pub async fn approve_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在批准销售价格，ID: {}", user_id, id);

        let price_model = sales_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        // 检查状态，只有待审批状态可以批准
        if price_model.status != "pending" {
            return Err(AppError::validation(format!(
                "只有待审批状态的价格可以批准，当前状态：{}",
                price_model.status
            )));
        }

        let mut price: sales_price::ActiveModel = price_model.into();
        price.status = Set("approved".to_string());
        price.approved_by = Set(Some(user_id));
        price.approved_at = Set(Some(chrono::Utc::now()));
        price.save(&*self.db).await?;

        info!("销售价格批准成功，ID: {}", id);
        Ok(())
    }

    pub async fn get_price_history(
        &self,
        product_id: i32,
    ) -> Result<Vec<sales_price::Model>, AppError> {
        info!("查询产品 {} 的价格历史", product_id);

        let history = sales_price::Entity::find()
            .filter(sales_price::Column::ProductId.eq(product_id))
            .order_by(sales_price::Column::EffectiveDate, Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(history)
    }

    /// 更新销售价格
    pub async fn update_price(
        &self,
        id: i32,
        req: UpdateSalesPriceInput,
    ) -> Result<sales_price::Model, AppError> {
        info!("更新销售价格，ID: {}", id);

        let price_model = sales_price::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售价格 {} 未找到", id)))?;

        let mut active: sales_price::ActiveModel = price_model.into_active_model();

        if let Some(product_id) = req.product_id {
            active.product_id = Set(product_id);
        }
        if let Some(customer_id) = req.customer_id {
            active.customer_id = Set(Some(customer_id));
        }
        if let Some(customer_type) = req.customer_type {
            active.customer_type = Set(Some(customer_type));
        }
        if let Some(price) = req.price {
            active.price = Set(price);
        }
        if let Some(currency) = req.currency {
            active.currency = Set(currency);
        }
        if let Some(min_order_qty) = req.min_order_qty {
            active.min_order_qty = Set(min_order_qty);
        }
        if let Some(effective_date) = req.effective_date {
            active.effective_date = Set(effective_date
                .parse()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?);
        }
        if let Some(expiry_date) = req.expiry_date {
            active.expiry_date =
                Set(Some(expiry_date.parse().map_err(|e| {
                    AppError::validation(format!("日期格式错误：{}", e))
                })?));
        }

        let updated = active.update(&*self.db).await?;
        info!("销售价格更新成功，ID: {}", updated.id);
        Ok(updated)
    }

    /// 删除销售价格
    pub async fn delete_price(&self, id: i32) -> Result<(), AppError> {
        info!("删除销售价格，ID: {}", id);

        let result = sales_price::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await?;

        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("销售价格 {} 未找到", id)));
        }

        info!("销售价格删除成功，ID: {}", id);
        Ok(())
    }

    /// BE-P 优化（2026-06-26）：补齐分页参数，避免数据量增长后全量返回。
    pub async fn list_strategies(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<sales_price::Model>, u64), AppError> {
        info!(
            "查询销售价格策略列表，页码：{}，每页：{}",
            page, page_size
        );

        let paginator = sales_price::Entity::find()
            .filter(sales_price::Column::Status.eq("active".to_string()))
            .order_by(sales_price::Column::Id, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let strategies = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((strategies, total))
    }
}
