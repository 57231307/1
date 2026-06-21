
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, Order,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use crate::cache::CacheService;
use crate::models::product::{self, Entity as ProductEntity};
use crate::models::product_color::{self, Entity as ProductColorEntity};
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

/// 产品服务（面料行业版）
pub struct ProductService {
    db: Arc<DatabaseConnection>,
    /// 可选 Redis 缓存（P12 批 1 性能优化）
    cache: Option<Arc<CacheService>>,
}

impl ProductService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db, cache: None }
    }

    /// 创建带 Redis 缓存的产品服务（P12 批 1 性能优化）
    pub fn with_cache(db: Arc<DatabaseConnection>, cache: Arc<CacheService>) -> Self {
        Self {
            db,
            cache: Some(cache),
        }
    }

    /// 构造产品缓存键（产品表为平台级，使用 tenant_id = 0）
    fn cache_key(product_id: i32) -> String {
        CacheService::build_key(0, "product", &product_id.to_string())
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

    /// 获取产品详情（命中 Redis 时直接返回缓存）
    pub async fn get_product(&self, id: i32) -> Result<product::Model, AppError> {
        let key = Self::cache_key(id);

        // 1. 尝试读缓存
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_json::<product::Model>(&key).await {
                return Ok(cached);
            }
        }

        // 2. 缓存未命中，查 DB
        let product = ProductEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", id)))?;

        // 3. 回写缓存
        if let Some(cache) = &self.cache {
            cache.set_json(&key, &product, None).await;
        }

        Ok(product)
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
    ) -> Result<product::Model, AppError> {
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
        Ok(result)
    }

    /// 删除产品
    pub async fn delete_product(&self, id: i32) -> Result<(), AppError> {
        let result = ProductEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("产品 ID {} 不存在", id)));
        }

        // 失效缓存：产品删除后缓存值无效
        if let Some(cache) = &self.cache {
            cache.invalidate(&Self::cache_key(id)).await;
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
    ) -> Result<product::Model, AppError> {
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
            Some(0),
        )
        .await?;

        // 失效缓存：产品字段更新使缓存值陈旧
        if let Some(cache) = &self.cache {
            cache.invalidate(&Self::cache_key(id)).await;
        }

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
    #[allow(clippy::too_many_arguments)]
    pub async fn create_product_color(
        &self,
        product_id: i32,
        color_no: String,
        color_name: String,
        pantone_code: Option<String>,
        color_type: String,
        dye_formula: Option<String>,
        extra_cost: f64,
    ) -> Result<product_color::Model, AppError> {
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
    #[allow(clippy::too_many_arguments)]
    pub async fn update_product_color(
        &self,
        id: i32,
        color_name: Option<String>,
        pantone_code: Option<String>,
        color_type: Option<String>,
        dye_formula: Option<String>,
        extra_cost: Option<f64>,
        is_active: Option<bool>,
    ) -> Result<product_color::Model, AppError> {
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
            Some(0),
        )
        .await?;
        Ok(result)
    }

    /// 删除产品色号
    pub async fn delete_product_color(&self, id: i32) -> Result<(), AppError> {
        let result = ProductColorEntity::delete_by_id(id).exec(&*self.db).await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("产品色号 ID {} 不存在", id)));
        }
        Ok(())
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
        example.insert("状态".to_string(), "active".to_string());
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
                .unwrap_or_else(|| "active".to_string());

            match self
                .create_product(
                    name.clone(),
                    code.clone(),
                    category_id,
                    specification,
                    unit.clone(),
                    standard_price,
                    cost_price,
                    description,
                    status,
                    product_type.clone(),
                    fabric_composition,
                    yarn_count,
                    density,
                    width,
                    gram_weight,
                    structure,
                    finish,
                    min_order_quantity,
                    lead_time,
                )
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
