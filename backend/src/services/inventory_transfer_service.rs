use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transaction;
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;
use serde::{Deserialize, Serialize};

/// 库存调拨详情响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryTransferDetail {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<InventoryTransferItemDetail>,
}

/// 库存调拨明细项详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryTransferItemDetail {
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub unit_cost: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建库存调拨请求
#[derive(Debug, Deserialize)]
pub struct CreateInventoryTransferRequest {
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryTransferItemRequest {
    pub product_id: Option<i32>,
    pub quantity: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
}

/// 更新库存调拨请求
#[derive(Debug, Deserialize)]
pub struct UpdateInventoryTransferRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

/// 库存调拨服务
pub struct InventoryTransferService {
    db: Arc<DatabaseConnection>,
}

impl InventoryTransferService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取库存调拨列表
    pub async fn list_transfers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        from_warehouse_id: Option<i32>,
        to_warehouse_id: Option<i32>,
        transfer_no: Option<String>,
    ) -> Result<PaginatedResponse<InventoryTransferDetail>, AppError> {
        let mut query = InventoryTransferEntity::find()
            .order_by(inventory_transfer::Column::CreatedAt, Order::Desc);

        // 应用过滤条件
        if let Some(s) = status {
            query = query.filter(inventory_transfer::Column::Status.eq(s));
        }
        if let Some(fid) = from_warehouse_id {
            query = query.filter(inventory_transfer::Column::FromWarehouseId.eq(fid));
        }
        if let Some(tid) = to_warehouse_id {
            query = query.filter(inventory_transfer::Column::ToWarehouseId.eq(tid));
        }
        if let Some(no) = transfer_no {
            query = query.filter(inventory_transfer::Column::TransferNo.contains(&no));
        }

        // 分页
        let paginator = query.paginate(&*self.db, page_req.page_size);
        let total = paginator.num_items().await?;
        let transfers: Vec<inventory_transfer::Model> =
            paginator.fetch_page(page_req.page - 1).await?;

        // 转换为响应格式
        let transfer_details: Vec<InventoryTransferDetail> = transfers
            .into_iter()
            .map(|transfer| InventoryTransferDetail {
                id: transfer.id,
                transfer_no: transfer.transfer_no,
                from_warehouse_id: transfer.from_warehouse_id,
                to_warehouse_id: transfer.to_warehouse_id,
                transfer_date: transfer.transfer_date,
                status: transfer.status,
                total_quantity: transfer.total_quantity,
                notes: transfer.notes,
                created_by: transfer.created_by,
                approved_by: transfer.approved_by,
                approved_at: transfer.approved_at,
                shipped_at: transfer.shipped_at,
                received_at: transfer.received_at,
                created_at: transfer.created_at,
                updated_at: transfer.updated_at,
                items: vec![], // 列表接口不返回明细项
            })
            .collect();

        Ok(PaginatedResponse::new(
            transfer_details,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取库存调拨详情（包含明细项）
    pub async fn get_transfer_detail(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 获取调拨主表数据
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 获取调拨明细项
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .order_by(inventory_transfer_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        // 转换为响应格式
        let item_details: Vec<InventoryTransferItemDetail> = items
            .into_iter()
            .map(|item| InventoryTransferItemDetail {
                id: item.id,
                transfer_id: item.transfer_id,
                product_id: item.product_id,
                quantity: item.quantity,
                shipped_quantity: item.shipped_quantity,
                received_quantity: item.received_quantity,
                unit_cost: item.unit_cost,
                notes: item.notes,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect();

        Ok(InventoryTransferDetail {
            id: transfer.id,
            transfer_no: transfer.transfer_no,
            from_warehouse_id: transfer.from_warehouse_id,
            to_warehouse_id: transfer.to_warehouse_id,
            transfer_date: transfer.transfer_date,
            status: transfer.status,
            total_quantity: transfer.total_quantity,
            notes: transfer.notes,
            created_by: transfer.created_by,
            approved_by: transfer.approved_by,
            approved_at: transfer.approved_at,
            shipped_at: transfer.shipped_at,
            received_at: transfer.received_at,
            created_at: transfer.created_at,
            updated_at: transfer.updated_at,
            items: item_details,
        })
    }

    /// 创建库存调拨
    pub async fn create_transfer(
        &self,
        request: CreateInventoryTransferRequest,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 生成调拨单号并检查唯一性
        let transfer_no = self.generate_transfer_no().await?;

        // 再次检查调拨单号是否已存在（防止并发冲突）
        let existing_transfer = InventoryTransferEntity::find()
            .filter(inventory_transfer::Column::TransferNo.eq(&transfer_no))
            .one(&txn)
            .await?;

        if existing_transfer.is_some() {
            tracing::error!("Transaction rolled back: 调拨单号 {} 已存在", transfer_no);
            txn.rollback().await?;
            return Err(AppError::business("调拨单号已存在，请重试".to_string()));
        }

        // 创建调拨主表
        let transfer = inventory_transfer::ActiveModel {
            id: Default::default(),
            transfer_no: sea_orm::ActiveValue::Set(transfer_no),
            from_warehouse_id: sea_orm::ActiveValue::Set(request.from_warehouse_id.unwrap_or(0)),
            to_warehouse_id: sea_orm::ActiveValue::Set(request.to_warehouse_id.unwrap_or(0)),
            transfer_date: sea_orm::ActiveValue::Set(
                request.transfer_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(
                request.status.unwrap_or_else(|| "pending".to_string()),
            ),
            total_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            notes: sea_orm::ActiveValue::Set(request.notes),
            created_by: sea_orm::ActiveValue::NotSet,
            approved_by: sea_orm::ActiveValue::NotSet,
            approved_at: sea_orm::ActiveValue::NotSet,
            shipped_at: sea_orm::ActiveValue::NotSet,
            received_at: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };

        let transfer_entity = transfer.insert(&txn).await?;

        // 检查调出仓库库存是否充足
        let from_warehouse_id = request.from_warehouse_id.unwrap_or(0);
        let items = request.items.unwrap_or_default();
        self.check_from_warehouse_inventory(&from_warehouse_id, &items, &txn)
            .await?;

        // 创建调拨明细项并计算总数量
        let mut total_quantity = rust_decimal::Decimal::ZERO;

        for item_req in items {
            let quantity = item_req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
            total_quantity += quantity;

            // 创建明细项
            let item = inventory_transfer_item::ActiveModel {
                id: Default::default(),
                transfer_id: sea_orm::ActiveValue::Set(transfer_entity.id),
                product_id: sea_orm::ActiveValue::Set(item_req.product_id.unwrap_or(0)),
                quantity: sea_orm::ActiveValue::Set(quantity),
                shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                unit_cost: sea_orm::ActiveValue::NotSet,
                notes: sea_orm::ActiveValue::Set(item_req.notes),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            };

            item.insert(&txn).await?;
        }

        // 更新调拨单总数量
        let transfer_id = transfer_entity.id;
        let mut transfer_update: inventory_transfer::ActiveModel = transfer_entity.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 更新库存调拨
    pub async fn update_transfer(
        &self,
        transfer_id: i32,
        request: UpdateInventoryTransferRequest,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，已完成的调拨单不允许修改
        if transfer.status == "completed" {
            return Err(AppError::business("调拨单已完成，不允许修改".to_string()));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新调拨主表
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        if let Some(status) = request.status {
            transfer_update.status = sea_orm::ActiveValue::Set(status);
        }
        if let Some(notes) = request.notes {
            transfer_update.notes = sea_orm::ActiveValue::Set(Some(notes));
        }
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 如果需要更新明细项
        if let Some(items) = request.items {
            // 删除原有明细项
            InventoryTransferItemEntity::delete_many()
                .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
                .exec(&txn)
                .await?;

            // 重新计算总数量并创建新明细项
            let mut total_quantity = rust_decimal::Decimal::ZERO;

            for item_req in items {
                let quantity = item_req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
                total_quantity += quantity;

                let item = inventory_transfer_item::ActiveModel {
                    id: Default::default(),
                    transfer_id: sea_orm::ActiveValue::Set(transfer_id),
                    product_id: sea_orm::ActiveValue::Set(item_req.product_id.unwrap_or(0)),
                    quantity: sea_orm::ActiveValue::Set(quantity),
                    shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    unit_cost: sea_orm::ActiveValue::NotSet,
                    notes: sea_orm::ActiveValue::Set(item_req.notes),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };

                item.insert(&txn).await?;
            }

            // 更新调拨单总数量
            let transfer_entity = InventoryTransferEntity::find_by_id(transfer_id)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::business("调拨单不存在"))?;
            let mut transfer_update: inventory_transfer::ActiveModel = transfer_entity.into();
            transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
            transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                transfer_update,
                Some(0),
            )
            .await?;
        }

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 审核库存调拨
    pub async fn approve_transfer(
        &self,
        transfer_id: i32,
        approved: bool,
        notes: Option<String>,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有待审核的调拨单可以审核
        if transfer.status != "pending" {
            return Err(AppError::business(
                "只有待审核状态的调拨单可以审核".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        if approved {
            transfer_update.status = sea_orm::ActiveValue::Set("approved".to_string());
            transfer_update.approved_by = sea_orm::ActiveValue::NotSet; // 实际应从认证信息获取
            transfer_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        } else {
            transfer_update.status = sea_orm::ActiveValue::Set("rejected".to_string());
        }
        if let Some(n) = notes {
            transfer_update.notes = sea_orm::ActiveValue::Set(Some(n));
        }
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 发出库存调拨
    pub async fn ship_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有已审核的调拨单可以发出
        if transfer.status != "approved" {
            return Err(AppError::business(
                "只有已审核状态的调拨单可以发出".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 获取调拨明细项
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(&txn)
            .await?;

        // 批量获取源仓库库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        // 扣减源仓库库存
        for item in items {
            // 查找源仓库库存记录
            let stock = stock_map.get(&item.product_id);

            if let Some(stock_model) = stock {
                // 检查库存是否充足
                if stock_model.quantity_on_hand < item.quantity {
                    tracing::error!("Transaction rolled back: 产品 {} 库存不足", item.product_id);
                    txn.rollback().await?;
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足",
                        item.product_id
                    )));
                }

                // 保存需要使用的值
                let stock_id = stock_model.id;
                let _quantity_on_hand = stock_model.quantity_on_hand;
                let quantity_meters = stock_model.quantity_meters;
                let quantity_kg = stock_model.quantity_kg;
                let expected_version = stock_model.version;
                let batch_no = stock_model.batch_no.clone();
                let color_no = stock_model.color_no.clone();
                let dye_lot_no = stock_model.dye_lot_no.clone();
                let grade = stock_model.grade.clone();
                let _stock_model = stock_model.clone();

                // 扣减库存（带乐观锁）
                let new_quantity_meters = quantity_meters - item.quantity;
                // Calculate kg reduction proportionally
                let new_quantity_kg = if quantity_meters > rust_decimal::Decimal::ZERO {
                    quantity_kg - (quantity_kg * item.quantity / quantity_meters)
                } else {
                    quantity_kg
                };

                // 使用乐观锁条件更新：只有 version 匹配时才更新
                let update_result = inventory_stock::Entity::update_many()
                    .col_expr(
                        inventory_stock::Column::QuantityOnHand,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityOnHand)
                            .sub(sea_orm::sea_query::Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                            .sub(sea_orm::sea_query::Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityMeters,
                        sea_orm::sea_query::Expr::val(new_quantity_meters),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityKg,
                        sea_orm::sea_query::Expr::val(new_quantity_kg),
                    )
                    .col_expr(
                        inventory_stock::Column::Version,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                    )
                    .filter(inventory_stock::Column::Id.eq(stock_id))
                    .filter(inventory_stock::Column::Version.eq(expected_version))
                    .exec(&txn)
                    .await?;

                // 检查乐观锁是否成功
                if update_result.rows_affected == 0 {
                    tracing::error!("Transaction rolled back: 产品 {} 并发冲突", item.product_id);
                    txn.rollback().await?;
                    return Err(AppError::business(format!(
                        "产品 {} 库存记录已被其他用户修改，请重试",
                        item.product_id
                    )));
                }

                // 记录 TRANSFER_OUT 库存流水
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_OUT".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.from_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(quantity_kg - new_quantity_kg),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(quantity_meters)),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(quantity_kg)),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_quantity_meters)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_quantity_kg)),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨出库 - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;

                // 更新明细项已发出数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.shipped_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &txn,
                    "auto_audit",
                    item_update,
                    Some(0),
                )
                .await?;
            } else {
                tracing::error!(
                    "Transaction rolled back: 产品 {} 在源仓库无库存记录",
                    item.product_id
                );
                txn.rollback().await?;
                return Err(AppError::business(format!(
                    "产品 {} 在源仓库无库存记录",
                    item.product_id
                )));
            }
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("shipped".to_string());
        transfer_update.shipped_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 接收库存调拨
    pub async fn receive_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有已发出的调拨单可以接收
        if transfer.status != "shipped" {
            return Err(AppError::business(
                "只有已发出状态的调拨单可以接收".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 获取调拨明细项
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(&txn)
            .await?;

        // 批量获取目标仓库库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        // 增加目标仓库库存
        for item in items {
            // 查找目标仓库库存记录
            let stock = stock_map.get(&item.product_id);

            if let Some(stock_model) = stock {
                // 保存需要使用的值
                let stock_id = stock_model.id;
                let _quantity_on_hand = stock_model.quantity_on_hand;
                let quantity_meters = stock_model.quantity_meters;
                let quantity_kg = stock_model.quantity_kg;
                let expected_version = stock_model.version;
                let batch_no = stock_model.batch_no.clone();
                let color_no = stock_model.color_no.clone();
                let dye_lot_no = stock_model.dye_lot_no.clone();
                let grade = stock_model.grade.clone();
                let _stock_model = stock_model.clone();

                // 增加库存（带乐观锁）
                let new_quantity_meters = quantity_meters + item.quantity;
                // Calculate kg increase proportionally based on source stock
                let source_stock = InventoryStockEntity::find()
                    .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                    .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                    .one(&txn)
                    .await?;
                let source_kg_per_meter = if let Some(ref src) = source_stock {
                    if src.quantity_meters > rust_decimal::Decimal::ZERO {
                        src.quantity_kg / src.quantity_meters
                    } else {
                        rust_decimal::Decimal::ZERO
                    }
                } else {
                    rust_decimal::Decimal::ZERO
                };
                let new_quantity_kg = quantity_kg + (item.quantity * source_kg_per_meter);

                // 使用乐观锁条件更新：只有 version 匹配时才更新
                let update_result = inventory_stock::Entity::update_many()
                    .col_expr(
                        inventory_stock::Column::QuantityOnHand,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityOnHand)
                            .add(sea_orm::sea_query::Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                            .add(sea_orm::sea_query::Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityMeters,
                        sea_orm::sea_query::Expr::val(new_quantity_meters),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityKg,
                        sea_orm::sea_query::Expr::val(new_quantity_kg),
                    )
                    .col_expr(
                        inventory_stock::Column::Version,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                    )
                    .filter(inventory_stock::Column::Id.eq(stock_id))
                    .filter(inventory_stock::Column::Version.eq(expected_version))
                    .exec(&txn)
                    .await?;

                // 检查乐观锁是否成功
                if update_result.rows_affected == 0 {
                    tracing::error!("Transaction rolled back: 产品 {} 并发冲突", item.product_id);
                    txn.rollback().await?;
                    return Err(AppError::business(format!(
                        "产品 {} 库存记录已被其他用户修改，请重试",
                        item.product_id
                    )));
                }

                // 记录 TRANSFER_IN 库存流水
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(quantity_meters)),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(quantity_kg)),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_quantity_meters)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_quantity_kg)),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨入库 - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;

                // 更新明细项已接收数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.received_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &txn,
                    "auto_audit",
                    item_update,
                    Some(0),
                )
                .await?;
            } else {
                // 如果目标仓库没有库存记录，创建新记录
                // 需要从源仓库的库存记录中获取面料行业字段
                let source_stock = InventoryStockEntity::find()
                    .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                    .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                    .one(&txn)
                    .await?;

                let batch_no = source_stock
                    .as_ref()
                    .map(|s| s.batch_no.clone())
                    .unwrap_or_default();
                let color_no = source_stock
                    .as_ref()
                    .map(|s| s.color_no.clone())
                    .unwrap_or_default();
                let dye_lot_no = source_stock.as_ref().and_then(|s| s.dye_lot_no.clone());
                let grade = source_stock
                    .as_ref()
                    .map(|s| s.grade.clone())
                    .unwrap_or_else(|| "一等品".to_string());
                let gram_weight = source_stock.as_ref().and_then(|s| s.gram_weight);
                let width = source_stock.as_ref().and_then(|s| s.width);
                let production_date = source_stock.as_ref().and_then(|s| s.production_date);
                let expiry_date = source_stock.as_ref().and_then(|s| s.expiry_date);

                // 计算源仓库的公斤/米比率
                let source_kg_per_meter = if let Some(ref src) = source_stock {
                    if src.quantity_meters > rust_decimal::Decimal::ZERO {
                        src.quantity_kg / src.quantity_meters
                    } else {
                        rust_decimal::Decimal::ZERO
                    }
                } else {
                    rust_decimal::Decimal::ZERO
                };

                let new_stock = inventory_stock::ActiveModel {
                    id: Default::default(),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    quantity_on_hand: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_available: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_reserved: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    quantity_incoming: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_point: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    last_count_date: sea_orm::ActiveValue::NotSet,
                    last_movement_date: sea_orm::ActiveValue::NotSet,
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    // 面料行业特色字段 - 从源仓库库存复制
                    batch_no: sea_orm::ActiveValue::Set(batch_no.clone()),
                    color_no: sea_orm::ActiveValue::Set(color_no.clone()),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no.clone()),
                    grade: sea_orm::ActiveValue::Set(grade.clone()),
                    production_date: sea_orm::ActiveValue::Set(production_date),
                    expiry_date: sea_orm::ActiveValue::Set(expiry_date),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(item.quantity * source_kg_per_meter),
                    gram_weight: sea_orm::ActiveValue::Set(gram_weight),
                    width: sea_orm::ActiveValue::Set(width),
                    location_id: sea_orm::ActiveValue::NotSet,
                    shelf_no: sea_orm::ActiveValue::NotSet,
                    layer_no: sea_orm::ActiveValue::NotSet,
                    bin_location: sea_orm::ActiveValue::NotSet,
                    stock_status: sea_orm::ActiveValue::Set("正常".to_string()),
                    quality_status: sea_orm::ActiveValue::Set("合格".to_string()),
                    version: sea_orm::ActiveValue::Set(0),
                };
                new_stock.insert(&txn).await?;

                // 记录 TRANSFER_IN 库存流水（新建库存记录的情况）
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(item.quantity * source_kg_per_meter),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(
                        rust_decimal::Decimal::ZERO,
                    )),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(
                        rust_decimal::Decimal::ZERO,
                    )),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(item.quantity)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(
                        item.quantity * source_kg_per_meter,
                    )),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨入库（新建库存） - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;
            }
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        transfer_update.received_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    // 生成调拨单号
    crate::impl_generate_no!(
        generate_transfer_no,
        "TRF",
        inventory_transfer::Entity,
        inventory_transfer::Column::TransferNo
    );

    /// 检查调出仓库库存是否充足
    async fn check_from_warehouse_inventory(
        &self,
        from_warehouse_id: &i32,
        items: &[InventoryTransferItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 批量获取库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items
            .iter()
            .map(|item| item.product_id.unwrap_or(0))
            .collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(*from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            let product_id = item.product_id.unwrap_or(0);
            let quantity = item.quantity.unwrap_or(rust_decimal::Decimal::ZERO);

            // 查询调出仓库的库存
            let stock = stock_map.get(&product_id);

            match stock {
                Some(s) if s.quantity_available >= quantity => {
                    // 库存充足，继续检查下一个产品
                    continue;
                }
                Some(s) => {
                    return Err(AppError::business(format!(
                        "调出仓库库存不足，产品 {}，当前库存：{}，需要调拨：{}",
                        product_id, s.quantity_available, quantity
                    )));
                }
                None => {
                    return Err(AppError::business(format!(
                        "产品 {} 在调出仓库没有库存记录",
                        product_id
                    )));
                }
            }
        }
        Ok(())
    }
}
