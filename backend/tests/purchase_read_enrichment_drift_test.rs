//! 采购模块「JOIN 富化出参列 + 列表筛选三处一致」防漂移回归（确定性源码文本扫描，零依赖）。
//!
//! 背景缺陷类：前端列表列绑定后端 JOIN 派生名（creator_name / supplier_name /
//! receipt_no / product_name 等），但后端读模型缺该字段或 service 未真正 JOIN；
//! 以及前端有筛选控件、query 参数已下发，但后端查询 DTO 缺字段 → serde 静默丢弃 →
//! 界面「能筛」但结果永不变。两者都既不报错也不变红，属最危险的「功能没真实接入」。
//!
//! 断言均为「包含」型（源码事实编译期可见），规避「不含某串」被注释自败的坑。

use std::fs;
use std::path::{Path, PathBuf};

fn src(p: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 采购订单列表/详情：DTO 补 creator_name + received_amount；crud 单次查询 JOIN users + 标量子查询；
/// 详情明细经 handler LEFT JOIN products 补 product_name/product_code。
#[test]
fn purchase_order_read_enrichment_is_wired() {
    let dto = src("src/services/po/order.rs");
    assert!(
        dto.contains("pub creator_name: Option<String>"),
        "PurchaseOrderDto 必须声明 creator_name（created_by -> users 姓名）"
    );
    assert!(
        dto.contains("pub received_amount: Option<rust_decimal::Decimal>"),
        "PurchaseOrderDto 必须声明 received_amount（按入库单聚合，可空）"
    );

    let crud = src("src/services/po/order_ops/crud.rs");
    assert!(
        crud.contains("column_as(user::Column::RealName, \"creator_name\")"),
        "list/get_order 必须 JOIN users.real_name 取 creator_name"
    );
    assert!(
        crud.contains("Relation::Creator.def()"),
        "list/get_order 必须经 purchase_order::Relation::Creator 做 LEFT JOIN"
    );
    assert!(
        crud.contains("received_amount_subquery")
            && crud.contains("purchase_receipt::Column::TotalAmount"),
        "received_amount 必须由 SUM(purchase_receipt.total_amount) 关联子查询聚合，禁止占位"
    );
    // 付款状态无来源：绝不出现在订单读模型/富化里
    assert!(
        !dto.contains("payment_status"),
        "payment_status 无 purchase_order 侧真实来源，禁止臆造该列"
    );

    let handler = src("src/handlers/purchase_order_handler.rs");
    assert!(
        handler.contains("column_as(crate::models::product::Column::Name, \"product_name\")")
            && handler
                .contains("column_as(crate::models::product::Column::Code, \"product_code\")")
            && handler.contains("Relation::Product.def()"),
        "get_order 明细必须 LEFT JOIN products 补 product_name/product_code"
    );
}

/// 采购质检列表：读模型补 receipt_no/supplier_name/inspector_name；service JOIN 三表；筛选补
/// keyword / result / inspection_date 范围；handler DTO 收口对应 query 键。
#[test]
fn purchase_inspection_read_enrichment_and_filters_are_wired() {
    let svc = src("src/services/purchase_inspection_service.rs");
    for field in [
        "pub receipt_no: Option<String>",
        "pub supplier_name: Option<String>",
        "pub inspector_name: Option<String>",
    ] {
        assert!(
            svc.contains(field),
            "PurchaseInspectionView 缺 JOIN 列 {}",
            field
        );
    }
    assert!(
        svc.contains("Relation::Receipt.def()")
            && svc.contains("Relation::Supplier.def()")
            && svc.contains("Relation::Inspector.def()"),
        "list_inspections 必须 LEFT JOIN receipt/supplier/inspector"
    );
    assert!(
        svc.contains("Column::InspectionResult.eq("),
        "list_inspections 必须对 inspection_result 做等值筛选"
    );
    assert!(
        svc.contains("Column::InspectionDate.gte(") && svc.contains("Column::InspectionDate.lte("),
        "list_inspections 必须对 inspection_date 做范围筛选"
    );
    assert!(
        svc.contains("safe_like_pattern(")
            && svc.contains("purchase_inspection::Column::InspectionNo")
            && svc.contains("purchase_receipt::Column::ReceiptNo.like("),
        "keyword 必须经 safe_like_pattern 匹配 inspection_no 或 receipt_no"
    );

    let handler = src("src/handlers/purchase_inspection_handler.rs");
    for field in [
        "pub keyword: Option<String>",
        "pub result: Option<String>",
        "pub inspection_date_from: Option<String>",
        "pub inspection_date_to: Option<String>",
    ] {
        assert!(
            handler.contains(field),
            "InspectionQueryParams 缺前端下发键 {}（缺则被 serde 丢弃）",
            field
        );
    }
}

/// 采购价格列表：读模型补 product_name/product_code/supplier_name；service JOIN products/suppliers。
#[test]
fn purchase_price_read_enrichment_is_wired() {
    let svc = src("src/services/purchase_price_service.rs");
    for field in [
        "pub product_name: Option<String>",
        "pub product_code: Option<String>",
        "pub supplier_name: Option<String>",
    ] {
        assert!(
            svc.contains(field),
            "PurchasePriceView 缺 JOIN 列 {}",
            field
        );
    }
    assert!(
        svc.contains("Relation::Product.def()") && svc.contains("Relation::Supplier.def()"),
        "get_prices_list 必须 LEFT JOIN products/suppliers"
    );
}

/// 采购退货列表：读模型补 purchase_order_no/supplier_name/created_by_name；service JOIN 三表；
/// keyword（return_no）+ return_date 范围筛选；handler ReturnQueryParams 收口 keyword/start_date/end_date。
#[test]
fn purchase_return_read_enrichment_and_filters_are_wired() {
    let svc = src("src/services/purchase_return_service.rs");
    for field in [
        "pub purchase_order_no: Option<String>",
        "pub supplier_name: Option<String>",
        "pub created_by_name: Option<String>",
    ] {
        assert!(
            svc.contains(field),
            "PurchaseReturnView 缺 JOIN 列 {}",
            field
        );
    }
    assert!(
        svc.contains("Relation::Order.def()")
            && svc.contains("Relation::Supplier.def()")
            && svc.contains("Relation::Creator.def()"),
        "list_returns 必须 LEFT JOIN order/supplier/creator"
    );
    assert!(
        svc.contains("Column::ReturnNo.like(") && svc.contains("safe_like_pattern("),
        "keyword 必须经 safe_like_pattern 匹配 return_no"
    );
    assert!(
        svc.contains("Column::ReturnDate.gte(") && svc.contains("Column::ReturnDate.lte("),
        "list_returns 必须对 return_date 做范围筛选"
    );

    let handler = src("src/handlers/purchase_return_handler.rs");
    for field in [
        "pub keyword: Option<String>",
        "pub start_date: Option<String>",
        "pub end_date: Option<String>",
    ] {
        assert!(
            handler.contains(field),
            "ReturnQueryParams 缺前端下发键 {}",
            field
        );
    }
}

/// 采购合同列表：读模型补 created_by_name；service JOIN users；signed_date 范围筛选；
/// handler ContractQuery 收 date_range（重复键数组）。
#[test]
fn purchase_contract_read_enrichment_and_date_filter_are_wired() {
    let svc = src("src/services/purchase_contract_service.rs");
    assert!(
        svc.contains("pub created_by_name: Option<String>"),
        "PurchaseContractView 缺 JOIN 列 created_by_name"
    );
    assert!(
        svc.contains("Relation::Creator.def()"),
        "get_list 必须 LEFT JOIN users 取 created_by_name"
    );
    assert!(
        svc.contains("Column::SignedDate.gte(") && svc.contains("Column::SignedDate.lte("),
        "get_list 必须对 signed_date 做范围筛选"
    );
    assert!(
        svc.contains("pub date_range: Option<Vec<String>>"),
        "ContractQueryParams 必须含 date_range 字段"
    );

    let handler = src("src/handlers/purchase_contract_handler.rs");
    assert!(
        handler.contains("pub date_range: Option<Vec<String>>"),
        "ContractQuery 必须收 date_range，否则前端下发的该键被 serde 丢弃"
    );
}
