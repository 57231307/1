//! 库存调拨主流程服务（inv/move）
//!
//! 包含库存调拨单（Transfer）的核心 CRUD、状态机（pending → approved → shipped → completed）、
//! 单据号生成器等。原 `inventory_transfer_service.rs` 拆分而来。
//!
//! 子模块协作：
//! - 调拨明细与发出/接收的批次处理：batch.rs
//! - 调出仓库库存预检：stock.rs
//!
//! 注意：文件名 `inventory_move.rs`（非 `move.rs`），因为 `move` 是 Rust 关键字。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};

use crate::models::dto::PageRequest;
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
use crate::models::status::inventory_transfer as transfer_status;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::PaginatedResponse;

use super::{
    CreateInventoryTransferRequest, InventoryTransferDetail, InventoryTransferItemDetail,
    InventoryTransferService, UpdateInventoryTransferRequest,
};

impl InventoryTransferService {
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
        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (transfers, total): (Vec<inventory_transfer::Model>, u64) =
            paginate_with_total(paginator, page_req.page).await?;

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
        // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
        user_id: i32,
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

        // 调出/调入仓库缺失时拒绝创建调拨单，避免脏仓库 ID=0 记录
        let from_warehouse_id = request
            .from_warehouse_id
            .ok_or_else(|| AppError::validation("调拨单缺少调出仓库ID"))?;
        let to_warehouse_id = request
            .to_warehouse_id
            .ok_or_else(|| AppError::validation("调拨单缺少调入仓库ID"))?;

        // 创建调拨主表
        let transfer = inventory_transfer::ActiveModel {
            id: Default::default(),
            transfer_no: sea_orm::ActiveValue::Set(transfer_no),
            from_warehouse_id: sea_orm::ActiveValue::Set(from_warehouse_id),
            to_warehouse_id: sea_orm::ActiveValue::Set(to_warehouse_id),
            transfer_date: sea_orm::ActiveValue::Set(
                request.transfer_date.unwrap_or_else(chrono::Utc::now),
            ),
            status: sea_orm::ActiveValue::Set(
                request.status.unwrap_or_else(|| transfer_status::PENDING.to_string()),
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

        // 检查调出仓库库存是否充足（详见 stock.rs）
        let items = request.items.unwrap_or_default();
        self.check_from_warehouse_inventory(&from_warehouse_id, &items, &txn)
            .await?;

        // 创建调拨明细项并计算总数量
        let mut total_quantity = rust_decimal::Decimal::ZERO;

        for item_req in items {
            let quantity = item_req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
            total_quantity += quantity;

            // 物料 ID 缺失时拒绝创建调拨明细，避免脏 product_id=0 记录
            let item = inventory_transfer_item::ActiveModel {
                id: Default::default(),
                transfer_id: sea_orm::ActiveValue::Set(transfer_entity.id),
                product_id: sea_orm::ActiveValue::Set(
                    item_req
                        .product_id
                        .ok_or_else(|| AppError::validation("调拨明细缺少物料ID"))?,
                ),
                quantity: sea_orm::ActiveValue::Set(quantity),
                shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                unit_cost: sea_orm::ActiveValue::NotSet,
                notes: sea_orm::ActiveValue::Set(item_req.notes),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                // v14 批次 417：面料行业追溯字段（T-P0-1），使用 NotSet 让 DB 默认值处理
                color_no: sea_orm::ActiveValue::NotSet,
                dye_lot_no: sea_orm::ActiveValue::NotSet,
                batch_no: sea_orm::ActiveValue::NotSet,
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
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
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
        // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
        user_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，已完成的调拨单不允许修改
        if transfer.status == transfer_status::COMPLETED {
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
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
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

                // 物料 ID 缺失时拒绝创建调拨明细，避免脏 product_id=0 记录
                let item = inventory_transfer_item::ActiveModel {
                    id: Default::default(),
                    transfer_id: sea_orm::ActiveValue::Set(transfer_id),
                    product_id: sea_orm::ActiveValue::Set(
                        item_req
                            .product_id
                            .ok_or_else(|| AppError::validation("调拨明细缺少物料ID"))?,
                    ),
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
                // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
                Some(user_id),
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
        // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
        user_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 检查调拨单是否存在（行锁，串行化并发状态变更）
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有待审核的调拨单可以审核
        if transfer.status != transfer_status::PENDING {
            return Err(AppError::business(
                "只有待审核状态的调拨单可以审核".to_string(),
            ));
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        if approved {
            transfer_update.status = sea_orm::ActiveValue::Set(transfer_status::APPROVED.to_string());
            transfer_update.approved_by = sea_orm::ActiveValue::NotSet; // 实际应从认证信息获取
            transfer_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        } else {
            transfer_update.status = sea_orm::ActiveValue::Set(transfer_status::REJECTED.to_string());
        }
        if let Some(n) = notes {
            transfer_update.notes = sea_orm::ActiveValue::Set(Some(n));
        }
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
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

    /// 删除调拨单（仅 pending/rejected 状态）
    pub async fn delete_transfer(
        &self,
        transfer_id: i32,
        // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
        user_id: i32,
    ) -> Result<(), AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        if transfer.status == transfer_status::APPROVED
            || transfer.status == transfer_status::SHIPPED
            || transfer.status == transfer_status::COMPLETED
        {
            return Err(AppError::business(format!(
                "调拨单状态 {} 不允许删除",
                transfer.status
            )));
        }

        let txn = (*self.db).begin().await?;
        InventoryTransferItemEntity::delete_many()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .exec(&txn)
            .await?;
        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            InventoryTransferEntity,
            _,
        >(&txn, "inventory_transfer", transfer_id, Some(user_id))
        .await?;
        txn.commit().await?;
        Ok(())
    }
}
