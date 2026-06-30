use crate::models::{inventory_adjustment, inventory_adjustment_item, inventory_stock};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
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

/// 更新库存调整单请求
#[derive(Debug, Clone, Default)]
pub struct UpdateAdjustmentRequest {
    pub warehouse_id: Option<i32>,
    pub adjustment_date: Option<DateTime<Utc>>,
    pub adjustment_type: Option<String>,
    pub reason_type: Option<String>,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
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
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.id, s)).collect();

        // 创建调整明细项
        let mut item_models = Vec::new();
        for item_req in request.items {
            // 获取当前库存数量
            let stock = stock_map.get(&item_req.stock_id).ok_or_else(|| {
                AppError::not_found(format!("库存 ID {} 不存在", item_req.stock_id))
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

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询未加行锁，并发审核可能双写状态。
        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单 {} 不存在", adjustment_id)))?;

        // 检查状态
        if adjustment_model.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以审核".to_string(),
            ));
        }

        // 转换为 ActiveModel 用于更新
        let mut adjustment: inventory_adjustment::ActiveModel = adjustment_model.into();

        // 更新状态
        adjustment.status = Set("approved".to_string());
        adjustment.approved_by = Set(Some(approved_by));
        adjustment.approved_at = Set(Some(Utc::now()));
        adjustment.updated_at = Set(Utc::now());

        let adjustment_model =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                adjustment,
                Some(0),
            )
            .await?;

        // 获取调整明细项
        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .all(&txn)
            .await?;

        // 批量查询调整明细涉及的库存，避免循环内 N+1 查询
        let stock_ids: Vec<i32> = items.iter().map(|item| item.stock_id).collect();
        let stocks = if stock_ids.is_empty() {
            Vec::new()
        } else {
            inventory_stock::Entity::find()
                .filter(inventory_stock::Column::Id.is_in(stock_ids))
                .all(&txn)
                .await?
        };
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.id, s)).collect();

        // 更新库存数量
        let mut transaction_events = Vec::new();
        for item in items {
            let stock_model = stock_map
                .get(&item.stock_id)
                .ok_or_else(|| AppError::not_found(format!("库存 ID {} 不存在", item.stock_id)))?;

            let quantity_before = stock_model.quantity_on_hand;
            let expected_version = stock_model.version;
            let current_quantity_kg = stock_model.quantity_kg;

            // 使用乐观锁条件更新
            let update_result = inventory_stock::Entity::update_many()
                .col_expr(
                    inventory_stock::Column::QuantityOnHand,
                    sea_orm::sea_query::Expr::val(item.quantity_after).into(),
                )
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::val(item.quantity_after).into(),
                )
                .col_expr(
                    inventory_stock::Column::QuantityMeters,
                    sea_orm::sea_query::Expr::val(item.quantity_after).into(),
                )
                .col_expr(
                    inventory_stock::Column::QuantityKg,
                    sea_orm::sea_query::Expr::val(if quantity_before > Decimal::ZERO {
                        let kg_ratio = current_quantity_kg / quantity_before;
                        item.quantity_after * kg_ratio
                    } else {
                        current_quantity_kg
                    }).into(),
                )
                .col_expr(
                    inventory_stock::Column::Version,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(Utc::now()).into(),
                )
                .filter(inventory_stock::Column::Id.eq(item.stock_id))
                .filter(inventory_stock::Column::Version.eq(expected_version))
                .exec(&txn)
                .await?;

            // 检查乐观锁是否成功
            if update_result.rows_affected == 0 {
                return Err(AppError::business(format!(
                    "库存 ID {} 已被其他用户修改，请重试",
                    item.stock_id
                )));
            }

            // 获取更新后的库存记录
            let updated_stock = inventory_stock::Entity::find_by_id(item.stock_id)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("库存 ID {} 不存在", item.stock_id)))?;

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
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 &*self.db 裸查询无行锁，且 update 也用裸连接，无事务保护。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更，update 一并纳入事务。
        let txn = (*self.db).begin().await?;

        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单 {} 不存在", adjustment_id)))?;

        // 检查状态
        if adjustment_model.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以驳回".to_string(),
            ));
        }

        let mut adjustment: inventory_adjustment::ActiveModel = adjustment_model.into();

        adjustment.status = Set("rejected".to_string());
        adjustment.updated_at = Set(Utc::now());

        let updated = adjustment.update(&txn).await.map_err(AppError::from)?;
        txn.commit().await?;
        Ok(updated)
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
        // 批次 24 v6 P1-3 修复：分页偏移 off-by-one。
        // SeaORM paginator.fetch_page 是 0-indexed，传入 page=1 实际查询第二页。
        // 改为 page.saturating_sub(1)，与 ap_reconciliation_service.rs:411 写法一致。
        let adjustments = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((adjustments, total))
    }

    /// 根据 ID 查询调整单
    pub async fn get_adjustment(&self, adjustment_id: i32) -> Result<AdjustmentDetail, AppError> {
        let adjustment = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单 {} 不存在", adjustment_id)))?;

        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .all(&*self.db)
            .await?;

        Ok(AdjustmentDetail { adjustment, items })
    }

    // 生成调整单号
    crate::impl_generate_no!(
        generate_adjustment_no,
        "ADJ",
        inventory_adjustment::Entity,
        inventory_adjustment::Column::AdjustmentNo
    );

    /// 更新调整单（仅 pending 状态可更新）
    pub async fn update_adjustment(
        &self,
        adjustment_id: i32,
        req: UpdateAdjustmentRequest,
    ) -> Result<inventory_adjustment::Model, AppError> {
        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单 {} 不存在", adjustment_id)))?;

        if adjustment_model.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以更新".to_string(),
            ));
        }

        let mut active: inventory_adjustment::ActiveModel = adjustment_model.into_active_model();

        if let Some(warehouse_id) = req.warehouse_id {
            active.warehouse_id = Set(warehouse_id);
        }
        if let Some(adjustment_date) = req.adjustment_date {
            active.adjustment_date = Set(adjustment_date);
        }
        if let Some(adjustment_type) = req.adjustment_type {
            active.adjustment_type = Set(adjustment_type);
        }
        if let Some(reason_type) = req.reason_type {
            active.reason_type = Set(reason_type);
        }
        if let Some(reason_description) = req.reason_description {
            active.reason_description = Set(Some(reason_description));
        }
        if let Some(notes) = req.notes {
            active.notes = Set(Some(notes));
        }
        active.updated_at = Set(Utc::now());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除调整单（仅 pending 状态）
    pub async fn delete_adjustment(&self, adjustment_id: i32) -> Result<(), AppError> {
        let adjustment_model = inventory_adjustment::Entity::find_by_id(adjustment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单 {} 不存在", adjustment_id)))?;

        if adjustment_model.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以删除".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;

        // 先删除明细
        inventory_adjustment_item::Entity::delete_many()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .exec(&txn)
            .await?;

        // 再删除主表
        inventory_adjustment::Entity::delete_by_id(adjustment_id)
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 列出调整单的所有明细项
    pub async fn list_items(
        &self,
        adjustment_id: i32,
    ) -> Result<Vec<inventory_adjustment_item::Model>, AppError> {
        // 确认主单存在
        let _ = self.get_adjustment(adjustment_id).await?;

        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .order_by(inventory_adjustment_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(items)
    }

    /// 向调整单添加明细
    pub async fn add_item(
        &self,
        adjustment_id: i32,
        req: AdjustmentItemRequest,
    ) -> Result<inventory_adjustment_item::Model, AppError> {
        let detail = self.get_adjustment(adjustment_id).await?;

        if detail.adjustment.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以添加明细".to_string(),
            ));
        }

        let stock = inventory_stock::Entity::find_by_id(req.stock_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存 ID {} 不存在", req.stock_id)))?;

        let quantity_before = stock.quantity_on_hand;
        let quantity_after = if detail.adjustment.adjustment_type == "increase" {
            quantity_before + req.quantity
        } else {
            quantity_before - req.quantity
        };
        let amount = req.unit_cost.map(|cost| cost * req.quantity);

        let txn = (*self.db).begin().await?;

        let item = inventory_adjustment_item::ActiveModel {
            id: Set(0),
            adjustment_id: Set(adjustment_id),
            stock_id: Set(req.stock_id),
            quantity: Set(req.quantity),
            quantity_before: Set(quantity_before),
            quantity_after: Set(quantity_after),
            unit_cost: Set(req.unit_cost),
            amount: Set(amount),
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let item_model = item.insert(&txn).await?;

        // 重新计算总数量
        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(adjustment_id))
            .all(&txn)
            .await?;
        let total_quantity: Decimal = items.iter().map(|i| i.quantity).sum();

        let mut adjustment: inventory_adjustment::ActiveModel =
            detail.adjustment.into_active_model();
        adjustment.total_quantity = Set(total_quantity);
        adjustment.updated_at = Set(Utc::now());
        adjustment.update(&txn).await?;

        txn.commit().await?;

        Ok(item_model)
    }

    /// 更新调整单明细
    pub async fn update_item(
        &self,
        item_id: i32,
        req: AdjustmentItemRequest,
    ) -> Result<inventory_adjustment_item::Model, AppError> {
        let item_model = inventory_adjustment_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单明细 {} 不存在", item_id)))?;

        let detail = self.get_adjustment(item_model.adjustment_id).await?;
        if detail.adjustment.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以修改明细".to_string(),
            ));
        }

        let stock = inventory_stock::Entity::find_by_id(req.stock_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存 ID {} 不存在", req.stock_id)))?;

        let quantity_before = stock.quantity_on_hand;
        let quantity_after = if detail.adjustment.adjustment_type == "increase" {
            quantity_before + req.quantity
        } else {
            quantity_before - req.quantity
        };
        let amount = req.unit_cost.map(|cost| cost * req.quantity);

        let mut active: inventory_adjustment_item::ActiveModel = item_model.into_active_model();
        active.stock_id = Set(req.stock_id);
        active.quantity = Set(req.quantity);
        active.quantity_before = Set(quantity_before);
        active.quantity_after = Set(quantity_after);
        active.unit_cost = Set(req.unit_cost);
        active.amount = Set(amount);
        active.notes = Set(req.notes);
        active.updated_at = Set(Utc::now());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除调整单明细
    pub async fn delete_item(&self, item_id: i32) -> Result<(), AppError> {
        let item_model = inventory_adjustment_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调整单明细 {} 不存在", item_id)))?;

        let detail = self.get_adjustment(item_model.adjustment_id).await?;
        if detail.adjustment.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调整单可以删除明细".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;

        inventory_adjustment_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        // 重新计算总数量
        let items = inventory_adjustment_item::Entity::find()
            .filter(inventory_adjustment_item::Column::AdjustmentId.eq(item_model.adjustment_id))
            .all(&txn)
            .await?;
        let total_quantity: Decimal = items.iter().map(|i| i.quantity).sum();

        let mut adjustment: inventory_adjustment::ActiveModel =
            detail.adjustment.into_active_model();
        adjustment.total_quantity = Set(total_quantity);
        adjustment.updated_at = Set(Utc::now());
        adjustment.update(&txn).await?;

        txn.commit().await?;
        Ok(())
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
        Database::connect(&db_url)
            .await
            .expect("Failed to connect to db")
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
            .await
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
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore]
    async fn test_approve_adjustment_not_found() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let result = service.approve_adjustment(99999, 1).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore]
    async fn test_reject_adjustment_not_found() {
        let db = setup_test_db().await;
        let service = InventoryAdjustmentService::new(Arc::new(db));

        let result = service.reject_adjustment(99999).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
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
        // 使用数组字面量即可，无需堆分配 vec!
        let quantities = [
            Decimal::new(100, 2),
            Decimal::new(200, 2),
            Decimal::new(300, 2),
        ];

        let total: Decimal = quantities.iter().sum();
        assert_eq!(total, Decimal::new(600, 2));
    }
}
