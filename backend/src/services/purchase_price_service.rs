use crate::models::purchase_price;
use crate::models::status::master_data;
use crate::models::{product, supplier};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType,
    ModelTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 采购价格读模型：实体列 + LEFT JOIN 关联出的产品名 / 产品编码 / 供应商名（实体仅有外键 ID）。
///
/// JOIN 名列均 `Option<String>`；实体自身列按 `purchase_price` 约束保持原类型。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchasePriceView {
    pub id: i32,
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    pub effective_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub supplier_name: Option<String>,
}

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
///
/// `unit`（计量单位）与 `price_type`（价格类型）为创建必填：价格必依附计量单位与价格类型，
/// 对应列 `purchase_prices.unit` / `purchase_prices.price_type` 为 NOT NULL 且无数据库默认值。
/// 缺失/空字符串由 validator 返回 4xx VALIDATION_ERROR（单一 AppError 信封），
/// 不再依赖 DB NOT NULL 约束裸抛 500 DATABASE_ERROR。
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePurchasePriceInput {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: rust_decimal::Decimal,
    pub currency: Option<String>,
    #[validate(length(min = 1, message = "计量单位不能为空"))]
    pub unit: String,
    #[validate(length(min = 1, message = "价格类型不能为空"))]
    pub price_type: String,
    pub min_order_qty: Option<rust_decimal::Decimal>,
    pub effective_date: Option<String>,
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
    ) -> Result<(Vec<PurchasePriceView>, u64), AppError> {
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

        // 总数在无 JOIN 的基础查询上统计：所有 JOIN 均为多对一（不倍增行），单次查询无 N+1。
        let total = query.clone().count(&*self.db).await?;

        let prices = query
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::Code, "product_code")
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .join(JoinType::LeftJoin, purchase_price::Relation::Product.def())
            .join(JoinType::LeftJoin, purchase_price::Relation::Supplier.def())
            .order_by(purchase_price::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<PurchasePriceView>()
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
            currency: Set(req
                .currency
                .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            unit: Set(req.unit),
            price_type: Set(req.price_type),
            min_order_qty: Set(req.min_order_qty.unwrap_or_default()),
            effective_date: Set(req
                .effective_date
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string())
                .parse()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?),
            expiry_date: Set(req.expiry_date.and_then(|d| d.parse().ok())),
            status: Set(master_data::PENDING.to_string()),
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
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        Ok(price)
    }

    /// 批准采购价格
    pub async fn approve_price(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在批准采购价格，ID: {}", user_id, id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let price_model = purchase_price::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        let mut price: purchase_price::ActiveModel = price_model.into();
        price.status = Set(master_data::APPROVED.to_string());
        price.approved_by = Set(Some(user_id));

        // 使用 update_with_audit 在事务内同步写入审计日志
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            price,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

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
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?
            .into();

        price_model.price = Set(price);
        if let Some(ed) = expiry_date {
            price_model.expiry_date =
                Set(Some(ed.parse().map_err(|e| {
                    AppError::validation(format!("日期格式错误：{}", e))
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
            .ok_or_else(|| AppError::not_found(format!("采购价格 {} 未找到", id)))?;

        price.delete(&*self.db).await?;
        info!("采购价格删除成功，ID: {}", id);
        Ok(())
    }
}
