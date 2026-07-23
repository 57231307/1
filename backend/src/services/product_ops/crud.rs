//! 产品 Service CRUD 子模块（product_ops/crud）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的 6 个产品 CRUD 方法：
//! - `generate_product_code`：生成产品编码（DocumentNumberGenerator）
//! - `list_products`：分页 + 过滤查询
//! - `get_product`：详情查询（Redis 读穿透 + 写失效）
//! - `create_product`：创建（含面料行业字段，事务提交后同步 ES）
//! - `delete_product`：硬删除（审计日志 + 失效缓存 + 删除 ES 文档）
//! - `update_product`：更新（审计日志 + 失效缓存 + 同步 ES）
//!
//! 跨模块调用：
//! - create/update/delete 调用 `sync::sync_product_to_es`（`pub(crate)`）
//! - `import_export` 子模块跨模块调用 `list_products` / `create_product`（保持 `pub`）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, Order, QueryFilter, QueryOrder, Set,
};

use crate::models::product::{self, Entity as ProductEntity};
use crate::services::product_service::{CreateProductArgs, ProductService, UpdateProductArgs};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::redis_cache::{
    cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json, DEFAULT_CACHE_TTL_SECS,
};
use crate::utils::sql_escape::safe_like_pattern;

impl ProductService {
    /// 生成产品编码
    pub async fn generate_product_code(&self) -> Result<String, crate::utils::error::AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "PRD",
            product::Entity,
            product::Column::Code,
        )
        .await
    }

    /// 获取产品列表（支持分页和过滤）
    pub async fn list_products(
        &self,
        _page: u64,
        _page_size: u64,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<(Vec<product::Model>, u64), AppError> {
        let mut query = ProductEntity::find();

        // 应用过滤条件
        if let Some(cat_id) = category_id {
            query = query.filter(product::Column::CategoryId.eq(cat_id));
        }

        if let Some(s) = status {
            query = query.filter(product::Column::Status.eq(s));
        }

        if let Some(keyword) = search {
            let pattern = safe_like_pattern(&keyword);
            query = query.filter(
                product::Column::Name
                    .like(&pattern)
                    .or(product::Column::Code.like(&pattern)),
            );
        }

        // 获取总数
        let total = match query.clone().count(&*self.db).await {
            Ok(count) => {
                tracing::info!("产品查询总数: {}", count);
                count
            }
            Err(e) => {
                tracing::error!("查询产品总数失败: {:?}", e);
                return Err(AppError::from(e));
            }
        };

        // 应用分页和排序
        let products = match query
            .order_by(product::Column::Code, Order::Asc)
            .into_model::<product::Model>()
            .all(&*self.db)
            .await
            .map_err(AppError::from)
        {
            Ok(products) => {
                tracing::info!("查询到 {} 个产品", products.len());
                products
            }
            Err(e) => {
                tracing::error!("查询产品列表失败: {:?}", e);
                return Err(e);
            }
        };

        Ok((products, total))
    }

    /// 获取产品详情
    pub async fn get_product(&self, id: i32) -> Result<product::Model, AppError> {
        // P0-D03：先查 Redis 缓存
        let cache_key_str = cache_key("product", id);
        if let Some(cached) = redis_cache_get_json::<product::Model>(&cache_key_str).await {
            return Ok(cached);
        }

        // 缓存未命中 → 查询 DB
        let product = ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?;

        // 回填 Redis 缓存（5 分钟 TTL）
        redis_cache_set_json(&cache_key_str, &product, DEFAULT_CACHE_TTL_SECS).await;

        Ok(product)
    }

    /// 创建产品（面料行业版）
    ///
    /// 批次 339 v10 复审 P3 修复：签名从 19 参数改为单一参数对象 `CreateProductArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn create_product(
        &self,
        args: CreateProductArgs,
    ) -> Result<product::Model, AppError> {
        let CreateProductArgs {
            name,
            code,
            category_id,
            specification,
            unit,
            standard_price,
            cost_price,
            description,
            status,
            product_type,
            fabric_composition,
            yarn_count,
            density,
            width,
            gram_weight,
            structure,
            finish,
            min_order_quantity,
            lead_time,
        } = args;
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
            is_deleted: Set(false),
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
            supplier_product_code: sea_orm::ActiveValue::NotSet,
            supplier_id: sea_orm::ActiveValue::NotSet,
            is_batch_managed: sea_orm::ActiveValue::NotSet,
            batch_level: sea_orm::ActiveValue::NotSet,
        };

        let result = active_model.insert(&*self.db).await?;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        self.sync_product_to_es(&result, "create").await;

        Ok(result)
    }

    /// 删除产品
    pub async fn delete_product(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ProductEntity,
            _,
        >(&*self.db, "product", id, Some(user_id))
        .await?;

        // P0-D03：失效产品缓存（产品已删除）
        redis_cache_del(&cache_key("product", id)).await;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后删除 ES 文档（最终一致性）
        // 产品是硬删除，ES 文档也需删除
        if let Err(e) = self.search_syncer.delete_product(id).await {
            tracing::warn!(
                error = %e,
                product_id = id,
                operation = "delete",
                "ES 产品删除失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }

        Ok(())
    }

    /// 更新产品（面料行业版）
    ///
    /// 批次 339 v10 复审 P3 修复：签名从 19 参数改为单一参数对象 `UpdateProductArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn update_product(
        &self,
        args: UpdateProductArgs,
    ) -> Result<product::Model, AppError> {
        let UpdateProductArgs {
            id,
            name,
            specification,
            unit,
            standard_price,
            cost_price,
            description,
            status,
            product_type,
            fabric_composition,
            yarn_count,
            density,
            width,
            gram_weight,
            structure,
            finish,
            min_order_quantity,
            lead_time,
            user_id,
        } = args;
        let mut product: product::ActiveModel = ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?
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

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            product,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        // P0-D03：失效产品缓存（产品信息已更新）
        redis_cache_del(&cache_key("product", id)).await;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        self.sync_product_to_es(&result, "update").await;

        Ok(result)
    }
}
