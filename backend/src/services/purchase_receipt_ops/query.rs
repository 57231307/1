//! 采购入库-查询子模块（purchase_receipt_ops/query）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 3 个只读查询方法：
//! - `list_receipts`：分页查询入库单列表（接入 paginate_with_total 统一分页逻辑）
//! - `get_receipt`：查询单个入库单详情
//! - `list_receipt_items`：查询入库单的明细列表
//!
//! 纯只读方法，无跨模块调用需求。

use sea_orm::sea_query::{Condition, Expr, Query, SelectStatement};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};

use crate::models::{
    purchase_order, purchase_receipt, purchase_receipt_item, supplier, user, warehouse,
};
use crate::services::purchase_receipt_dto::PurchaseReceiptDto;
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;

/// 明细物料名关键字相关子查询（EXISTS，非 JOIN）：`purchase_receipt_item.material_name`
/// 与主表是一对多，若为凑关键字改 LEFT JOIN 会行倍增、污染分页 total；
/// 子查询形态先例 `po/order_ops/crud.rs:633-645`（received_amount_subquery 同构相关列写法）。
/// 模式串已由 `safe_like_pattern` 转义 `%_\`，走参数化占位，无字符串拼接 SQL。
fn material_name_exists_subquery(pattern: &str) -> SelectStatement {
    Query::select()
        .expr(Expr::col((
            purchase_receipt_item::Entity,
            purchase_receipt_item::Column::Id,
        )))
        .from(purchase_receipt_item::Entity)
        .and_where(
            Expr::col((
                purchase_receipt_item::Entity,
                purchase_receipt_item::Column::ReceiptId,
            ))
            .equals((purchase_receipt::Entity, purchase_receipt::Column::Id)),
        )
        .and_where(
            Expr::col((
                purchase_receipt_item::Entity,
                purchase_receipt_item::Column::MaterialName,
            ))
            .like(pattern),
        )
        .to_owned()
}

impl PurchaseReceiptService {
    /// 获取入库单列表（分页）—— 单次 LEFT JOIN 查询富化名称字段，无 N+1。
    ///
    /// 关联关系均为 many-to-one（入库单 -> 供应商/仓库/采购订单/创建人），
    /// LEFT JOIN 不会产生行倍增，分页计数安全。
    ///
    /// `total` 由 `paginate_with_total` 对**同一已过滤查询**做 `num_items()` 统计
    /// （utils/pagination.rs:20-21），关键字/仓库/日期条件天然计入过滤后行数。
    #[allow(clippy::too_many_arguments)]
    pub async fn list_receipts(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        order_id: Option<i32>,
        keyword: Option<String>,
        warehouse_id: Option<i32>,
        receipt_date_from: Option<NaiveDate>,
        receipt_date_to: Option<NaiveDate>,
    ) -> Result<(Vec<PurchaseReceiptDto>, u64), AppError> {
        let mut query = purchase_receipt::Entity::find()
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(purchase_order::Column::OrderNo, "purchase_order_no")
            .column_as(user::Column::RealName, "created_by_name")
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Supplier.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Warehouse.def(),
            )
            .join(JoinType::LeftJoin, purchase_receipt::Relation::Order.def())
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Creator.def(),
            );

        if let Some(ref status) = status {
            query = query.filter(purchase_receipt::Column::ReceiptStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_receipt::Column::SupplierId.eq(supplier_id));
        }
        if let Some(order_id) = order_id {
            query = query.filter(purchase_receipt::Column::OrderId.eq(order_id));
        }
        if let Some(warehouse_id) = warehouse_id {
            query = query.filter(purchase_receipt::Column::WarehouseId.eq(warehouse_id));
        }
        // 日期区间含首尾：receipt_date 是 DATE 列（models/purchase_receipt.rs:35 NaiveDate，
        // 非时间戳），lte(当日) 即含上界整日，无需销售侧「次日不含」补偿
        // （so/order_query.rs:173-185 那套只适用于 timestamp 列）。
        if let Some(from) = receipt_date_from {
            query = query.filter(purchase_receipt::Column::ReceiptDate.gte(from));
        }
        if let Some(to) = receipt_date_to {
            query = query.filter(purchase_receipt::Column::ReceiptDate.lte(to));
        }
        // 关键字：入库单号 或 明细物料名（EXISTS 相关子查询，见文件头注释）。
        // 用 LIKE 而非 ILIKE：ILIKE 是 PG 方言，sea-query 的 sqlite 构建器直接
        // unimplemented!() panic，无法 sqlite 真跑；采购域列表既有写法即
        // safe_like_pattern + like（po/order_ops/crud.rs:695-702），不另起第二套口径。
        // 空串在 handler 边界已由 empty_str_as_none 归一为 None，此处 filter 双保险。
        if let Some(kw) = keyword.as_deref().filter(|s| !s.is_empty()) {
            let pattern = safe_like_pattern(kw);
            query = query.filter(
                Condition::any()
                    .add(purchase_receipt::Column::ReceiptNo.like(&pattern))
                    .add(Expr::exists(material_name_exists_subquery(&pattern))),
            );
        }

        let paginator = query
            .order_by(purchase_receipt::Column::CreatedAt, Order::Desc)
            .into_model::<PurchaseReceiptDto>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取入库单详情 —— 同样 LEFT JOIN 富化名称字段。
    pub async fn get_receipt(&self, receipt_id: i32) -> Result<PurchaseReceiptDto, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(purchase_order::Column::OrderNo, "purchase_order_no")
            .column_as(user::Column::RealName, "created_by_name")
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Supplier.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Warehouse.def(),
            )
            .join(JoinType::LeftJoin, purchase_receipt::Relation::Order.def())
            .join(
                JoinType::LeftJoin,
                purchase_receipt::Relation::Creator.def(),
            )
            .into_model::<PurchaseReceiptDto>()
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
