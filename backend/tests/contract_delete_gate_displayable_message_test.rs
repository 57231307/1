//! 删除腿与状态门拒绝「可外显点名文案」契约锁。
//!
//! 功能：锁定删除类业务门被拒时的出参契约——状态码仍 400、错误族仍是可展示族
//! `AppError::BusinessErrorDisplayable`（`code=BUSINESS_ERROR`）、出参 message
//! 携带业务原因关键词；并锁定各改造点的构造点形态（构造器与文案绑定），
//! 任何一枚改回脱敏族 `AppError::business` 都会让本锁变红。
//! 调用方：`cargo test`（CI 后端门禁与本地 prepush 同源），无活库依赖，
//! 纯函数门直接调用，DB 侧门以源码扫描绑定判据。
//! 入参：无外部输入；夹具为状态词表常量（`models::status`）与源文件文本。
//! 传给谁：断言结果只用于测试判定，不写任何存储。
//! 存什么·存哪里：不落数据；源码扫描读取 `backend/src` 下被锁文件的现行文本。
//!
//! 改回通用族必红的机理：
//! - 行为面（纯函数门）：枚断 `matches!(err, AppError::BusinessErrorDisplayable(_))`，
//!   改回 `AppError::business` 后变体即失配；同时断言 message 含业务关键词且
//!   不等于脱敏常量「业务处理失败」，构造器换族会同时打中这两条断言。
//! - 扫描面（DB 侧门）：在压掉空白的源码文本上同时搜
//!   `AppError::business_displayable("<该门文案>")` 完整绑定串；改回 `business(`
//!   或删掉文案，绑定串都不再存在，即红。关键词只断语义核心，不断整句以外修饰。

use axum::response::IntoResponse;
use axum::{http::StatusCode, response::Response};
use bingxi_backend::models::status::{dye_recipe, lab_dip_request, production_recipe};
use bingxi_backend::services::dye_recipe_service::DyeRecipeService;
use bingxi_backend::services::lab_dip_service::LabDipRequestService;
use bingxi_backend::services::production_recipe_service::ProductionRecipeService;
use bingxi_backend::utils::error::AppError;

/// 脱敏常量镜像：出参若等于它即说明该门回退到了不可外显族。
const SANITIZED_BUSINESS: &str = "业务处理失败";

/// 断言一枚拒绝是「可展示族 + 400 + BUSINESS_ERROR + 关键词外显」的完整契约。
fn assert_displayable_gate_contract(err: AppError, keywords: &[&str]) {
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "删除门拒绝必须是可展示族 BusinessErrorDisplayable，实际={err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR", "错误族 code 不得移动");
    let resp: Response = err.clone().into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "状态门拒绝的 HTTP 状态码必须保持 400"
    );
    let msg = err.to_response().message;
    assert_ne!(
        msg, SANITIZED_BUSINESS,
        "出参 message 不得回退为脱敏常量，实际={msg}"
    );
    for kw in keywords {
        assert!(msg.contains(kw), "出参 message 应点名业务原因「{kw}」，实际={msg}");
    }
}

/// 染配方可删除纯函数门：已审核被拒时必须外显「删除」与出路「停用」。
#[test]
fn dye_recipe_delete_gate_is_displayable_and_names_reason() {
    let err = DyeRecipeService::validate_can_delete(Some(dye_recipe::APPROVED))
        .expect_err("已审核配方必须被删除门拒绝");
    assert_displayable_gate_contract(err, &["配方", "删除", "停用"]);
}

/// 大货配方删除纯函数门：非草稿被拒时必须外显「草稿」与「删除」。
#[test]
fn production_recipe_delete_gate_is_displayable_and_names_reason() {
    let err = ProductionRecipeService::validate_can_delete(production_recipe::APPROVED)
        .expect_err("已审核大货配方必须被删除门拒绝");
    assert_displayable_gate_contract(err, &["草稿", "删除"]);
}

/// 对色申请单删除纯函数门：非待处理被拒时必须外显「待处理」与「删除」。
#[test]
fn lab_dip_delete_gate_is_displayable_and_names_reason() {
    let err = LabDipRequestService::validate_can_delete(lab_dip_request::SAMPLING)
        .expect_err("打样中对色申请必须被删除门拒绝");
    assert_displayable_gate_contract(err, &["待处理", "删除"]);
}

/// 压掉全部空白后的源码文本（构造器与文案绑定判据用，规避换行缩进差异）。
fn source_normalized(rel_path: &str) -> String {
    let path = format!("{}/../{}", env!("CARGO_MANIFEST_DIR"), rel_path);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("被锁源文件必须可读 {}: {e}", path));
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 扫描面判据：构造器与文案必须成对绑定出现。
///
/// 把一枚改造点改回 `AppError::business(...)`（哪怕保留同一文案）、或把文案改回
/// 「当前状态：{}」插值形态，绑定串都会从压空文本中消失，本断言必红。
/// 绑定串按前缀匹配：判据尾部标点（引号/括号/逗号）由源码换行与尾逗号形态决定，
/// 不参与比较，避免排版差异造成假红。
fn assert_displayable_call_locked(rel_path: &str, constructor_and_message: &str) {
    let bound: String = constructor_and_message
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let bound = bound.trim_end_matches(['"', ')', ',']);
    let normalized = source_normalized(rel_path);
    assert!(
        normalized.contains(&bound),
        "{rel_path} 中必须存在可外显构造绑定串 {bound}；\
         若该门被改回脱敏族 business(...) 或非点名文案，此锁必红"
    );
}

/// 反向自证：被收口的删除门旧通用文案不允许在任何被锁文件中复活。
#[test]
fn legacy_generic_delete_gate_messages_are_gone() {
    let forbidden = [
        "订单状态不允许删除，当前状态：",
        "订单状态不允许修改，当前状态：",
        "订单状态不允许添加明细，当前状态：",
        "订单状态不允许修改明细，当前状态：",
        "订单状态不允许删除明细，当前状态：",
        "入库单状态不允许删除，当前状态：",
        "入库单状态不允许删除明细，当前状态：",
        "调拨单状态{}不允许删除",
        "退货单状态不允许删除，当前状态：",
        "订单状态为{}，不允许删除",
        "订单状态为{}，不允许修改",
        "当前状态不可删除（仅draft可删除）",
        "当前状态{}不可删除（仅pending可删除）",
        "对色结果{}不可删除（仅pending状态可删除）",
        "流转卡状态为{}，仅pending/terminated状态可删除",
        "只有待审核状态的订单可以删除，当前状态：",
        "仅草稿(draft)状态可删除，当前状态:",
        "付款申请状态为{}，不可删除",
        "应付单状态为{}，不可删除",
    ];
    let files = [
        "backend/src/services/po/order_ops/crud.rs",
        "backend/src/services/po/receipt.rs",
        "backend/src/services/so/order_crud.rs",
        "backend/src/services/so/fabric_order.rs",
        "backend/src/services/inv/inventory_move.rs",
        "backend/src/services/inv/batch.rs",
        "backend/src/services/sales_return_service.rs",
        "backend/src/services/production_recipe_service.rs",
        "backend/src/services/lab_dip_service.rs",
        "backend/src/services/lab_dip_ops/sample.rs",
        "backend/src/services/flow_card_ops/card_crud.rs",
        "backend/src/services/outsourcing_ops/order.rs",
        "backend/src/services/outsourcing_ops/receipt.rs",
        "backend/src/services/energy_ops/allocation_record.rs",
        "backend/src/services/energy_ops/allocation_rule.rs",
        "backend/src/services/energy_ops/consumption.rs",
        "backend/src/services/chemical_ops/requisition.rs",
        "backend/src/services/ap_payment_request_service.rs",
        "backend/src/services/ap_invoice_ops/crud.rs",
    ];
    let mut all_norm = String::new();
    for f in files {
        all_norm.push_str(&source_normalized(f));
    }
    for pat in forbidden {
        let pat_norm: String = pat.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            !all_norm.contains(&pat_norm),
            "旧通用文案「{pat}」不允许复活（删除门必须点名可外显原因）"
        );
    }
}

/// 逐枚点名：各删除门的可外显构造绑定串必须原样在场。
#[test]
fn each_delete_gate_constructor_is_locked() {
    let cases: &[(&str, &str)] = &[
        (
            "backend/src/services/po/order_ops/crud.rs",
            r#"AppError::business_displayable("仅草稿状态的采购订单可以删除")"#,
        ),
        (
            "backend/src/services/po/order_ops/crud.rs",
            r#"AppError::business_displayable("仅草稿或已驳回状态的采购订单可以修改")"#,
        ),
        (
            "backend/src/services/po/receipt.rs",
            r#"AppError::business_displayable("仅草稿状态的采购订单可以添加明细")"#,
        ),
        (
            "backend/src/services/po/receipt.rs",
            r#"AppError::business_displayable("仅草稿状态的采购订单可以修改明细")"#,
        ),
        (
            "backend/src/services/po/receipt.rs",
            r#"AppError::business_displayable("仅草稿状态的采购订单可以删除明细")"#,
        ),
        (
            "backend/src/services/so/order_crud.rs",
            r#"AppError::business_displayable("已发货或已完成的订单不能删除")"#,
        ),
        (
            "backend/src/services/so/order_crud.rs",
            r#"AppError::business_displayable("已发货或已完成的订单不能修改")"#,
        ),
        (
            "backend/src/services/inv/inventory_move.rs",
            r#"AppError::business_displayable("已审核、已发货或已完成的调拨单不能删除")"#,
        ),
        (
            "backend/src/services/inv/batch.rs",
            r#"AppError::business_displayable("已发货或已完成的调拨单不能删除明细")"#,
        ),
        (
            "backend/src/services/sales_return_service.rs",
            r#"AppError::business_displayable("仅草稿状态的销售退货单可以删除")"#,
        ),
        (
            "backend/src/services/so/fabric_order.rs",
            r#"AppError::business_displayable("只有待审核状态的面料订单可以删除")"#,
        ),
        (
            "backend/src/services/outsourcing_ops/order.rs",
            r#"AppError::business_displayable("仅草稿状态的委外订单可以删除")"#,
        ),
        (
            "backend/src/services/outsourcing_ops/receipt.rs",
            r#"AppError::business_displayable("仅草稿状态的委外收回入库单可以删除")"#,
        ),
        (
            "backend/src/services/outsourcing_ops/voucher.rs",
            r#"AppError::business_displayable("已过账凭证不可删除")"#,
        ),
        (
            "backend/src/services/energy_ops/allocation_record.rs",
            r#"AppError::business_displayable("仅草稿状态的分摊记录可以删除")"#,
        ),
        (
            "backend/src/services/energy_ops/allocation_rule.rs",
            r#"AppError::business_displayable("仅草稿状态的分摊规则可以删除")"#,
        ),
        (
            "backend/src/services/energy_ops/consumption.rs",
            r#"AppError::business_displayable("仅草稿状态的能耗记录可以删除")"#,
        ),
        (
            "backend/src/services/chemical_ops/requisition.rs",
            r#"AppError::business_displayable("仅草稿状态的领用单可以删除")"#,
        ),
        (
            "backend/src/services/ap_payment_request_service.rs",
            r#"AppError::business_displayable("仅草稿或已驳回状态的付款申请可以删除")"#,
        ),
        (
            "backend/src/services/ap_invoice_ops/crud.rs",
            r#"AppError::business_displayable("仅草稿状态的应付单可以删除")"#,
        ),
        (
            "backend/src/services/inventory_adjustment_service.rs",
            r#"AppError::business_displayable("只有待审核状态的调整单可以删除"#,
        ),
        (
            "backend/src/services/inventory_count_service.rs",
            r#"AppError::business_displayable("只有待盘点状态的盘点单可以删除"#,
        ),
        (
            "backend/src/services/lab_dip_ops/sample.rs",
            r#"AppError::business_displayable("仅待处理状态的对色结果可以删除")"#,
        ),
        (
            "backend/src/services/lab_dip_service.rs",
            r#"AppError::business_displayable("仅待处理状态的对色申请单可以删除")"#,
        ),
        (
            "backend/src/services/production_recipe_service.rs",
            r#"AppError::business_displayable("仅草稿状态的生产配方可以删除")"#,
        ),
        (
            "backend/src/services/dye_recipe_service.rs",
            r#"AppError::business_displayable("已审核的配方不允许删除，请先停用""#,
        ),
        (
            "backend/src/services/fabric_inspection_service.rs",
            r#"AppError::business_displayable("仅待验布状态的验布记录可以删除")"#,
        ),
        (
            "backend/src/services/fabric_inspection_service.rs",
            r#"AppError::business_displayable("仅验布中的验布记录可以删除疵点明细")"#,
        ),
        (
            "backend/src/services/flow_card_ops/card_crud.rs",
            r#"AppError::business_displayable("仅待排缸或已终止状态的流转卡可以删除")"#,
        ),
        (
            "backend/src/services/purchase_receipt_ops/crud.rs",
            r#"AppError::business_displayable("仅草稿状态的入库单可以删除")"#,
        ),
        (
            "backend/src/services/purchase_receipt_ops/items.rs",
            r#"AppError::business_displayable("仅草稿状态的入库单可以删除明细")"#,
        ),
        (
            "backend/src/services/dye_batch_state_machine_ops/rework.rs",
            r#"AppError::business_displayable("回修中或已完成的回修单不可删除")"#,
        ),
        (
            "backend/src/services/crm/opp.rs",
            r#"AppError::business_displayable("已赢单的商机不能删除")"#,
        ),
        (
            "backend/src/services/chemical_ops/category.rs",
            r#"AppError::business_displayable("存在子分类，无法删除，请先处理子分类")"#,
        ),
        (
            "backend/src/services/product_category_service.rs",
            r#"AppError::business_displayable("该类别存在子类别，无法删除，请先处理子类别""#,
        ),
        (
            "backend/src/services/department_service.rs",
            r#"AppError::business_displayable("该部门存在子部门，无法删除，请先处理子部门""#,
        ),
        (
            "backend/src/services/department_service.rs",
            r#"AppError::business_displayable("该部门下仍有用户，请先移除用户的部门关联后再删除")"#,
        ),
        (
            "backend/src/services/business_mode_ops/config.rs",
            r#"AppError::business_displayable("该业务模式已被单据引用，不可删除，请先处理关联单据")"#,
        ),
        (
            "backend/src/services/customer_ops/crud.rs",
            r#"AppError::business_displayable("该客户已删除，无需重复操作")"#,
        ),
        (
            "backend/src/services/role_service.rs",
            r#"AppError::business_displayable("系统角色不允许删除")"#,
        ),
        (
            "backend/src/services/role_permission_service.rs",
            r#"AppError::business_displayable("系统角色不允许删除")"#,
        ),
        (
            "backend/src/services/role_permission_service.rs",
            r#"AppError::business_displayable("系统角色的权限不允许删除""#,
        ),
        (
            "backend/src/services/ar/recon_ops/lifecycle.rs",
            r#"AppError::business_displayable("只有草稿状态的对账单可以删除""#,
        ),
        (
            "backend/src/services/ar_invoice_service.rs",
            r#"AppError::business_displayable("非草稿状态的应收单无法删除""#,
        ),
        (
            "backend/src/services/account_subject_service.rs",
            r#"AppError::business_displayable("科目已被凭证使用，不能删除，请先处理相关凭证")"#,
        ),
        (
            "backend/src/services/crm/customer_team_share_service.rs",
            r#"AppError::business_displayable("该成员已不在团队中，无需重复移除")"#,
        ),
        (
            "backend/src/services/crm/customer_team_share_service.rs",
            r#"AppError::business_displayable("主负责人不可直接移除，请先变更主负责人")"#,
        ),
        (
            "backend/src/handlers/missing_handlers.rs",
            r#"AppError::business_displayable("已结账的会计期间不可删除""#,
        ),
        (
            "backend/src/handlers/missing_handlers.rs",
            r#"AppError::business_displayable("该会计期间已存在凭证，不可删除，请先处理期内凭证")"#,
        ),
        (
            "backend/src/handlers/logistics_handler.rs",
            r#"AppError::business_displayable("已送达或已签收的运单不可删除")"#,
        ),
    ];
    for (file, bound) in cases {
        assert_displayable_call_locked(file, bound);
    }
}

/// 保持脱敏的删除门不许被顺手改族：坏数据/余额/内部标识三类必须仍是
/// `AppError::business(...)` 绑定形态（改回 displayable 塞标识属违例）。
#[test]
fn kept_sanitized_gates_stay_in_sanitized_family() {
    let cases: &[(&str, &str)] = &[
        (
            "backend/src/services/purchase_receipt_service.rs",
            r#"AppError::business(format!("入库单 {} 的检验状态""#,
        ),
        (
            "backend/src/services/account_subject_service.rs",
            r#"AppError::business(format!("科目有 {} 条余额记录，不能删除""#,
        ),
        (
            "backend/src/services/so/order_crud.rs",
            r#"AppError::business(format!("商机 {} 不存在""#,
        ),
        (
            "backend/src/services/po/order_ops/crud.rs",
            r#"AppError::business(format!("第 {} 行色号""#,
        ),
    ];
    for (file, bound) in cases {
        assert_displayable_call_locked(file, bound);
    }
}
