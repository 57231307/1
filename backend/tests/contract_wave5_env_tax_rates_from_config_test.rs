//! 环保税"污染当量值 / 适用税额"分源契约锁（PR #942 wave5 决策定案 #6）
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
//! 断言点（四类，缺一即锁失效）：
//! ① 未登记污染物名 → 400（VALIDATION_ERROR）+ 真实文案外显 + 失败信封内**无任何税额字段**、
//!    不留任何"按 1kg 算出来的数"、表内无痕；
//! ② 已登记 COD/氨氮/VOCs/污泥（含别名）→ 法定当量值逐值断言（防搬运过程改数）；
//! ③ 税额未配置 → 显式业务失败（400 BUSINESS_ERROR、文案含"环保税适用税额未配置"）；
//!    注入配置值后 → 当量数与税额=当量数×配置值 逐值断具体数字（不只断状态码）；
//! ④ 禁回潮源码扫描（include_str!）：服务源码内不得再出现任何 `Decimal::new(`/
//!    `Decimal::from_parts(` 裸字面量（含旧硬编码税额 `Decimal::new(24, 1)` 与 `_ => 1kg`
//!    兜底残留），且必须引用常量模块；常量模块内法定值逐项在场（防"删常量当迁移"）。
//!
//! 覆盖策略：纯函数注入 + 真 PostgreSQL（test_common::setup_test_db，路线一
//! #4669 判责：表结构唯一来源 = backend/migration，不再自建 sqlite 同构表）
//! 跑真实 service 全链路（无 mock）。

mod test_common;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bingxi_backend::constants::environmental_tax::statutory_pollution_equivalent;
use bingxi_backend::models::pollutant_discharge_record::{
    Column as DischargeColumn, Entity as DischargeEntity,
};
use bingxi_backend::services::environmental_tax_service::{
    CreateDischargeRecordRequest, EnvironmentalTaxService,
};
use bingxi_backend::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};
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
        created_by: Some(1),
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
        .create_discharge_record(discharge_req("总磷", dec("1")))
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
        .create_discharge_record(discharge_req("COD", dec("100")))
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
        .create_discharge_record(discharge_req("氨氮", dec("10")))
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
