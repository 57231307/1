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
//!    另用含税 100 按 13% 反算钉死「差额」而非「独立舍入」（乘法独立舍入会多算出 11.51）。
//! ② 真调生产建单入口 PurchaseOrderService::create_order，逐档建单落库后回读
//!    purchase_order_item 的 subtotal 与 tax_amount 与 total_amount 三列，断钩稽恒等式
//!    与逐档具体数字（不只断接口成功），证明除法反算腿贯通到落库。
//!    suppliers 属迁移种子参照表不清空，供应商用时间戳后缀自建专属行，互不碰撞。
//! ③ 静态正向公式禁回潮锁，逐行遍历内核与全部价税写入腿源码，
//!    不只看首个匹配、不因注释遮蔽、不按中文字面量放行，改回乘法必红。
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
fn assert_split(tax_percent: Decimal, inclusive: Decimal, expect_net: Decimal, expect_tax: Decimal) {
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

    // 方向钉死的反例，含税 100 按 13% 反算净额 88.50、税额取差额 11.50。
    // 被禁止的「独立舍入」会先把 88.50 乘 13% 得 11.51，净额加税额 = 100.01 与含税额 100 差 1 分，
    // 票面与台账不钩稽；本锁要求税额逐分等于含税减净额。
    let edge = tax::split_from_inclusive(dec("100"), dec("13"));
    assert_eq!(edge.net, dec("88.50"), "含税 100 按 13% 除法反算净额必须是 88.50");
    assert_eq!(edge.tax, dec("11.50"), "税额必须是差额 11.50，不得是独立舍入的 11.51");
    assert_eq!(edge.net + edge.tax, edge.inclusive, "钩稽恒等式必须逐分成立");
    let forbidden_independent_tax = (edge.net * dec("13") / dec("100")).round_dp(2);
    assert_ne!(
        forbidden_independent_tax, edge.tax,
        "被禁止的「净额乘税率」独立舍入得 {forbidden_independent_tax}，与差额 {} 不同，正是要拦的形态",
        edge.tax
    );

    // 单价聚合只含「单价 乘 数量」，税率不进算式，不构成造含税
    assert_eq!(tax::inclusive_amount(dec("11.30"), dec("10")), dec("113.00"));
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
        assert_eq!(line.list_net, dec(expect_net), "税率 {rate}% 行净额必须是除法反算值");
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

        assert_eq!(item.tax_percent, dec(rate), "税率 {rate}% 百分数口径必须原样落库");
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

/// 扫描对象，价税分离内核与全部价税写入腿源码
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
        "src/services/po/order_ops/crud.rs",
        include_str!("../src/services/po/order_ops/crud.rs"),
    ),
    ("src/services/po/price.rs", include_str!("../src/services/po/price.rs")),
    (
        "src/services/so/order_crud.rs",
        include_str!("../src/services/so/order_crud.rs"),
    ),
    (
        "src/services/purchase_return_service.rs",
        include_str!("../src/services/purchase_return_service.rs"),
    ),
    (
        "src/services/sales_return_service.rs",
        include_str!("../src/services/sales_return_service.rs"),
    ),
];

/// 把一行源码去掉空白，避免用空格形态绕过（`* (Decimal::ONE +` 与 `*(Decimal::ONE+` 同一目标）。
fn compact(line: &str) -> String {
    line.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 是否为注释行（含 doc 注释），仅按整行起始判定，不因此放过代码里的违规
fn is_comment_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

#[test]
fn tax_path_has_no_forward_multiply_formula() {
    // 被禁止的硬编码增值税档位乘数（历史违规形态）
    const FORBIDDEN_TIER_LITERALS: &[&str] = &["1.13", "0.13", "1.09", "1.06", "1.055"];
    for (path, raw) in TAX_PATH_SOURCES {
        for (idx, line) in raw.replace('\r', "").lines().enumerate() {
            if is_comment_line(line) {
                continue;
            }
            let c = compact(line);
            // 造含税，乘 (1 + 税率)。除法反算写的是 `/ (Decimal::ONE + ...)`，不在禁列
            assert!(
                !c.contains("*(Decimal::ONE+"),
                "{path} 第 {} 行出现「乘 (1 + 税率)」造含税的正向公式: {}",
                idx + 1,
                line.trim()
            );
            // 造税额，净额 乘 税率。除法反算是 `tax_percent / PERCENT`，也不禁观察腿的 `* PERCENT`
            assert!(
                !c.contains("*tax_percent") && !c.contains("*tax_rate"),
                "{path} 第 {} 行出现「乘税率」造税额的写法: {}",
                idx + 1,
                line.trim()
            );
            // 硬编码增值税档位乘数
            for lit in FORBIDDEN_TIER_LITERALS {
                assert!(
                    !c.contains(lit),
                    "{path} 第 {} 行出现硬编码增值税档位 {lit}: {}",
                    idx + 1,
                    line.trim()
                );
            }
        }
    }
}

/// 正向锁，价税写入腿必须真的调用内核反算单源，而不是各自写一套合并逻辑（防删调用改回内联）
#[test]
fn tax_write_legs_delegate_to_kernel_single_source() {
    let po_crud = include_str!("../src/services/po/order_ops/crud.rs").replace('\r', "");
    let so_crud = include_str!("../src/services/so/order_crud.rs").replace('\r', "");
    let preturn = include_str!("../src/services/purchase_return_service.rs").replace('\r', "");
    let sreturn = include_str!("../src/services/sales_return_service.rs").replace('\r', "");
    for (path, src) in [
        ("po/order_ops/crud.rs", &po_crud),
        ("so/order_crud.rs", &so_crud),
        ("purchase_return_service.rs", &preturn),
        ("sales_return_service.rs", &sreturn),
    ] {
        assert!(
            src.contains("tax::line_split") && src.contains("tax::ensure_reconciled"),
            "{path} 行级价税必须经内核 line_split 单源并做 ensure_reconciled 钩稽自检，不得各写一套合并逻辑"
        );
    }
}
