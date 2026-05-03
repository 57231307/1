use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;

use crate::models::product::{self, Entity as ProductEntity};
use crate::models::product_color::{self, Entity as ProductColorEntity};

/// 创建产品色号输入结构体
#[derive(Debug, Clone)]
pub struct CreateProductColorInput {
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    pub color_type: String,
    pub dye_formula: Option<String>,
    pub extra_cost: f64,
}

/// 产品服务（面料行业版）
pub struct ProductService {
    db: Arc<DatabaseConnection>,
}

impl ProductService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取产品列表（支持分页和过滤）
    #[allow(unused_variables)]
    pub async fn list_products(
        &self,
        page: u64,
        page_size: u64,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<product::Model>, u64), sea_orm::DbErr> {
        let mut query = ProductEntity::find();

        // 应用过滤条件
        if let Some(cat_id) = category_id {
            query = query.filter(product::Column::CategoryId.eq(cat_id));
        }

        if let Some(s) = status {
            query = query.filter(product::Column::Status.eq(s));
        }

        if let Some(keyword) = search {
            query = query.filter(
                product::Column::Name
                    .like(format!("%{}%", keyword))
                    .or(product::Column::Code.like(format!("%{}%", keyword))),
            );
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 应用分页和排序
        let products = query
            .order_by(product::Column::Code, Order::Asc)
            .into_model::<product::Model>()
            .all(&*self.db)
            .await?;

        Ok((products, total))
    }

    /// 获取产品详情
    pub async fn get_product(&self, id: i32) -> Result<product::Model, sea_orm::DbErr> {
        ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品 ID {} 不存在", id)))
    }

    /// 创建产品（面料行业版）
    #[allow(clippy::too_many_arguments)]
    pub async fn create_product(
        &self,
        name: String,
        code: String,
        category_id: Option<i32>,
        specification: Option<String>,
        unit: String,
        standard_price: Option<f64>,
        cost_price: Option<f64>,
        description: Option<String>,
        status: String,
        // 面料行业字段
        product_type: String,
        fabric_composition: Option<String>,
        yarn_count: Option<String>,
        density: Option<String>,
        width: Option<f64>,
        gram_weight: Option<f64>,
        structure: Option<String>,
        finish: Option<String>,
        min_order_quantity: Option<f64>,
        lead_time: Option<i32>,
    ) -> Result<product::Model, sea_orm::DbErr> {
        let active_model = product::ActiveModel {
            id: NotSet,
            name: Set(name),
            code: Set(code),
            category_id: Set(category_id),
            specification: Set(specification),
            unit: Set(unit),
            standard_price: Set(
                standard_price.map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
            ),
            cost_price: Set(
                cost_price.map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
            ),
            description: Set(description),
            status: Set(status),
            // 面料行业字段
            product_type: Set(product_type),
            fabric_composition: Set(fabric_composition),
            yarn_count: Set(yarn_count),
            density: Set(density),
            width: Set(width.map(|w| Decimal::from_f64_retain(w).unwrap_or(Decimal::ZERO))),
            gram_weight: Set(
                gram_weight.map(|g| Decimal::from_f64_retain(g).unwrap_or(Decimal::ZERO))
            ),
            structure: Set(structure),
            finish: Set(finish),
            min_order_quantity: Set(
                min_order_quantity.map(|q| Decimal::from_f64_retain(q).unwrap_or(Decimal::ZERO))
            ),
            lead_time: Set(lead_time),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            is_deleted: sea_orm::ActiveValue::NotSet,
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 删除产品
    pub async fn delete_product(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        let result = ProductEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!(
                "产品 ID {} 不存在",
                id
            )));
        }
        Ok(())
    }

    /// 更新产品（面料行业版）
    #[allow(clippy::too_many_arguments)]
    pub async fn update_product(
        &self,
        id: i32,
        name: Option<String>,
        specification: Option<String>,
        unit: Option<String>,
        standard_price: Option<f64>,
        cost_price: Option<f64>,
        description: Option<String>,
        status: Option<String>,
        // 面料行业字段
        product_type: Option<String>,
        fabric_composition: Option<String>,
        yarn_count: Option<String>,
        density: Option<String>,
        width: Option<f64>,
        gram_weight: Option<f64>,
        structure: Option<String>,
        finish: Option<String>,
        min_order_quantity: Option<f64>,
        lead_time: Option<i32>,
    ) -> Result<product::Model, sea_orm::DbErr> {
        let mut product: product::ActiveModel = ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品 ID {} 不存在", id)))?
            .into();

        if let Some(n) = name {
            product.name = Set(n);
        }
        if let Some(spec) = specification {
            product.specification = Set(Some(spec));
        }
        if let Some(u) = unit {
            product.unit = Set(u);
        }
        if let Some(sp) = standard_price {
            product.standard_price =
                Set(Some(Decimal::from_f64_retain(sp).unwrap_or(Decimal::ZERO)));
        }
        if let Some(cp) = cost_price {
            product.cost_price = Set(Some(Decimal::from_f64_retain(cp).unwrap_or(Decimal::ZERO)));
        }
        if let Some(d) = description {
            product.description = Set(Some(d));
        }
        if let Some(s) = status {
            product.status = Set(s);
        }
        // 面料行业字段
        if let Some(pt) = product_type {
            product.product_type = Set(pt);
        }
        if let Some(fc) = fabric_composition {
            product.fabric_composition = Set(Some(fc));
        }
        if let Some(yc) = yarn_count {
            product.yarn_count = Set(Some(yc));
        }
        if let Some(den) = density {
            product.density = Set(Some(den));
        }
        if let Some(w) = width {
            product.width = Set(Some(Decimal::from_f64_retain(w).unwrap_or(Decimal::ZERO)));
        }
        if let Some(gw) = gram_weight {
            product.gram_weight = Set(Some(Decimal::from_f64_retain(gw).unwrap_or(Decimal::ZERO)));
        }
        if let Some(st) = structure {
            product.structure = Set(Some(st));
        }
        if let Some(fi) = finish {
            product.finish = Set(Some(fi));
        }
        if let Some(moq) = min_order_quantity {
            product.min_order_quantity =
                Set(Some(Decimal::from_f64_retain(moq).unwrap_or(Decimal::ZERO)));
        }
        if let Some(lt) = lead_time {
            product.lead_time = Set(Some(lt));
        }

        product.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", product, Some(0)).await?;
        Ok(result)
    }

    // ========== 色号管理方法 ==========

    /// 获取产品的色号列表
    pub async fn list_product_colors(
        &self,
        product_id: i32,
    ) -> Result<Vec<product_color::Model>, sea_orm::DbErr> {
        ProductColorEntity::find()
            .filter(product_color::Column::ProductId.eq(product_id))
            .filter(product_color::Column::IsActive.eq(true))
            .order_by(product_color::Column::ColorNo, Order::Asc)
            .all(&*self.db)
            .await
    }

    /// 创建产品色号
    pub async fn create_product_color(
        &self,
        product_id: i32,
        color_no: String,
        color_name: String,
        pantone_code: Option<String>,
        color_type: String,
        dye_formula: Option<String>,
        extra_cost: f64,
    ) -> Result<product_color::Model, sea_orm::DbErr> {
        let active_model = product_color::ActiveModel {
            id: Set(0),
            product_id: Set(product_id),
            color_no: Set(color_no),
            color_name: Set(color_name),
            pantone_code: Set(pantone_code),
            color_type: Set(color_type),
            dye_formula: Set(dye_formula),
            extra_cost: Set(Decimal::from_f64_retain(extra_cost).unwrap_or(Decimal::ZERO)),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            is_deleted: sea_orm::ActiveValue::NotSet,
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 批量创建产品色号
    pub async fn batch_create_product_colors(
        &self,
        product_id: i32,
        colors: Vec<CreateProductColorInput>,
    ) -> Result<Vec<product_color::Model>, sea_orm::DbErr> {
        let mut results = Vec::new();

        for input in colors {
            let result = self
                .create_product_color(
                    product_id,
                    input.color_no,
                    input.color_name,
                    input.pantone_code,
                    input.color_type,
                    input.dye_formula,
                    input.extra_cost,
                )
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 更新产品色号
    pub async fn update_product_color(
        &self,
        id: i32,
        color_name: Option<String>,
        pantone_code: Option<String>,
        color_type: Option<String>,
        dye_formula: Option<String>,
        extra_cost: Option<f64>,
        is_active: Option<bool>,
    ) -> Result<product_color::Model, sea_orm::DbErr> {
        let mut color: product_color::ActiveModel = ProductColorEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("产品色号 ID {} 不存在", id)))?
            .into();

        if let Some(cn) = color_name {
            color.color_name = Set(cn);
        }
        if let Some(pc) = pantone_code {
            color.pantone_code = Set(Some(pc));
        }
        if let Some(ct) = color_type {
            color.color_type = Set(ct);
        }
        if let Some(df) = dye_formula {
            color.dye_formula = Set(Some(df));
        }
        if let Some(ec) = extra_cost {
            color.extra_cost = Set(Decimal::from_f64_retain(ec).unwrap_or(Decimal::ZERO));
        }
        if let Some(active) = is_active {
            color.is_active = Set(active);
        }

        color.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(&*self.db, "auto_audit", color, Some(0)).await?;
        Ok(result)
    }

    /// 删除产品色号
    pub async fn delete_product_color(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        let result = ProductColorEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(sea_orm::DbErr::RecordNotFound(format!(
                "产品色号 ID {} 不存在",
                id
            )));
        }
        Ok(())
    }

    /// 根据色号查询产品色号
    pub async fn find_color_by_product_and_color_no(
        &self,
        product_id: i32,
        color_no: &str,
    ) -> Result<product_color::Model, sea_orm::DbErr> {
        ProductColorEntity::find()
            .filter(product_color::Column::ProductId.eq(product_id))
            .filter(product_color::Column::ColorNo.eq(color_no))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!(
                    "产品 {} 的色号 {} 不存在",
                    product_id, color_no
                ))
            })
    }
}
