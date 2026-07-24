//! 采购入库-查询子模块（purchase_receipt_ops/query）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 3 个只读查询方法：
//! - `list_receipts`：分页查询入库单列表（接入 paginate_with_total 统一分页逻辑）
//! - `get_receipt`：查询单个入库单详情
//! - `list_receipt_items`：查询入库单的明细列表
//!
//! 纯只读方法，无跨模块调用需求。

use sea_orm::{ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder};

use crate::models::{purchase_receipt, purchase_receipt_item};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// 获取入库单列表（分页）
    pub async fn list_receipts(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        order_id: Option<i32>,
    ) -> Result<(Vec<purchase_receipt::Model>, u64), AppError> {
        let mut query = purchase_receipt::Entity::find();

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_receipt::Column::ReceiptStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_receipt::Column::SupplierId.eq(supplier_id));
        }
        if let Some(order_id) = order_id {
            query = query.filter(purchase_receipt::Column::OrderId.eq(order_id));
        }

        // 分页查询
        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_receipt::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取入库单详情
    pub async fn get_receipt(&self, receipt_id: i32) -> Result<purchase_receipt::Model, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        Ok(receipt)
    }

    /// 获取入库明细列表
    pub async fn list_receipt_items(
        &self,
        receipt_id: i32,
    ) -> Result<Vec<purchase_receipt_item::Model>, AppError> {
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .order_by(purchase_receipt_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(items)
    }
}
