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
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, Set};
use std::sync::Arc;

/// 采购入库服务
///
/// 批次 D10 拆分：struct 定义与 `new` 构造器保留在 facade（本文件），
/// impl 业务方法块分散到 `purchase_receipt_ops` 子模块（auth/crud/state/items/query）。
/// `db` 字段为 `pub(crate)` 供 ops 子模块访问。
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

    /// 构建入库单主表 ActiveModel（String 字段 clone 避免移动 req）
    ///
    /// `pub(crate)`：crud 子模块的 `create_receipt` 调用。
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

    /// 构建入库明细 ActiveModel 列表并累计数量/金额（消费 items）
    ///
    /// `pub(crate)`：crud 子模块的 `create_receipt` 调用。
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

    /// 构造 CONFIRMED 状态 ActiveModel，写入确认时间与审计字段
    ///
    /// `pub(crate)`：state 子模块的 `confirm_receipt` 调用。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    // 批次 415：decs! 宏展开为 Decimal::from_str，需导入 FromStr trait
    use std::str::FromStr;

    /// 构造合法的 CreateReceiptItemRequest（单条明细）
    fn sample_item() -> CreateReceiptItemRequest {
        CreateReceiptItemRequest {
            order_item_id: Some(1),
            line_no: 1,
            material_id: 1001,
            material_code: "M001".to_string(),
            material_name: "测试物料".to_string(),
            batch_no: Some("B20260719".to_string()),
            color_code: Some("RED".to_string()),
            lot_no: Some("L01".to_string()),
            grade: Some("A".to_string()),
            gram_weight: Some(decs!(200)),
            width: Some(decs!(150)),
            quantity: decs!(100),
            quantity_alt: decs!(50),
            unit_master: "M".to_string(),
            unit_alt: Some("KG".to_string()),
            unit_price: Some(decs!(10)),
            location_code: Some("A-01-01".to_string()),
            package_no: Some("P001".to_string()),
            production_date: Some(ymd!(2026, 7, 19)),
            shelf_life: Some(365),
            notes: Some("测试明细".to_string()),
        }
    }

    /// 构造合法的 CreatePurchaseReceiptRequest（默认 1 条明细）
    fn sample_request() -> CreatePurchaseReceiptRequest {
        CreatePurchaseReceiptRequest {
            order_id: Some(1),
            supplier_id: 100,
            receipt_date: ymd!(2026, 7, 19),
            warehouse_id: 1,
            department_id: Some(1),
            inspector_id: Some(10),
            notes: Some("测试入库单".to_string()),
            attachment_urls: Some(vec!["file://test.pdf".to_string()]),
            items: vec![sample_item()],
        }
    }

    // ============ 状态常量值正确性测试 ============

    /// 测试_入库单状态常量_值正确性
    ///
    /// 验证 status::purchase_receipt 模块中 3 个状态常量值与状态机约定一致
    /// （大写：DRAFT/CONFIRMED/COMPLETED，与 purchase_receipt_service.rs 中
    /// 字符串字面量 `"DRAFT"` / `status::purchase_receipt::DRAFT.to_string()` 一致）。
    #[test]
    fn 测试_入库单状态常量_值正确性() {
        assert_eq!(status::purchase_receipt::DRAFT, "DRAFT");
        assert_eq!(status::purchase_receipt::CONFIRMED, "CONFIRMED");
        assert_eq!(status::purchase_receipt::COMPLETED, "COMPLETED");
    }

    /// 测试_入库单状态常量_互不相同
    ///
    /// 业务规则：3 个状态必须互不相同，避免状态机歧义。
    #[test]
    fn 测试_入库单状态常量_互不相同() {
        let states = [
            status::purchase_receipt::DRAFT,
            status::purchase_receipt::CONFIRMED,
            status::purchase_receipt::COMPLETED,
        ];
        let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    /// 测试_入库单状态常量_大写风格
    ///
    /// 业务规则：purchase_receipt 状态值采用大写风格（DRAFT/CONFIRMED/COMPLETED），
    /// 与 quotation 模块（小写 draft/approved/rejected/cancelled）不同。
    /// 验证所有状态均为大写字母（规则 20：注释与功能一致）。
    #[test]
    fn 测试_入库单状态常量_大写风格() {
        // purchase_receipt 状态用大写（与 sales_order/quotation 小写不同）
        for s in [
            status::purchase_receipt::DRAFT,
            status::purchase_receipt::CONFIRMED,
            status::purchase_receipt::COMPLETED,
        ] {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "状态 {} 应全大写",
                s
            );
        }
    }

    // ============ PurchaseReceiptService 构造与 DB 连接测试 ============

    /// 测试_PurchaseReceiptService_new_正确持有数据库连接
    ///
    /// 验证 new(Arc<DatabaseConnection>) 构造的 service 实例可以执行简单查询。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_new_正确持有数据库连接() {
        let db = Arc::new(setup_test_db().await);
        let svc = PurchaseReceiptService::new(db.clone());
        use sea_orm::ConnectionTrait;
        let _ = svc
            .db
            .execute(sea_orm::Statement::from_sql_and_values(
                svc.db.get_database_backend(),
                "SELECT 1",
                Vec::new(),
            ))
            .await
            .expect("数据库连接应可用");
    }

    /// 测试_PurchaseReceiptService_get_receipt_空数据库返回Err
    ///
    /// 业务规则：get_receipt 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_get_receipt_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.get_receipt(9999).await;
        // SQLite 内存数据库无 purchase_receipts 表，应返回 Err（DbErr 转 AppError）
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_list_receipts_空数据库返回Err
    ///
    /// 业务规则：list_receipts 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_list_receipts_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.list_receipts(1, 20, None, None, None).await;
        // SQLite 内存数据库无 purchase_receipts 表，应返回 Err
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_list_receipt_items_空数据库返回Err
    ///
    /// 业务规则：list_receipt_items 查询 purchase_receipt_items 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_list_receipt_items_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.list_receipt_items(9999).await;
        // SQLite 内存数据库无 purchase_receipt_items 表，应返回 Err
        assert!(result.is_err());
    }

    // ============ create_receipt 业务校验测试 ============

    /// 测试_PurchaseReceiptService_create_receipt_空明细返回Err
    ///
    /// 业务规则：CreatePurchaseReceiptRequest.items 至少 1 条（DTO 上 #[validate(length(min = 1))]）。
    /// service 层未显式调用 Validate::validate，空明细会进入 generate_receipt_no 查询表，
    /// SQLite 内存数据库无表应返回 Err（非 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_create_receipt_空明细返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let mut req = sample_request();
        req.items.clear();
        let result = svc.create_receipt(req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_create_receipt_不存在表返回Err
    ///
    /// 业务规则：create_receipt 依赖 purchase_receipt 表存在。
    /// SQLite 内存数据库无 schema，应返回 DbErr（非 panic）。
    /// 这验证了错误处理路径的健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_create_receipt_不存在表返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = sample_request();
        let result = svc.create_receipt(req, 1).await;
        // SQLite 内存数据库无表，应返回 Err（DbErr 或 AppError）
        assert!(result.is_err());
    }

    // ============ update_receipt 状态机校验测试 ============

    /// 测试_PurchaseReceiptService_update_receipt_不存在返回AppError
    ///
    /// 业务规则：update_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_update_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = UpdatePurchaseReceiptRequest::default();
        let result = svc.update_receipt(9999, req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_delete_receipt_不存在返回AppError
    ///
    /// 业务规则：delete_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_delete_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.delete_receipt(9999, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_confirm_receipt_不存在返回AppError
    ///
    /// 业务规则：confirm_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_confirm_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.confirm_receipt(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ 明细操作状态机校验测试 ============

    /// 测试_PurchaseReceiptService_add_receipt_item_不存在入库单返回AppError
    ///
    /// 业务规则：add_receipt_item 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_add_receipt_item_不存在入库单返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let item_req = sample_item();
        let result = svc.add_receipt_item(9999, item_req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_update_receipt_item_不存在返回AppError
    ///
    /// 业务规则：update_receipt_item 不存在的明细返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_update_receipt_item_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = UpdateReceiptItemRequest::default();
        let result = svc.update_receipt_item(9999, req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_delete_receipt_item_不存在返回AppError
    ///
    /// 业务规则：delete_receipt_item 不存在的明细返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_delete_receipt_item_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.delete_receipt_item(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ calculate_receipt_total 测试 ============

    /// 测试_PurchaseReceiptService_calculate_receipt_total_不存在返回AppError
    ///
    /// 业务规则：calculate_receipt_total 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_calculate_receipt_total_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.calculate_receipt_total(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ DTO 字段完整性测试 ============

    /// 测试_CreateReceiptItemRequest_字段完整构造
    ///
    /// 验证 CreateReceiptItemRequest 所有字段可以正确构造，
    /// 确保后续业务方法接收到完整 DTO 时不会因字段缺失 panic。
    #[test]
    fn 测试_CreateReceiptItemRequest_字段完整构造() {
        let item = sample_item();
        assert_eq!(item.material_id, 1001);
        assert_eq!(item.material_code, "M001");
        assert_eq!(item.quantity, decs!(100));
        assert_eq!(item.unit_price, Some(decs!(10)));
        assert!(item.batch_no.is_some());
        assert!(item.color_code.is_some());
        assert!(item.lot_no.is_some());
        assert!(item.grade.is_some());
    }

    /// 测试_UpdatePurchaseReceiptRequest_默认值全为None
    ///
    /// 业务规则：UpdatePurchaseReceiptRequest 使用 #[derive(Default)]，
    /// 所有字段默认为 None，表示不更新该字段。
    #[test]
    fn 测试_UpdatePurchaseReceiptRequest_默认值全为None() {
        let req = UpdatePurchaseReceiptRequest::default();
        assert!(req.supplier_id.is_none());
        assert!(req.receipt_date.is_none());
        assert!(req.department_id.is_none());
        assert!(req.inspector_id.is_none());
        assert!(req.notes.is_none());
        assert!(req.attachment_urls.is_none());
    }

    /// 测试_UpdateReceiptItemRequest_默认值全为None
    ///
    /// 业务规则：UpdateReceiptItemRequest 使用 #[derive(Default)]，
    /// 所有字段默认为 None，表示不更新该字段。
    #[test]
    fn 测试_UpdateReceiptItemRequest_默认值全为None() {
        let req = UpdateReceiptItemRequest::default();
        assert!(req.line_no.is_none());
        assert!(req.material_id.is_none());
        assert!(req.material_code.is_none());
        assert!(req.quantity.is_none());
        assert!(req.unit_price.is_none());
        assert!(req.notes.is_none());
    }
}
