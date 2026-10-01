//! 采购入库 Service（facade）
//!
//! 本文件为 facade：保留 `PurchaseReceiptService` struct 定义、`new` 构造器、
//! 单号生成宏 `impl_generate_no!`（`generate_receipt_no`）、3 个纯函数
//! （`build_receipt_active_model` / `build_receipt_items_and_totals` /
//! `build_completed_receipt_active_model`）以及单元测试模块。
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
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, Set};
use std::sync::Arc;

/// 采购入库服务
/// 批次 D10 拆分：struct 定义与 `new` 构造器保留在 facade（本文件），；impl 业务方法块分散到 `purchase_receipt_ops` 子模块（auth/crud/state/items/query）。；`db` 字段为 `pub(crate)` 供 ops 子模块访问。
pub struct PurchaseReceiptService {
    pub db: Arc<DatabaseConnection>,
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

    /// 质检门控唯一判定入口（「合格方可入库/结算」）：采购收货单 `inspection_status`
    /// 能否进入库存写入与结算流转，仅此一处判定；确认入库
    /// （`purchase_receipt_ops::state::confirm_receipt`）与应付结算
    /// （`ap_invoice_ops::receipt::find_receipt_and_check_exists`）复用本函数，不得另写第二套比较。
    ///
    /// 裁定：每一张收货单都必须先有质检结论回写才允许入库/结算，依据（实地取证）：
    /// - 词表 `purchase_receipt_inspection` 全集只有三态（models/status/purchase_inventory.rs），
    ///   且 PASSED 的定义语义就是「质检合格：允许后续入库/结算流转」，PENDING 是「待检验」；
    /// - 生产 DDL（migration m0009）该列 `VARCHAR(20) NOT NULL DEFAULT 'PENDING'`，建单固定
    ///   置 PENDING（`build_receipt_active_model`）——不存在"免检收货"的第四态/NULL 形态，
    ///   全仓也不存在按品类/配置豁免质检的开关（检索 免检/需质检/inspection_required/
    ///   need_inspection 零命中）；化料/验布等其它检验域写的是各自表的各自列，不改变本列语义；
    /// - 本列离开 PENDING 的唯一途径是采购质检完成回写（`to_receipt_inspection_status`）
    ///   或通用质检记录回写（`from_inspection_result`），两条回写都可按 receipt_id 关联任意收货单。
    ///   因此 PENDING 必须拒绝——放行 PENDING 等于门控形同虚设（绝大多数新建收货单恒为 PENDING）。
    ///
    /// NULL 分支说明：列 NOT NULL、实体字段为 `String`（非 `Option<String>`），NULL 在类型层
    /// 不可达；若历史/坏数据出现词表外取值，fail-closed 走脱敏 `business` 报数据完整性问题，
    /// 绝不静默放行。错误文案按公开业务规则外显（`business_displayable`），不含单号——
    /// 入口界面已携带单据上下文，文案保持稳定可预期。
    ///
    /// `action` 为业务动作文案变量（"确认入库"/"生成应付结算"），仅参与句式拼接，
    /// 门控判定本身与动作无关。
    pub(crate) fn ensure_receipt_inspection_allows_flow(
        receipt: &purchase_receipt::Model,
        action: &str,
    ) -> Result<(), AppError> {
        let inspection = receipt.inspection_status.as_str();
        if inspection == status::purchase_receipt_inspection::PASSED {
            return Ok(());
        }
        if inspection == status::purchase_receipt_inspection::REJECTED {
            return Err(AppError::business_displayable(format!(
                "质检不合格的收货单不能{action}，请先处理不合格品"
            )));
        }
        if inspection == status::purchase_receipt_inspection::PENDING {
            return Err(AppError::business_displayable(format!(
                "收货单质检尚未完成，只有质检合格的收货单才能{action}，请先完成质检并录入结论"
            )));
        }
        // 词表外取值只能来自坏数据/历史遗留：按数据完整性问题整单拒绝（脱敏族，
        // 与既有状态门控 `lock_and_validate_receipt_txn` 的 business 口径一致），不猜测归类
        Err(AppError::business(format!(
            "入库单 {} 的检验状态「{inspection}」不在词表内（PENDING/PASSED/REJECTED），无法判定入库资格，需人工核查数据",
            receipt.receipt_no
        )))
    }

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
            inspection_status: Set(status::purchase_receipt_inspection::PENDING.to_string()),
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

            item_active_models.push(Self::build_receipt_item_active_model(
                item_req, receipt_id, amount,
            ));
        }
        (
            item_active_models,
            total_quantity,
            total_quantity_alt,
            total_amount,
        )
    }

    /// 单条入库明细请求 → ActiveModel；建单与追加明细共用，保证两条入口落库字段一致
    ///
    /// 库存四维维度字段（色号/缸号/批次/等级/克重/幅宽/库位）必须带全：
    /// 确认入库按「产品 + 色号 + 缸号 + 批次 + 等级」定位或新建库存行，
    /// 缺任一维度都会把收到的货落到一条维度不完整的库存行上，四维查询再也检索不到。
    pub(crate) fn build_receipt_item_active_model(
        item_req: CreateReceiptItemRequest,
        receipt_id: i32,
        amount: Decimal,
    ) -> purchase_receipt_item::ActiveModel {
        purchase_receipt_item::ActiveModel {
            receipt_id: Set(receipt_id),
            order_item_id: Set(item_req.order_item_id),
            product_id: Set(item_req.material_id),
            line_no: Set(item_req.line_no),
            material_code: Set(item_req.material_code.clone()),
            material_name: Set(item_req.material_name.clone()),
            quantity: Set(item_req.quantity),
            quantity_alt: Set(Some(item_req.quantity_alt)),
            unit_master: Set(item_req.unit_master.clone()),
            unit_alt: Set(item_req.unit_alt.clone()),
            unit_price: Set(Some(
                item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)),
            )),
            amount: Set(Some(amount)),
            batch_no: Set(item_req.batch_no),
            color_code: Set(item_req.color_code),
            lot_no: Set(item_req.lot_no),
            grade: Set(item_req.grade),
            gram_weight: Set(item_req.gram_weight),
            width: Set(item_req.width),
            location_code: Set(item_req.location_code),
            piece_no: Set(item_req.piece_no),
            package_no: Set(item_req.package_no),
            production_date: Set(item_req.production_date),
            shelf_life: Set(item_req.shelf_life),
            notes: Set(item_req.notes),
            ..Default::default()
        }
    }

    /// 构造 COMPLETED 状态 ActiveModel，写入确认时间与审计字段（`pub(crate)`：state 子模块的 `confirm_receipt` 调用。）
    /// 确认事务内即完成库存入库与订单已收数量推进，落账成功即为收货终态，不再依赖异步事件补状态。
    pub(crate) fn build_completed_receipt_active_model(
        receipt: purchase_receipt::Model,
        user_id: i32,
    ) -> purchase_receipt::ActiveModel {
        let now = chrono::Utc::now();
        let mut active: purchase_receipt::ActiveModel = receipt.into();
        active.receipt_status = Set(status::purchase_receipt::COMPLETED.to_string());
        active.confirmed_at = Set(Some(now));
        active.confirmed_by = Set(Some(user_id));
        active.updated_by = Set(Some(user_id));
        active.updated_at = Set(now);
        active
    }
}

#[cfg(test)]
mod inspection_gate_tests {
    //! 「合格方可入库/结算」门控判定矩阵（真实生产入口
    //! `ensure_receipt_inspection_allows_flow`，非复刻实现）。
    //! 端到端零漂移/顺序由 backend/tests/contract_wave6_receipt_gate_test.rs 钉住。

    use super::*;
    use crate::models::status::purchase_inventory::purchase_receipt_inspection;
    use crate::utils::error::AppError;

    fn receipt_with_inspection(inspection_status: &str) -> purchase_receipt::Model {
        let now = chrono::Utc::now();
        purchase_receipt::Model {
            id: 1,
            receipt_no: "GR-GATE-TEST-001".to_string(),
            order_id: None,
            supplier_id: 1,
            receipt_date: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            warehouse_id: 1,
            department_id: None,
            receiver_id: None,
            inspector_id: None,
            inspection_status: inspection_status.to_string(),
            receipt_status: status::purchase_receipt::DRAFT.to_string(),
            total_quantity: Decimal::ZERO,
            total_quantity_alt: Decimal::ZERO,
            total_amount: Decimal::ZERO,
            notes: None,
            attachment_urls: None,
            created_by: 1,
            created_at: now,
            updated_by: None,
            updated_at: now,
            confirmed_at: None,
            confirmed_by: None,
        }
    }

    #[test]
    fn only_passed_allows_flow() {
        // 词表全集逐字符判定：ALL 中仅 PASSED 放行（PENDING/REJECTED 均拒）
        for token in purchase_receipt_inspection::ALL {
            let receipt = receipt_with_inspection(token);
            let result =
                PurchaseReceiptService::ensure_receipt_inspection_allows_flow(&receipt, "确认入库");
            assert_eq!(
                result.is_ok(),
                *token == purchase_receipt_inspection::PASSED,
                "词表取值 {token:?} 的放行判定必须与「仅 PASSED 放行」口径一致"
            );
        }
    }

    #[test]
    fn rejected_message_is_exact_and_displayable() {
        let receipt = receipt_with_inspection(purchase_receipt_inspection::REJECTED);
        let err =
            PurchaseReceiptService::ensure_receipt_inspection_allows_flow(&receipt, "确认入库")
                .expect_err("REJECTED 必须拒绝");
        assert!(
            matches!(&err, AppError::BusinessErrorDisplayable(_)),
            "公开业务规则必须外显族，实际: {err:?}"
        );
        let body = err.to_response();
        assert_eq!(body.code, "BUSINESS_ERROR");
        assert_eq!(
            body.message, "质检不合格的收货单不能确认入库，请先处理不合格品",
            "确认入库入口的 REJECTED 文案必须与裁定原文逐字符一致（不含单号，走 displayable）"
        );
    }

    #[test]
    fn pending_is_rejected_with_actionable_message() {
        // 裁定：每单必经质检（判定依据见 ensure_receipt_inspection_allows_flow 文档注释），
        // PENDING（质检未完成）放行等于门控形同虚设，必须拒绝且文案给出可行动路径
        let receipt = receipt_with_inspection(purchase_receipt_inspection::PENDING);
        let err =
            PurchaseReceiptService::ensure_receipt_inspection_allows_flow(&receipt, "确认入库")
                .expect_err("PENDING 必须拒绝");
        let body = err.to_response();
        assert_eq!(body.code, "BUSINESS_ERROR");
        assert!(
            body.message.contains("质检尚未完成") && body.message.contains("完成质检"),
            "PENDING 拒绝文案必须说明原因与行动路径，实际: {}",
            body.message
        );
    }

    #[test]
    fn outside_vocabulary_fails_closed_masked() {
        // 列 NOT NULL、NULL 在类型层不可达；词表外坏值（如历史中文 token）fail-closed：
        // 按数据完整性问题脱敏拒绝（business），绝不静默放行、也不外显坏数据原文
        for bad in ["待检", "passed", "INSPECTING", ""] {
            let receipt = receipt_with_inspection(bad);
            let err =
                PurchaseReceiptService::ensure_receipt_inspection_allows_flow(&receipt, "确认入库")
                    .expect_err("词表外取值必须 fail-closed 拒绝");
            assert!(
                matches!(&err, AppError::BusinessError(_)),
                "词表外 {bad:?} 必须走脱敏 business（数据完整性问题），实际: {err:?}"
            );
            let body = err.to_response();
            assert_eq!(body.code, "BUSINESS_ERROR");
            assert!(
                !body.message.contains(bad) || bad.is_empty(),
                "脱敏出参不得回显词表外原值 {bad:?}，实际: {}",
                body.message
            );
        }
    }
}
