use bingxi_backend::services::data_permission_service::*;
use bingxi_backend::services::quotation_pricing_service::*;
use bingxi_backend::services::sensitive_action_alert::*;
use bingxi_backend::services::stock_alert::*;
use bingxi_backend::utils::incoterms::*;


#[test]
fn test_from_code_uppercase() {
    assert_eq!(Incoterms2020::from_code("fob").unwrap(), Incoterms2020::Fob);
    assert_eq!(Incoterms2020::from_code("FOB").unwrap(), Incoterms2020::Fob);
    assert_eq!(Incoterms2020::from_code("CIF").unwrap(), Incoterms2020::Cif);
    // V15 P0-B14：新增 6 种术语解析验证
    assert_eq!(Incoterms2020::from_code("fca").unwrap(), Incoterms2020::Fca);
    assert_eq!(Incoterms2020::from_code("CPT").unwrap(), Incoterms2020::Cpt);
    assert_eq!(Incoterms2020::from_code("cip").unwrap(), Incoterms2020::Cip);
    assert_eq!(Incoterms2020::from_code("dpu").unwrap(), Incoterms2020::Dpu);
    assert_eq!(Incoterms2020::from_code("fas").unwrap(), Incoterms2020::Fas);
    assert_eq!(Incoterms2020::from_code("cfr").unwrap(), Incoterms2020::Cfr);
}

#[test]
fn test_from_code_invalid() {
    assert!(Incoterms2020::from_code("ABC").is_err());
    assert!(Incoterms2020::from_code("").is_err());
}

#[test]
fn test_all_eleven_terms() {
    // V15 P0-B14：11 种术语全量校验
    assert_eq!(Incoterms2020::all().len(), 11);
    for term in Incoterms2020::all() {
        // 双向解析校验：code → from_code → code 应保持一致
        let code = term.code();
        assert_eq!(Incoterms2020::from_code(code).unwrap(), term);
    }
}

#[test]
fn test_includes_insurance() {
    assert!(Incoterms2020::Cif.includes_insurance());
    assert!(Incoterms2020::Cip.includes_insurance());
    assert!(Incoterms2020::Ddp.includes_insurance());
    assert!(!Incoterms2020::Fob.includes_insurance());
    assert!(!Incoterms2020::Exw.includes_insurance());
    assert!(!Incoterms2020::Fca.includes_insurance());
    assert!(!Incoterms2020::Cpt.includes_insurance());
    assert!(!Incoterms2020::Cfr.includes_insurance());
}

#[test]
fn test_includes_freight() {
    // EXW / FCA / FAS 不含运费
    assert!(!Incoterms2020::Exw.includes_freight());
    assert!(!Incoterms2020::Fca.includes_freight());
    assert!(!Incoterms2020::Fas.includes_freight());
    // 其他 8 种均含运费
    assert!(Incoterms2020::Fob.includes_freight());
    assert!(Incoterms2020::Cif.includes_freight());
    assert!(Incoterms2020::Cpt.includes_freight());
    assert!(Incoterms2020::Cip.includes_freight());
    assert!(Incoterms2020::Cfr.includes_freight());
    assert!(Incoterms2020::Dap.includes_freight());
    assert!(Incoterms2020::Dpu.includes_freight());
    assert!(Incoterms2020::Ddp.includes_freight());
}

#[test]
fn test_requires_duty_paid() {
    assert!(Incoterms2020::Ddp.requires_duty_paid());
    assert!(!Incoterms2020::Cif.requires_duty_paid());
    assert!(!Incoterms2020::Fob.requires_duty_paid());
    assert!(!Incoterms2020::Dap.requires_duty_paid());
}

#[test]
fn test_description_not_empty() {
    for term in Incoterms2020::all() {
        assert!(!term.description().is_empty());
        assert!(!term.risk_transfer_point().is_empty());
    }
}

#[test]
fn test_is_sea_only() {
    // 海运专用：FAS / FOB / CFR / CIF
    assert!(Incoterms2020::Fas.is_sea_only());
    assert!(Incoterms2020::Fob.is_sea_only());
    assert!(Incoterms2020::Cfr.is_sea_only());
    assert!(Incoterms2020::Cif.is_sea_only());
    // 任意运输：其他 7 种
    assert!(!Incoterms2020::Exw.is_sea_only());
    assert!(!Incoterms2020::Fca.is_sea_only());
    assert!(!Incoterms2020::Cpt.is_sea_only());
    assert!(!Incoterms2020::Cip.is_sea_only());
    assert!(!Incoterms2020::Dap.is_sea_only());
    assert!(!Incoterms2020::Dpu.is_sea_only());
    assert!(!Incoterms2020::Ddp.is_sea_only());
}

#[test]
fn test_cost_bearer() {
    // V15 P2 23.5 缺陷3：结构化费用承担方
    assert_eq!(Incoterms2020::Exw.cost_bearer(), CostBearer::Buyer);
    assert_eq!(Incoterms2020::Dap.cost_bearer(), CostBearer::Seller);
    assert_eq!(Incoterms2020::Dpu.cost_bearer(), CostBearer::Seller);
    assert_eq!(Incoterms2020::Ddp.cost_bearer(), CostBearer::Seller);
    assert_eq!(Incoterms2020::Fob.cost_bearer(), CostBearer::Both);
    assert_eq!(Incoterms2020::Cif.cost_bearer(), CostBearer::Both);
    assert_eq!(Incoterms2020::Cpt.cost_bearer(), CostBearer::Both);
    assert_eq!(Incoterms2020::Fca.cost_bearer(), CostBearer::Both);
}

#[test]
fn test_clearance_party() {
    // V15 P2 23.5 缺陷3：出口清关除 EXW 外卖方负责
    assert_eq!(
        Incoterms2020::Exw.export_clearance_party(),
        Party::Buyer
    );
    assert_eq!(
        Incoterms2020::Fob.export_clearance_party(),
        Party::Seller
    );
    assert_eq!(
        Incoterms2020::Ddp.export_clearance_party(),
        Party::Seller
    );
    // 进口清关除 DDP 外买方负责
    assert_eq!(
        Incoterms2020::Ddp.import_clearance_party(),
        Party::Seller
    );
    assert_eq!(
        Incoterms2020::Fob.import_clearance_party(),
        Party::Buyer
    );
    assert_eq!(
        Incoterms2020::Exw.import_clearance_party(),
        Party::Buyer
    );
}