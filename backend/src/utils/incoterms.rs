//! Incoterms 2020 贸易术语工具
//!
//! 提供 5 种主要 Incoterms 代码的解析与业务含义查询。
//! 批次 111 P1-2：已接入 quotation_service.validate_price_terms（创建/更新报价单时校验+日志记录业务元数据）

use serde::{Deserialize, Serialize};

/// Incoterms 2020 贸易术语枚举
///
/// 面料行业常用的 5 种术语：
/// - FOB: 装运港船上交货
/// - CIF: 成本+保险+运费
/// - EXW: 工厂交货
/// - DDP: 完税后交货
/// - DAP: 目的地交货
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Incoterms2020 {
    /// 装运港船上交货（Free On Board）
    Fob,
    /// 成本+保险+运费（Cost, Insurance and Freight）
    Cif,
    /// 工厂交货（Ex Works）
    Exw,
    /// 完税后交货（Delivered Duty Paid）
    Ddp,
    /// 目的地交货（Delivered At Place）
    Dap,
}

impl Incoterms2020 {
    /// 从字符串解析
    pub fn from_code(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "FOB" => Ok(Incoterms2020::Fob),
            "CIF" => Ok(Incoterms2020::Cif),
            "EXW" => Ok(Incoterms2020::Exw),
            "DDP" => Ok(Incoterms2020::Ddp),
            "DAP" => Ok(Incoterms2020::Dap),
            _ => Err(format!("不支持的 Incoterms 代码: {}", s)),
        }
    }

    /// 返回术语的代码字符串
    pub fn code(&self) -> &'static str {
        match self {
            Incoterms2020::Fob => "FOB",
            Incoterms2020::Cif => "CIF",
            Incoterms2020::Exw => "EXW",
            Incoterms2020::Ddp => "DDP",
            Incoterms2020::Dap => "DAP",
        }
    }

    /// 是否包含保险（CIF / DDP 包含）
    pub fn includes_insurance(&self) -> bool {
        matches!(self, Incoterms2020::Cif | Incoterms2020::Ddp)
    }

    /// 是否包含运费（EXW 不包含，其他都包含）
    pub fn includes_freight(&self) -> bool {
        !matches!(self, Incoterms2020::Exw)
    }

    /// 是否需要卖方支付关税（仅 DDP）
    pub fn requires_duty_paid(&self) -> bool {
        matches!(self, Incoterms2020::Ddp)
    }

    /// 中文业务描述
    pub fn description(&self) -> &'static str {
        match self {
            Incoterms2020::Fob => "装运港船上交货（卖方承担装船前费用和风险）",
            Incoterms2020::Cif => "成本+保险+运费（卖方承担到目的港的运费和保险）",
            Incoterms2020::Exw => "工厂交货（买方承担几乎所有费用和风险）",
            Incoterms2020::Ddp => "完税后交货（卖方承担所有费用包括关税）",
            Incoterms2020::Dap => "目的地交货（卖方承担运费但不含关税）",
        }
    }

    /// 返回所有支持的术语
    pub fn all() -> [Incoterms2020; 5] {
        [
            Incoterms2020::Fob,
            Incoterms2020::Cif,
            Incoterms2020::Exw,
            Incoterms2020::Ddp,
            Incoterms2020::Dap,
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
    }

    #[test]
    fn test_from_code_invalid() {
        assert!(Incoterms2020::from_code("ABC").is_err());
        assert!(Incoterms2020::from_code("").is_err());
    }

    #[test]
    fn test_includes_insurance() {
        assert!(Incoterms2020::Cif.includes_insurance());
        assert!(Incoterms2020::Ddp.includes_insurance());
        assert!(!Incoterms2020::Fob.includes_insurance());
        assert!(!Incoterms2020::Exw.includes_insurance());
    }

    #[test]
    fn test_includes_freight() {
        assert!(!Incoterms2020::Exw.includes_freight());
        assert!(Incoterms2020::Fob.includes_freight());
        assert!(Incoterms2020::Cif.includes_freight());
        assert!(Incoterms2020::Ddp.includes_freight());
        assert!(Incoterms2020::Dap.includes_freight());
    }

    #[test]
    fn test_requires_duty_paid() {
        assert!(Incoterms2020::Ddp.requires_duty_paid());
        assert!(!Incoterms2020::Cif.requires_duty_paid());
        assert!(!Incoterms2020::Fob.requires_duty_paid());
    }

    #[test]
    fn test_description_not_empty() {
        for term in Incoterms2020::all() {
            assert!(!term.description().is_empty());
        }
    }
}
