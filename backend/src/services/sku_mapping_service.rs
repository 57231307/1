//! 供应商商品/色号对照表服务（sku_mapping_service）
//!
//! 提供 product_supplier_mappings 表的 CRUD、批量导入（UPSERT）及转采购 SKU 解析。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, JoinType, NotSet, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Set,
};
use serde::Serialize;
use std::sync::Arc;

use crate::models::{
    product, product_color, product_supplier_mapping, supplier, supplier_product,
    supplier_product_color,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

// =====================================================
// DTO（JOIN 富化出参）
// =====================================================

/// 对照表视图对象（单次 LEFT JOIN 查询填充，无 N+1）
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct SkuMappingDto {
    pub id: i32,
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub is_primary: bool,
    pub priority: i32,
    pub supplier_price: Option<rust_decimal::Decimal>,
    pub min_order_quantity: Option<rust_decimal::Decimal>,
    pub lead_time: Option<i32>,
    pub is_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<i32>,
    pub updated_by: Option<i32>,
    pub remarks: Option<String>,
    // ---- JOIN 富化字段 ----
    /// 我方产品编码
    pub product_code: Option<String>,
    /// 我方色号编码
    pub color_no: Option<String>,
    /// 供应商名称
    pub supplier_name: Option<String>,
    /// 供应商商品编码
    pub supplier_product_code: Option<String>,
    /// 供应商色号
    pub supplier_color_no: Option<String>,
}

/// resolve 返回结果
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedSku {
    pub mapping_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_product_code: String,
    pub supplier_color_no: Option<String>,
    pub supplier_price: Option<rust_decimal::Decimal>,
    pub lead_time: Option<i32>,
}

/// 创建/更新输入
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpsertSkuMappingInput {
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_price: Option<rust_decimal::Decimal>,
    pub min_order_quantity: Option<rust_decimal::Decimal>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

/// 列表查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SkuMappingQueryParams {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub is_enabled: Option<bool>,
    /// 按我方色号模糊搜索（高基数色号场景）。
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 导入单行数据（JSON body 中的每行）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImportMappingRow {
    pub product_code: String,
    pub color_no: Option<String>,
    pub supplier_code: String,
    pub supplier_product_code: String,
    pub supplier_color_no: Option<String>,
    pub supplier_price: Option<rust_decimal::Decimal>,
    pub min_order_quantity: Option<rust_decimal::Decimal>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

/// 导入结果（照 utils/import_export.rs 形状）
#[derive(Debug, Clone, Serialize)]
pub struct ImportMappingResult {
    pub total_count: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub errors: Vec<crate::utils::import_export::ImportError>,
}

// =====================================================
// Service
// =====================================================

pub struct SkuMappingService {
    db: Arc<DatabaseConnection>,
}

impl SkuMappingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 构建带 JOIN 富化的基查询
    fn base_dto_query() -> sea_orm::Select<product_supplier_mapping::Entity> {
        product_supplier_mapping::Entity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product_color::Column::ColorNo, "color_no")
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(
                supplier_product::Column::ProductCode,
                "supplier_product_code",
            )
            .column_as(supplier_product_color::Column::ColorNo, "supplier_color_no")
            .join(
                JoinType::LeftJoin,
                product_supplier_mapping::Relation::Product.def(),
            )
            .join(
                JoinType::LeftJoin,
                product_supplier_mapping::Relation::ProductColor.def(),
            )
            .join(
                JoinType::LeftJoin,
                product_supplier_mapping::Relation::Supplier.def(),
            )
            .join(
                JoinType::LeftJoin,
                product_supplier_mapping::Relation::SupplierProduct.def(),
            )
            .join(
                JoinType::LeftJoin,
                product_supplier_mapping::Relation::SupplierProductColor.def(),
            )
    }

    /// 列表（分页）
    pub async fn list(
        &self,
        params: SkuMappingQueryParams,
    ) -> Result<(Vec<SkuMappingDto>, u64), AppError> {
        let page = params.page.unwrap_or(1).clamp(1, 1000);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let mut query = Self::base_dto_query();

        if let Some(pid) = params.product_id {
            query = query.filter(product_supplier_mapping::Column::ProductId.eq(pid));
        }
        if let Some(sid) = params.supplier_id {
            query = query.filter(product_supplier_mapping::Column::SupplierId.eq(sid));
        }
        if let Some(enabled) = params.is_enabled {
            query = query.filter(product_supplier_mapping::Column::IsEnabled.eq(enabled));
        }
        // 按我方色号模糊搜索（色号基数高时的检索入口）；空串由请求侧剔除。
        if let Some(kw) = params
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            query = query.filter(product_color::Column::ColorNo.contains(kw));
        }

        query = query.order_by(product_supplier_mapping::Column::Priority, Order::Asc);

        let paginator = query
            .into_model::<SkuMappingDto>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page).await?;

        Ok((items, total))
    }

    /// 单条详情
    pub async fn get(&self, id: i32) -> Result<SkuMappingDto, AppError> {
        let dto = Self::base_dto_query()
            .filter(product_supplier_mapping::Column::Id.eq(id))
            .into_model::<SkuMappingDto>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("SKU 对照记录 {}", id)))?;
        Ok(dto)
    }

    /// 创建对照
    pub async fn create(
        &self,
        input: UpsertSkuMappingInput,
        user_id: i32,
    ) -> Result<SkuMappingDto, AppError> {
        self.validate_refs(&input).await?;

        let now = chrono::Utc::now();
        let active = product_supplier_mapping::ActiveModel {
            id: NotSet,
            product_id: Set(input.product_id),
            product_color_id: Set(input.product_color_id),
            supplier_id: Set(input.supplier_id),
            supplier_product_id: Set(input.supplier_product_id),
            supplier_product_color_id: Set(input.supplier_product_color_id),
            is_primary: Set(input.is_primary.unwrap_or(false)),
            priority: Set(input.priority.unwrap_or(1)),
            supplier_price: Set(input.supplier_price),
            min_order_quantity: Set(input.min_order_quantity),
            lead_time: Set(input.lead_time),
            is_enabled: Set(input.is_enabled.unwrap_or(true)),
            remarks: Set(input.remarks),
            created_at: Set(now),
            updated_at: Set(now),
            created_by: Set(Some(user_id)),
            updated_by: Set(Some(user_id)),
        };
        let inserted = active.insert(&*self.db).await.map_err(|e| {
            if is_unique_violation(&e) {
                AppError::business("该产品+色号+供应商组合的对照记录已存在，请勿重复创建")
            } else {
                AppError::from(e)
            }
        })?;

        self.get(inserted.id).await
    }

    /// 更新对照
    pub async fn update(
        &self,
        id: i32,
        input: UpsertSkuMappingInput,
        user_id: i32,
    ) -> Result<SkuMappingDto, AppError> {
        let existing = product_supplier_mapping::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("SKU 对照记录 {}", id)))?;

        self.validate_refs(&input).await?;

        let mut active: product_supplier_mapping::ActiveModel = existing.into();
        active.product_id = Set(input.product_id);
        active.product_color_id = Set(input.product_color_id);
        active.supplier_id = Set(input.supplier_id);
        active.supplier_product_id = Set(input.supplier_product_id);
        active.supplier_product_color_id = Set(input.supplier_product_color_id);
        active.is_primary = Set(input.is_primary.unwrap_or(active.is_primary.unwrap()));
        active.priority = Set(input.priority.unwrap_or(active.priority.unwrap()));
        active.supplier_price = Set(input.supplier_price);
        active.min_order_quantity = Set(input.min_order_quantity);
        active.lead_time = Set(input.lead_time);
        active.is_enabled = Set(input.is_enabled.unwrap_or(active.is_enabled.unwrap()));
        active.remarks = Set(input.remarks);
        active.updated_at = Set(chrono::Utc::now());
        active.updated_by = Set(Some(user_id));

        let updated = active.update(&*self.db).await.map_err(|e| {
            if is_unique_violation(&e) {
                AppError::business("该产品+色号+供应商组合的对照记录已存在，请勿重复创建")
            } else {
                AppError::from(e)
            }
        })?;

        self.get(updated.id).await
    }

    /// 删除对照
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let result = product_supplier_mapping::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!("SKU 对照记录 {}", id)));
        }
        Ok(())
    }

    /// 解析供应商 SKU（转采购翻译核心方法）
    ///
    /// 按 (product_id, product_color_id, supplier_id) 查启用映射，
    /// 找到 priority 最小的一条返回 `Some`；找不到返回 `Ok(None)`。
    /// 由调用方按受众决定措辞（转采购可见面须中性，勿泄露调货/自制）。
    pub async fn resolve_supplier_sku(
        &self,
        product_id: i32,
        product_color_id: Option<i32>,
        supplier_id: i32,
    ) -> Result<Option<ResolvedSku>, AppError> {
        let mut query = product_supplier_mapping::Entity::find()
            .filter(product_supplier_mapping::Column::ProductId.eq(product_id))
            .filter(product_supplier_mapping::Column::SupplierId.eq(supplier_id))
            .filter(product_supplier_mapping::Column::IsEnabled.eq(true));

        // product_color_id 为 None 时查 product_color_id IS NULL 的行
        match product_color_id {
            Some(cid) => {
                query = query.filter(product_supplier_mapping::Column::ProductColorId.eq(cid));
            }
            None => {
                query = query.filter(product_supplier_mapping::Column::ProductColorId.is_null());
            }
        }

        let Some(mapping) = query
            .order_by(product_supplier_mapping::Column::Priority, Order::Asc)
            .one(&*self.db)
            .await?
        else {
            return Ok(None);
        };

        // 取供应商商品编码和色号
        let sp = supplier_product::Entity::find_by_id(mapping.supplier_product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::internal("供应商商品记录数据异常：关联不存在"))?;

        let supplier_color_no = match mapping.supplier_product_color_id {
            Some(spc_id) => {
                let spc = supplier_product_color::Entity::find_by_id(spc_id)
                    .one(&*self.db)
                    .await?;
                spc.map(|c| c.color_no)
            }
            None => None,
        };

        Ok(Some(ResolvedSku {
            mapping_id: mapping.id,
            supplier_product_id: mapping.supplier_product_id,
            supplier_product_color_id: mapping.supplier_product_color_id,
            supplier_product_code: sp.product_code,
            supplier_color_no,
            supplier_price: mapping.supplier_price,
            lead_time: mapping.lead_time,
        }))
    }

    /// 批量导入（UPSERT 幂等：ON CONFLICT (product_id, product_color_id, supplier_id) DO UPDATE）
    pub async fn import_batch(
        &self,
        rows: Vec<ImportMappingRow>,
        user_id: i32,
    ) -> Result<ImportMappingResult, AppError> {
        let mut result = ImportMappingResult {
            total_count: rows.len(),
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
        };

        for (idx, row) in rows.iter().enumerate() {
            let row_num = idx + 1;

            // 校验 product_code 存在
            let prod = match product::Entity::find()
                .filter(product::Column::Code.eq(&row.product_code))
                .one(&*self.db)
                .await
            {
                Ok(Some(p)) => p,
                Ok(None) => {
                    result.error_count += 1;
                    result
                        .errors
                        .push(crate::utils::import_export::ImportError {
                            row: row_num,
                            column: "product_code".to_string(),
                            message: "产品编码不存在".to_string(),
                            value: row.product_code.clone(),
                        });
                    continue;
                }
                Err(e) => return Err(AppError::from(e)),
            };

            // 校验 color_no 对应的 product_color_id
            let product_color_id = match &row.color_no {
                Some(cn) if !cn.is_empty() => {
                    match product_color::Entity::find()
                        .filter(product_color::Column::ProductId.eq(prod.id))
                        .filter(product_color::Column::ColorNo.eq(cn))
                        .one(&*self.db)
                        .await
                    {
                        Ok(Some(pc)) => Some(pc.id),
                        Ok(None) => {
                            result.error_count += 1;
                            result
                                .errors
                                .push(crate::utils::import_export::ImportError {
                                    row: row_num,
                                    column: "color_no".to_string(),
                                    message: "该产品的色号不存在".to_string(),
                                    value: cn.clone(),
                                });
                            continue;
                        }
                        Err(e) => return Err(AppError::from(e)),
                    }
                }
                _ => None,
            };

            // 校验 supplier_code 存在
            let sup = match supplier::Entity::find()
                .filter(supplier::Column::SupplierCode.eq(&row.supplier_code))
                .one(&*self.db)
                .await
            {
                Ok(Some(s)) => s,
                Ok(None) => {
                    result.error_count += 1;
                    result
                        .errors
                        .push(crate::utils::import_export::ImportError {
                            row: row_num,
                            column: "supplier_code".to_string(),
                            message: "供应商编码不存在".to_string(),
                            value: row.supplier_code.clone(),
                        });
                    continue;
                }
                Err(e) => return Err(AppError::from(e)),
            };

            // 校验 supplier_product_code 存在
            let sup_prod = match supplier_product::Entity::find()
                .filter(supplier_product::Column::SupplierId.eq(sup.id))
                .filter(supplier_product::Column::ProductCode.eq(&row.supplier_product_code))
                .one(&*self.db)
                .await
            {
                Ok(Some(sp)) => sp,
                Ok(None) => {
                    result.error_count += 1;
                    result
                        .errors
                        .push(crate::utils::import_export::ImportError {
                            row: row_num,
                            column: "supplier_product_code".to_string(),
                            message: "供应商商品编码不存在".to_string(),
                            value: row.supplier_product_code.clone(),
                        });
                    continue;
                }
                Err(e) => return Err(AppError::from(e)),
            };

            // 校验 supplier_color_no 对应的 supplier_product_color_id
            let supplier_product_color_id = match &row.supplier_color_no {
                Some(scn) if !scn.is_empty() => {
                    match supplier_product_color::Entity::find()
                        .filter(supplier_product_color::Column::SupplierProductId.eq(sup_prod.id))
                        .filter(supplier_product_color::Column::ColorNo.eq(scn))
                        .one(&*self.db)
                        .await
                    {
                        Ok(Some(spc)) => Some(spc.id),
                        Ok(None) => {
                            result.error_count += 1;
                            result
                                .errors
                                .push(crate::utils::import_export::ImportError {
                                    row: row_num,
                                    column: "supplier_color_no".to_string(),
                                    message: "供应商色号不存在".to_string(),
                                    value: scn.clone(),
                                });
                            continue;
                        }
                        Err(e) => return Err(AppError::from(e)),
                    }
                }
                _ => None,
            };

            // UPSERT: INSERT ... ON CONFLICT (product_id, product_color_id, supplier_id) DO UPDATE
            let now = chrono::Utc::now();
            let insert_result = Self::upsert_mapping(
                &*self.db,
                prod.id,
                product_color_id,
                sup.id,
                sup_prod.id,
                supplier_product_color_id,
                row.supplier_price,
                row.min_order_quantity,
                row.lead_time,
                row.is_primary.unwrap_or(false),
                row.priority.unwrap_or(1),
                row.is_enabled.unwrap_or(true),
                row.remarks.clone(),
                user_id,
                now,
            )
            .await;

            match insert_result {
                Ok(()) => {
                    result.success_count += 1;
                }
                Err(e) => {
                    result.error_count += 1;
                    result
                        .errors
                        .push(crate::utils::import_export::ImportError {
                            row: row_num,
                            column: "_".to_string(),
                            message: format!("写入失败: {}", e),
                            value: format!(
                                "{}|{}|{}",
                                row.product_code,
                                row.color_no.as_deref().unwrap_or(""),
                                row.supplier_code
                            ),
                        });
                }
            }
        }

        tracing::info!(
            "SKU 对照表批量导入完成: total={}, success={}, errors={}",
            result.total_count,
            result.success_count,
            result.error_count
        );
        Ok(result)
    }

    /// 执行 UPSERT（ON CONFLICT DO UPDATE）
    #[allow(clippy::too_many_arguments)]
    async fn upsert_mapping(
        db: &DatabaseConnection,
        product_id: i32,
        product_color_id: Option<i32>,
        supplier_id: i32,
        supplier_product_id: i32,
        supplier_product_color_id: Option<i32>,
        supplier_price: Option<rust_decimal::Decimal>,
        min_order_quantity: Option<rust_decimal::Decimal>,
        lead_time: Option<i32>,
        is_primary: bool,
        priority: i32,
        is_enabled: bool,
        remarks: Option<String>,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sea_orm::DbErr> {
        use sea_orm::{DbBackend, Statement, Value};

        let sql = r#"
            INSERT INTO product_supplier_mappings
                (product_id, product_color_id, supplier_id, supplier_product_id,
                 supplier_product_color_id, supplier_price, min_order_quantity, lead_time,
                 is_primary, priority, is_enabled, remarks, created_at, updated_at, created_by, updated_by)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            ON CONFLICT (product_id, product_color_id, supplier_id) DO UPDATE SET
                supplier_product_id = EXCLUDED.supplier_product_id,
                supplier_product_color_id = EXCLUDED.supplier_product_color_id,
                supplier_price = EXCLUDED.supplier_price,
                min_order_quantity = EXCLUDED.min_order_quantity,
                lead_time = EXCLUDED.lead_time,
                is_primary = EXCLUDED.is_primary,
                priority = EXCLUDED.priority,
                is_enabled = EXCLUDED.is_enabled,
                remarks = EXCLUDED.remarks,
                updated_at = EXCLUDED.updated_at,
                updated_by = EXCLUDED.updated_by
        "#;

        let values: Vec<Value> = vec![
            product_id.into(),
            product_color_id
                .map(|v| v.into())
                .unwrap_or(Value::Int(None)),
            supplier_id.into(),
            supplier_product_id.into(),
            supplier_product_color_id
                .map(|v| v.into())
                .unwrap_or(Value::Int(None)),
            supplier_price
                .map(|d| Value::String(Some(d.to_string())))
                .unwrap_or(Value::String(None)),
            min_order_quantity
                .map(|d| Value::String(Some(d.to_string())))
                .unwrap_or(Value::String(None)),
            lead_time.map(|v| v.into()).unwrap_or(Value::Int(None)),
            is_primary.into(),
            priority.into(),
            is_enabled.into(),
            remarks
                .map(|s| Value::String(Some(s)))
                .unwrap_or(Value::String(None)),
            now.naive_utc().into(),
            now.naive_utc().into(),
            user_id.into(),
            user_id.into(),
        ];

        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, values);
        db.execute_raw(stmt).await?;
        Ok(())
    }

    /// 校验引用的产品/色号/供应商/供应商商品/供应商色号是否存在
    async fn validate_refs(&self, input: &UpsertSkuMappingInput) -> Result<(), AppError> {
        // 产品
        product::Entity::find_by_id(input.product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::validation(format!("产品 ID {} 不存在", input.product_id)))?;

        // 色号（如果提供了）
        if let Some(cid) = input.product_color_id {
            let pc = product_color::Entity::find_by_id(cid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::validation(format!("产品色号 ID {} 不存在", cid)))?;
            if pc.product_id != input.product_id {
                return Err(AppError::validation("产品色号不属于指定的产品"));
            }
        }

        // 供应商
        supplier::Entity::find_by_id(input.supplier_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("供应商 ID {} 不存在", input.supplier_id))
            })?;

        // 供应商商品
        let sp = supplier_product::Entity::find_by_id(input.supplier_product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!(
                    "供应商商品 ID {} 不存在",
                    input.supplier_product_id
                ))
            })?;
        if sp.supplier_id != input.supplier_id {
            return Err(AppError::validation("供应商商品不属于指定的供应商"));
        }

        // 供应商色号（如果提供了）
        if let Some(spc_id) = input.supplier_product_color_id {
            let spc = supplier_product_color::Entity::find_by_id(spc_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::validation(format!("供应商色号 ID {} 不存在", spc_id)))?;
            if spc.supplier_product_id != input.supplier_product_id {
                return Err(AppError::validation("供应商色号不属于指定的供应商商品"));
            }
        }

        Ok(())
    }
}

/// 判断 sea_orm 错误是否为唯一约束冲突
fn is_unique_violation(err: &sea_orm::DbErr) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("unique") || msg.contains("duplicate key") || msg.contains("23505")
}
