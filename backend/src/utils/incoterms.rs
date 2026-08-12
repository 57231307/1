//! Incoterms 2020 贸易术语工具
//!
//! V15 P0-B14（Batch 482）：补齐 11 种 Incoterms 2020 标准术语。
//! 原仅 5 种（FOB/CIF/EXW/DDP/DAP），新增 6 种（FCA/CPT/CIP/DPU/FAS/CFR）。
//! 补齐后覆盖集装箱贸易（FCA）、空运/快递（CPT/CIP）、海运（CFR/FAS）、
//! 卸货场景（DPU）等全部贸易场景，避免术语误用引发国际贸易纠纷。
//!
//! 接入点：quotation_service.validate_price_terms（创建/更新报价单时校验+日志记录业务元数据）

use serde::{Deserialize, Serialize};

/// Incoterms 2020 贸易术语枚举（11 种全量，按适用运输方式分类）
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

/// 主费用承担方（结构化责任划分，V15 P2 23.5 缺陷3 修复：用于报价单/合同生成责任条款）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostBearer {
    /// 卖方承担主要费用
    Seller,
    /// 买方承担主要费用
    Buyer,
    /// 买卖双方按风险转移点共担
    Both,
}

/// 清关责任方（出口/进口清关，V15 P2 23.5 缺陷3 修复：明确买卖双方清关责任）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    /// 卖方
    Seller,
    /// 买方
    Buyer,
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

    /// 是否包含保险（CIF / CIP / DDP 强制或默认包含保险）
    pub fn includes_insurance(&self) -> bool {
        matches!(
            self,
            Incoterms2020::Cif | Incoterms2020::Cip | Incoterms2020::Ddp
        )
    }

    /// 是否包含运费（EXW / FCA / FAS 不含主运费，其他术语均含）
    pub fn includes_freight(&self) -> bool {
        !matches!(
            self,
            Incoterms2020::Exw | Incoterms2020::Fca | Incoterms2020::Fas
        )
    }

    /// 是否需要卖方支付关税（仅 DDP）
    pub fn requires_duty_paid(&self) -> bool {
        matches!(self, Incoterms2020::Ddp)
    }

    /// 中文业务描述
    pub fn description(&self) -> &'static str {
        match self {
            Incoterms2020::Exw => "工厂交货（买方承担几乎所有费用和风险）",
            Incoterms2020::Fca => {
                "货交承运人（集装箱贸易最常用，卖方在指定地点将货物交付给买方指定的承运人）"
            }
            Incoterms2020::Cpt => {
                "运费付至（卖方支付运费到目的地，风险在货交第一承运人时转移给买方）"
            }
            Incoterms2020::Cip => "运费+保险付至（同 CPT，卖方另付保险，常用于空运/快递）",
            Incoterms2020::Dap => "目的地交货（卖方承担运费到目的地，不含卸货不含关税）",
            Incoterms2020::Dpu => {
                "目的地卸货交货（同 DAP，但卖方负责卸货，唯一要求卖方卸货的术语）"
            }
            Incoterms2020::Ddp => "完税后交货（卖方承担所有费用包括关税，卖方责任最大的术语）",
            Incoterms2020::Fas => "船边交货（卖方将货物置于船边即完成交货，海运专用）",
            Incoterms2020::Fob => "装运港船上交货（卖方承担装船前费用和风险，海运专用）",
            Incoterms2020::Cfr => "成本加运费（同 CIF 但不含保险，海运专用）",
            Incoterms2020::Cif => "成本+保险+运费（卖方承担到目的港的运费和保险，海运专用）",
        }
    }

    /// 风险转移点描述（用于报价单 PDF 显示，明确买卖双方风险划分点）
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

    /// 是否仅适用海运/内河运输（FAS / FOB / CFR / CIF 仅海运，其他可任意运输方式）
    pub fn is_sea_only(&self) -> bool {
        matches!(
            self,
            Incoterms2020::Fas | Incoterms2020::Fob | Incoterms2020::Cfr | Incoterms2020::Cif
        )
    }

    /// 主费用承担方（结构化，V15 P2 23.5 缺陷3 修复）
    /// 规则：EXW=买方；DAP/DPU/DDP=卖方；FCA/FAS/FOB/CPT/CIP/CFR/CIF=共担（卖方承担装运/主运费，买方承担后续费用）
    pub fn cost_bearer(&self) -> CostBearer {
        match self {
            Incoterms2020::Exw => CostBearer::Buyer,
            Incoterms2020::Dap | Incoterms2020::Dpu | Incoterms2020::Ddp => CostBearer::Seller,
            Incoterms2020::Cpt
            | Incoterms2020::Cip
            | Incoterms2020::Cfr
            | Incoterms2020::Cif
            | Incoterms2020::Fca
            | Incoterms2020::Fas
            | Incoterms2020::Fob => CostBearer::Both,
        }
    }

    /// 出口清关责任方（V15 P2 23.5 缺陷3 修复：除 EXW 外卖方负责出口清关）
    pub fn export_clearance_party(&self) -> Party {
        match self {
            Incoterms2020::Exw => Party::Buyer,
            _ => Party::Seller,
        }
    }

    /// 进口清关责任方（V15 P2 23.5 缺陷3 修复：除 DDP 外买方负责进口清关）
    pub fn import_clearance_party(&self) -> Party {
        match self {
            Incoterms2020::Ddp => Party::Seller,
            _ => Party::Buyer,
        }
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
