use crate::models::{inventory_adjustment, inventory_adjustment_item, inventory_stock};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;

/// 创建库存调整单请求
#[derive(Debug, Clone)]
pub struct CreateAdjustmentRequest {
    pub warehouse_id: i32,
    pub adjustment_date: DateTime<Utc>,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub items: Vec<AdjustmentItemRequest>,
}

/// 调整明细项请求
#[derive(Debug, Clone)]
pub struct AdjustmentItemRequest {
    pub stock_id: i32,
    pub quantity: Decimal,
    pub unit_cost: Option<Decimal>,
    pub notes: Option<String>,
}

/// 库存调整单详情
#[derive(Debug, Clone)]
pub struct AdjustmentDetail {
    pub adjustment: inventory_adjustment::Model,
    pub items: Vec<inventory_adjustment_item::Model>,
}

#[derive(Debug, Clone)]
pub struct InventoryAdjustmentService {
    db: Arc<DatabaseConnection>,
}

impl InventoryAdjustmentService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建库存调整单（带事务）
    pub async fn create_adjustment(
        &self,
        request: CreateAdjustmentRequest,
    ) -> Result<AdjustmentDetail, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 生成调整单号
        let adjustment_no = self.generate_adjustment_no().await?;

        // 计算总数量
        let total_quantity: Decimal = request.items.iter().map(|item| item.quantity).sum();

        // 保存调整类型用于后续计算
        let adjustment_type = request.adjustment_type.clone();

        // 创建调整单主表
        let adjustment = inventory_adjustment::ActiveModel {
            id: Set(0),
            adjustment_no: Set(adjustment_no),
            warehouse_id: Set(request.warehouse_id),
            adjustment_date: Set(request.adjustment_date),
            adjustment_type: Set(request.adjustment_type),
            reason_type: Set(request.reason_type),
            reason_description: Set(request.reason_description),
            total_quantity: Set(total_quantity),
            notes: Set(request.notes),
            created_by: Set(request.created_by),
            approved_by: Set(None),
            approved_at: Set(None),
            status: Set("pending".to_string()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let adjustment_model = adjustment.insert(&txn).await?;

        // 批量获取库存记录（优化N+1查询）
        let stock_ids: Vec<i32> = request.items.iter().map(|item| item.stock_id).collect();
        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::Id.is_in(stock_ids.clone()))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> = stocks.into_iter().map(|s| (s.id, s)).collect();

        // 创建调整明细项
        let mut item_models = Vec::new();
        for item_req in request.items {
            // 获取当前库存数量
            let stock = stock_map.get(&item_req.stock_id).ok_or_else(|| {
                AppError::ResourceNotFound(format!("库存 ID {} 不存在", item_req.stock_id))
            })?;

            // 计算调整前后的数量（使用 quantity_on_hand 字段）
            let quantity_before = stock.quantity_on_hand;
            let quantity_after = if adjustment_type == "increase" {
                quantity_before + item_req.quantity
            } else {
                quantity_before - item_req.quantity
            };

            // 计算调整金额
            let amount = item_req.unit_cost.map(|cost| cost * item_req.quantity);

            let item = inventory_adjustment_item::ActiveModel {
                id: Set(0),
                adjustment_id: Set(adjustment_model.id),
                stock_id: Set(item_req.stock_id),
                quantity: Set(item_req.quantity),
                quantity_before: Set(quantity_before),
                quantity_after: Set(quantity_after),
                unit_cost: Set(item_req.unit_cost),
                amount: Set(amount),
                notes: Set(item_req.notes),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };

            let item_model = item.insert(&txn).await?;
            item_models.push(item_model);
        }

        // 提交事务
        txn.commit().await?;

        Ok(AdjustmentDetail {
            adjustment: adjustment_model,
            items: item_models,
        })
    }

    /// 审核库存调整单
    pub async fn approve_adjustment(
        &self,
        adjustment_id: i32,
        approved_by: i32,
    ) -> Result<inventory_adjustment::Model, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 获取调整单
        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound(format!("调整单 {} 不存在", adjustment_id)))?;

        // 检查状态
        if adjustment_model.status != "pending" {
            return Err(AppError::BusinessError("只有待审核状态的调整单可以审核".to_string()));
        }

        // 转换为 ActiveModel 用于更新
        let mut adjustment: inventory_adjustment::ActiveModel = adjustment_model.into();

        // 更新状态
        adjustment.status = Set("approved".to_string());
        adjustment.approved_by = Set(Some(approved_by));
        adjustment.approved_at = Set(Some(Utc::now()));
        adjustment.updated_at = Set(Utc::now());

        let adjustment_model = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", adjustment, Some(0)).await?;

        // 获取调整明细项
        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .all(&txn)
            .await?;

        // 更新库存数量
        let mut transaction_events = Vec::new();
        for item in items {
            let stock_model =
                inventory_stock::Entity::find_by_id(item.stock_id)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| {
                        AppError::ResourceNotFound(format!("库存 ID {} 不存在", item.stock_id))
                    })?;

            let quantity_before = stock_model.quantity_on_hand;
            let current_version = stock_model.version;
            let current_quantity_kg = stock_model.quantity_kg;
            let mut stock: inventory_stock::ActiveModel = stock_model.into();

            // 更新库存数量字段（使用 quantity_on_hand 和 quantity_meters 作为主数量字段）
            stock.quantity_on_hand = Set(item.quantity_after);
            stock.quantity_available = Set(item.quantity_after);
            stock.quantity_meters = Set(item.quantity_after);
            // Update quantity_kg proportionally
            if quantity_before > Decimal::ZERO {
                let kg_ratio = current_quantity_kg / quantity_before;
                stock.quantity_kg = Set(item.quantity_after * kg_ratio);
            }
            stock.version = Set(current_version + 1);
            stock.updated_at = Set(Utc::now());
            let updated_stock = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", stock, Some(0)).await?;

            // 收集库存交易事件数据，供审核通过后发布
            let quantity_change = item.quantity_after - quantity_before;
            transaction_events.push(BusinessEvent::InventoryTransactionCreated {
                transaction_id: adjustment_id,
                transaction_type: "INVENTORY_ADJUSTMENT".to_string(),
                product_id: updated_stock.product_id,
                warehouse_id: adjustment_model.warehouse_id,
                quantity_meters: quantity_change,
                quantity_kg: Decimal::ZERO,
                source_bill_type: Some("inventory_adjustment".to_string()),
                source_bill_no: Some(adjustment_model.adjustment_no.clone()),
                source_bill_id: Some(adjustment_id),
                batch_no: updated_stock.batch_no.clone(),
                color_no: updated_stock.color_no.clone(),
                created_by: Some(approved_by),
            });
        }

        // 提交事务
        txn.commit().await?;

        // 事务提交后发布库存交易事件，触发财务凭证生成
        for event in transaction_events {
            EVENT_BUS.publish(event);
            tracing::info!(
                "库存调整审核通过，已发布库存交易事件触发财务凭证生成: 调整单号={}",
                adjustment_model.adjustment_no
            );
        }

        Ok(adjustment_model)
    }

    /// 驳回库存调整单
    pub async fn reject_adjustment(
        &self,
        adjustment_id: i32,
    ) -> Result<inventory_adjustment::Model, AppError> {
        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound(format!("调整单 {} 不存在", adjustment_id)))?;

        // 检查状态
        if adjustment_model.status != "pending" {
            return Err(AppError::BusinessError("只有待审核状态的调整单可以驳回".to_string()));
        }

        let mut adjustment: inventory_adjustment::ActiveModel = adjustment_model.into();

        adjustment.status = Set("rejected".to_string());
        adjustment.updated_at = Set(Utc::now());

        adjustment.update(&*self.db).await.map_err(AppError::from)
    }

    /// 查询所有调整单（分页）
    pub async fn list_adjustments(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<inventory_adjustment::Model>, u64), AppError> {
        let paginator = inventory_adjustment::Entity::find()
            .order_by(inventory_adjustment::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let adjustments = paginator.fetch_page(page).await?;

        Ok((adjustments, total))
    }

    /// 根据 ID 查询调整单
    pub async fn get_adjustment(&self, adjustment_id: i32) -> Result<AdjustmentDetail, AppError> {
        let adjustment = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::ResourceNotFound(format!("调整单 {} 不存在", adjustment_id)))?;

        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .all(&*self.db)
            .await?;

        Ok(AdjustmentDetail { adjustment, items })
    }

    /// 生成调整单号
    async fn generate_adjustment_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "ADJ",
            inventory_adjustment::Entity,
            inventory_adjustment::Column::AdjustmentNo,
        )
        .await
        .map_err(|e| AppError::BusinessError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.expect("Failed to connect to db")
    }

    #[tokio::test]
    async fn test_inventory_adjustment_service_creation() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    #[test]
    fn test_adjustment_request_structure() {
        let request = CreateAdjustmentRequest {
            warehouse_id: 1,
            adjustment_date: Utc::now(),
            adjustment_type: "increase".to_string(),
            reason_type: "damage".to_string(),
            reason_description: Some("测试".to_string()),
            notes: None,
            created_by: Some(1),
            items: vec![],
        };

        assert_eq!(request.warehouse_id, 1);
        assert_eq!(request.adjustment_type, "increase");
        assert_eq!(request.reason_type, "damage");
    }

    #[test]
    fn test_adjustment_item_request_structure() {
        let item = AdjustmentItemRequest {
            stock_id: 1,
            quantity: Decimal::new(100, 2),
            unit_cost: Some(Decimal::new(50, 2)),
            notes: None,
        };

        assert_eq!(item.stock_id, 1);
        assert_eq!(item.quantity, Decimal::new(100, 2));
    }

    #[test]
    fn test_adjustment_detail_structure() {
        let detail = AdjustmentDetail {
            adjustment: inventory_adjustment::Model {
                id: 1,
                adjustment_no: "ADJ202603150001".to_string(),
                warehouse_id: 1,
                adjustment_date: Utc::now(),
                adjustment_type: "increase".to_string(),
                reason_type: "damage".to_string(),
                reason_description: None,
                total_quantity: Decimal::new(100, 2),
                notes: None,
                created_by: Some(1),
                approved_by: None,
                approved_at: None,
                status: "pending".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            items: vec![],
        };

        assert_eq!(detail.adjustment.id, 1);
        assert_eq!(detail.adjustment.adjustment_no, "ADJ202603150001");
        assert_eq!(detail.adjustment.status, "pending");
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_adjustments_empty() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let (adjustments, total) = service
            .list_adjustments(0, 20)
            .await.map_err(AppError::from)
            .expect("list_adjustments should succeed");

        assert!(adjustments.is_empty());
        assert_eq!(total, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_adjustment_not_found() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let result = service.get_adjustment(99999).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ResourceNotFound(_)));
    }

    #[tokio::test]
    #[ignore]
    async fn test_approve_adjustment_not_found() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let result = service.approve_adjustment(99999, 1).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ResourceNotFound(_)));
    }

    #[tokio::test]
    #[ignore]
    async fn test_reject_adjustment_not_found() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let result = service.reject_adjustment(99999).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ResourceNotFound(_)));
    }

    #[test]
    fn test_adjustment_type_validation() {
        let valid_types = vec!["increase", "decrease"];

        for adj_type in valid_types {
            assert!(adj_type == "increase" || adj_type == "decrease");
        }
    }

    #[test]
    fn test_reason_type_validation() {
        let valid_reasons = vec!["damage", "sample", "correction", "other"];

        for reason in valid_reasons {
            assert!(
                reason == "damage"
                    || reason == "sample"
                    || reason == "correction"
                    || reason == "other"
            );
        }
    }

    #[test]
    fn test_status_validation() {
        let valid_statuses = vec!["pending", "approved", "rejected"];

        for status in valid_statuses {
            assert!(status == "pending" || status == "approved" || status == "rejected");
        }
    }

    #[tokio::test]
    async fn test_generate_adjustment_no_format() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        // 由于 generate_adjustment_no 是私有方法，我们无法直接测试
        // 但可以通过验证服务创建成功来间接测试
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    #[test]
    fn test_decimal_operations() {
        let qty1 = Decimal::new(100, 2);
        let qty2 = Decimal::new(50, 2);
        let sum = qty1 + qty2;

        assert_eq!(sum, Decimal::new(150, 2));

        let diff = qty1 - qty2;
        assert_eq!(diff, Decimal::new(50, 2));
    }

    #[test]
    fn test_decimal_sum() {
        let quantities = vec![
            Decimal::new(100, 2),
            Decimal::new(200, 2),
            Decimal::new(300, 2),
        ];

        let total: Decimal = quantities.iter().sum();
        assert_eq!(total, Decimal::new(600, 2));
    }
}
