//! Incoterms 2020 贸易术语工具
//!
//! 提供 5 种主要 Incoterms 代码的解析与业务含义查询。
//! 当前为销售报价单模块预留 API，尚未被业务引用。
//! Week 2 任务 6 - 销售报价单模块
//! 创建时间: 2026-06-16

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
#[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
pub enum Incoterms2020 {
    /// 装运港船上交货（Free On Board）
    FOB,
    /// 成本+保险+运费（Cost, Insurance and Freight）
    CIF,
    /// 工厂交货（Ex Works）
    EXW,
    /// 完税后交货（Delivered Duty Paid）
    DDP,
    /// 目的地交货（Delivered At Place）
    DAP,
}

impl Incoterms2020 {
    /// 从字符串解析
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn from_code(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "FOB" => Ok(Incoterms2020::FOB),
            "CIF" => Ok(Incoterms2020::CIF),
            "EXW" => Ok(Incoterms2020::EXW),
            "DDP" => Ok(Incoterms2020::DDP),
            "DAP" => Ok(Incoterms2020::DAP),
            _ => Err(format!("不支持的 Incoterms 代码: {}", s)),
        }
    }

    /// 返回术语的代码字符串
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn code(&self) -> &'static str {
        match self {
            Incoterms2020::FOB => "FOB",
            Incoterms2020::CIF => "CIF",
            Incoterms2020::EXW => "EXW",
            Incoterms2020::DDP => "DDP",
            Incoterms2020::DAP => "DAP",
        }
    }

    /// 是否包含保险（CIF / DDP 包含）
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn includes_insurance(&self) -> bool {
        matches!(self, Incoterms2020::CIF | Incoterms2020::DDP)
    }

    /// 是否包含运费（EXW 不包含，其他都包含）
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn includes_freight(&self) -> bool {
        !matches!(self, Incoterms2020::EXW)
    }

    /// 是否需要卖方支付关税（仅 DDP）
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn requires_duty_paid(&self) -> bool {
        matches!(self, Incoterms2020::DDP)
    }

    /// 中文业务描述
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn description(&self) -> &'static str {
        match self {
            Incoterms2020::FOB => "装运港船上交货（卖方承担装船前费用和风险）",
            Incoterms2020::CIF => "成本+保险+运费（卖方承担到目的港的运费和保险）",
            Incoterms2020::EXW => "工厂交货（买方承担几乎所有费用和风险）",
            Incoterms2020::DDP => "完税后交货（卖方承担所有费用包括关税）",
            Incoterms2020::DAP => "目的地交货（卖方承担运费但不含关税）",
        }
    }

    /// 返回所有支持的术语
    #[allow(dead_code)] // TODO(tech-debt): 销售报价单模块接入后移除
    pub fn all() -> [Incoterms2020; 5] {
        [
            Incoterms2020::FOB,
            Incoterms2020::CIF,
            Incoterms2020::EXW,
            Incoterms2020::DDP,
            Incoterms2020::DAP,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_code_uppercase() {
        assert_eq!(Incoterms2020::from_code("fob").unwrap(), Incoterms2020::FOB);
        assert_eq!(Incoterms2020::from_code("FOB").unwrap(), Incoterms2020::FOB);
        assert_eq!(Incoterms2020::from_code("CIF").unwrap(), Incoterms2020::CIF);
    }

    #[test]
    fn test_from_code_invalid() {
        assert!(Incoterms2020::from_code("ABC").is_err());
        assert!(Incoterms2020::from_code("").is_err());
    }

    #[test]
    fn test_includes_insurance() {
        assert!(Incoterms2020::CIF.includes_insurance());
        assert!(Incoterms2020::DDP.includes_insurance());
        assert!(!Incoterms2020::FOB.includes_insurance());
        assert!(!Incoterms2020::EXW.includes_insurance());
    }

    #[test]
    fn test_includes_freight() {
        assert!(!Incoterms2020::EXW.includes_freight());
        assert!(Incoterms2020::FOB.includes_freight());
        assert!(Incoterms2020::CIF.includes_freight());
        assert!(Incoterms2020::DDP.includes_freight());
        assert!(Incoterms2020::DAP.includes_freight());
    }

    #[test]
    fn test_requires_duty_paid() {
        assert!(Incoterms2020::DDP.requires_duty_paid());
        assert!(!Incoterms2020::CIF.requires_duty_paid());
        assert!(!Incoterms2020::FOB.requires_duty_paid());
    }

    #[test]
    fn test_description_not_empty() {
        for term in Incoterms2020::all() {
            assert!(!term.description().is_empty());
        }
    }
}
