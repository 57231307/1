//! Incoterms 2020 贸易术语工具
//!
//! V15 P0-B14（Batch 482）：补齐 11 种 Incoterms 2020 标准术语。
//! 原仅 5 种（FOB/CIF/EXW/DDP/DAP），新增 6 种（FCA/CPT/CIP/DPU/FAS/CFR）。
//! 补齐后覆盖集装箱贸易（FCA）、空运/快递（CPT/CIP）、海运（CFR/FAS）、
//! 卸货场景（DPU）等全部贸易场景，避免术语误用引发国际贸易纠纷。
//!
//! 接入点：quotation_service.validate_price_terms（创建/更新报价单时校验+日志记录业务元数据）

use serde::{Deserialize, Serialize};

/// Incoterms 2020 贸易术语枚举（11 种全量）
///
/// 按 Incoterms 2020 官方分类（按适用运输方式）：
/// - 任意运输方式（含多式联运）：EXW / FCA / CPT / CIP / DAP / DPU / DDP
/// - 海运/内河运输：FAS / FOB / CFR / CIF
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Incoterms2020 {
    // ===== 任意运输方式 =====
    /// 工厂交货（Ex Works）— 买方承担几乎所有费用和风险
    Exw,
    /// 货交承运人（Free Carrier）— 集装箱贸易最常用，替代 FOB 用于集装箱
    Fca,
    /// 运费付至（Carriage Paid To）— 卖方付运费到目的地，风险在货交第一承运人时转移
    Cpt,
    /// 运费+保险付至（Carriage and Insurance Paid To）— 同 CPT 但卖方另付保险
    Cip,
    /// 目的地交货（Delivered At Place）— 卖方付运费到目的地，不含卸货不含关税
    Dap,
    /// 目的地卸货交货（Delivered at Place Unloaded）— 同 DAP 但卖方负责卸货
    Dpu,
    /// 完税后交货（Delivered Duty Paid）— 卖方承担所有费用包括关税
    Ddp,
    // ===== 海运/内河运输 =====
    /// 船边交货（Free Alongside Ship）— 卖方将货物置于船边即完成交货
    Fas,
    /// 装运港船上交货（Free On Board）— 卖方承担装船前费用和风险
    Fob,
    /// 成本加运费（Cost and Freight）— 同 CIF 但不含保险
    Cfr,
    /// 成本+保险+运费（Cost, Insurance and Freight）— 卖方承担到目的港的运费和保险
    Cif,
}

impl Incoterms2020 {
    /// 从字符串解析（大小写不敏感）
    pub fn from_code(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "EXW" => Ok(Incoterms2020::Exw),
            "FCA" => Ok(Incoterms2020::Fca),
            "CPT" => Ok(Incoterms2020::Cpt),
            "CIP" => Ok(Incoterms2020::Cip),
            "DAP" => Ok(Incoterms2020::Dap),
            "DPU" => Ok(Incoterms2020::Dpu),
            "DDP" => Ok(Incoterms2020::Ddp),
            "FAS" => Ok(Incoterms2020::Fas),
            "FOB" => Ok(Incoterms2020::Fob),
            "CFR" => Ok(Incoterms2020::Cfr),
            "CIF" => Ok(Incoterms2020::Cif),
            _ => Err(format!("不支持的 Incoterms 代码: {}", s)),
        }
    }

    /// 返回术语的代码字符串（大写）
    pub fn code(&self) -> &'static str {
        match self {
            Incoterms2020::Exw => "EXW",
            Incoterms2020::Fca => "FCA",
            Incoterms2020::Cpt => "CPT",
            Incoterms2020::Cip => "CIP",
            Incoterms2020::Dap => "DAP",
            Incoterms2020::Dpu => "DPU",
            Incoterms2020::Ddp => "DDP",
            Incoterms2020::Fas => "FAS",
            Incoterms2020::Fob => "FOB",
            Incoterms2020::Cfr => "CFR",
            Incoterms2020::Cif => "CIF",
        }
    }

    /// 是否包含保险
    ///
    /// Incoterms 2020 包含保险的术语：CIF / CIP / DDP
    /// - CIF/CIP：卖方为买方投保运输保险（强制）
    /// - DDP：卖方承担所有费用含保险（虽未强制投保但实务默认包含）
    pub fn includes_insurance(&self) -> bool {
        matches!(
            self,
            Incoterms2020::Cif | Incoterms2020::Cip | Incoterms2020::Ddp
        )
    }

    /// 是否包含运费
    ///
    /// Incoterms 2020 不含运费的术语：EXW / FCA / FAS（卖方仅负责交付到指定地点或船边，不支付主运费）
    /// 其他术语均含运费
    pub fn includes_freight(&self) -> bool {
        !matches!(self, Incoterms2020::Exw | Incoterms2020::Fca | Incoterms2020::Fas)
    }

    /// 是否需要卖方支付关税（仅 DDP）
    pub fn requires_duty_paid(&self) -> bool {
        matches!(self, Incoterms2020::Ddp)
    }

    /// 中文业务描述
    pub fn description(&self) -> &'static str {
        match self {
            Incoterms2020::Exw => "工厂交货（买方承担几乎所有费用和风险）",
            Incoterms2020::Fca => "货交承运人（集装箱贸易最常用，卖方在指定地点将货物交付给买方指定的承运人）",
            Incoterms2020::Cpt => "运费付至（卖方支付运费到目的地，风险在货交第一承运人时转移给买方）",
            Incoterms2020::Cip => "运费+保险付至（同 CPT，卖方另付保险，常用于空运/快递）",
            Incoterms2020::Dap => "目的地交货（卖方承担运费到目的地，不含卸货不含关税）",
            Incoterms2020::Dpu => "目的地卸货交货（同 DAP，但卖方负责卸货，唯一要求卖方卸货的术语）",
            Incoterms2020::Ddp => "完税后交货（卖方承担所有费用包括关税，卖方责任最大的术语）",
            Incoterms2020::Fas => "船边交货（卖方将货物置于船边即完成交货，海运专用）",
            Incoterms2020::Fob => "装运港船上交货（卖方承担装船前费用和风险，海运专用）",
            Incoterms2020::Cfr => "成本加运费（同 CIF 但不含保险，海运专用）",
            Incoterms2020::Cif => "成本+保险+运费（卖方承担到目的港的运费和保险，海运专用）",
        }
    }

    /// 风险转移点描述（Incoterms 2020 关键合规字段）
    ///
    /// 用于报价单 PDF 显示，明确买卖双方风险划分点
    pub fn risk_transfer_point(&self) -> &'static str {
        match self {
            Incoterms2020::Exw => "卖方工厂（买方提货后风险归买方）",
            Incoterms2020::Fca => "货交承运人（指定地点交付承运人后风险归买方）",
            Incoterms2020::Cpt => "货交第一承运人（运费由卖方承担但风险已转移）",
            Incoterms2020::Cip => "货交第一承运人（运费+保险由卖方承担但风险已转移）",
            Incoterms2020::Dap => "目的地（货物到达目的地准备好卸货时风险归买方）",
            Incoterms2020::Dpu => "目的地卸货后（卖方卸货完成后风险归买方）",
            Incoterms2020::Ddp => "目的地（卖方承担到买方收货的所有风险）",
            Incoterms2020::Fas => "装运港船边（货物置于船边后风险归买方）",
            Incoterms2020::Fob => "装运港船上（货物越过船舷后风险归买方）",
            Incoterms2020::Cfr => "装运港船上（运费由卖方承担但风险在装运港转移）",
            Incoterms2020::Cif => "装运港船上（运费+保险由卖方承担但风险在装运港转移）",
        }
    }

    /// 是否仅适用海运/内河运输
    ///
    /// FAS / FOB / CFR / CIF 仅适用海运；其他术语可适用任意运输方式
    pub fn is_sea_only(&self) -> bool {
        matches!(
            self,
            Incoterms2020::Fas | Incoterms2020::Fob | Incoterms2020::Cfr | Incoterms2020::Cif
        )
    }

    /// 返回所有支持的术语（11 种全量）
    pub fn all() -> [Incoterms2020; 11] {
        [
            Incoterms2020::Exw,
            Incoterms2020::Fca,
            Incoterms2020::Cpt,
            Incoterms2020::Cip,
            Incoterms2020::Dap,
            Incoterms2020::Dpu,
            Incoterms2020::Ddp,
            Incoterms2020::Fas,
            Incoterms2020::Fob,
            Incoterms2020::Cfr,
            Incoterms2020::Cif,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
