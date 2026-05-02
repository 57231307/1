use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
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
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Vec<InventoryTransferItemRequest>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryTransferItemRequest {
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
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
    ) -> Result<PaginatedResponse<InventoryTransferDetail>, sea_orm::DbErr> {
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
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
        // 获取调拨主表数据
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存调拨单 {} 未找到", transfer_id))
            })?;

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
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
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
            return Err(sea_orm::DbErr::Custom("调拨单号已存在，请重试".to_string()));
        }

        // 创建调拨主表
        let transfer = inventory_transfer::ActiveModel {
            id: Default::default(),
            transfer_no: sea_orm::ActiveValue::Set(transfer_no),
            from_warehouse_id: sea_orm::ActiveValue::Set(request.from_warehouse_id),
            to_warehouse_id: sea_orm::ActiveValue::Set(request.to_warehouse_id),
            transfer_date: sea_orm::ActiveValue::Set(
                request.transfer_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(request.status),
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
        self.check_from_warehouse_inventory(&request.from_warehouse_id, &request.items, &txn)
            .await?;

        // 创建调拨明细项并计算总数量
        let mut total_quantity = rust_decimal::Decimal::ZERO;

        for item_req in request.items {
            total_quantity += &item_req.quantity;

            // 创建明细项
            let item = inventory_transfer_item::ActiveModel {
                id: Default::default(),
                transfer_id: sea_orm::ActiveValue::Set(transfer_entity.id),
                product_id: sea_orm::ActiveValue::Set(item_req.product_id),
                quantity: sea_orm::ActiveValue::Set(item_req.quantity),
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
        transfer_update.update(&txn).await?;

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
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存调拨单 {} 未找到", transfer_id))
            })?;

        // 检查状态，已完成的调拨单不允许修改
        if transfer.status == "completed" {
            return Err(sea_orm::DbErr::Custom(
                "调拨单已完成，不允许修改".to_string(),
            ));
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
        transfer_update.update(&txn).await?;

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
                total_quantity += &item_req.quantity;

                let item = inventory_transfer_item::ActiveModel {
                    id: Default::default(),
                    transfer_id: sea_orm::ActiveValue::Set(transfer_id),
                    product_id: sea_orm::ActiveValue::Set(item_req.product_id),
                    quantity: sea_orm::ActiveValue::Set(item_req.quantity),
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
                .ok_or_else(|| sea_orm::DbErr::Custom("调拨单不存在".to_string()))?;
            let mut transfer_update: inventory_transfer::ActiveModel = transfer_entity.into();
            transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
            transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            transfer_update.update(&txn).await?;
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
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存调拨单 {} 未找到", transfer_id))
            })?;

        // 检查状态，只有待审核的调拨单可以审核
        if transfer.status != "pending" {
            return Err(sea_orm::DbErr::Custom(
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
        transfer_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 发出库存调拨
    pub async fn ship_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存调拨单 {} 未找到", transfer_id))
            })?;

        // 检查状态，只有已审核的调拨单可以发出
        if transfer.status != "approved" {
            return Err(sea_orm::DbErr::Custom(
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

        // 扣减源仓库库存
        for item in items {
            // 查找源仓库库存记录
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(&txn)
                .await?;

            if let Some(stock_model) = stock {
                // 检查库存是否充足
                if stock_model.quantity_on_hand < item.quantity {
                    tracing::error!("Transaction rolled back: 产品 {} 库存不足", item.product_id);
                    txn.rollback().await?;
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 库存不足",
                        item.product_id
                    )));
                }

                // 保存需要使用的值
                let quantity_on_hand = stock_model.quantity_on_hand;
                let quantity_meters = stock_model.quantity_meters;

                // 扣减库存
                let mut stock_active: inventory_stock::ActiveModel = stock_model.into();
                stock_active.quantity_on_hand =
                    sea_orm::ActiveValue::Set(quantity_on_hand - item.quantity);
                stock_active.quantity_available =
                    sea_orm::ActiveValue::Set(quantity_on_hand - item.quantity);
                stock_active.quantity_meters =
                    sea_orm::ActiveValue::Set(quantity_meters - item.quantity);
                stock_active.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                stock_active.update(&txn).await?;

                // 更新明细项已发出数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.shipped_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                item_update.update(&txn).await?;
            } else {
                tracing::error!("Transaction rolled back: 产品 {} 在源仓库无库存记录", item.product_id);
                txn.rollback().await?;
                return Err(sea_orm::DbErr::Custom(format!(
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
        transfer_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 接收库存调拨
    pub async fn receive_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, sea_orm::DbErr> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存调拨单 {} 未找到", transfer_id))
            })?;

        // 检查状态，只有已发出的调拨单可以接收
        if transfer.status != "shipped" {
            return Err(sea_orm::DbErr::Custom(
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

        // 增加目标仓库库存
        for item in items {
            // 查找目标仓库库存记录
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.to_warehouse_id))
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(&txn)
                .await?;

            if let Some(stock_model) = stock {
                // 增加库存
                let quantity_on_hand = stock_model.quantity_on_hand;
                let quantity_meters = stock_model.quantity_meters;
                let mut stock_active: inventory_stock::ActiveModel = stock_model.into();
                stock_active.quantity_on_hand =
                    sea_orm::ActiveValue::Set(quantity_on_hand + item.quantity);
                stock_active.quantity_available =
                    sea_orm::ActiveValue::Set(quantity_on_hand + item.quantity);
                stock_active.quantity_meters =
                    sea_orm::ActiveValue::Set(quantity_meters + item.quantity);
                stock_active.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                stock_active.update(&txn).await?;

                // 更新明细项已接收数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.received_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                item_update.update(&txn).await?;
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
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    production_date: sea_orm::ActiveValue::Set(production_date),
                    expiry_date: sea_orm::ActiveValue::Set(expiry_date),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    gram_weight: sea_orm::ActiveValue::Set(gram_weight),
                    width: sea_orm::ActiveValue::Set(width),
                    location_id: sea_orm::ActiveValue::NotSet,
                    shelf_no: sea_orm::ActiveValue::NotSet,
                    layer_no: sea_orm::ActiveValue::NotSet,
                    bin_location: sea_orm::ActiveValue::NotSet,
                    stock_status: sea_orm::ActiveValue::Set("正常".to_string()),
                    quality_status: sea_orm::ActiveValue::Set("合格".to_string()),
                };
                new_stock.insert(&txn).await?;
            }
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        transfer_update.received_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        transfer_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 生成调拨单号
    async fn generate_transfer_no(&self) -> Result<String, sea_orm::DbErr> {
        DocumentNumberGenerator::generate_no(
            &self.db,
            "TRF",
            inventory_transfer::Entity,
            inventory_transfer::Column::TransferNo,
        )
        .await
        .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))
    }

    /// 检查调出仓库库存是否充足
    async fn check_from_warehouse_inventory(
        &self,
        from_warehouse_id: &i32,
        items: &[InventoryTransferItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), sea_orm::DbErr> {
        for item in items {
            // 查询调出仓库的库存
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(*from_warehouse_id))
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            match stock {
                Some(s) if s.quantity_available >= item.quantity => {
                    // 库存充足，继续检查下一个产品
                    continue;
                }
                Some(s) => {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "调出仓库库存不足，产品 {}，当前库存：{}，需要调拨：{}",
                        item.product_id, s.quantity_available, item.quantity
                    )));
                }
                None => {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 在调出仓库没有库存记录",
                        item.product_id
                    )));
                }
            }
        }
        Ok(())
    }
}
