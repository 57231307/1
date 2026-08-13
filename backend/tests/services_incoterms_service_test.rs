use bingxi_backend::utils::incoterms::*;
use rust_decimal::Decimal;


#[test]
fn test_calculate_costs_exw_no_freight() {
    // EXW 不含运费/保费/关税，仅含产品成本
    let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
        Incoterms2020::Exw,
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
fn test_calculate_costs_cif_includes_freight_insurance() {
    // CIF 含运费和保险，不含关税
    let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
        Incoterms2020::Cif,
        Decimal::from(1000),
        Some(Decimal::from(100)),
        Some(Decimal::from(50)),
        Some(Decimal::from(200)),
    );
    assert_eq!(p, Decimal::from(1000));
    assert_eq!(f, Some(Decimal::from(100)));
    assert_eq!(i, Some(Decimal::from(50)));
    assert_eq!(d, None);
}

#[test]
fn test_calculate_costs_ddp_includes_all() {
    // DDP 含运费/保费/关税
    let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
        Incoterms2020::Ddp,
        Decimal::from(1000),
        Some(Decimal::from(100)),
        Some(Decimal::from(50)),
        Some(Decimal::from(200)),
    );
    assert_eq!(p, Decimal::from(1000));
    assert_eq!(f, Some(Decimal::from(100)));
    assert_eq!(i, Some(Decimal::from(50)));
    assert_eq!(d, Some(Decimal::from(200)));
}

#[test]
fn test_calculate_costs_fob_freight_only() {
    // FOB 含运费，不含保险/关税
    let (p, f, i, d) = IncotermsService::calculate_costs_by_incoterm(
        Incoterms2020::Fob,
        Decimal::from(1000),
        Some(Decimal::from(100)),
        Some(Decimal::from(50)),
        Some(Decimal::from(200)),
    );
    assert_eq!(p, Decimal::from(1000));
    assert_eq!(f, Some(Decimal::from(100)));
    assert_eq!(i, None);
    assert_eq!(d, None);
}