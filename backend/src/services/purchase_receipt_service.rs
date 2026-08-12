//! 采购入库 Service（facade）
//!
//! 本文件为 facade：保留 `PurchaseReceiptService` struct 定义、`new` 构造器、
//! 单号生成宏 `impl_generate_no!`（`generate_receipt_no`）、3 个纯函数
//! （`build_receipt_active_model` / `build_receipt_items_and_totals` /
//! `build_confirmed_receipt_active_model`）以及单元测试模块。
//!
//! 业务 impl 块已按职责拆分到 [`crate::services::purchase_receipt_ops`] 子模块：
//! - `auth`：管理员身份校验 `is_admin_user`（`pub(crate)`，供 crud/items 跨模块调用）
//! - `crud`：入库单 CRUD（create_receipt / update_receipt / delete_receipt + update_receipt_totals）
//! - `state`：状态流转（confirm_receipt + lock_and_validate_receipt_txn + publish_events_and_generate_ap）
//! - `items`：入库明细 CRUD + 总金额重算（add/update/delete_receipt_item + calculate_receipt_total[_txn]）
//! - `query`：列表/详情/明细查询（list_receipts / get_receipt / list_receipt_items）
//!
//! `db` 字段声明为 `pub(crate)`，purchase_receipt_ops 子模块的 impl 块可直接访问。
//! 跨 ops 子模块调用的纯函数（build_*）声明为 `pub(crate)`。
//! 外部调用路径不变：`crate::services::purchase_receipt_service::PurchaseReceiptService`
//! 与 `crate::services::purchase_receipt_dto::*` 均保持稳定。
//!
//! 历史注释：
//! - 批次 101 v6 复审 P2 修复：calculate_receipt_total_txn / calculate_receipt_total 审计操作人 Some(0) 占位符改为真实 user_id，三处内部调用方同步透传 user_id（P2-6）。

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest,
};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, Set};
use std::sync::Arc;

/// 采购入库服务
/// 批次 D10 拆分：struct 定义与 `new` 构造器保留在 facade（本文件），；impl 业务方法块分散到 `purchase_receipt_ops` 子模块（auth/crud/state/items/query）。；`db` 字段为 `pub(crate)` 供 ops 子模块访问。
pub struct PurchaseReceiptService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl PurchaseReceiptService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成入库单号
    // 格式：GR + 年月日 + 三位序号（GR20260315001）
    //
    // 单号生成宏保留在 facade：generate_receipt_no 为 `pub`，
    // crud 子模块的 create_receipt 通过 `self.generate_receipt_no()` 调用。
    crate::impl_generate_no!(
        generate_receipt_no,
        "PR",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo
    );

    // =====================================================
    // 纯函数（无 &self / &db 访问）：保留在 facade，`pub(crate)` 供 ops 子模块调用
    // =====================================================

    /// 构建入库单主表 ActiveModel（String 字段 clone 避免移动 req）（`pub(crate)`：crud 子模块的 `create_receipt` 调用。）
    pub(crate) fn build_receipt_active_model(
        req: &CreatePurchaseReceiptRequest,
        receipt_no: String,
        user_id: i32,
    ) -> purchase_receipt::ActiveModel {
        purchase_receipt::ActiveModel {
            receipt_no: Set(receipt_no),
            order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            receipt_date: Set(req.receipt_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            receiver_id: Set(Some(user_id)),
            inspector_id: Set(req.inspector_id),
            inspection_status: Set("PENDING".to_string()),
            receipt_status: Set(status::purchase_receipt::DRAFT.to_string()),
            notes: Set(req.notes.clone()),
            attachment_urls: Set(req.attachment_urls.clone()),
            created_by: Set(user_id),
            ..Default::default()
        }
    }

    /// 构建入库明细 ActiveModel 列表并累计数量/金额（消费 items）（`pub(crate)`：crud 子模块的 `create_receipt` 调用。）
    pub(crate) fn build_receipt_items_and_totals(
        items: Vec<CreateReceiptItemRequest>,
        receipt_id: i32,
    ) -> (
        Vec<purchase_receipt_item::ActiveModel>,
        Decimal,
        Decimal,
        Decimal,
    ) {
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);
        let mut total_amount = Decimal::new(0, 0);
        let mut item_active_models: Vec<purchase_receipt_item::ActiveModel> =
            Vec::with_capacity(items.len());
        for item_req in items {
            let amount =
                item_req.quantity * item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0));
            total_quantity += item_req.quantity;
            total_quantity_alt += item_req.quantity_alt;
            total_amount += amount;

            item_active_models.push(purchase_receipt_item::ActiveModel {
                receipt_id: Set(receipt_id),
                order_item_id: Set(item_req.order_item_id),
                product_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                quantity_alt: Set(Some(item_req.quantity_alt)),
                unit_price: Set(Some(
                    item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)),
                )),
                amount: Set(Some(amount)),
                notes: Set(item_req.notes),
                ..Default::default()
            });
        }
        (
            item_active_models,
            total_quantity,
            total_quantity_alt,
            total_amount,
        )
    }

    /// 构造 CONFIRMED 状态 ActiveModel，写入确认时间与审计字段（`pub(crate)`：state 子模块的 `confirm_receipt` 调用。）
    pub(crate) fn build_confirmed_receipt_active_model(
        receipt: purchase_receipt::Model,
        user_id: i32,
    ) -> purchase_receipt::ActiveModel {
        let now = chrono::Utc::now();
        let mut active: purchase_receipt::ActiveModel = receipt.into();
        active.receipt_status = Set(status::purchase_receipt::CONFIRMED.to_string());
        active.confirmed_at = Set(Some(now));
        active.confirmed_by = Set(Some(user_id));
        active.updated_by = Set(Some(user_id));
        active.updated_at = Set(now);
        active
    }
}
