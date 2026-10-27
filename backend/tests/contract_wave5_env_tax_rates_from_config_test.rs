//! 环保税"污染当量值 / 适用税额"分源契约锁
//!
//! 权威链（本锁的唯一判据）：
//! - **污染当量值＝法定不可调值**：唯一来源 `backend/src/constants/environmental_tax.rs`
//!   （COD=1kg、氨氮=0.5kg、VOCs=0.5kg、污泥=1吨——kg 口径即 1000，逐项搬自改造前
//!   `environmental_tax_service.rs:167-173` 的 match 分支，只搬源不改数）。
//!   未登记污染物必须被显式拒绝（`statutory_pollution_equivalent` 返回 `None` → 校验错误），
//!   禁止旧的 `_ => 1kg` 兜底继续算出看似正常的税额。
//! - **适用税额＝地方可变值**：配置项 `AppSettings::env_tax_rate_per_equivalent`
//!   （env 覆盖键 `ENV_TAX_RATE_PER_EQUIVALENT`；法定幅度每污染当量 1.2–12 元、由省级确定）。
//!   **无硬编码默认值**：未配置时计税显式返回业务错误并记 warn，服务启动不受影响；
//!   旧的 `Decimal::new(24, 1)` 固定 2.4 元"简化口径"由本锁钉死禁回潮。
//!
//! 断言点（六类，缺一即锁失效）
//! ① 未登记污染物名 → 400（VALIDATION_ERROR）+ 真实文案外显 + 失败信封内**无任何税额字段**、
//!    不留任何"按 1kg 算出来的数"、表内无痕；
//! ② 已登记 COD/氨氮/VOCs/污泥（含别名）→ 法定当量值逐值断言（防搬运过程改数）；
//! ③ 税额未配置 → 显式业务失败（400 BUSINESS_ERROR、文案含"环保税适用税额未配置"）；
//!    注入配置值后 → 当量数与税额=当量数×配置值 逐值断具体数字（不只断状态码）；
//! ④ 禁回潮源码扫描（include_str!）：服务源码内不得再出现任何 `Decimal::new(`/
//!    `Decimal::from_parts(` 裸字面量（含旧硬编码税额 `Decimal::new(24, 1)` 与 `_ => 1kg`
//!    兜底残留），且必须引用常量模块，常量模块内法定值逐项在场（防"删常量当迁移"）。
//! ⑤ 增值税税率同型锁，五处价税服务源码与税额内核不得出现税率档位字面量
//!    （Decimal::new(13、0.13、1.13），内核只换算不取值，税额一律按**百分数**口径
//!    逐值断言（1000 乘 13 除 100 得 130、100 含税得 113、130 除 1000 反推得 13、费基非正返回 None）。
//! ⑥ 税率来源分层口径的行为锁（真调生产建单路径，不用源码字符串冒充断言）。
//!    人工建单缺 tax_rate 必须得 400 VALIDATION_ERROR（缺键点名属必填族，与缺数量缺单价同码）
//!    且订单表与明细表双双无痕，
//!    自动补货链路无税率来源，由 price.rs 显式声明无进项税，落库有痕且
//!    tax_percent/tax_amount 逐值为 0（基数非 0，证明 0 是声明值而非空行巧合）。
//!    两侧都禁止回落 13 之类的税率档位。
//!
//! 覆盖策略：纯函数注入 + 真 PostgreSQL（test_common::setup_test_db；
//! 表结构唯一来源 = backend/migration，不再自建 sqlite 同构表）
//! 跑真实 service 全链路（无 mock）。

mod test_common;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bingxi_backend::constants::environmental_tax::statutory_pollution_equivalent;
use bingxi_backend::models::pollutant_discharge_record::{
    Column as DischargeColumn, Entity as DischargeEntity,
};
use bingxi_backend::models::purchase_order_item::{Column as PoItemColumn, Entity as PoItemEntity};
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{product, purchase_order, supplier, warehouse};
use bingxi_backend::services::environmental_tax_service::{
    CreateDischargeRecordRequest, EnvironmentalTaxService,
};
use bingxi_backend::services::po::CreateOrderItemRequest;
use bingxi_backend::services::po::CreatePurchaseOrderRequest;
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::services::po::price::ShortageAlertParams;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::tax;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn record_count(db: &DatabaseConnection) -> usize {
    DischargeEntity::find()
        .all(db)
        .await
        .expect("回查排放记录表失败")
        .len()
}

fn discharge_req(pollutant_name: &str, discharge_amount: Decimal) -> CreateDischargeRecordRequest {
    CreateDischargeRecordRequest {
        discharge_type: "wastewater".to_string(),
        pollutant_name: pollutant_name.to_string(),
        discharge_amount,
        discharge_unit: Some("kg".to_string()),
        concentration: None,
        concentration_unit: None,
        period_year: 2026,
        period_month: 9,
        monitoring_point: None,
        remarks: None,
    }
}

// =========================================================
// ① 未登记污染物 → 400 + 真实文案，且绝不是"按 1kg 出一个数"
// =========================================================

#[tokio::test]
async fn unregistered_pollutant_rejected_4xx_no_default_equivalent() {
    let db = setup_test_db().await;
    // 税额已配置：确保拒绝原因只可能是"污染物未登记"，而非配置缺失
    let svc = EnvironmentalTaxService::new(Arc::new(db.clone()), Some(dec("2.4")));

    let err = svc
        .create_discharge_record(discharge_req("总磷", dec("1")), 1)
        .await
        .expect_err("未登记污染物必须被显式拒绝（即便排放量恰好为 1，也不得按 1kg 兜底计税）");
    assert!(
        matches!(&err, AppError::ValidationErrorDisplayable(m) if m == "该污染物暂未配置法定污染当量值，请先登记后再计税"),
        "必须是可外显校验错误且文案逐字为登记口径，实际: {err:?}"
    );

    let body = err.to_response();
    assert_eq!(body.code, "VALIDATION_ERROR");
    assert_eq!(
        body.message,
        "该污染物暂未配置法定污染当量值，请先登记后再计税"
    );

    let resp: Response = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "未登记污染物必须 4xx"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("读取失败信封 body 失败");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("失败信封应为合法 JSON");
    let rendered = json.to_string();
    // 失败信封只含 code/message/trace_id/timestamp 四键，返回体里不得出现任何税额字段
    for key in ["tax_amount", "tax_unit_equivalent"] {
        assert!(
            !rendered.contains(key),
            "被拒绝计税的返回体不得出现税额字段 {key}，实际: {rendered}"
        );
    }
    assert_eq!(
        record_count(&db).await,
        0,
        "未登记污染物拒绝路径不得落任何排放记录"
    );
}

// =========================================================
// ② 已登记污染物的法定当量值逐值断言（防搬运过程改数）
// =========================================================

#[test]
fn statutory_equivalents_match_legal_values_item_by_item() {
    // 逐项对照改造前 environmental_tax_service.rs:167-173 的 match 分支取值
    for name in ["COD", "cod"] {
        assert_eq!(
            statutory_pollution_equivalent(name),
            Some(dec("1")),
            "{name} 法定污染当量值必须为 1kg"
        );
    }
    for name in ["氨氮", "NH3-N"] {
        assert_eq!(
            statutory_pollution_equivalent(name),
            Some(dec("0.5")),
            "{name} 法定污染当量值必须为 0.5kg"
        );
    }
    for name in ["VOCs", "vocs"] {
        assert_eq!(
            statutory_pollution_equivalent(name),
            Some(dec("0.5")),
            "{name} 法定污染当量值必须为 0.5kg"
        );
    }
    for name in ["污泥", "sludge"] {
        assert_eq!(
            statutory_pollution_equivalent(name),
            Some(dec("1000")),
            "{name} 法定污染当量值为 1 吨（本仓 kg 口径即 1000）"
        );
    }
    // 未登记 = None（不存在任何默认当量值）
    for name in ["总磷", "悬浮物", "石油类", ""] {
        assert_eq!(
            statutory_pollution_equivalent(name),
            None,
            "{name} 未登记，必须返回 None 由调用方显式拒绝"
        );
    }
}

// =========================================================
// ③ 适用税额：未配置显式失败；注入配置后逐值断"税额 = 当量数 × 配置值"
// =========================================================

#[tokio::test]
async fn missing_tax_rate_fails_explicitly_before_any_write() {
    let db = setup_test_db().await;
    // 未配置税额（部署态）：COD 虽已登记，计税也必须显式失败，不得按任何默认值算
    let svc = EnvironmentalTaxService::new(Arc::new(db.clone()), None);

    let err = svc
        .create_discharge_record(discharge_req("COD", dec("100")), 1)
        .await
        .expect_err("适用税额未配置时计税必须显式失败");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains("环保税适用税额未配置")),
        "未配置税额必须外显真实业务文案（含'环保税适用税额未配置'），实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    let resp: Response = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "未配置税额必须 4xx 显式失败"
    );
    assert_eq!(
        record_count(&db).await,
        0,
        "计税显式失败路径不得落任何排放记录"
    );
}

#[test]
fn injected_tax_rate_multiplies_equivalent_exactly() {
    // 模拟省级在法定幅度 1.2–12 元内确定的适用税额注入（纯函数、非生产默认值）
    let rate = dec("3.6");

    let cases: [(&str, &str, &str, &str); 4] = [
        // (污染物, 排放量kg, 期望当量数, 期望税额 = 当量数 × 3.6)
        ("COD", "100", "100", "360"),
        ("氨氮", "25", "50", "180"),
        ("VOCs", "50", "100", "360"),
        ("污泥", "3000", "3", "10.8"),
    ];
    for (name, amount, expect_eq, expect_tax) in cases {
        let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
            "wastewater",
            name,
            dec(amount),
            None,
            Some(rate),
        )
        .unwrap_or_else(|e| panic!("{name} 已登记且税额已注入，计税应成功: {e}"));
        assert_eq!(
            equivalent,
            dec(expect_eq),
            "{name} 污染当量数 = 排放量 ÷ 法定当量值"
        );
        assert_eq!(
            tax,
            dec(expect_tax),
            "{name} 应缴税额必须 = 当量数 × 注入配置值（逐值断数字）"
        );
    }
}

#[tokio::test]
async fn create_discharge_record_with_injected_rate_persists_configured_amount() {
    let db = setup_test_db().await;
    let svc = EnvironmentalTaxService::new(Arc::new(db.clone()), Some(dec("1.2")));

    let model = svc
        .create_discharge_record(discharge_req("氨氮", dec("10")), 1)
        .await
        .expect("已登记污染物 + 已注入税额，全链路应成功");
    // 当量数 = 10 / 0.5 = 20；税额 = 20 × 1.2（注入的法定下限值）= 24
    assert_eq!(model.tax_unit_equivalent, Some(dec("20")));
    assert_eq!(model.tax_amount, dec("24"));

    // 回读落库值（不是只信 insert 回显）
    let stored = DischargeEntity::find()
        .filter(DischargeColumn::PollutantName.eq("氨氮"))
        .one(&db)
        .await
        .expect("回查失败")
        .expect("记录应已落库");
    assert_eq!(
        stored.tax_amount,
        dec("24"),
        "落库税额必须是当量数×配置值，不得是任何硬编码值"
    );
}

// =========================================================
// ④ 禁回潮源码扫描（include_str!）
// =========================================================

#[test]
fn no_hardcoded_equivalents_or_tax_rate_can_return_to_service_source() {
    let svc_src = include_str!("../src/services/environmental_tax_service.rs").replace('\r', "");

    // 重构后服务源码不得再出现任何构造 Decimal 的裸数字字面量（旧 `_ => 1kg` 兜底、
    // `Decimal::new(1,0)/new(5,1)/new(1000,0)` 当量表、`Decimal::new(24, 1)` 硬编码税额
    // 全部由本条一并拦截——当量值只能来自常量模块，税额只能来自配置注入）
    assert!(
        !svc_src.contains("Decimal::new(") && !svc_src.contains("Decimal::from_parts("),
        "environmental_tax_service.rs 不得出现裸 Decimal 数字字面量（当量值/税额均分源）"
    );
    // 显式点名历史违规形态（即便日后有人换写法，这两条文案锁仍在）
    assert!(
        !svc_src.contains("Decimal::new(24, 1)"),
        "硬编码税额 2.4 禁止回潮"
    );
    assert!(
        !svc_src.contains("_ => Decimal::new(1, 0)"),
        "未知污染物默认 1kg 兜底禁止回潮"
    );
    assert!(
        !svc_src.contains("简化口径"),
        "自认'简化口径'的违规注释不得复活"
    );
    // 必须经法定当量值唯一来源取数（正向锁：防止删掉分源结构改回内联）
    assert!(
        svc_src.contains("statutory_pollution_equivalent"),
        "计税必须引用 constants::environmental_tax 法定当量值唯一来源"
    );

    // 常量模块逐值在场（防"删常量当迁移"）：1kg / 0.5kg / 1000kg 的 const 构造
    let const_src = include_str!("../src/constants/environmental_tax.rs").replace('\r', "");
    assert!(
        const_src.contains("Decimal::from_parts(1, 0, 0, false, 0)"),
        "COD 法定当量值 1kg 必须在常量模块在场"
    );
    assert!(
        const_src.contains("Decimal::from_parts(5, 0, 0, false, 1)"),
        "氨氮/VOCs 法定当量值 0.5kg 必须在常量模块在场"
    );
    assert!(
        const_src.contains("Decimal::from_parts(1000, 0, 0, false, 0)"),
        "污泥法定当量值 1 吨（kg 口径 1000）必须在常量模块在场"
    );
    // 登记表未命中必须回 None（显式拒绝的源头判据），不得有兜底数值
    assert!(
        const_src.contains("_ => None"),
        "常量模块未登记污染物必须返回 None，禁止默认当量值回潮"
    );
}

// =========================================================
// ⑤ 增值税税率，单位口径内核 + 禁档位字面量回潮（同型锁扩展，不另造判据）
// =========================================================

/// 禁回潮扫描对象，五处价税计算服务源码 + 税额内核
const VAT_TIER_SCAN_SOURCES: &[(&str, &str)] = &[
    (
        "src/services/quotation_pricing_service.rs",
        include_str!("../src/services/quotation_pricing_service.rs"),
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
        "src/services/outsourcing_ops/receipt.rs",
        include_str!("../src/services/outsourcing_ops/receipt.rs"),
    ),
];

#[test]
fn no_vat_rate_tier_literal_can_return_to_service_sources() {
    for (path, raw) in VAT_TIER_SCAN_SOURCES {
        let src = raw.replace('\r', "");
        assert!(
            !src.contains("Decimal::new(13") && !src.contains("0.13") && !src.contains("1.13"),
            "{path}: 不得出现增值税税率档位字面量（`Decimal::new(13`/`0.13`/`1.13`）——\
             档位只能来自人工录入值、上游单据已存税率或部署配置；\
             `Decimal::new(13, 2)` 实际是 0.13，按百分数口径列存储会把税额算错 100 倍。"
        );
        assert!(
            !src.contains("unwrap_or(Decimal::new(13"),
            "{path}: 缺税率时禁止用档位字面量兜底（人工链路点名拒绝、自动链路显式声明无进项税）"
        );
    }
    // 内核自身不得持有任何税率档位（只换算、不取值）
    let kernel = include_str!("../src/utils/tax.rs").replace('\r', "");
    assert!(
        !kernel.contains("Decimal::new(13") && !kernel.contains("Decimal::from_parts(13"),
        "utils::tax 内核禁止持有税率档位：税率一律由调用方从真实来源传入"
    );
}

#[test]
fn vat_tax_kernel_reverses_from_inclusive_in_percent_unit() {
    // 方向口径：含税金额是事实，净额一律由除法反算、税额一律由差额得到。
    // 百分数口径 13（即 13%）：含税额 1130.00 ÷ 1.13 = 净额 1000.00、税额 = 差额 130.00
    let split = tax::split_from_inclusive(dec("1130"), dec("13"));
    assert_eq!(split.inclusive, dec("1130.00"));
    assert_eq!(split.net, dec("1000.00"));
    assert_eq!(split.tax, dec("130.00"));
    assert_eq!(
        split.net + split.tax,
        split.inclusive,
        "钩稽恒等式必须逐分成立"
    );

    // 含税 1000.00 按 13% 反算：净额 884.9557… 收敛 884.96，税额取差额 115.04。
    // 旧写法「1000 × 13% = 130、总额 1130」是乘法造税额，会把应开票额抬高一档，已废。
    let raw = tax::split_from_inclusive(dec("1000"), dec("13"));
    assert_eq!(raw.net, dec("884.96"));
    assert_eq!(raw.tax, dec("115.04"));
    assert_eq!(raw.net + raw.tax, raw.inclusive);
    // 0 税率与免税走同一枚除法（退化为等值），不开例外分支
    let exempt = tax::split_from_inclusive(dec("1000"), Decimal::ZERO);
    assert_eq!(exempt.net, dec("1000.00"));
    assert_eq!(exempt.tax, Decimal::ZERO);
    // 单价聚合只含「单价 × 数量」，税率不进算式
    assert_eq!(tax::inclusive_amount(dec("100"), dec("10")), dec("1000.00"));
    // 行级单源：含税单价 16.95 × 20 = 339.00 含税，13% 反算净额 300.00、税额 39.00
    let line = tax::line_split(dec("16.95"), dec("20"), Some(Decimal::ZERO), dec("13"));
    assert_eq!(line.list_net, dec("300.00"));
    assert_eq!(line.tax, dec("39.00"));
    assert_eq!(line.unit_price, dec("15.00"));
    assert_eq!(line.deal_inclusive, line.deal_net + line.tax);
    // 钩稽校验：分位一致放行，差一分即点名
    tax::ensure_reconciled(dec("884.96"), dec("115.04"), dec("1000.00"))
        .expect("净额加税额等于含税额必须放行");
    let err = tax::ensure_reconciled(dec("884.95"), dec("115.04"), dec("1000.00"))
        .expect_err("1 分差必须拒绝，不许静默凑平");
    assert!(
        err.to_response().message.contains("价税不钩稽"),
        "拒绝文案必须点名不钩稽，实际: {}",
        err.to_response().message
    );
    // 单价钩稽门：一分以内算同源，超一分说明调用方自己乘了税率
    assert!(tax::unit_price_matches(dec("15.00"), dec("15.00")));
    assert!(!tax::unit_price_matches(dec("16.00"), dec("15.00")));
    // 分摊腿：已记税额按占比分摊，税率不进算式；分母为 0 返回 None 由调用方点名处理
    assert_eq!(
        tax::prorate_tax(dec("130"), dec("5"), dec("10")),
        Some(dec("65.00"))
    );
    assert_eq!(tax::prorate_tax(dec("130"), dec("5"), Decimal::ZERO), None);
    // 由单据已记税额与费基**观察**适用税率（百分数），130 除 1000 得 13
    assert_eq!(
        tax::derive_tax_percent(dec("130"), dec("1000")),
        Some(dec("13"))
    );
    // 费基非正无法观察，返回 None，由调用方点名处理（内核绝不回落档位）
    assert_eq!(tax::derive_tax_percent(dec("13"), Decimal::ZERO), None);
    assert_eq!(tax::derive_tax_percent(dec("13"), dec("-1")), None);
    // 缺含税单价时：0 税率按算术事实退化放行，非 0 税率点名拒绝（不造含税价）
    assert_eq!(
        tax::require_inclusive_unit_price(None, dec("100"), Decimal::ZERO, "unit_price_with_tax")
            .expect("0 税率行不含税价即含税价，应放行"),
        dec("100")
    );
    let missing =
        tax::require_inclusive_unit_price(None, dec("100"), dec("13"), "unit_price_with_tax")
            .expect_err("税率非 0 缺含税单价必须拒绝");
    assert!(missing.to_response().message.contains("缺少含税单价"));
}

/// 方向锁：内核源码里不得存在「乘 (1 + 税率)」或「净额 × 税率」造含税/造税额的公式。
/// 只允许 `÷ (1 + 税率/100)` 与 `含税 − 净额` 两种写法，加上 `单价 × 数量`（事实聚合）
/// 与 `税额 × 分子 ÷ 分母`（分摊）。逐行遍历内核源码，不只看首个匹配。
#[test]
fn vat_tax_kernel_has_no_forward_multiply_formula() {
    let kernel = include_str!("../src/utils/tax.rs").replace('\r', "");
    for (idx, line) in kernel.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") || code.starts_with("//!") {
            continue;
        }
        // 造含税：× (1 + 税率/100) —— 反向公式只允许出现在除号右侧
        assert!(
            !code.contains("* (Decimal::ONE +") && !code.contains("*(Decimal::ONE +"),
            "utils/tax.rs 第 {} 行出现乘 (1 + 税率) 造含税的正向公式: {}",
            idx + 1,
            code
        );
        // 造税额：净额 × 税率 / 100（除 100 与乘税率同行出现）
        if code.contains("tax_percent") || code.contains("rate") {
            assert!(
                !(code.contains("* tax_percent") || code.contains("* rate")),
                "utils/tax.rs 第 {} 行出现乘税率造税额的写法: {}",
                idx + 1,
                code
            );
        }
        assert!(
            !code.contains("1.13") && !code.contains("0.13"),
            "utils/tax.rs 第 {} 行出现硬编码税率档位: {}",
            idx + 1,
            code
        );
    }
}

/// 自动补货/人工建单两条链路的税率行为锁共用的主数据前置，
/// 启用状态供应商（price.rs 取活跃供应商的唯一条件）+ 带成本价的产品
/// （成本价是自动补货行的单价来源，必须非 0，否则税额 0 只是空行巧合）+ 仓库。
/// suppliers/departments 属迁移播种参照表（夹具不清空），故供应商用时间戳后缀
/// 自建专属行，避免与种子行及并发用例互撞。
async fn seed_po_master_data(db: &DatabaseConnection) -> (i32, i32, i32) {
    let suffix = Utc::now().timestamp_nanos_opt().expect("系统时钟不可用");
    let now = Utc::now();

    let prod = product::ActiveModel {
        name: Set(format!("税率锁物料-{suffix}")),
        code: Set(format!("TAXLOCK-{suffix}")),
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
        warehouse_code: Set(format!("WH-TAXLOCK-{suffix}")),
        name: Set("税率锁测试仓".to_string()),
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
        supplier_code: Set(format!("SUP-TAXLOCK-{suffix}")),
        supplier_name: Set(format!("税率锁测试供应商-{suffix}")),
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

/// 人工建单请求，只锁定税率这一个变量（tax_rate = None 即"没录"）。
fn po_create_request(
    supplier_id: i32,
    warehouse_id: i32,
    product_id: i32,
    tax_rate: Option<Decimal>,
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
            // 含税单价 113 是录入事实，数量 10 ⇒ 成交含税额 1130.00，
            // 按 13% 除法反算得净额 1000.00、税额 130.00（与不含税单价 100 钩稽一致）
            unit_price: Some(dec("100")),
            unit_price_with_tax: Some(dec("113")),
            quantity_ordered: Some(dec("10")),
            quantity_alt_ordered: None,
            tax_rate,
            discount_percent: None,
            quantity_tolerance_pct: None,
            color_no: None,
            notes: None,
        }]),
    }
}

/// 人工建单缺 tax_rate 的行为锁，真调生产建单入口 PurchaseOrderService::create_order，
/// 必须是 400 VALIDATION_ERROR（缺键点名属必填族，文案外显"该怎么填"、不含记录标识），
/// 且订单表与明细表双双无痕——税率拒绝发生在算金额之前，事务整体回滚，不允许留下半张单。
#[tokio::test]
async fn manual_order_line_missing_tax_rate_is_rejected_with_zero_write() {
    let db = setup_test_db().await;
    let (supplier_id, warehouse_id, product_id) = seed_po_master_data(&db).await;

    let err = PurchaseOrderService::new(Arc::new(db.clone()))
        .create_order(
            po_create_request(supplier_id, warehouse_id, product_id, None),
            100,
        )
        .await
        .expect_err("人工建单缺 tax_rate 必须被拒，按 0 代填会把漏录洗成免税");

    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "缺税率是必填族缺键点名，必须与缺数量缺单价同码（VALIDATION_ERROR），实得 {err:?}"
    );
    let message = err.to_response().message;
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "缺税率必须 4xx");
    assert!(
        message.contains("缺少税率 tax_rate") && message.contains("请录入税率后重试"),
        "拒绝文案必须点名该怎么填，实际: {message}"
    );
    assert!(
        !message.contains("line_no") && !message.contains("物料"),
        "拒绝文案不得内插记录标识，实际: {message}"
    );
    assert_eq!(
        purchase_order::Entity::find()
            .count(&db)
            .await
            .expect("统计订单行数失败"),
        0,
        "被拒的建单不得留下订单主表行"
    );
    assert_eq!(
        PoItemEntity::find()
            .count(&db)
            .await
            .expect("统计明细行数失败"),
        0,
        "被拒的建单不得留下明细行"
    );

    // 同一条链路的反面判据，合法百分数税率必须能建单落库（含税 1130.00 按 13% 除法反算
    // 得净额 1000.00、税额 130.00），防止把"拒缺值"实现成"拒一切"。
    let order = PurchaseOrderService::new(Arc::new(db.clone()))
        .create_order(
            po_create_request(supplier_id, warehouse_id, product_id, Some(dec("13"))),
            100,
        )
        .await
        .expect("提交合法百分数税率 13 的建单必须成功");
    let item = PoItemEntity::find()
        .filter(PoItemColumn::OrderId.eq(order.id))
        .one(&db)
        .await
        .expect("回查明细失败")
        .expect("合法建单应已落明细行");
    assert_eq!(item.tax_percent, dec("13"), "百分数口径必须原样落库");
    assert_eq!(
        item.subtotal,
        dec("1000"),
        "净额必须是含税额 1130 除以 1.13 的反算值"
    );
    assert_eq!(item.tax_amount, dec("130"), "税额必须是 1130 − 1000 的差额");
    assert_eq!(
        item.subtotal + item.tax_amount,
        item.total_amount,
        "落库三值必须逐分钩稽（净额 + 税额 = 含税）"
    );
}

/// 自动补货链路的行为锁，真调生产入口 create_purchase_suggestion_from_shortage
/// （无人工录入环节、无税率来源），必须由 price.rs 显式声明无进项税并**落库有痕**，
/// tax_percent 与 tax_amount 都是 0，而 subtotal（不含税基数）非 0——
/// 证明 0 税额是声明值，不是空行或没算出来的巧合，也绝不出现 13 之类的档位。
#[tokio::test]
async fn auto_replenishment_suggestion_persists_declared_no_input_tax() {
    let db = setup_test_db().await;
    let (_supplier_id, _warehouse_id, product_id) = seed_po_master_data(&db).await;

    let order = PurchaseOrderService::new(Arc::new(db.clone()))
        .create_purchase_suggestion_from_shortage(ShortageAlertParams {
            material_id: product_id,
            material_name: "税率锁物料".to_string(),
            material_code: "TAXLOCK-AUTO".to_string(),
            required_quantity: dec("100"),
            available_quantity: dec("40"),
            shortage_quantity: dec("60"),
            shortage_level: "Critical".to_string(),
            affected_orders_count: 1,
        })
        .await
        .expect("自动补货链路不得因缺税率来源被关掉");

    let item = PoItemEntity::find()
        .filter(PoItemColumn::OrderId.eq(order.id))
        .one(&db)
        .await
        .expect("回查自动补货明细失败")
        .expect("自动补货建议必须已落库（有痕）");
    assert!(
        item.subtotal > Decimal::ZERO,
        "基数必须非 0，否则税额 0 只是空行巧合，实际 subtotal={}",
        item.subtotal
    );
    assert_eq!(
        item.tax_percent,
        tax::NO_INPUT_TAX_PERCENT,
        "自动链路落库税率必须是显式声明的无进项税 0，既不是 13 也不是任何档位"
    );
    assert_eq!(item.tax_amount, Decimal::ZERO, "无进项税行的税额必须为 0");
}
