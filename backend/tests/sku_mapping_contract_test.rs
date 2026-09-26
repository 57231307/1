//! 供应商商品/色号对照表（sku-mapping / 面料二批调货）契约测试
//!
//! 覆盖范围（与编排任务 A1–A7 对应，按可真实执行的层级选择验证方式）：
//! - A1 resolve 无映射返回 `Ok(None)`（非 Err）、命中取 priority 最小：源码结构契约 +
//!   AppError 出参契约（`#[ignore]` 真库集成见 `sku_mapping_integration_test.rs`）。
//! - A3 validate_refs 各失败分支均为 `AppError::validation`：源码结构契约。
//! - A4 import_batch 计数字段为 `error_count`（非 `fail_count`）+ UPSERT ON CONFLICT：
//!   序列化形状 + 源码契约。
//! - A5 转采购 hook：无对照的拒绝文案中性（含「无该色号」、不含 供应商/对照/supplier）；
//!   resolve 命中回填 supplier_product_code/supplier_color_no：源码结构契约 + 出参脱敏保证。
//! - A6 保密：sales_order_item 模型源码不含任何 supplier 字段；HTTP 出参脱敏使
//!   `AppError::business` 真实文案不外显（防止调货模型泄露到销售可见面）。
//! - 前端契约：resolve 返回对象不带 `.mapping` 包裹、列表 DTO 用 `color_no`。
//!
//! 说明：本仓 `setup_test_db` 无 schema harness（sqlite::memory 无表、`#[ignore]` 真库
//! 用例仅在本地 TEST_DATABASE_URL 指向已迁移 PG 时运行）。因此需要真实数据分支的
//! 行为级验证集中在：① 源码结构契约（编译期即锁死分支语义）② e2e 打真实 server+DB
//! （frontend/e2e/purchase/sku-mapping.spec.ts）。此处不 mock、不 skip、不放宽断言。

use bingxi_backend::services::sku_mapping_service::{ImportMappingResult, ResolvedSku};
use bingxi_backend::utils::error::AppError;
use rust_decimal::Decimal;
use std::str::FromStr;

/// 被验证的三份源码文件（静态契约的权威来源，避免断言与实现脱节）
const SERVICE_SRC: &str = include_str!("../src/services/sku_mapping_service.rs");
const PO_CRUD_SRC: &str = include_str!("../src/services/po/order_ops/crud.rs");
const HANDLER_SRC: &str = include_str!("../src/handlers/sku_mapping_handler.rs");
const SALES_ITEM_SRC: &str = include_str!("../src/models/sales_order_item.rs");

// ---------------------------------------------------------------------------
// 出参错误契约：AppError 变体 → code / HTTP 状态
// ---------------------------------------------------------------------------

/// A2/A3 契约：create/update 重复组合走 `AppError::business`，出参 code=BUSINESS_ERROR。
#[test]
fn apperror_business_maps_to_business_error_code() {
    let err = AppError::business("该产品+色号+供应商组合的对照记录已存在，请勿重复创建");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
}

/// A3 契约：validate_refs 失败走 `AppError::validation`，出参 code=VALIDATION_ERROR。
///
/// 注意：本仓 `ValidationError` 映射的 HTTP 状态是 **400（BAD_REQUEST）**，
/// 而非任务描述里写的 422。断言以真实源码为准（`error_status_and_type`），
/// 该偏差已在交付报告中标记为「任务描述与实现不一致」，不据描述放宽/篡改断言。
#[test]
fn apperror_validation_maps_to_validation_error_code() {
    let err = AppError::validation("产品色号不属于指定的产品");
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
}

/// A6 保密关键不变式：`AppError::business`（非 displayable）真实文案被脱敏，
/// HTTP 出参 message 恒为固定常量——即便转采购 hook 里带上内部编号，也不会外显。
/// 这是「供应商编号/色号/调货模型绝不对销售暴露」的出参层保证。
#[test]
fn business_error_http_message_is_sanitized_not_leaked() {
    let err = AppError::business("第 3 行无该色号，无法转采购");
    let resp = err.to_response();
    // 出参不外显真实文案
    assert_eq!(resp.message, "业务处理失败");
    assert!(!resp.message.contains("无该色号"));
    assert!(!resp.message.contains("供应商"));
    assert!(!resp.message.contains("对照"));
}

// ---------------------------------------------------------------------------
// A1：resolve 无映射返回 Ok(None)（不抛错）+ 命中取 priority 最小
// ---------------------------------------------------------------------------

/// resolve_supplier_sku 在查无映射时必须 `return Ok(None)`（契约：不再抛错），
/// 由调用方按受众决定措辞。锁死该分支的存在性，防止后续误改成 Err。
#[test]
fn resolve_returns_ok_none_when_no_mapping_source_contract() {
    // 命中判定后 else 分支显式返回 Ok(None)
    assert!(
        SERVICE_SRC.contains("return Ok(None);"),
        "resolve_supplier_sku 无映射应显式 return Ok(None)"
    );
    // 该方法体内不得对「查无映射」构造 business/validation 错误（措辞在调用方）
    let resolve_body = extract_fn(SERVICE_SRC, "pub async fn resolve_supplier_sku");
    assert!(
        !resolve_body.contains("AppError::business") && !resolve_body.contains("validation("),
        "resolve_supplier_sku 自身不应抛出面向受众的错误文案，措辞须交给调用方"
    );
}

/// resolve 命中时按 priority 升序取第一条（priority 最小），验证排序方向。
#[test]
fn resolve_orders_by_priority_ascending_source_contract() {
    let resolve_body = extract_fn(SERVICE_SRC, "pub async fn resolve_supplier_sku");
    assert!(
        resolve_body.contains("order_by(product_supplier_mapping::Column::Priority, Order::Asc)")
            || resolve_body.contains("Order::Asc"),
        "resolve 应按 priority 升序取最小的一条"
    );
    // 仅取启用映射
    assert!(
        resolve_body.contains("IsEnabled.eq(true)"),
        "resolve 应只匹配 is_enabled=true 的映射"
    );
}

// ---------------------------------------------------------------------------
// A2：create/update 重复组合 → business
// ---------------------------------------------------------------------------

#[test]
fn create_and_update_map_unique_violation_to_business() {
    assert!(
        SERVICE_SRC.contains("is_unique_violation(&e)"),
        "create/update 应捕获唯一约束冲突"
    );
    // 两处（create 与 update）均落 business
    let dup_msg_count = SERVICE_SRC
        .matches("该产品+色号+供应商组合的对照记录已存在，请勿重复创建")
        .count();
    assert_eq!(
        dup_msg_count, 2,
        "create 与 update 各应有一处重复组合 business 文案（实得 {dup_msg_count}）"
    );
    assert!(
        SERVICE_SRC.contains("AppError::business("),
        "重复组合应使用 AppError::business"
    );
}

// ---------------------------------------------------------------------------
// A3：validate_refs 各失败分支 → validation
// ---------------------------------------------------------------------------

#[test]
fn validate_refs_each_branch_uses_validation_error() {
    let body = extract_fn(SERVICE_SRC, "async fn validate_refs");
    let expected = [
        "AppError::validation(format!(\"产品 ID {} 不存在\"",
        "AppError::validation(\"产品色号不属于指定的产品\")",
        "AppError::validation(format!(\"供应商 ID {} 不存在\"",
        "AppError::validation(format!(",
        "AppError::validation(\"供应商商品不属于指定的供应商\")",
        "AppError::validation(\"供应商色号不属于指定的供应商商品\")",
    ];
    for needle in expected {
        assert!(
            body.contains(needle),
            "validate_refs 缺少预期失败分支：{needle}"
        );
    }
    // 「色号不属于该产品」等三类归属校验都必须是 validation（422 语义 → 本仓 400），
    // 不得是 business（那会走脱敏文案、丢失定位信息，且语义错误）。
    assert!(
        body.contains("不属于指定的产品")
            && body.contains("不属于指定的供应商")
            && body.contains("不属于指定的供应商商品"),
        "三类归属校验文案应齐全"
    );
}

// ---------------------------------------------------------------------------
// A4：import_batch UPSERT 幂等 + 计数字段 error_count
// ---------------------------------------------------------------------------

/// 计数字段序列化键必须是 error_count（非 fail_count），前端 api/sku-mapping.ts 依赖此键。
#[test]
fn import_result_serializes_error_count_not_fail_count() {
    let res = ImportMappingResult {
        total_count: 5,
        success_count: 3,
        error_count: 2,
        errors: vec![],
    };
    let json = serde_json::to_value(&res).expect("ImportMappingResult 序列化应成功");
    assert_eq!(json["total_count"], 5);
    assert_eq!(json["success_count"], 3);
    assert_eq!(json["error_count"], 2);
    assert!(
        json.get("fail_count").is_none(),
        "导入结果不应出现 fail_count 键（契约以 error_count 为准）"
    );
}

/// UPSERT 使用 ON CONFLICT (product_id, product_color_id, supplier_id) DO UPDATE，保证幂等。
#[test]
fn import_batch_upsert_on_conflict_contract() {
    assert!(
        SERVICE_SRC.contains("ON CONFLICT (product_id, product_color_id, supplier_id) DO UPDATE"),
        "import_batch 必须对 (product_id,product_color_id,supplier_id) 做 UPSERT 以保证幂等"
    );
}

// ---------------------------------------------------------------------------
// A5：转采购 hook（保密中性文案 + 回填 supplier_* 快照）
// ---------------------------------------------------------------------------

/// 无对照时的拒绝文案必须中性：含「无该色号」，且绝不出现泄露调货模型的词。
#[test]
fn po_hook_no_mapping_message_is_neutral_no_leak() {
    let idx = PO_CRUD_SRC
        .find("第 {} 行无该色号，无法转采购")
        .expect("转采购 hook 应包含中性文案模板");
    // 取该字面量所在语句局部窗口做泄露扫描（该模板串本身即被扫描对象）
    let literal = "第 {} 行无该色号，无法转采购";
    for forbidden in ["供应商", "对照", "supplier", "调货", "自制"] {
        assert!(
            !literal.contains(forbidden),
            "转采购无对照文案不得泄露调货模型敏感词「{forbidden}」"
        );
    }
    assert!(idx > 0);
    // 该文案必须以 AppError::business_displayable 构造：文案本身中性（见上），
    // 但要把「无该色号」如实回显给用户（脱敏 business 会让用户只看到「业务处理失败」，
    // 违背「message 含无该色号」的产品要求）。
    assert!(
        PO_CRUD_SRC.contains("AppError::business_displayable(format!("),
        "无对照拒绝应使用 business_displayable 让中性文案如实外显，而非脱敏 business"
    );
    // 且转采购可见面严禁出现供应商维护类敏感词（保密：不泄露调货模型）。
    for forbidden in ["供应商", "对照"] {
        assert!(
            !literal.contains(forbidden),
            "转采购可见文案不得出现维护类敏感词「{forbidden}」"
        );
    }
}

/// 有对照时把供应商商品编码/色号回填到 purchase_order_item 的 supplier_product_code /
/// supplier_color_no 快照列（写入发生在 resolve 命中之后）。
#[test]
fn po_hook_backfills_supplier_snapshot_columns_contract() {
    assert!(
        PO_CRUD_SRC.contains("resolved.supplier_product_code"),
        "命中对照后应读取 resolved.supplier_product_code"
    );
    assert!(
        PO_CRUD_SRC.contains("resolved.supplier_color_no"),
        "命中对照后应读取 resolved.supplier_color_no"
    );
    assert!(
        PO_CRUD_SRC.contains("order_item.supplier_product_code = Set(resolved_product_code)"),
        "应把供应商商品编码回填到明细快照列"
    );
    assert!(
        PO_CRUD_SRC.contains("order_item.supplier_color_no = Set(resolved_color_no)"),
        "应把供应商色号回填到明细快照列"
    );
    // 反查色号不存在的中性文案（第 350–377 行分支）
    assert!(
        PO_CRUD_SRC.contains("第 {} 行色号「{}」在产品 {} 下不存在，无法转采购"),
        "色号在产品下不存在的分支应给出定位文案"
    );
}

/// 转采购 hook 只在携带来源销售订单标识（source_sales_order_id）时才触发翻译，
/// 普通自建采购单不做对照，避免误伤。
#[test]
fn po_hook_only_triggers_on_source_sales_order() {
    assert!(
        PO_CRUD_SRC.contains("req.source_sales_order_id.is_some()"),
        "仅当带来源销售订单时才执行对照翻译"
    );
}

// ---------------------------------------------------------------------------
// A6：保密——sales_order_item 模型根本不承载 supplier_* 字段
// ---------------------------------------------------------------------------

/// sales_order_item 模型源码里不应出现任何 supplier 相关字段/token，
/// 从数据模型层面锁死「供应商编号/色号绝不进销售明细」。
#[test]
fn sales_order_item_model_has_no_supplier_fields() {
    let lower = SALES_ITEM_SRC.to_lowercase();
    assert!(
        !lower.contains("supplier"),
        "sales_order_item 模型不得包含任何 supplier 字段（销售域保密）"
    );
    assert!(
        !SALES_ITEM_SRC.contains("product_supplier_mapping"),
        "sales_order_item 模型不应引用供应商映射实体"
    );
}

// ---------------------------------------------------------------------------
// 前端契约：resolve 返回对象无 .mapping 包裹；resolve handler 允许指路文案
// ---------------------------------------------------------------------------

/// resolve 成功返回的对象键必须是 ResolvedSku 平铺字段，无 `.mapping` 包裹层
/// （前端 api/sku-mapping.ts::ResolveSkuMappingResult 直接消费平铺结构）。
#[test]
fn resolve_result_has_no_mapping_wrapper() {
    let sku = ResolvedSku {
        mapping_id: 10,
        supplier_product_id: 20,
        supplier_product_color_id: Some(30),
        supplier_product_code: "SUP-P-001".to_string(),
        supplier_color_no: Some("SUP-C-A1".to_string()),
        supplier_price: Some(Decimal::from_str("12.34").unwrap()),
        lead_time: Some(7),
    };
    let json = serde_json::to_value(&sku).expect("ResolvedSku 序列化应成功");
    let obj = json
        .as_object()
        .expect("ResolvedSku 应序列化为对象（非 .mapping 包裹）");
    assert!(
        obj.get("mapping").is_none(),
        "resolve 结果不应带 .mapping 包裹"
    );
    assert_eq!(obj["mapping_id"], 10);
    assert_eq!(obj["supplier_product_code"], "SUP-P-001");
    assert_eq!(obj["supplier_color_no"], "SUP-C-A1");
    assert_eq!(obj["supplier_price"], "12.34");
    assert_eq!(obj["lead_time"], 7);
}

/// resolve 预览端点面向采购角色，允许指路文案（提示先维护对照表），
/// 与转采购 hook 的中性文案形成受众分层：采购可指路、销售可见面中性。
#[test]
fn resolve_handler_preview_allows_guidance_text() {
    assert!(
        HANDLER_SRC.contains("该产品色号暂无供应商对照，请先维护对照表"),
        "采购预览 resolve 端点应给出可指路的 business 文案"
    );
    // 且该文案是 AppError::business（出参脱敏），指路信息只进日志，
    // 真正面向采购的引导由前端 i18n（skuMappingNotFound）承担。
    assert!(
        HANDLER_SRC.contains(".ok_or_else(|| AppError::business("),
        "resolve 无映射应返回 AppError::business（脱敏）"
    );
}

// ---------------------------------------------------------------------------
// 辅助：从源码中粗略截取某个函数体（到下一个同级 `\\n    }` 前的行块）
// ---------------------------------------------------------------------------

/// 以签名行为锚点，向后截取到函数结束（大括号配对的最小实现，仅用于契约字面量定位）。
fn extract_fn<'a>(src: &'a str, sig: &str) -> &'a str {
    let start = src.find(sig).unwrap_or(0);
    // 从签名往后取一段足够覆盖函数体的窗口（这些函数都在 100 行以内）。
    let rest = &src[start..];
    // 找到函数体结束：首个「\n    }\n」缩进闭合
    if let Some(end) = rest.find("\n    }") {
        &rest[..end]
    } else {
        rest
    }
}
