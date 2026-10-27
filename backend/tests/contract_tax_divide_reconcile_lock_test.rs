//! 含税价税分离方向锁，逐档税率活体钩稽与正向公式禁回潮静态锁
//!
//! 判据（本锁唯一权威来源，utils::tax 内核文档已写明）：
//! - **方向**，含税金额或含税单价是录入事实，价税分离只允许一条腿，
//!   `净额 = 含税 ÷ (1 + 税率% ÷ 100)` 先四舍五入到分，`税额 = 含税 − 净额` 取差额。
//!   禁止 `净额 × 税率` 造税额、`× (1 + 税率)` 造含税、`× 1.13` 硬编码档位。
//! - **钩稽恒等式**，`净额 + 税额 == 含税` 逐分成立，杜绝两处各自舍入的 1 分差。
//! - **0 税率与免税**走同一枚除法（退化为等值），不开例外分支。
//!
//! 覆盖三块，缺一即锁失效，
//! ① 纯内核逐档钩稽与方向，税率档必须含 0 与 13 与 9 与 6 与一枚小数税率 5.5，
//!    另用含税 100.01 按 13% 反算钉死「差额 11.51」而非「独立舍入 11.50」
//!    （净额乘税率再舍入与含税事实差 1 分，票面不钩稽）。
//! ② 真调生产建单入口 PurchaseOrderService::create_order，逐档建单落库后回读
//!    purchase_order_item 的 subtotal 与 tax_amount 与 total_amount 三列，断钩稽恒等式
//!    与逐档具体数字（不只断接口成功），证明除法反算腿贯通到落库。
//!    suppliers 属迁移种子参照表不清空，供应商用时间戳后缀自建专属行，互不碰撞。
//! ③ 静态正向公式禁回潮锁，逐行遍历内核与全部价税写入腿（后端 14 个文件）与
//!    前端 11 个含税录入与派生腿源码，禁列形态含「乘 (Decimal::ONE + 税率)」「乘 (1 + 税率)」
//!    「乘 tax 前缀标识符（含点号接收者形态 `*item.tax_percent` 与 `*row.taxRate`）」
//!    「乘号括号窗口内税率与 +1 或 +Decimal::ONE 反序加数并存」与硬编码档位字面量；
//!    注释剥离走逐行状态机（`//` 与行首 `/*` 与 `<!--` 起始的块注释整体剔除，
//!    闭合标记之后的文本与行尾注释前的代码照样被扫），rustfmt 与 prettier 换行到
//!    续行行首的乘号（`* (Decimal::ONE` 与 `* tax_percent`）不再被误当块注释续行跳过，
//!    注释里的说明性公式也不会混进拼串层造成误报；
//!    不只看首个匹配（每个 `*(` 与每个 `*` 出现点逐一判定），不按中文字面量放行界面文案；
//!    后端与前端都把「去注释后的全文拼串」再过一遍同一组判定，跨行遮蔽形态照样命中。
//!    扫描清单由 PINNED 钉列逐条点名，删条目等于开缺口，即判红。
//!    判定器自身另有一枚形态自检锁，逐个喂禁止形态与合法形态，漏判或误判都判红。
//!    正向委托锁逐文件钉内核引用与单源入口，删引用或内联第二套公式同样判红。
//!
//! 覆盖策略，纯函数注入 加 真 PostgreSQL（test_common::setup_test_db），跑真实 service 全链路，无 mock。

mod test_common;

use bingxi_backend::models::purchase_order_item::{Column as PoItemColumn, Entity as PoItemEntity};
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{product, supplier, warehouse};
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::services::po::{CreateOrderItemRequest, CreatePurchaseOrderRequest};
use bingxi_backend::utils::tax;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// 一行含税事实与档率，断钩稽恒等式与方向（净额是除法反算值、税额是差额、非独立舍入）。
/// 入参，inclusive 含税金额、tax_percent 百分数税率、expect_net 期望净额、expect_tax 期望税额。
fn assert_split(
    tax_percent: Decimal,
    inclusive: Decimal,
    expect_net: Decimal,
    expect_tax: Decimal,
) {
    let split = tax::split_from_inclusive(inclusive, tax_percent);
    assert_eq!(
        split.net, expect_net,
        "税率 {tax_percent}% 含税 {inclusive} 的净额必须是除法反算值"
    );
    assert_eq!(
        split.tax, expect_tax,
        "税率 {tax_percent}% 含税 {inclusive} 的税额必须是「含税 − 净额」的差额"
    );
    assert_eq!(
        split.net + split.tax,
        split.inclusive,
        "税率 {tax_percent}% 钩稽恒等式 净额 + 税额 == 含税 必须逐分成立"
    );
    // 方向钉死，税率非 0 时净额严格小于含税额（除法反算），税额严格大于 0
    if tax_percent > Decimal::ZERO {
        assert!(
            split.net < split.inclusive && split.tax > Decimal::ZERO,
            "税率 {tax_percent}% 必须把含税额拆成更小的净额加正税额（除法反算方向）"
        );
    } else {
        assert_eq!(
            split.net, split.inclusive,
            "0 税率行除法退化为等值，净额即含税额、税额为 0，不开例外分支"
        );
        assert_eq!(split.tax, Decimal::ZERO, "0 税率行税额必须为 0");
    }
}

// =========================================================
// ① 纯内核逐档钩稽与方向
// =========================================================

#[test]
fn kernel_reconciles_across_all_tax_tiers() {
    // 逐档，0 与 13 与 9 与 6 与一枚小数税率 5.5，净额由含税反算、税额取差额
    assert_split(dec("0"), dec("1130.00"), dec("1130.00"), Decimal::ZERO);
    assert_split(dec("13"), dec("1130.00"), dec("1000.00"), dec("130.00"));
    assert_split(dec("9"), dec("1090.00"), dec("1000.00"), dec("90.00"));
    assert_split(dec("6"), dec("1060.00"), dec("1000.00"), dec("60.00"));
    assert_split(dec("5.5"), dec("1055.00"), dec("1000.00"), dec("55.00"));

    // 方向钉死的反例，含税 100 按 13% 反算净额 88.50、税额取差额 11.50，钩稽逐分成立。
    let edge = tax::split_from_inclusive(dec("100"), dec("13"));
    assert_eq!(
        edge.net,
        dec("88.50"),
        "含税 100 按 13% 除法反算净额必须是 88.50"
    );
    assert_eq!(edge.tax, dec("11.50"), "税额必须是差额 11.50");
    assert_eq!(
        edge.net + edge.tax,
        edge.inclusive,
        "钩稽恒等式必须逐分成立"
    );

    // 两处各自独立舍入必炸 1 分差的钉死样本，含税 100.01 按 13%：
    // 除法反算净额 100.01 ÷ 1.13 = 88.504424… 收敛 88.50，税额取差额 11.51，
    // 净额加税额逐分等于 100.01。被禁止的「净额乘税率」独立舍入是
    // 88.50 × 13 ÷ 100 = 11.505 收敛 11.50，与差额 11.51 差 1 分，
    // 且 88.50 + 11.50 = 100.00 与含税事实 100.01 不钩稽，正是要拦的形态。
    let diff_case = tax::split_from_inclusive(dec("100.01"), dec("13"));
    assert_eq!(
        diff_case.net,
        dec("88.50"),
        "含税 100.01 按 13% 除法反算净额必须是 88.50"
    );
    assert_eq!(
        diff_case.tax,
        dec("11.51"),
        "税额必须是差额 11.51（含税 100.01 减净额 88.50），不是独立舍入的 11.50"
    );
    assert_eq!(
        diff_case.net + diff_case.tax,
        diff_case.inclusive,
        "钩稽恒等式必须逐分成立，独立舍入产生的 1 分差在这里必红"
    );
    let forbidden_independent_tax = (diff_case.net * dec("13") / dec("100")).round_dp(2);
    assert_eq!(
        forbidden_independent_tax,
        dec("11.50"),
        "被禁止的「净额乘税率」独立舍入在本样本必须得 11.50，用于证明它与差额 11.51 分化"
    );
    assert_ne!(
        forbidden_independent_tax, diff_case.tax,
        "「净额乘税率」独立舍入得 {forbidden_independent_tax}，与差额 {} 差 1 分，正是要拦的形态",
        diff_case.tax
    );

    // 单价聚合只含「单价 乘 数量」，税率不进算式，不构成造含税
    assert_eq!(
        tax::inclusive_amount(dec("11.30"), dec("10")),
        dec("113.00")
    );
}

#[test]
fn kernel_line_split_reconciles_across_all_tax_tiers() {
    // 行级单源逐档，含税单价 数量，无折扣，断 subtotal 减 discount 加 tax 等于 total
    let cases: [(&str, &str, &str, &str, &str); 5] = [
        // (含税单价, 数量, 税率, 期望净额, 期望税额)
        ("11.30", "10", "13", "100.00", "13.00"),
        ("10.90", "10", "9", "100.00", "9.00"),
        ("10.60", "10", "6", "100.00", "6.00"),
        ("10.55", "10", "5.5", "100.00", "5.50"),
        ("10.00", "10", "0", "100.00", "0.00"),
    ];
    for (inc_unit, qty, rate, expect_net, expect_tax) in cases {
        let line = tax::line_split(dec(inc_unit), dec(qty), None, dec(rate));
        assert_eq!(
            line.list_net,
            dec(expect_net),
            "税率 {rate}% 行净额必须是除法反算值"
        );
        assert_eq!(line.tax, dec(expect_tax), "税率 {rate}% 行税额必须是差额");
        assert_eq!(
            line.deal_net + line.tax,
            line.deal_inclusive,
            "税率 {rate}% 行级钩稽 净额加税额等含税额必须成立"
        );
        assert_eq!(
            line.list_net - line.discount_net + line.tax,
            line.deal_inclusive,
            "税率 {rate}% 落库三列 subtotal 减 discount 加 tax 等 total 必须逐分成立"
        );
        tax::ensure_reconciled(line.deal_net, line.tax, line.deal_inclusive)
            .unwrap_or_else(|e| panic!("税率 {rate}% 行级三值必须通过钩稽自检: {e}"));
    }
}

// =========================================================
// ② 真调生产建单入口逐档落库钩稽（活体）
// =========================================================

/// 主数据前置，启用供应商、带成本价产品、仓库，供应商用时间戳后缀自建专属行
/// （suppliers 属迁移种子参照表，夹具不清空，故须私有号段避免与种子行及并发用例互撞）。
async fn seed_po_master_data(db: &sea_orm::DatabaseConnection) -> (i32, i32, i32) {
    let suffix = Utc::now().timestamp_nanos_opt().expect("系统时钟不可用");
    let now = Utc::now();

    let prod = product::ActiveModel {
        name: Set(format!("逐档钩稽物料-{suffix}")),
        code: Set(format!("TAXTIER-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        cost_price: Set(Some(dec("100"))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed product 失败");

    let wh = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-TAXTIER-{suffix}")),
        name: Set("逐档钩稽测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed warehouse 失败");

    let sup = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-TAXTIER-{suffix}")),
        supplier_name: Set(format!("逐档钩稽供应商-{suffix}")),
        supplier_short_name: Set(String::new()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(String::new()),
        registered_address: Set(String::new()),
        legal_representative: Set(String::new()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("固定日期不可构造")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set(String::new()),
        bank_account: Set(String::new()),
        contact_phone: Set(String::new()),
        status: Set(Some(master_data::ACTIVE.to_string())),
        is_processor: Set(false),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed supplier 失败");

    (sup.id, wh.id, prod.id)
}

/// 单行建单请求，含税单价是录入事实，不含税单价提交为期望反算值（与内核同源，能通过单价钩稽门）。
fn po_request(
    supplier_id: i32,
    warehouse_id: i32,
    product_id: i32,
    inclusive_unit: Decimal,
    net_unit: Decimal,
    tax_rate: Decimal,
) -> CreatePurchaseOrderRequest {
    CreatePurchaseOrderRequest {
        supplier_id,
        order_date: NaiveDate::from_ymd_opt(2026, 9, 20).expect("固定日期不可构造"),
        expected_delivery_date: None,
        warehouse_id: Some(warehouse_id),
        department_id: Some(1),
        currency: None,
        exchange_rate: None,
        payment_terms: None,
        shipping_terms: None,
        notes: None,
        attachment_urls: None,
        source_sales_order_id: None,
        qualification_waiver_reason: None,
        items: Some(vec![CreateOrderItemRequest {
            line_no: Some(1),
            material_id: Some(product_id),
            unit_price: Some(net_unit),
            unit_price_with_tax: Some(inclusive_unit),
            quantity_ordered: Some(dec("10")),
            quantity_alt_ordered: None,
            tax_rate: Some(tax_rate),
            discount_percent: None,
            quantity_tolerance_pct: None,
            color_no: None,
            notes: None,
        }]),
    }
}

/// 逐档税率真调建单，落库后回读三列断钩稽恒等式与具体净额与税额与含税额。
#[tokio::test]
async fn live_create_order_reconciles_per_tax_tier() {
    let db = setup_test_db().await;
    let (supplier_id, warehouse_id, product_id) = seed_po_master_data(&db).await;
    let svc = PurchaseOrderService::new(Arc::new(db.clone()));

    // (税率, 含税单价, 不含税单价, 期望净额subtotal, 期望税额, 期望含税total)
    let tiers: &[(&str, &str, &str, &str, &str, &str)] = &[
        ("0", "10.00", "10.00", "100.00", "0.00", "100.00"),
        ("13", "11.30", "10.00", "100.00", "13.00", "113.00"),
        ("9", "10.90", "10.00", "100.00", "9.00", "109.00"),
        ("6", "10.60", "10.00", "100.00", "6.00", "106.00"),
        ("5.5", "10.55", "10.00", "100.00", "5.50", "105.50"),
    ];

    for (rate, inc_unit, net_unit, expect_subtotal, expect_tax, expect_total) in tiers {
        let order = svc
            .create_order(
                po_request(
                    supplier_id,
                    warehouse_id,
                    product_id,
                    dec(inc_unit),
                    dec(net_unit),
                    dec(rate),
                ),
                100,
            )
            .await
            .unwrap_or_else(|e| panic!("税率 {rate}% 合法建单必须成功: {e}"));

        let item = PoItemEntity::find()
            .filter(PoItemColumn::OrderId.eq(order.id))
            .one(&db)
            .await
            .expect("回查明细失败")
            .expect("建单应已落明细行");

        assert_eq!(
            item.tax_percent,
            dec(rate),
            "税率 {rate}% 百分数口径必须原样落库"
        );
        assert_eq!(
            item.subtotal,
            dec(expect_subtotal),
            "税率 {rate}% 净额 subtotal 必须是含税除法反算值"
        );
        assert_eq!(
            item.tax_amount,
            dec(expect_tax),
            "税率 {rate}% 税额必须是含税额减净额的差额"
        );
        assert_eq!(
            item.total_amount,
            dec(expect_total),
            "税率 {rate}% 含税额必须等于录入含税事实的聚合"
        );
        assert_eq!(
            item.subtotal + item.tax_amount,
            item.total_amount,
            "税率 {rate}% 落库三值必须逐分钩稽，净额加税额等于含税"
        );
    }
}

// =========================================================
// ③ 静态正向公式禁回潮锁（税额路径不存在乘 1+税率 的正向公式）
// =========================================================

/// 扫描对象，价税分离内核与全部价税写入腿源码（后端）。
/// 清单按写入腿逐点对齐，新增或删减价税写入腿时必须同步本清单，漏一个文件即锁失效。
const TAX_PATH_SOURCES: &[(&str, &str)] = &[
    ("src/utils/tax.rs", include_str!("../src/utils/tax.rs")),
    (
        "src/services/quotation_pricing_service.rs",
        include_str!("../src/services/quotation_pricing_service.rs"),
    ),
    (
        "src/services/quotation_ops/calc.rs",
        include_str!("../src/services/quotation_ops/calc.rs"),
    ),
    (
        "src/services/quotation_ops/crud.rs",
        include_str!("../src/services/quotation_ops/crud.rs"),
    ),
    (
        "src/services/quotation_ops/update.rs",
        include_str!("../src/services/quotation_ops/update.rs"),
    ),
    (
        "src/services/po/order_ops/crud.rs",
        include_str!("../src/services/po/order_ops/crud.rs"),
    ),
    (
        "src/services/po/price.rs",
        include_str!("../src/services/po/price.rs"),
    ),
    (
        "src/services/po/receipt.rs",
        include_str!("../src/services/po/receipt.rs"),
    ),
    (
        "src/services/so/order_crud.rs",
        include_str!("../src/services/so/order_crud.rs"),
    ),
    (
        "src/services/so/delivery_ops/ship.rs",
        include_str!("../src/services/so/delivery_ops/ship.rs"),
    ),
    (
        "src/services/purchase_return_service.rs",
        include_str!("../src/services/purchase_return_service.rs"),
    ),
    (
        "src/services/sales_return_service.rs",
        include_str!("../src/services/sales_return_service.rs"),
    ),
    (
        "src/services/ap_invoice_ops/receipt.rs",
        include_str!("../src/services/ap_invoice_ops/receipt.rs"),
    ),
    (
        "src/services/outsourcing_ops/receipt.rs",
        include_str!("../src/services/outsourcing_ops/receipt.rs"),
    ),
];

/// 扫描对象，前端含税录入与派生腿源码（与后端同一方向口径，同源反算单源在 utils/money.ts）。
/// 清单按「含税单价是录入事实」的界面逐点对齐，报价、销售订单、采购订单、销售退货四条录入腿
/// 与一枚单源都在列内；采购退货与发票与合同三处前端不自行派生金额（只回显后端权威值），
/// 所以不在本清单，一旦将来在前端加出这三条派生腿必须同步加进本清单与下面的钉列。
const FRONTEND_TAX_SOURCES: &[(&str, &str)] = &[
    (
        "frontend/src/utils/money.ts",
        include_str!("../../frontend/src/utils/money.ts"),
    ),
    (
        "frontend/src/views/quotations/components/QuotationItemEditor.vue",
        include_str!("../../frontend/src/views/quotations/components/QuotationItemEditor.vue"),
    ),
    (
        "frontend/src/views/quotations/create.vue",
        include_str!("../../frontend/src/views/quotations/create.vue"),
    ),
    (
        "frontend/src/views/quotations/detail.vue",
        include_str!("../../frontend/src/views/quotations/detail.vue"),
    ),
    (
        "frontend/src/views/sales/OrderFormDialog.vue",
        include_str!("../../frontend/src/views/sales/OrderFormDialog.vue"),
    ),
    (
        "frontend/src/views/sales/composables/useOlv.ts",
        include_str!("../../frontend/src/views/sales/composables/useOlv.ts"),
    ),
    (
        "frontend/src/views/purchase/composables/useCreate.ts",
        include_str!("../../frontend/src/views/purchase/composables/useCreate.ts"),
    ),
    (
        "frontend/src/views/purchase/components/PurchaseCreateDialog.vue",
        include_str!("../../frontend/src/views/purchase/components/PurchaseCreateDialog.vue"),
    ),
    (
        "frontend/src/views/sales-returns/composables/useSr.ts",
        include_str!("../../frontend/src/views/sales-returns/composables/useSr.ts"),
    ),
    (
        "frontend/src/views/sales-returns/components/ReturnEditDialog.vue",
        include_str!("../../frontend/src/views/sales-returns/components/ReturnEditDialog.vue"),
    ),
    (
        "frontend/src/views/sales-returns/components/ReturnDetailDialog.vue",
        include_str!("../../frontend/src/views/sales-returns/components/ReturnDetailDialog.vue"),
    ),
];

/// 后端扫描清单的权威钉列。与 [`TAX_PATH_SOURCES`] 必须是同一集合，
/// 由 [`static_scan_manifest_covers_every_tax_leg`] 逐条点名核对；
/// 从 TAX_PATH_SOURCES 里删掉任何一条（或在写入腿新文件里写正向公式却不加进清单）都判红。
const PINNED_BACKEND_TAX_LEGS: &[&str] = &[
    "src/utils/tax.rs",
    "src/services/quotation_pricing_service.rs",
    "src/services/quotation_ops/calc.rs",
    "src/services/quotation_ops/crud.rs",
    "src/services/quotation_ops/update.rs",
    "src/services/po/order_ops/crud.rs",
    "src/services/po/price.rs",
    "src/services/po/receipt.rs",
    "src/services/so/order_crud.rs",
    "src/services/so/delivery_ops/ship.rs",
    "src/services/purchase_return_service.rs",
    "src/services/sales_return_service.rs",
    "src/services/ap_invoice_ops/receipt.rs",
    "src/services/outsourcing_ops/receipt.rs",
];

/// 前端扫描清单的权威钉列，语义与 [`PINNED_BACKEND_TAX_LEGS`] 相同。
const PINNED_FRONTEND_TAX_LEGS: &[&str] = &[
    "frontend/src/utils/money.ts",
    "frontend/src/views/quotations/components/QuotationItemEditor.vue",
    "frontend/src/views/quotations/create.vue",
    "frontend/src/views/quotations/detail.vue",
    "frontend/src/views/sales/OrderFormDialog.vue",
    "frontend/src/views/sales/composables/useOlv.ts",
    "frontend/src/views/purchase/composables/useCreate.ts",
    "frontend/src/views/purchase/components/PurchaseCreateDialog.vue",
    "frontend/src/views/sales-returns/composables/useSr.ts",
    "frontend/src/views/sales-returns/components/ReturnEditDialog.vue",
    "frontend/src/views/sales-returns/components/ReturnDetailDialog.vue",
];

/// 把一行源码去掉空白，避免用空格形态绕过（`* (Decimal::ONE +` 与 `*(Decimal::ONE+` 同一目标）。
fn compact(line: &str) -> String {
    line.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 块注释闭合标记的最早出现处（`*/` 与 `-->` 两种，取更靠前者与其长度）。
/// 传给谁 [`code_part_of_line`]，用于跨行推进块注释状态。
fn block_closer(text: &str) -> Option<(usize, usize)> {
    let star = text.find("*/").map(|i| (i, 2));
    let html = text.find("-->").map(|i| (i, 3));
    match (star, html) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// 剥掉一行的注释部分，出参是本行剩下的可扫描代码原文，`in_block` 是跨行推进的块注释态。
///
/// 入参，line 是源码一行、in_block 是进本行之前的块注释态（调用方持有，逐行传递）。
/// 出参是本行代码部分（可能为空串，空即本行全是注释）；进本行之后的块注释态写回 in_block。
/// 判定标记只认注释起始序列本身（`//`、`/*`、`<!--`），闭合认 `*/` 与 `-->`；
/// 块注释结束同一行若还接代码，那段代码照样返回给调用方扫（`<!-- 说明 --><div>` 与
/// `/* 备注 */ let x = ...` 都逃不掉）；行尾注释从出现处截断，前面的代码照样返回。
///
/// 为什么不用「行首是 `*` 就当注释」的旧判定：rustfmt 与 prettier 会把二元乘号换行到
/// 续行行首，写出 `* (Decimal::ONE + tax_percent / PERCENT)` 与 `* tax_percent / 100` 这种
/// 形态，其行首与块注释续行的 `*` 完全同形，旧判定把这类行整行跳过，正向公式改回乘法
/// 也照样绿（变异自证实测过的遮蔽盲区）。状态机把两件事分开：真注释按状态整体剔除，
/// 代码态里的 `*` 起头行一律保留，既不放过续行里的乘法，也不把注释星号混进拼串层误报。
///
/// 已知取舍，行中出现 `//` 即按行尾注释起点截断，字符串字面量里的 `https://` 会让本行
/// 后半段不进扫描。25 枚扫描对象逐行核过，没有同一行既带 URL 又带价税算式的写法
/// （`://` 与 tax 同行零命中），真出现这种写法时按行拆分代码即可，不放宽本判定。
fn code_part_of_line(line: &str, in_block: &mut bool) -> String {
    let mut rest = line.to_string();
    if *in_block {
        match block_closer(&rest) {
            Some((pos, len)) => {
                *in_block = false;
                rest = rest[pos + len..].to_string();
            }
            None => return String::new(),
        }
    }
    loop {
        let trimmed = rest.trim_start();
        let indent = rest.len() - trimmed.len();
        // 整行起始的行注释（含 //! 与 ///）与块注释起始
        if trimmed.starts_with("//") {
            return String::new();
        }
        let opens_block = if trimmed.starts_with("/*") {
            Some(2)
        } else if trimmed.starts_with("<!--") {
            Some(4)
        } else {
            None
        };
        if let Some(open_len) = opens_block {
            return match block_closer(&rest[indent + open_len..]) {
                // 同行开闭的块注释，保留注释后的部分继续判定
                Some((pos, len)) => {
                    rest = rest[indent + open_len + pos + len..].to_string();
                    continue;
                }
                None => {
                    *in_block = true;
                    String::new()
                }
            };
        }
        // 行中出现的注释起始，取最早的一处处理
        let candidates: [(usize, usize, bool); 3] = [
            (rest.find("//"), 2, false),
            (rest.find("/*"), 2, true),
            (rest.find("<!--"), 4, true),
        ]
        .map(|(pos, len, is_block)| (pos.unwrap_or(usize::MAX), len, is_block));
        let earliest = candidates
            .iter()
            .min_by_key(|(pos, _, _)| *pos)
            .copied()
            .expect("候选数组恒有三项");
        if earliest.0 == usize::MAX {
            return rest;
        }
        if !earliest.2 {
            // 行尾注释，截掉注释部分，前面的代码照扫
            return rest[..earliest.0].to_string();
        }
        match block_closer(&rest[earliest.0 + earliest.1..]) {
            Some((pos, len)) => {
                // 行中块注释且同行闭合，注释前后两段拼接后一起返回
                return format!(
                    "{} {}",
                    &rest[..earliest.0],
                    &rest[earliest.0 + earliest.1 + pos + len..]
                );
            }
            None => {
                *in_block = true;
                return rest[..earliest.0].to_string();
            }
        }
    }
}

/// 逐行剥注释，出参是本文件的代码行清单（1 基行号与剥掉注释后的代码原文）。
/// 传给谁 [`scan_tax_leg_source`] 与拼串补扫，注释行不进清单也不进拼串。
fn code_lines_of(src: &str) -> Vec<(usize, String)> {
    let mut in_block = false;
    let mut out = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        let code = code_part_of_line(line, &mut in_block);
        if !code.trim().is_empty() {
            out.push((idx + 1, code));
        }
    }
    out
}

/// 被禁止的硬编码增值税档位乘数（历史违规形态）。
/// 判定用「相邻字符不是数字」防 `11.13` 这类无关小数被误伤；
/// 本锁宁可误报、绝不漏报：漏报就是正向公式回潮无人知。
fn contains_tier_literal(c: &str, lit: &str) -> bool {
    c.match_indices(lit).any(|(idx, _)| {
        let before_ok = idx == 0 || !c.as_bytes()[idx - 1].is_ascii_digit();
        let after_idx = idx + lit.len();
        let after_ok = after_idx >= c.len() || !c.as_bytes()[after_idx].is_ascii_digit();
        before_ok && after_ok
    })
}

/// 扫描对象共用的档位字面量禁列。
const FORBIDDEN_TIER_LITERALS: &[&str] = &["1.13", "0.13", "1.09", "1.06", "1.055"];

/// 判定「乘号后紧跟一个括号，括号内是 1 + 税率 形态」，覆盖全部限定前缀变体：
/// `*(Decimal::ONE+`、`*(rust_decimal::Decimal::ONE+`、`*(1+` 以及任何
/// `*(<路径前缀>::Decimal::ONE+` 形态。变异自证轮实测限定前缀曾绕过窄形态禁列，
/// 本判定按「每个 `*(` 出现点」逐个检查其后 48 字符，不是只看首个匹配。
/// 出参是首个命中的 `*(` 字节偏移（供判红消息取片段），未命中返回 None。
/// 除法反算写的是 `/ (Decimal::ONE + ...)` 与 `/ (1 + ...)`，乘号是 `*(`，除法是 `/(`，
/// 两个形态互不误伤。
fn mul_one_plus_hit(c: &str) -> Option<usize> {
    c.match_indices("*(").map(|(idx, _)| idx).find(|&idx| {
        let rest: String = c[idx + 2..].chars().take(48).collect();
        rest.starts_with("1+") || rest.contains("Decimal::ONE+")
    })
}

/// 判定「乘号括号窗口内税率与加数反序并存」，覆盖正序判定漏掉的两个形态：
/// `*(taxRate+1)` 与 `*(tax_percent / 100 + Decimal::ONE)`（加数在税率之后）。
/// 与 [`mul_one_plus_hit`] 同样逐 `*(` 出现点检查其后 48 字符，不是只看首个匹配。
/// 命中条件是「窗口含税率词根（大小写不敏感）且含相邻非数字的 +1 加数，或含
/// +Decimal::ONE」；`*(i.unit_price_with_tax||0)` 这类窗口含 with_tax 词根但没有
/// +1 加数的合法聚合不会命中（with_tax 结尾是含税事实不是税率变量，且窗口内没有加法）。
fn mul_tax_reverse_addend_hit(c: &str) -> Option<usize> {
    c.match_indices("*(").map(|(idx, _)| idx).find(|&idx| {
        let rest: String = c[idx + 2..].chars().take(48).collect();
        let lower = rest.to_lowercase();
        if !lower.contains("tax") {
            return false;
        }
        let plus_one = rest
            .match_indices("+1")
            .any(|(i, _)| i + 2 >= rest.len() || !rest.as_bytes()[i + 2].is_ascii_digit());
        plus_one || lower.contains("+decimal::one")
    })
}

/// 判定「乘号后直接跟税率标识符（含点号接收者形态）」，覆盖 `*tax`、`*item.tax_percent`、
/// `*row.taxRate`。旧形态只钉乘号后紧跟 `tax` 前缀一种，中间隔接收者点号的漏判
/// （变异自证侦察确认 `*row.taxRate` 能绕过）。逐 `*` 出现点解析其后的
/// `[字母数字_]+(\.[字母数字_]+)*` 标识符路径，任一分段以 tax 开头（大小写不敏感）即命中；
/// `*unit_price_with_tax` 这类以 with_tax 结尾的含税事实聚合腿不在禁列（与既有口径一致）。
/// 出参是命中处 `*` 的字节偏移，未命中返回 None。
fn mul_by_tax_path_identifier_hit(c: &str) -> Option<usize> {
    let b = c.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'*' {
            let mut j = i + 1;
            loop {
                let seg_start = j;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                if j == seg_start {
                    break;
                }
                if c[seg_start..j].to_ascii_lowercase().starts_with("tax") {
                    return Some(i);
                }
                if j < b.len()
                    && b[j] == b'.'
                    && j + 1 < b.len()
                    && (b[j + 1].is_ascii_alphanumeric() || b[j + 1] == b'_')
                {
                    j += 1;
                    continue;
                }
                break;
            }
        }
        i += 1;
    }
    None
}

/// 对一段（已去空白的）源码文本施加全部禁止形态，出参是命中的形态清单（空即合法）。
/// 每项带命中点的字节偏移，供判红消息定位。后端与前端共用本函数，
/// 两侧禁列同集合、同顺序，不存在「后端钉了限定前缀、前端只钉裸 1」这类两侧同缺的漏判。
/// 逐出现点判定，不是只看首个匹配：括号窗口两形态对每一个 `*(` 出现点各查其后 48 字符，
/// 标识符形态对每一个 `*` 出现点各解析其后的标识符路径。
fn forward_formula_hits(c: &str) -> Vec<(&'static str, usize)> {
    let mut hits: Vec<(&'static str, usize)> = Vec::new();
    // 造含税，乘 (1 + 税率)，含任意限定前缀变体
    if let Some(idx) = mul_one_plus_hit(c) {
        hits.push(("「乘 (1 + 税率)」造含税（含限定前缀与空格形态）", idx));
    }
    // 反序加数形态：`*(tax_percent / 100 + Decimal::ONE)` 与 `*(taxRate + 1)`
    if let Some(idx) = mul_tax_reverse_addend_hit(c) {
        hits.push((
            "「乘括号窗口内税率与 +1 或 +Decimal::ONE 反序加数」造含税",
            idx,
        ));
    }
    // 造税额，乘任何以 tax 开头的标识符（含点号接收者、驼峰与下划线两种命名；
    // 合法形态里 `*` 之后只会跟 quantity/percent/share/numerator 等非税率标识符，
    // unit_price_with_tax 这类以 tax 结尾的含税事实不在禁列，禁的是 tax 开头）
    if let Some(idx) = mul_by_tax_path_identifier_hit(c) {
        hits.push(("「乘 tax 开头标识符（含点号接收者与驼峰形态）」造税额", idx));
    }
    hits
}

/// 取命中点附近的文本片段（前后各 60 字符），让文件级拼串判红也能定位到具体写法。
fn hit_snippet(c: &str, offset: usize) -> String {
    let char_idx = c[..offset.min(c.len())].chars().count();
    let chars: Vec<char> = c.chars().collect();
    let start = char_idx.saturating_sub(60).min(chars.len());
    let end = (char_idx + 60).min(chars.len());
    chars[start..end].iter().collect()
}

/// 对一行代码施加全部禁止形态（含硬编码档位字面量，档位字面量只在逐行层判：
/// 跨行接缝可能拼出与税率无关的小数，拼串层判它只产误报）。命中即 panic 点名文件、行号与原文。
/// 逐行逐点施加，不是找到第一个就停：任何一行漏检都算锁失效。
fn assert_line_free_of_forward_formula(path: &str, line_no: usize, code: &str, c: &str) {
    let hits = forward_formula_hits(c);
    assert!(
        hits.is_empty(),
        "{path} 第 {line_no} 行出现{}的正向公式（命中点附近 {}）: {}",
        hits.iter()
            .map(|(form, _)| *form)
            .collect::<Vec<&str>>()
            .join("、"),
        hits.first()
            .map(|(_, offset)| hit_snippet(c, *offset))
            .unwrap_or_default(),
        code.trim()
    );
    for lit in FORBIDDEN_TIER_LITERALS {
        assert!(
            !contains_tier_literal(c, lit),
            "{path} 第 {line_no} 行出现硬编码增值税档位 {lit}: {}",
            code.trim()
        );
    }
}

/// 逐行加跨行拼串扫描一枚价税腿源码，后端与前端共用同一套判定。
/// 拼串层把「剥完注释的代码行按行序拼接」再过一遍括号窗口与标识符三形态，
/// 兜住乘号与括号内容被格式化器拆到不同行的碎形态；
/// 注释按状态机整体剔除，既不会让续行乘号被当成注释漏扫，也不会让注释里的说明性公式进拼串误报。
fn scan_tax_leg_source(path: &str, raw: &str) {
    let src = raw.replace('\r', "");
    let code_lines = code_lines_of(&src);
    assert!(
        !code_lines.is_empty(),
        "{path} 剥注释后没有任何可扫描代码行，扫描对象被掏空等于锁失效"
    );
    for (line_no, code) in &code_lines {
        assert_line_free_of_forward_formula(path, *line_no, code, &compact(code));
    }
    let joined: String = code_lines
        .iter()
        .map(|(_, code)| compact(code))
        .collect::<Vec<String>>()
        .join("");
    let joined_hits = forward_formula_hits(&joined);
    assert!(
        joined_hits.is_empty(),
        "{path} 出现跨行拼接后成立的{}正向公式（去注释全文拼串命中，片段 {}）",
        joined_hits
            .iter()
            .map(|(form, _)| *form)
            .collect::<Vec<&str>>()
            .join("、"),
        joined_hits
            .first()
            .map(|(_, offset)| hit_snippet(&joined, *offset))
            .unwrap_or_default(),
    );
}

/// 后端价税写入腿的正向公式禁回潮锁，逐行加跨行拼串全量施加判定。
#[test]
fn tax_path_has_no_forward_multiply_formula() {
    for (path, raw) in TAX_PATH_SOURCES {
        scan_tax_leg_source(path, raw);
    }
}

/// 前端同口径锁，含税单价是录入事实，派生腿只准除法反算。
/// 历史违规点位（OrderFormDialog 硬编码 0.13 与 1.13、QuotationItemEditor 浮点乘 1+税率）
/// 一旦改回乘法必须判红并点名文件与行号。
#[test]
fn frontend_tax_legs_have_no_forward_multiply_formula() {
    for (path, raw) in FRONTEND_TAX_SOURCES {
        scan_tax_leg_source(path, raw);
    }
}

/// 扫描清单完整性锁，钉列与扫描数组必须是同一集合。
/// 从 TAX_PATH_SOURCES 或 FRONTEND_TAX_SOURCES 里删掉任何一条（正向公式的回潮面就此消失一处），
/// 或者钉列与实际清单漂移（两侧同缺同序的盲区），都在此逐条点名判红，不靠人记得住清单。
#[test]
fn static_scan_manifest_covers_every_tax_leg() {
    for pinned in PINNED_BACKEND_TAX_LEGS {
        assert!(
            TAX_PATH_SOURCES.iter().any(|(path, _)| path == pinned),
            "后端价税写入腿 {pinned} 不在扫描清单里，删清单条目等于给正向公式开缺口"
        );
    }
    assert_eq!(
        TAX_PATH_SOURCES.len(),
        PINNED_BACKEND_TAX_LEGS.len(),
        "后端扫描清单与钉列条数不一致，存在清单外的多余条目或钉列漏项（两侧同缺同序即在此判红）"
    );
    for pinned in PINNED_FRONTEND_TAX_LEGS {
        assert!(
            FRONTEND_TAX_SOURCES.iter().any(|(path, _)| path == pinned),
            "前端含税录入与派生腿 {pinned} 不在扫描清单里，删清单条目等于给正向公式开缺口"
        );
    }
    assert_eq!(
        FRONTEND_TAX_SOURCES.len(),
        PINNED_FRONTEND_TAX_LEGS.len(),
        "前端扫描清单与钉列条数不一致，存在清单外的多余条目或钉列漏项"
    );
    // 每枚扫描对象必须真的有内容（include_str! 指向空文件或被截断等于假绿）
    for (path, raw) in TAX_PATH_SOURCES.iter().chain(FRONTEND_TAX_SOURCES.iter()) {
        assert!(
            raw.len() > 200,
            "{path} 的源码原文不足 200 字节，扫描对象疑似被掏空"
        );
    }
}

/// 判定器形态自检锁。变异自证两轮证明的是「源码真被扫到、判红能点名」，
/// 本锁证明的是判定函数本身，三件事缺一不可：
/// ① 每一种禁止书写形态都命中（限定前缀、空格、反序加数、点号接收者、驼峰与下划线、档位字面量）；
/// ② 合法形态一律不误判（除法反算、单价乘数量的事实聚合、税额分摊、观察式反推税率），
///    否则下一轮真改动会因为误报被人为放宽禁列，锁就废了；
/// ③ 一行里既有合法 `*(` 又有违规 `*(` 时照样命中（防只看首个匹配）。
#[test]
fn forward_formula_judges_reject_every_shape_and_tolerate_legal_forms() {
    // ① 禁止形态，逐条必须命中
    let forbidden: &[&str] = &[
        "letinclusive=net*(Decimal::ONE+tax_percent/PERCENT);",
        "letinclusive=net*(rust_decimal::Decimal::ONE+tax_percent/dec100);",
        "letinclusive=net*(1+taxPercent/100);",
        "letinclusive=net*(taxRate+1);",
        "letinclusive=net*(tax_percent/100+Decimal::ONE);",
        "lettax=net*tax_percent/100;",
        "lettax=item.tax_amount*item.tax_percent;",
        "lettax=row.total*taxRate;",
        "lettax=qty*tax;",
        "letinclusive=net*1.13;",
        "lettax=net*0.13;",
        "letinclusive=net*1.055;",
        // 只看首个匹配的盲区，前一处 *( 合法、后一处 *( 违规
        "letx=f*(q+1)*(Decimal::ONE+tax_percent/PERCENT);",
    ];
    for form in forbidden {
        let hits = forward_formula_hits(form);
        let tier = FORBIDDEN_TIER_LITERALS
            .iter()
            .any(|lit| contains_tier_literal(form, lit));
        assert!(
            !hits.is_empty() || tier,
            "判定器漏判禁止形态 {form}，正向公式换个写法就能绕过本锁"
        );
    }
    // ② 合法形态，一律不得命中（命中即误报，真改动会被当成违规而被放宽）
    let legal: &[&str] = &[
        "letnet=(inclusive.round_dp(2)/(Decimal::ONE+tax_percent/PERCENT)).round_dp(2);",
        "letnet=roundToCent(roundToCent(inclusive)/(1+taxPercent/100));",
        "letnet=netFromInclusive(inclusive,tax_rate);",
        "lettax=inclusive-net;",
        "letamount=(unit_price_with_tax*quantity).round_dp(2);",
        "letdiscount=(list_inclusive*percent/PERCENT).round_dp(2);",
        "lettax_share=(recorded_tax*share_numerator/share_denominator).round_dp(2);",
        "letpct=tax_amount/base*PERCENT;",
        "letsum=net+tax;",
        "letpool=i.unit_price_with_tax*(qty||0);",
        // 单价聚合里乘号后跟的是数量与含税事实，不是税率
        "lettotal=price*quantity_with_tax;",
    ];
    for ok in legal {
        let hits = forward_formula_hits(ok);
        assert!(
            hits.is_empty()
                && !FORBIDDEN_TIER_LITERALS
                    .iter()
                    .any(|lit| contains_tier_literal(ok, lit)),
            "判定器误判合法形态 {ok}，命中 {:?}，误报会让后来人放宽禁列",
            hits.iter().map(|(form, _)| *form).collect::<Vec<&str>>()
        );
    }
    // ③ 注释剥离状态机，续行行首的乘号不得被当成块注释跳过（遮蔽盲区），
    //    块注释里的说明性公式不得进拼串（误报盲区）
    let split_formula =
        "fn f() {\n    let inclusive = net\n        * (Decimal::ONE + tax_percent / PERCENT);\n}\n";
    let lines = code_lines_of(&split_formula.replace('\r', ""));
    assert!(
        lines
            .iter()
            .any(|(_, code)| forward_formula_hits(&compact(code)).len() == 1),
        "行首乘号的算子续行被注释判定吞掉，跨行遮蔽的假绿盲区复活"
    );
    let doc_formula = "/*\n * 说明 禁止 net * (Decimal::ONE + tax_percent / PERCENT) 这种写法\n */\nlet net = inclusive;\n";
    let doc_lines = code_lines_of(&doc_formula.replace('\r', ""));
    assert!(
        doc_lines
            .iter()
            .all(|(_, code)| forward_formula_hits(&compact(code)).is_empty()),
        "块注释里的说明性公式混进了扫描清单，注释形态会产误报"
    );
    assert_eq!(
        doc_lines.len(),
        1,
        "块注释剥完应当只剩一行真代码，实得 {:?}",
        doc_lines
    );
}

/// 正向锁，价税写入腿必须真的调用内核反算单源，而不是各自写一套合并逻辑（防删调用改回内联）。
/// 逐文件按各自的合法单源入口断言，缺任何一个调用点即判红点名。
#[test]
fn tax_write_legs_delegate_to_kernel_single_source() {
    // 行级或拆分级写入腿：必须经 tax::line_split 并做 ensure_reconciled 钩稽自检
    for path in [
        "src/services/po/order_ops/crud.rs",
        "src/services/purchase_return_service.rs",
        "src/services/sales_return_service.rs",
    ] {
        let src = read_backend_source(path);
        assert!(
            src.contains("tax::line_split") && src.contains("tax::ensure_reconciled"),
            "{path} 行级价税必须经内核 line_split 单源并做 ensure_reconciled 钩稽自检，不得各写一套合并逻辑"
        );
    }
    // 销售订单腿：行金额与信用基数共用同一单源 calculate_sales_item_amounts，
    // 内核入口 require_inclusive_unit_price 与 line_split 与 ensure_reconciled 三处都必须在
    let so_crud = read_backend_source("src/services/so/order_crud.rs");
    assert!(
        so_crud.contains("tax::line_split")
            && so_crud.contains("tax::require_inclusive_unit_price")
            && so_crud.contains("tax::ensure_reconciled"),
        "src/services/so/order_crud.rs 建单行金额与信用基数腿必须全走内核，缺一即判红"
    );
    // 报价域腿：行金额一律经 quotation_line_amounts 单源（其内部走内核除法反算）
    for path in [
        "src/services/quotation_ops/calc.rs",
        "src/services/quotation_ops/crud.rs",
        "src/services/quotation_ops/update.rs",
    ] {
        let src = read_backend_source(path);
        assert!(
            src.contains("quotation_line_amounts"),
            "{path} 报价行金额必须经 quotation_line_amounts 单源，不得内联第二套公式"
        );
    }
    // 内核引用腿：收货重算、发货分摊、自动补货、报价采价、应付进项、委外转出都必须引用 utils tax
    for path in [
        "src/services/po/receipt.rs",
        "src/services/so/delivery_ops/ship.rs",
        "src/services/po/price.rs",
        "src/services/quotation_pricing_service.rs",
        "src/services/ap_invoice_ops/receipt.rs",
        "src/services/outsourcing_ops/receipt.rs",
    ] {
        let src = read_backend_source(path);
        assert!(
            src.contains("utils::tax::"),
            "{path} 价税相关计算必须引用 utils tax 内核单源，删引用等于绕过本锁"
        );
    }
    // 前端派生腿：同源单函数 netFromInclusive 必须保有一处除法反算，
    // 报价明细编辑器、报价表头合计、销售订单建单行与采购订单建单行四条派生腿
    // 必须真调该单源，不许内联第二套公式
    let money = read_frontend_source("frontend/src/utils/money.ts");
    assert!(
        money.contains("export function netFromInclusive")
            && money.contains("/ (1 + taxPercent / 100)"),
        "frontend/src/utils/money.ts 必须保有一处「含税 ÷ (1 + 税率% ÷ 100)」除法反算单源"
    );
    for path in [
        "frontend/src/views/quotations/components/QuotationItemEditor.vue",
        "frontend/src/views/quotations/create.vue",
        "frontend/src/views/sales/OrderFormDialog.vue",
        "frontend/src/views/purchase/composables/useCreate.ts",
    ] {
        let src = read_frontend_source(path);
        assert!(
            src.contains("netFromInclusive"),
            "{path} 不含税净额必须真调 money.ts 的 netFromInclusive 单源，内联第二套公式即判红"
        );
    }
    // 税额取差额的单源入口，采购与销售两条建单腿都必须真调
    for path in [
        "frontend/src/views/sales/OrderFormDialog.vue",
        "frontend/src/views/purchase/composables/useCreate.ts",
    ] {
        let src = read_frontend_source(path);
        assert!(
            src.contains("taxFromInclusive"),
            "{path} 税额必须真调 money.ts 的 taxFromInclusive（含税减净额取差额），不得自己乘税率造税额"
        );
    }
    // 销售订单行含税单价还原腿：只能由已落库含税额除数量还原，禁止按税率倒造含税价
    let olv = read_frontend_source("frontend/src/views/sales/composables/useOlv.ts");
    assert!(
        olv.contains("total_amount") && olv.contains("roundToCent"),
        "frontend/src/views/sales/composables/useOlv.ts 的含税单价必须继续由已落库含税额还原，改回按税率倒造即判红"
    );
}

/// 按路径读取后端扫描对象原文，清单外路径直接 panic（防正向锁悄悄失焦）。
fn read_backend_source(path: &str) -> String {
    TAX_PATH_SOURCES
        .iter()
        .find(|(p, _)| *p == path)
        .unwrap_or_else(|| panic!("后端扫描清单缺 {path}，正向锁覆盖面漏了写入腿"))
        .1
        .replace('\r', "")
}

/// 按路径读取前端扫描对象原文，清单外路径直接 panic。
fn read_frontend_source(path: &str) -> String {
    FRONTEND_TAX_SOURCES
        .iter()
        .find(|(p, _)| *p == path)
        .unwrap_or_else(|| panic!("前端扫描清单缺 {path}，正向锁覆盖面漏了派生腿"))
        .1
        .replace('\r', "")
}
