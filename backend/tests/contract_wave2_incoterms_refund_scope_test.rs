//! Wave2 定稿口径契约测试（纯函数级，无需数据库）：
//! 1) Incoterms FOB 不含主运费（ICC Incoterms 2020：主运费由买方订立并承担，
//!    与 FCA/FAS 同族；卖方仅承担装船前费用），CFR/CIF 仍含主运费；
//! 2) 退税申报金额聚合只计入 status=verified 的报关单，
//!    pending/cancelled 等未核验行不进申报基数。

use bingxi_backend::models::export_customs_declaration::Model as CustomsModel;
use bingxi_backend::services::export_refund_service::ExportRefundService;
use bingxi_backend::services::incoterms_service::IncotermsService;
use bingxi_backend::utils::incoterms::{CostBearer, Incoterms2020};
use rust_decimal::Decimal;

fn customs(no: &str, status: &str, amount: i64, rate: i64) -> CustomsModel {
    CustomsModel {
        declaration_no: no.to_string(),
        status: status.to_string(),
        total_amount: Decimal::from(amount),
        exchange_rate: Decimal::from(rate),
        ..Default::default()
    }
}

#[test]
fn test_fob_excludes_main_freight() {
    // FOB 与 EXW/FCA/FAS 同为不含主运费集合（主运费归买方）
    assert!(!Incoterms2020::Fob.includes_freight());
    assert!(!Incoterms2020::Exw.includes_freight());
    assert!(!Incoterms2020::Fca.includes_freight());
    assert!(!Incoterms2020::Fas.includes_freight());
}

#[test]
fn test_cfr_cif_still_include_main_freight() {
    // C 组术语卖方订立主运费，口径不受 FOB 调整影响
    assert!(Incoterms2020::Cfr.includes_freight());
    assert!(Incoterms2020::Cif.includes_freight());
    assert!(Incoterms2020::Cpt.includes_freight());
    assert!(Incoterms2020::Cip.includes_freight());
}

#[test]
fn test_fob_cost_bearer_remains_both() {
    // FOB 主费用承担方仍为共担：卖方承担装船前费用、买方订立主运费
    assert_eq!(Incoterms2020::Fob.cost_bearer(), CostBearer::Both);
}

#[test]
fn test_calculate_costs_fob_drops_freight() {
    // 静态试算端点的同一底层函数：FOB 传入运费不得进入价格构成
    let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
        Incoterms2020::Fob,
        Decimal::from(1000),
        Some(Decimal::from(100)),
        Some(Decimal::from(50)),
        Some(Decimal::from(200)),
    );
    assert_eq!(p, Decimal::from(1000));
    assert_eq!(f, None);
    assert_eq!(i, None);
    assert_eq!(d, None);
}

#[test]
fn test_refund_aggregation_counts_only_verified_customs() {
    let rows = vec![
        customs("CD-V1", "verified", 1000, 2),
        customs("CD-P1", "pending", 500, 3),
        customs("CD-C1", "cancelled", 700, 4),
    ];
    // 只有 verified 行进基数：1000×2=2000；pending(1500)/cancelled(2800) 不进基数
    assert_eq!(
        ExportRefundService::verified_export_sales_amount(&rows),
        Decimal::from(2000)
    );
}

#[test]
fn test_refund_aggregation_excludes_all_unverified() {
    // 未核验行整体不进基数（含空集合）：申报基数为 0
    assert_eq!(
        ExportRefundService::verified_export_sales_amount(&[]),
        Decimal::ZERO
    );
    let rows = vec![
        customs("CD-P2", "pending", 1000, 2),
        customs("CD-P3", "pending", 300, 1),
    ];
    assert_eq!(
        ExportRefundService::verified_export_sales_amount(&rows),
        Decimal::ZERO
    );
}
