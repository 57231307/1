//! 批量操作服务
//! 提供批量创建、更新、删除功能

use crate::models::audit_log::{OperationType, Severity};
use crate::models::product;
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
use sea_orm::DatabaseTransaction;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

/// 产品批量更新请求
// P2 2-9 修复：补 Serialize derive，满足 validator::Validate 宏的 add_param 约束
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchUpdateProductRequest {
    pub id: i32,
    pub name: Option<String>,
    pub category_id: Option<i32>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<String>,
    pub cost_price: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

/// 批量更新失败回滚上下文：封装 user_id/total/description/errors 等参数
///
/// 批次 488 D08-1 拆分：用于 build_rollback_result 在 helper 间传递上下文
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

    /// 批量创建产品
    ///
    /// P1 8-4 修复：接收 user_id 参数，操作完成后记录汇总审计日志
    /// P2 2-9 修复：用 db.begin() 包裹批量操作，任一失败则整体回滚，避免部分写入
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
            // 解析价格
            let standard_price = req
                .standard_price
                .as_ref()
                .and_then(|s| s.parse::<rust_decimal::Decimal>().ok());
            let cost_price = req
                .cost_price
                .as_ref()
                .and_then(|s| s.parse::<rust_decimal::Decimal>().ok());

            let product = product::ActiveModel {
                id: Default::default(),
                name: Set(req.name.clone()),
                code: Set(req.code.clone()),
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
                product_type: Set(req
                    .product_type
                    .clone()
                    .unwrap_or_else(|| "成品布".to_string())),
                fabric_composition: Set(req.fabric_composition.clone()),
                yarn_count: Set(req.yarn_count.clone()),
                density: Set(req.density.clone()),
                width: Set(req.width),
                gram_weight: Set(req.gram_weight),
                structure: Set(req.structure.clone()),
                finish: Set(req.finish.clone()),
                min_order_quantity: Set(req.min_order_quantity),
                lead_time: Set(req.lead_time),
                supplier_product_code: sea_orm::ActiveValue::NotSet,
                supplier_id: sea_orm::ActiveValue::NotSet,
                is_batch_managed: sea_orm::ActiveValue::NotSet,
                batch_level: sea_orm::ActiveValue::NotSet,
            };

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
                    let event = AuditEvent {
                        user_id: Some(user_id),
                        username: None,
                        operation_type: OperationType::Create,
                        severity: Severity::Warn,
                        resource_type: Some("product_batch".to_string()),
                        resource_id: None,
                        resource_name: Some(format!("batch_create_{}", chrono::Utc::now().timestamp())),
                        description: Some(format!(
                            "批量创建产品（已回滚）：总数={} 成功=0 失败={} 首错索引={}",
                            requests.len(), failed, index
                        )),
                        request_method: Some("POST".to_string()),
                        request_path: Some("/api/v1/erp/products/batch/create".to_string()),
                        before_snapshot: None,
                        after_snapshot: Some(serde_json::json!({
                            "total": requests.len(),
                            "created": 0,
                            "failed": failed,
                            "rolled_back": true,
                            "failed_indexes": errors.iter().map(|e| e.index).collect::<Vec<_>>(),
                        })),
                    };
                    let svc = Arc::new(AuditLogService::new(self.db.clone()));
                    svc.record_async(event, None);
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
        let event = AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: OperationType::Create,
            severity: Severity::Info,
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_create_{}", chrono::Utc::now().timestamp())),
            description: Some(format!(
                "批量创建产品：总数={} 成功={} 失败=0",
                requests.len(),
                created
            )),
            request_method: Some("POST".to_string()),
            request_path: Some("/api/v1/erp/products/batch/create".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": requests.len(),
                "created": created,
                "failed": 0,
                "failed_indexes": errors.iter().map(|e| e.index).collect::<Vec<_>>(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(self.db.clone()));
        svc.record_async(event, None);

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

    /// 批量更新产品
    ///
    /// P1 8-4 修复：接收 user_id 参数，操作完成后记录汇总审计日志
    /// P2 2-9 修复：用 db.begin() 包裹批量操作，任一失败则整体回滚，避免部分写入
    ///
    /// 批次 488 D08-1 拆分：主函数仅做协调，事务边界保留（txn.commit() 仍在主函数）；
    /// helper 通过 &txn 引用参与事务，不再调用 txn.rollback()（事务 drop 自动回滚）。
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
            let outcome: Result<product::Model, (String, bool)> = match product_map.get(&req.id).cloned() {
                Some(product_model) => {
                    let product: product::ActiveModel = product_model.into();
                    let product = Self::apply_incremental_updates(product, req);
                    product.update(&txn).await.map_err(|e| (e.to_string(), false))
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
                    errors.push(BatchError { index, message: msg });
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

    /// 批量查询产品构建 HashMap，避免循环内 N+1 查询
    ///
    /// 批次 488 D08-1 拆分：从事务内一次性 is_in 批量查询，构建 HashMap 供循环内 O(1) 查找。
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
        Ok(existing_products
            .into_iter()
            .map(|p| (p.id, p))
            .collect())
    }

    /// 应用增量更新到 ActiveModel（只更新请求中提供的字段）
    ///
    /// 批次 488 D08-1 拆分：纯函数，无 txn 依赖，便于单测。
    fn apply_incremental_updates(
        mut product: product::ActiveModel,
        req: &BatchUpdateProductRequest,
    ) -> product::ActiveModel {
        // 增量更新
        if let Some(name) = &req.name {
            product.name = Set(name.clone());
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
        product.updated_at = Set(chrono::Utc::now());
        product
    }

    /// 构建批量更新失败回滚上下文
    ///
    /// 批次 488 D08-1 拆分：复用两种失败场景（更新失败/产品不存在），
    /// is_not_found=true 使用"已回滚-产品不存在"描述，否则使用"已回滚"描述。
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

    /// 构建批量更新失败回滚结果并异步记录审计日志
    ///
    /// 批次 488 D08-1 拆分：从主函数提取审计日志构建 + BatchResult 组装，主函数仅做协调。
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

    /// 记录批量更新成功的汇总审计日志
    ///
    /// 批次 488 D08-1 拆分：从主函数提取审计日志构建，主函数仅做协调。
    fn record_success_audit(user_id: i32, total: usize, updated: usize, db: Arc<DatabaseConnection>) {
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

    /// 批量删除产品
    ///
    /// P1 8-4 修复：接收 user_id 参数，Some(0) 改为 Some(user_id)
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
