//! 批量操作服务
//! 提供批量创建、更新、删除功能

use crate::models::audit_log::{OperationType, Severity};
use crate::models::product;
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
// 每匹米数与每卷米数的同源补齐与产品单条链路共用同一真源，避免批量入口各写一套语义
use crate::services::product_service::pair_piece_roll_meters;
// 建品必录校验（产品类型显式 加 面料类幅宽与克重与成分）与取值域校验（词表内、幅宽克重为正、量价非负）
// 与单条、CSV 导入共用同一判据函数组，批量入口不得绕过，否则按品类分级必填在批量面失守
use crate::services::product_service::{
    validate_product_create_fields, validate_product_numeric_domains,
};
use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
use sea_orm::DatabaseTransaction;
use sea_orm::TransactionTrait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

/// 批量操作结果
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResult<T> {
    pub success: bool,
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub failed: usize,
    pub data: Vec<T>,
    pub errors: Vec<BatchError>,
}

/// 批量操作错误
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchError {
    pub index: usize,
    pub message: String,
}

/// 产品批量创建请求
// P2 2-9 修复：补 Serialize derive，满足 validator::Validate 宏的 add_param 约束
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchCreateProductRequest {
    pub name: String,
    pub code: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<String>,
    pub cost_price: Option<String>,
    pub description: Option<String>,
    // 面料行业特色字段
    pub product_type: Option<String>,
    pub fabric_composition: Option<String>,
    pub yarn_count: Option<String>,
    pub density: Option<String>,
    pub width: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub structure: Option<String>,
    pub finish: Option<String>,
    pub min_order_quantity: Option<rust_decimal::Decimal>,
    pub lead_time: Option<i32>,
    // 每匹米数与每卷米数换算元数据。创建侧为两态，带值即落值。只带其中一个值时另一列由
    // 同源补齐成同值，两个都不带时两列落 NULL（与产品单条创建链路同一语义）。
    pub meters_per_piece: Option<rust_decimal::Decimal>,
    pub meters_per_roll: Option<rust_decimal::Decimal>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的键缺席不等于显式 null 语义所需）。
///
/// 为何需要。serde_json 对双层 Option 的默认反序列化在遇到 JSON null 时将外层塌成 None，
/// 与键缺席不可区分，批量编辑清空每匹米数就无法送达后端。本适配器把字段先按内层 Option
/// 反序列化再包一层。键缺席（配合 serde default）为 None 即保持原值，显式 null 为
/// Some(None) 即清空该列，有值为 Some(Some(v)) 即覆盖。
/// 与 product_handler 等处的同型私有适配器一致。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 产品批量更新请求
// P2 2-9 修复：补 Serialize derive，满足 validator::Validate 宏的 add_param 约束
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchUpdateProductRequest {
    pub id: i32,
    pub name: Option<String>,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<String>,
    pub cost_price: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    // 每匹米数与每卷米数换算元数据，三态部分更新。键缺席保持原值、显式 null 清空为 NULL、
    // 有值覆盖。只声明其中一个值时另一列由同源补齐成同样的声明（与单条更新链路同一语义，
    // 清空走显式置空，不取旧值兜底）。
    #[serde(default, deserialize_with = "double_option")]
    pub meters_per_piece: Option<Option<rust_decimal::Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub meters_per_roll: Option<Option<rust_decimal::Decimal>>,
}

/// 批量更新失败回滚上下文：封装 user_id/total/description/errors 等参数（批次 488 D08-1 拆分：用于 build_rollback_result 在 helper 间传递上下文）
struct BatchUpdateRollbackContext {
    user_id: i32,
    total: usize,
    description: String,
    errors: Vec<BatchError>,
}

#[derive(Debug, Clone)]
pub struct BatchService {
    db: Arc<DatabaseConnection>,
}

impl BatchService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 批量创建产品（P1 8-4 修复：接收 user_id 参数，操作完成后记录汇总审计日志；P2 2-9 修复：用 db.begin() 包裹批量操作，任一失败则整体回滚，避免部分写入）
    pub async fn batch_create_products(
        &self,
        user_id: i32,
        requests: Vec<BatchCreateProductRequest>,
    ) -> Result<BatchResult<product::Model>, AppError> {
        // P2 2-9 修复：整体包裹事务，任一失败回滚全部
        let txn = self.db.begin().await?;
        let mut created = 0;
        let mut data = Vec::new();
        let mut errors = Vec::new();

        for (index, req) in requests.iter().enumerate() {
            // 建品必录校验（产品类型显式 加 面料类幅宽与克重与成分）与单条 HTTP、CSV 导入共用
            // 同一判据函数 validate_product_create_fields。批量入口逐行先校验，任一不过即整体回滚，
            // 与下方插入失败的整单回滚语义一致，避免部分写入。
            // 取值域校验（类型在词表内、幅宽克重为正、量价非负）与创建侧和更新侧共用
            // validate_product_numeric_domains。批量此前只过必录门不过取值域门，
            // 导致词表外类型与负幅宽这类入参在批量面能落库、在单条面被拒，三入口判据不齐。
            // 批量入参 width 与 gram_weight 本就是 Decimal，直接透传比较，不做 f64 往返、不兜底。
            // 价格两列在批量入参里是字符串形态（DECIMAL 列的线上形态），先严格按十进制解析再交
            // 取值域判据，解析不了就点名拒绝，不再在构造处静默折叠成 None 把单价写空。
            let normalized = Self::parse_batch_decimal(req.standard_price.as_ref(), "标准价格")
                .and_then(|sp| {
                    Self::parse_batch_decimal(req.cost_price.as_ref(), "成本价格")
                        .map(|cp| (sp, cp))
                })
                .and_then(|(standard_price, cost_price)| {
                    validate_product_create_fields(
                        req.product_type.as_deref(),
                        req.width,
                        req.gram_weight,
                        req.fabric_composition.as_deref(),
                    )
                    .and_then(|()| {
                        validate_product_numeric_domains(
                            req.product_type.as_deref(),
                            req.width,
                            req.gram_weight,
                            req.min_order_quantity,
                            req.lead_time,
                            standard_price,
                            cost_price,
                        )
                    })
                    .map(|()| (standard_price, cost_price))
                });
            let (standard_price, cost_price) = match normalized {
                Ok(prices) => prices,
                Err(e) => {
                    txn.rollback().await.ok();
                    errors.push(BatchError {
                        index,
                        message: e.to_string(),
                    });
                    let failed = errors.len();
                    Self::record_batch_create_audit(
                        &self.db,
                        user_id,
                        &requests,
                        0,
                        &errors,
                        true,
                    );
                    return Ok(BatchResult {
                        success: false,
                        total: requests.len(),
                        created: 0,
                        updated: 0,
                        failed,
                        data: vec![],
                        errors,
                    });
                }
            };

            let product = Self::build_product_active_model(req, standard_price, cost_price);

            match product.insert(&txn).await {
                Ok(model) => {
                    created += 1;
                    data.push(model);
                }
                Err(e) => {
                    // P2 2-9 修复：事务内任一失败则整体回滚，避免部分写入
                    txn.rollback().await.ok();
                    errors.push(BatchError {
                        index,
                        message: e.to_string(),
                    });
                    let failed = errors.len();
                    // 审计日志记录在主连接上（非事务），确保回滚后仍可追溯
                    Self::record_batch_create_audit(&self.db, user_id, &requests, 0, &errors, true);
                    return Ok(BatchResult {
                        success: false,
                        total: requests.len(),
                        created: 0,
                        updated: 0,
                        failed,
                        data: vec![],
                        errors,
                    });
                }
            }
        }
        txn.commit().await?;

        // P1 8-4 修复：批量创建完成后记录汇总审计日志
        Self::record_batch_create_audit(&self.db, user_id, &requests, created, &errors, false);

        Ok(BatchResult {
            success: true,
            total: requests.len(),
            created,
            updated: 0,
            failed: 0,
            data,
            errors,
        })
    }

    /// 批量入参的 DECIMAL 字符串列严格解析。入参 raw 为批量请求里的原值，label 为该列的用户可见名称。
    /// 返回 Ok(None) 表示未提交该列，Ok(Some(v)) 表示解析成功，解析不了即点名拒绝。
    /// 调用方为 batch_create_products 的逐行前置检查，解析结果随后下传 build_product_active_model
    /// 落库，并交 validate_product_numeric_domains 判非负。
    /// 为什么不再用 .ok() 静默折叠成 None。那等于把用户填了但写错的单价悄悄写成空值，
    /// 用户看到的落库结果与自己提交的内容不一致，且价格列的取值意图被吞掉。
    fn parse_batch_decimal(
        raw: Option<&String>,
        label: &str,
    ) -> Result<Option<rust_decimal::Decimal>, AppError> {
        let Some(text) = raw else {
            return Ok(None);
        };
        if text.trim().is_empty() {
            return Ok(None);
        }
        rust_decimal::Decimal::from_str(text.trim()).map(Some).map_err(|_| {
            AppError::validation_displayable(format!("{label} \"{text}\" 不是合法十进制数"))
        })
    }

    /// 从请求构建产品 ActiveModel
    /// 价格两列由调用方在逐行校验阶段严格解析后下传（解析失败已在调用处点名拒绝），
    /// 本函数不再自行 .ok() 解析，避免把用户填的单价静默丢成 None。
    fn build_product_active_model(
        req: &BatchCreateProductRequest,
        standard_price: Option<rust_decimal::Decimal>,
        cost_price: Option<rust_decimal::Decimal>,
    ) -> product::ActiveModel {
        // 米数两列先过同源补齐再落 ActiveModel，只带一个值时另一列同值落库，
        // 两个都不带时两列落 NULL，与产品单条创建链路同一形态
        let (meters_per_piece, meters_per_roll) =
            pair_piece_roll_meters(req.meters_per_piece, req.meters_per_roll);

        product::ActiveModel {
            id: Default::default(),
            name: Set(req.name.clone()),
            code: Set(req.code.clone()),
            barcode: Set(req.barcode.clone()),
            category_id: Set(req.category_id),
            specification: Set(req.specification.clone()),
            unit: Set(req.unit.clone().unwrap_or_else(|| "件".to_string())),
            standard_price: Set(standard_price),
            cost_price: Set(cost_price),
            description: Set(req.description.clone()),
            status: Set(master_data::ACTIVE.to_string()),
            is_deleted: Set(false),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            // 面料行业特色字段
            // 产品类型已由上方 validate_product_create_fields 保证显式提交且非空白，这里落规范值，
            // 不再缺省成成品布（缺省会绕过「按品类分级必填」，且原处是硬编码中文副本，违反词表唯一源）。
            product_type: Set(req
                .product_type
                .clone()
                .map(|s| s.trim().to_string())
                .unwrap_or_default()),
            fabric_composition: Set(req.fabric_composition.clone()),
            yarn_count: Set(req.yarn_count.clone()),
            density: Set(req.density.clone()),
            width: Set(req.width),
            gram_weight: Set(req.gram_weight),
            structure: Set(req.structure.clone()),
            finish: Set(req.finish.clone()),
            min_order_quantity: Set(req.min_order_quantity),
            lead_time: Set(req.lead_time),
            meters_per_piece: Set(meters_per_piece),
            meters_per_roll: Set(meters_per_roll),
            supplier_product_code: sea_orm::ActiveValue::NotSet,
            supplier_id: sea_orm::ActiveValue::NotSet,
            is_batch_managed: sea_orm::ActiveValue::NotSet,
            batch_level: sea_orm::ActiveValue::NotSet,
            ..Default::default()
        }
    }

    /// 批量创建产品审计日志记录（成功/失败合并，rolled_back 区分）
    fn record_batch_create_audit(
        db: &DatabaseConnection,
        user_id: i32,
        requests: &[BatchCreateProductRequest],
        created: usize,
        errors: &[BatchError],
        rolled_back: bool,
    ) {
        let failed = errors.len();
        let (severity, description) = if rolled_back {
            (
                Severity::Warn,
                format!(
                    "批量创建产品（已回滚）：总数={} 成功=0 失败={} 首错索引={}",
                    requests.len(),
                    failed,
                    errors.first().map(|e| e.index).unwrap_or(0)
                ),
            )
        } else {
            (
                Severity::Info,
                format!(
                    "批量创建产品：总数={} 成功={} 失败=0",
                    requests.len(),
                    created
                ),
            )
        };

        let event = AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: OperationType::Create,
            severity,
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_create_{}", chrono::Utc::now().timestamp())),
            description: Some(description),
            request_method: Some("POST".to_string()),
            request_path: Some("/api/v1/erp/products/batch/create".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": requests.len(),
                "created": if rolled_back { 0 } else { created },
                "failed": failed,
                "rolled_back": rolled_back,
                "failed_indexes": errors.iter().map(|e| e.index).collect::<Vec<_>>(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(Arc::new(db.clone())));
        svc.record_async(event, None);
    }

    /// 批量更新产品
    /// P1 8-4 修复：接收 user_id 参数，操作完成后记录汇总审计日志；P2 2-9 修复：用 db.begin() 包裹批量操作，任一失败则整体回滚，避免部分写入；批次 488 D08-1 拆分：主函数仅做协调，事务边界保留（txn.commit() 仍在主函数）；helper 通过 &txn 引用参与事务，不再调用 txn.rollback()（事务 drop 自动回滚）。
    pub async fn batch_update_products(
        &self,
        user_id: i32,
        requests: Vec<BatchUpdateProductRequest>,
    ) -> Result<BatchResult<product::Model>, AppError> {
        // P2 2-9 修复：整体包裹事务，任一失败回滚全部
        let txn = self.db.begin().await?;
        let mut updated = 0;
        let mut data = Vec::new();
        let mut errors = Vec::new();

        let product_map = Self::load_existing_products(&txn, &requests).await?;

        for (index, req) in requests.iter().enumerate() {
            // 从批量查询结果中获取产品，避免循环内逐条查询
            // 错误元组 (message, is_not_found)：is_not_found 区分两种失败描述
            let outcome: Result<product::Model, (String, bool)> =
                match product_map.get(&req.id).cloned() {
                    Some(product_model) => {
                        let product: product::ActiveModel = product_model.into();
                        let product = Self::apply_incremental_updates(product, req);
                        product
                            .update(&txn)
                            .await
                            .map_err(|e| (e.to_string(), false))
                    }
                    None => Err((format!("产品 ID {} 不存在", req.id), true)),
                };
            match outcome {
                Ok(model) => {
                    updated += 1;
                    data.push(model);
                }
                Err((msg, is_not_found)) => {
                    // P2 2-9 修复：事务内任一失败则整体回滚，避免部分写入
                    // 事务 drop 自动回滚（不再调用 txn.rollback()，&DatabaseTransaction 引用无法调用）
                    errors.push(BatchError {
                        index,
                        message: msg,
                    });
                    let ctx = Self::build_failure_ctx(
                        user_id,
                        requests.len(),
                        index,
                        errors,
                        is_not_found,
                    );
                    return Ok(Self::build_rollback_result(&ctx, self.db.clone()));
                }
            }
        }
        txn.commit().await?;

        // P1 8-4 修复：批量更新完成后记录汇总审计日志
        Self::record_success_audit(user_id, requests.len(), updated, self.db.clone());

        Ok(BatchResult {
            success: true,
            total: requests.len(),
            created: 0,
            updated,
            failed: 0,
            data,
            errors,
        })
    }

    /// 批量查询产品构建 HashMap，避免循环内 N+1 查询（批次 488 D08-1 拆分：从事务内一次性 is_in 批量查询，构建 HashMap 供循环内 O(1) 查找。）
    async fn load_existing_products(
        txn: &DatabaseTransaction,
        requests: &[BatchUpdateProductRequest],
    ) -> Result<HashMap<i32, product::Model>, AppError> {
        // 批次 94 P2-4 修复：N+1 查询改批量查询 + HashMap 索引
        // 原循环内 find_by_id 逐条查询，N 个产品触发 N 次查询；
        // 改为事务内一次性 is_in 批量查询，构建 HashMap 供循环内 O(1) 查找。
        let batch_ids: Vec<i32> = requests.iter().map(|r| r.id).collect();
        let existing_products = product::Entity::find()
            .filter(product::Column::Id.is_in(batch_ids))
            .all(txn)
            .await?;
        Ok(existing_products.into_iter().map(|p| (p.id, p)).collect())
    }

    /// 应用增量更新到 ActiveModel（只更新请求中提供的字段）（批次 488 D08-1 拆分：纯函数，无 txn 依赖，便于单测。）
    fn apply_incremental_updates(
        mut product: product::ActiveModel,
        req: &BatchUpdateProductRequest,
    ) -> product::ActiveModel {
        // 增量更新
        if let Some(name) = &req.name {
            product.name = Set(name.clone());
        }
        if let Some(barcode) = &req.barcode {
            product.barcode = Set(Some(barcode.clone()));
        }
        if let Some(category_id) = req.category_id {
            product.category_id = Set(Some(category_id));
        }
        if let Some(spec) = &req.specification {
            product.specification = Set(Some(spec.clone()));
        }
        if let Some(unit) = &req.unit {
            product.unit = Set(unit.clone());
        }
        if let Some(price) = &req.standard_price {
            if let Ok(decimal) = price.parse::<rust_decimal::Decimal>() {
                product.standard_price = Set(Some(decimal));
            }
        }
        if let Some(price) = &req.cost_price {
            if let Ok(decimal) = price.parse::<rust_decimal::Decimal>() {
                product.cost_price = Set(Some(decimal));
            }
        }
        if let Some(desc) = &req.description {
            product.description = Set(Some(desc.clone()));
        }
        if let Some(status) = &req.status {
            product.status = Set(status.clone());
        }
        // 米数两列先过同源补齐（只声明一边时把同一份声明复制到另一边），再按三态落库。
        // 外层 None 即键缺席、保持原值。外层 Some 时内层值原样 Set，内层 None 清空为 NULL，
        // 内层 Some(v) 覆盖为该值。与单条更新链路同一判据，清空不取旧值兜底。
        let (meters_per_piece, meters_per_roll) =
            pair_piece_roll_meters(req.meters_per_piece, req.meters_per_roll);
        if let Some(piece) = meters_per_piece {
            product.meters_per_piece = Set(piece);
        }
        if let Some(roll) = meters_per_roll {
            product.meters_per_roll = Set(roll);
        }
        product.updated_at = Set(chrono::Utc::now());
        product
    }

    /// 构建批量更新失败回滚上下文（批次 488 D08-1 拆分：复用两种失败场景（更新失败/产品不存在），；is_not_found=true 使用"已回滚-产品不存在"描述，否则使用"已回滚"描述。）
    fn build_failure_ctx(
        user_id: i32,
        total: usize,
        first_error_index: usize,
        errors: Vec<BatchError>,
        is_not_found: bool,
    ) -> BatchUpdateRollbackContext {
        let failed = errors.len();
        let description = if is_not_found {
            format!(
                "批量更新产品（已回滚-产品不存在）：总数={} 失败={} 首错索引={}",
                total, failed, first_error_index
            )
        } else {
            format!(
                "批量更新产品（已回滚）：总数={} 成功=0 失败={} 首错索引={}",
                total, failed, first_error_index
            )
        };
        BatchUpdateRollbackContext {
            user_id,
            total,
            description,
            errors,
        }
    }

    /// 构建批量更新失败回滚结果并异步记录审计日志（批次 488 D08-1 拆分：从主函数提取审计日志构建 + BatchResult 组装，主函数仅做协调。）
    fn build_rollback_result(
        ctx: &BatchUpdateRollbackContext,
        db: Arc<DatabaseConnection>,
    ) -> BatchResult<product::Model> {
        let failed = ctx.errors.len();
        let event = AuditEvent {
            user_id: Some(ctx.user_id),
            username: None,
            operation_type: OperationType::Update,
            severity: Severity::Warn,
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_update_{}", chrono::Utc::now().timestamp())),
            description: Some(ctx.description.clone()),
            request_method: Some("PUT".to_string()),
            request_path: Some("/api/v1/erp/products/batch/update".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": ctx.total,
                "updated": 0,
                "failed": failed,
                "rolled_back": true,
                "failed_indexes": ctx.errors.iter().map(|e| e.index).collect::<Vec<_>>(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(db));
        svc.record_async(event, None);
        BatchResult {
            success: false,
            total: ctx.total,
            created: 0,
            updated: 0,
            failed,
            data: vec![],
            errors: ctx.errors.clone(),
        }
    }

    /// 记录批量更新成功的汇总审计日志（批次 488 D08-1 拆分：从主函数提取审计日志构建，主函数仅做协调。）
    fn record_success_audit(
        user_id: i32,
        total: usize,
        updated: usize,
        db: Arc<DatabaseConnection>,
    ) {
        let event = AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: OperationType::Update,
            severity: Severity::Info,
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_update_{}", chrono::Utc::now().timestamp())),
            description: Some(format!(
                "批量更新产品：总数={} 成功={} 失败=0",
                total, updated
            )),
            request_method: Some("PUT".to_string()),
            request_path: Some("/api/v1/erp/products/batch/update".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": total,
                "updated": updated,
                "failed": 0,
                "failed_indexes": Vec::<usize>::new(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(db));
        svc.record_async(event, None);
    }

    /// 批量删除产品（P1 8-4 修复：接收 user_id 参数，Some(0) 改为 Some(user_id)）
    pub async fn batch_delete_products(
        &self,
        user_id: i32,
        ids: Vec<i32>,
    ) -> Result<BatchResult<()>, AppError> {
        let mut failed = 0;
        let mut errors = Vec::new();

        for (index, id) in ids.iter().enumerate() {
            // P0 8-3 修复：delete 操作补审计日志
            // P1 8-4 修复：user_id 从 Some(0) 改为 Some(user_id)，避免审计操作人丢失
            let result = crate::services::audit_log_service::AuditLogService::delete_with_audit::<
                product::Entity,
                _,
            >(self.db.as_ref(), "product", *id, Some(user_id))
            .await;

            match result {
                Ok(_) => {
                    // 删除成功
                }
                Err(e) => {
                    failed += 1;
                    errors.push(BatchError {
                        index,
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(BatchResult {
            success: failed == 0,
            total: ids.len(),
            created: 0,
            updated: 0,
            failed,
            data: vec![],
            errors,
        })
    }
}
