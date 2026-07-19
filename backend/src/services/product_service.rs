
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use crate::models::product::{self, Entity as ProductEntity};
use crate::models::product_color::{self, Entity as ProductColorEntity};
// 批次 211 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::search::{ProductDoc, SearchClient, SearchSyncer};
// P0-D03（Batch 488）：Redis 分布式缓存接入（get_product 读穿透 + 写失效）
use crate::utils::redis_cache::{
    cache_key, redis_cache_del, redis_cache_get_json, redis_cache_set_json, DEFAULT_CACHE_TTL_SECS,
};
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;

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

/// 更新产品色号参数对象
///
/// 批次 330 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告
#[derive(Debug)]
pub struct UpdateProductColorParams {
    /// 色号 ID
    pub id: i32,
    /// 色号名称
    pub color_name: Option<String>,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// 色号类型
    pub color_type: Option<String>,
    /// 染色配方
    pub dye_formula: Option<String>,
    /// 额外成本
    pub extra_cost: Option<f64>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 操作人 ID（审计日志）
    pub user_id: i32,
}

/// 产品服务（面料行业版）
///
/// 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
/// - create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES
/// - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
pub struct ProductService {
    db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 125 接入
    search_syncer: Arc<SearchSyncer>,
}

/// 创建产品参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 create_product 的 too_many_arguments 警告。
/// 聚合创建产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct CreateProductArgs {
    /// 产品名称
    pub name: String,
    /// 产品编码
    pub code: String,
    /// 分类 ID
    pub category_id: Option<i32>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: String,
    /// 标准价
    pub standard_price: Option<f64>,
    /// 成本价
    pub cost_price: Option<f64>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: String,
    /// 产品类型（面料行业字段）
    pub product_type: String,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽
    pub width: Option<f64>,
    /// 克重
    pub gram_weight: Option<f64>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量
    pub min_order_quantity: Option<f64>,
    /// 交期（天）
    pub lead_time: Option<i32>,
}

/// 更新产品参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 update_product 的 too_many_arguments 警告。
/// 聚合更新产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct UpdateProductArgs {
    /// 产品 ID
    pub id: i32,
    /// 产品名称
    pub name: Option<String>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: Option<String>,
    /// 标准价
    pub standard_price: Option<f64>,
    /// 成本价
    pub cost_price: Option<f64>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: Option<String>,
    /// 产品类型
    pub product_type: Option<String>,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽
    pub width: Option<f64>,
    /// 克重
    pub gram_weight: Option<f64>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量
    pub min_order_quantity: Option<f64>,
    /// 交期（天）
    pub lead_time: Option<i32>,
    /// 操作人 ID
    pub user_id: i32,
}

impl ProductService {
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }

    /// 将 product::Model 转换为 ProductDoc 用于 ES 索引
    ///
    /// 批次 125 v8 复审 P1 修复：字段映射规则
    /// - category: join product_category 表取 name（category_id 索引意义不大）
    /// - color_no/pantone_code: 暂设 None（一对多关联复杂，后续迭代优化）
    /// - price: standard_price Decimal → f64
    /// - spec: specification 字段
    fn build_product_doc(&self, model: &product::Model) -> ProductDoc {
        ProductDoc {
            id: model.id,
            code: model.code.clone(),
            name: model.name.clone(),
            category: None, // 批次 125：暂设 None，后续迭代 join product_category
            spec: model.specification.clone(),
            unit: model.unit.clone(),
            color_no: None, // 批次 125：暂设 None，后续迭代 join product_color
            pantone_code: None, // 批次 125：暂设 None，后续迭代 join product_color
            price: model
                .standard_price
                .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0),
        }
    }

    /// 同步产品到 ES（最终一致性策略）
    ///
    /// 批次 125 v8 复审 P1 修复：ES 同步失败仅记录日志，不回滚 PG 事务。
    async fn sync_product_to_es(&self, model: &product::Model, operation: &str) {
        let doc = self.build_product_doc(model);
        if let Err(e) = self.search_syncer.sync_product(&doc).await {
            tracing::warn!(
                error = %e,
                product_id = model.id,
                product_code = %model.code,
                operation = operation,
                "ES 产品同步失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }
    }

    /// 生成产品编码
    pub async fn generate_product_code(&self) -> Result<String, crate::utils::error::AppError> {
        use crate::utils::number_generator::DocumentNumberGenerator;
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

    // ========== 色号管理方法 ==========

    /// 获取产品的色号列表
    pub async fn list_product_colors(
        &self,
        product_id: i32,
    ) -> Result<Vec<product_color::Model>, AppError> {
        ProductColorEntity::find()
            .filter(product_color::Column::ProductId.eq(product_id))
            .filter(product_color::Column::IsActive.eq(true))
            .order_by(product_color::Column::ColorNo, Order::Asc)
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 创建产品色号
    ///
    /// 批次 412 技术债务清理：签名从 7 参数改为 2 参数（product_id + CreateProductColorInput），
    /// 复用已有的 `CreateProductColorInput` 参数对象，消除 `clippy::too_many_arguments` 警告。
    pub async fn create_product_color(
        &self,
        product_id: i32,
        input: CreateProductColorInput,
    ) -> Result<product_color::Model, AppError> {
        let active_model = product_color::ActiveModel {
            id: Default::default(),
            product_id: Set(product_id),
            color_no: Set(input.color_no),
            color_name: Set(input.color_name),
            pantone_code: Set(input.pantone_code),
            color_type: Set(input.color_type),
            dye_formula: Set(input.dye_formula),
            extra_cost: Set(Decimal::from_f64_retain(input.extra_cost).unwrap_or(Decimal::ZERO)),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(result)
    }

    /// 批量创建产品色号
    pub async fn batch_create_product_colors(
        &self,
        product_id: i32,
        colors: Vec<CreateProductColorInput>,
    ) -> Result<Vec<product_color::Model>, AppError> {
        let mut results = Vec::new();

        for input in colors {
            let result = self.create_product_color(product_id, input).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 更新产品色号
    ///
    /// 批次 330 v10 复审 P3 修复：使用 UpdateProductColorParams 参数对象替代 8 个独立参数
    pub async fn update_product_color(
        &self,
        params: UpdateProductColorParams,
    ) -> Result<product_color::Model, AppError> {
        let id = params.id;
        let color_name = params.color_name;
        let pantone_code = params.pantone_code;
        let color_type = params.color_type;
        let dye_formula = params.dye_formula;
        let extra_cost = params.extra_cost;
        let is_active = params.is_active;
        let user_id = params.user_id;

        let mut color: product_color::ActiveModel = ProductColorEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品色号 ID {} 不存在", id)))?
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

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            color,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;
        Ok(result)
    }

    /// 删除产品色号
    pub async fn delete_product_color(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ProductColorEntity,
            _,
        >(&*self.db, "product_color", id, Some(user_id))
        .await
    }

    // ========== 数据导入导出方法 ==========

    /// 导出产品数据为 CSV 格式
    pub async fn export_products_to_csv(
        &self,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<u8>, AppError> {
        let (products, _total) = self
            .list_products(1, 10000, category_id, status, search)
            .await?;

        let headers = vec![
            "产品编码".to_string(),
            "产品名称".to_string(),
            "产品类型".to_string(),
            "类别ID".to_string(),
            "规格型号".to_string(),
            "计量单位".to_string(),
            "标准价格".to_string(),
            "成本价格".to_string(),
            "面料成分".to_string(),
            "纱支".to_string(),
            "密度".to_string(),
            "幅宽".to_string(),
            "克重".to_string(),
            "组织结构".to_string(),
            "后整理".to_string(),
            "最小起订量".to_string(),
            "交货期".to_string(),
            "状态".to_string(),
            "产品描述".to_string(),
        ];

        let rows: Vec<std::collections::HashMap<String, String>> = products
            .into_iter()
            .map(|p| {
                let mut row = std::collections::HashMap::new();
                row.insert("产品编码".to_string(), p.code);
                row.insert("产品名称".to_string(), p.name);
                row.insert("产品类型".to_string(), p.product_type);
                row.insert(
                    "类别ID".to_string(),
                    p.category_id.map(|id| id.to_string()).unwrap_or_default(),
                );
                row.insert("规格型号".to_string(), p.specification.unwrap_or_default());
                row.insert("计量单位".to_string(), p.unit);
                row.insert(
                    "标准价格".to_string(),
                    p.standard_price.map(|p| p.to_string()).unwrap_or_default(),
                );
                row.insert(
                    "成本价格".to_string(),
                    p.cost_price.map(|p| p.to_string()).unwrap_or_default(),
                );
                row.insert(
                    "面料成分".to_string(),
                    p.fabric_composition.unwrap_or_default(),
                );
                row.insert("纱支".to_string(), p.yarn_count.unwrap_or_default());
                row.insert("密度".to_string(), p.density.unwrap_or_default());
                row.insert(
                    "幅宽".to_string(),
                    p.width.map(|w| w.to_string()).unwrap_or_default(),
                );
                row.insert(
                    "克重".to_string(),
                    p.gram_weight.map(|g| g.to_string()).unwrap_or_default(),
                );
                row.insert("组织结构".to_string(), p.structure.unwrap_or_default());
                row.insert("后整理".to_string(), p.finish.unwrap_or_default());
                row.insert(
                    "最小起订量".to_string(),
                    p.min_order_quantity
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "交货期".to_string(),
                    p.lead_time.map(|l| l.to_string()).unwrap_or_default(),
                );
                row.insert("状态".to_string(), p.status);
                row.insert("产品描述".to_string(), p.description.unwrap_or_default());
                row
            })
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::business(format!("CSV 生成失败: {}", e)))
    }

    /// 生成产品导入模板
    pub fn generate_product_import_template() -> Result<Vec<u8>, AppError> {
        let headers = vec![
            "产品编码".to_string(),
            "产品名称".to_string(),
            "产品类型".to_string(),
            "类别ID".to_string(),
            "规格型号".to_string(),
            "计量单位".to_string(),
            "标准价格".to_string(),
            "成本价格".to_string(),
            "面料成分".to_string(),
            "纱支".to_string(),
            "密度".to_string(),
            "幅宽".to_string(),
            "克重".to_string(),
            "组织结构".to_string(),
            "后整理".to_string(),
            "最小起订量".to_string(),
            "交货期".to_string(),
            "状态".to_string(),
            "产品描述".to_string(),
        ];

        let mut example = std::collections::HashMap::new();
        example.insert("产品编码".to_string(), "FAB-001".to_string());
        example.insert("产品名称".to_string(), "纯棉坯布".to_string());
        example.insert("产品类型".to_string(), "坯布".to_string());
        example.insert("类别ID".to_string(), "1".to_string());
        example.insert("规格型号".to_string(), "40S*40S".to_string());
        example.insert("计量单位".to_string(), "米".to_string());
        example.insert("标准价格".to_string(), "15.50".to_string());
        example.insert("成本价格".to_string(), "12.00".to_string());
        example.insert("面料成分".to_string(), "100%棉".to_string());
        example.insert("纱支".to_string(), "40S".to_string());
        example.insert("密度".to_string(), "133*72".to_string());
        example.insert("幅宽".to_string(), "150.00".to_string());
        example.insert("克重".to_string(), "120.00".to_string());
        example.insert("组织结构".to_string(), "平纹".to_string());
        example.insert("后整理".to_string(), "防水".to_string());
        example.insert("最小起订量".to_string(), "1000".to_string());
        example.insert("交货期".to_string(), "15".to_string());
        example.insert("状态".to_string(), master_data::ACTIVE.to_string());
        example.insert("产品描述".to_string(), "高品质纯棉坯布".to_string());

        crate::utils::import_export::CsvImporter::generate_template(&headers, Some(&[example]))
            .map_err(|e| AppError::business(format!("模板生成失败: {}", e)))
    }

    /// 从产品 CSV 数据导入
    pub async fn import_products_from_csv(
        &self,
        data: &[u8],
    ) -> Result<crate::utils::import_export::ImportResult, AppError> {
        use crate::utils::import_export::{CsvImporter, FieldValidator, ImportResult};

        let records = CsvImporter::parse(data)?;
        let mut result = ImportResult::new();

        for (row_idx, row) in records.iter().enumerate() {
            result.add_total();
            let row_num = row_idx + 2;

            let code = match row.get("产品编码") {
                Some(v) => v,
                None => {
                    result.add_error(
                        row_num,
                        "产品编码".to_string(),
                        "缺少产品编码列".to_string(),
                        "".to_string(),
                    );
                    continue;
                }
            };
            if let Err(e) = FieldValidator::required(code, "产品编码") {
                result.add_error(row_num, "产品编码".to_string(), e, code.clone());
                continue;
            }
            // v11 批次 156 P2-D：接入 FieldValidator::max_length 校验编码长度
            if let Err(e) = FieldValidator::max_length(code, "产品编码", 50) {
                result.add_error(row_num, "产品编码".to_string(), e, code.clone());
                continue;
            }

            let name = match row.get("产品名称") {
                Some(v) => v,
                None => {
                    result.add_error(
                        row_num,
                        "产品名称".to_string(),
                        "缺少产品名称列".to_string(),
                        "".to_string(),
                    );
                    continue;
                }
            };
            if let Err(e) = FieldValidator::required(name, "产品名称") {
                result.add_error(row_num, "产品名称".to_string(), e, name.clone());
                continue;
            }
            // v11 批次 156 P2-D：接入 FieldValidator::max_length 校验名称长度
            if let Err(e) = FieldValidator::max_length(name, "产品名称", 200) {
                result.add_error(row_num, "产品名称".to_string(), e, name.clone());
                continue;
            }

            let product_type = match row.get("产品类型") {
                Some(v) => v,
                None => {
                    result.add_error(
                        row_num,
                        "产品类型".to_string(),
                        "缺少产品类型列".to_string(),
                        "".to_string(),
                    );
                    continue;
                }
            };
            if let Err(e) =
                FieldValidator::enum_value(product_type, "产品类型", &["坯布", "成品布", "辅料"])
            {
                result.add_error(row_num, "产品类型".to_string(), e, product_type.clone());
                continue;
            }

            let unit = match row.get("计量单位") {
                Some(v) => v,
                None => {
                    result.add_error(
                        row_num,
                        "计量单位".to_string(),
                        "缺少计量单位列".to_string(),
                        "".to_string(),
                    );
                    continue;
                }
            };
            if let Err(e) = FieldValidator::required(unit, "计量单位") {
                result.add_error(row_num, "计量单位".to_string(), e, unit.clone());
                continue;
            }

            let category_id = row.get("类别ID").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<i32>().ok()
                }
            });

            let standard_price = row.get("标准价格").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<f64>().ok()
                }
            });

            let cost_price = row.get("成本价格").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<f64>().ok()
                }
            });

            let specification = row.get("规格型号").filter(|v| !v.is_empty()).cloned();
            let fabric_composition = row.get("面料成分").filter(|v| !v.is_empty()).cloned();
            let yarn_count = row.get("纱支").filter(|v| !v.is_empty()).cloned();
            let density = row.get("密度").filter(|v| !v.is_empty()).cloned();
            let width = row.get("幅宽").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<f64>().ok()
                }
            });
            let gram_weight = row.get("克重").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<f64>().ok()
                }
            });
            let structure = row.get("组织结构").filter(|v| !v.is_empty()).cloned();
            let finish = row.get("后整理").filter(|v| !v.is_empty()).cloned();
            let min_order_quantity = row.get("最小起订量").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<f64>().ok()
                }
            });
            let lead_time = row.get("交货期").and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    v.parse::<i32>().ok()
                }
            });
            let description = row.get("产品描述").filter(|v| !v.is_empty()).cloned();

            let status = row
                .get("状态")
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
                .unwrap_or_else(|| master_data::ACTIVE.to_string());

            match self
                .create_product(CreateProductArgs {
                    name: name.clone(),
                    code: code.clone(),
                    category_id,
                    specification,
                    unit: unit.clone(),
                    standard_price,
                    cost_price,
                    description,
                    status,
                    product_type: product_type.clone(),
                    fabric_composition,
                    yarn_count,
                    density,
                    width,
                    gram_weight,
                    structure,
                    finish,
                    min_order_quantity,
                    lead_time,
                })
                .await
            {
                Ok(_) => result.add_success(),
                Err(e) => {
                    result.add_error(
                        row_num,
                        "数据库".to_string(),
                        format!("创建产品失败: {}", e),
                        code.clone(),
                    );
                }
            }
        }

        Ok(result)
    }
}
