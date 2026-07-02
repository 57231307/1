//! 批量操作服务
//! 提供批量创建、更新、删除功能

use crate::models::audit_log::{OperationType, Severity};
use crate::models::product;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchError {
    pub index: usize,
    pub message: String,
}

/// 产品批量创建请求
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
    pub async fn batch_create_products(
        &self,
        user_id: i32,
        requests: Vec<BatchCreateProductRequest>,
    ) -> Result<BatchResult<product::Model>, AppError> {
        let mut created = 0;
        let mut failed = 0;
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
                id: Set(0),
                name: Set(req.name.clone()),
                code: Set(req.code.clone()),
                category_id: Set(req.category_id),
                specification: Set(req.specification.clone()),
                unit: Set(req.unit.clone().unwrap_or_else(|| "件".to_string())),
                standard_price: Set(standard_price),
                cost_price: Set(cost_price),
                description: Set(req.description.clone()),
                status: Set("active".to_string()),
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

            match product.insert(self.db.as_ref()).await {
                Ok(model) => {
                    created += 1;
                    data.push(model);
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

        // P1 8-4 修复：批量创建完成后记录汇总审计日志
        let event = AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: OperationType::Create,
            severity: if failed == 0 { Severity::Info } else { Severity::Warn },
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_create_{}", chrono::Utc::now().timestamp())),
            description: Some(format!(
                "批量创建产品：总数={} 成功={} 失败={}",
                requests.len(),
                created,
                failed
            )),
            request_method: Some("POST".to_string()),
            request_path: Some("/api/v1/erp/products/batch/create".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": requests.len(),
                "created": created,
                "failed": failed,
                "failed_indexes": errors.iter().map(|e| e.index).collect::<Vec<_>>(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(self.db.clone()));
        svc.record_async(event, None);

        Ok(BatchResult {
            success: failed == 0,
            total: requests.len(),
            created,
            updated: 0,
            failed,
            data,
            errors,
        })
    }

    /// 批量更新产品
    ///
    /// P1 8-4 修复：接收 user_id 参数，操作完成后记录汇总审计日志
    pub async fn batch_update_products(
        &self,
        user_id: i32,
        requests: Vec<BatchUpdateProductRequest>,
    ) -> Result<BatchResult<product::Model>, AppError> {
        let mut updated = 0;
        let mut failed = 0;
        let mut data = Vec::new();
        let mut errors = Vec::new();

        for (index, req) in requests.iter().enumerate() {
            // 检查产品是否存在
            let existing = product::Entity::find_by_id(req.id)
                .one(self.db.as_ref())
                .await?;

            match existing {
                Some(product_model) => {
                    let mut product: product::ActiveModel = product_model.into();

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

                    match product.update(self.db.as_ref()).await {
                        Ok(model) => {
                            updated += 1;
                            data.push(model);
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
                None => {
                    failed += 1;
                    errors.push(BatchError {
                        index,
                        message: format!("产品 ID {} 不存在", req.id),
                    });
                }
            }
        }

        // P1 8-4 修复：批量更新完成后记录汇总审计日志
        let event = AuditEvent {
            user_id: Some(user_id),
            username: None,
            operation_type: OperationType::Update,
            severity: if failed == 0 { Severity::Info } else { Severity::Warn },
            resource_type: Some("product_batch".to_string()),
            resource_id: None,
            resource_name: Some(format!("batch_update_{}", chrono::Utc::now().timestamp())),
            description: Some(format!(
                "批量更新产品：总数={} 成功={} 失败={}",
                requests.len(),
                updated,
                failed
            )),
            request_method: Some("PUT".to_string()),
            request_path: Some("/api/v1/erp/products/batch/update".to_string()),
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "total": requests.len(),
                "updated": updated,
                "failed": failed,
                "failed_indexes": errors.iter().map(|e| e.index).collect::<Vec<_>>(),
            })),
        };
        let svc = Arc::new(AuditLogService::new(self.db.clone()));
        svc.record_async(event, None);

        Ok(BatchResult {
            success: failed == 0,
            total: requests.len(),
            created: 0,
            updated,
            failed,
            data,
            errors,
        })
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
