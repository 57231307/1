//! 定制订单工艺流程状态机
//!
//! V15 P0-B11（Batch 483）：补齐打样和报价环节
//! 7 阶段工艺流程：draft → lab_dip → quotation → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed
//! 设计依据：V15 审计报告 batch-19 §23.2 缺陷 1 + docs/superpowers/specs/2026-06-16-custom-order-design.md §3.3
//! 创建时间: 2026-06-17

use crate::models::status::custom_order as co_status;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// 定制订单状态枚举（7 阶段工艺 + 终态；V15 P0-B11 新增 LabDip/Quotation 插入 Draft 与 YarnPurchasing 之间，强制"打样→报价→生产"完整流程）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomOrderStatus {
    /// 草稿（初始状态）
    Draft,
    /// V15 P0-B11：打样中（关联 lab_dip_request，客户确认 OK 样后推进）
    LabDip,
    /// V15 P0-B11：报价中（关联 sales_quotation，报价审批通过后推进）
    Quotation,
    /// 纱线采购中
    YarnPurchasing,
    /// 染整中
    Dyeing,
    /// 后整理中
    Finishing,
    /// 交付中
    Delivery,
    /// 售后中
    AfterSales,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
}

impl CustomOrderStatus {
    /// 序列化为字符串（取值逐字符取自权威词表 `models/status/sales.rs::custom_order`，
    /// 禁止在本机内再写裸字面量，防止状态机与 DB CHECK/权威模块漂移）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => co_status::DRAFT,
            Self::LabDip => co_status::LAB_DIP,
            Self::Quotation => co_status::QUOTATION,
            Self::YarnPurchasing => co_status::YARN_PURCHASING,
            Self::Dyeing => co_status::DYEING,
            Self::Finishing => co_status::FINISHING,
            Self::Delivery => co_status::DELIVERY,
            Self::AfterSales => co_status::AFTER_SALES,
            Self::Completed => co_status::COMPLETED,
            Self::Cancelled => co_status::CANCELLED,
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

/// 实现 FromStr trait，替代原 inherent from_str 方法（消除 clippy::should_implement_trait 警告）
impl FromStr for CustomOrderStatus {
    type Err = StateMachineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 模式同样取自权威词表常量（&str const 可作 match 模式），
        // 保证解析域与写出域永远同源，含词表外 token（如 change_pending）
        // 一律 InvalidState 显式拒绝，不做静默兜底
        match s {
            co_status::DRAFT => Ok(Self::Draft),
            co_status::LAB_DIP => Ok(Self::LabDip),
            co_status::QUOTATION => Ok(Self::Quotation),
            co_status::YARN_PURCHASING => Ok(Self::YarnPurchasing),
            co_status::DYEING => Ok(Self::Dyeing),
            co_status::FINISHING => Ok(Self::Finishing),
            co_status::DELIVERY => Ok(Self::Delivery),
            co_status::AFTER_SALES => Ok(Self::AfterSales),
            co_status::COMPLETED => Ok(Self::Completed),
            co_status::CANCELLED => Ok(Self::Cancelled),
            _ => Err(StateMachineError::InvalidState(s.to_string())),
        }
    }
}

/// 状态机错误
#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("非法状态: {0}")]
    InvalidState(String),
    #[error("状态转换非法: {from} → {to}")]
    InvalidTransition { from: String, to: String },
}

/// 状态机推进：返回下一状态（V15 P0-B11 7 阶段转换：draft→lab_dip→quotation→yarn_purchasing→dyeing→finishing→delivery→after_sales→completed，任意非终态→cancelled）
pub fn next_status(current: &str) -> Result<CustomOrderStatus, StateMachineError> {
    let cur = current.parse::<CustomOrderStatus>()?;

    if cur.is_terminal() {
        return Err(StateMachineError::InvalidTransition {
            from: current.to_string(),
            to: "next".to_string(),
        });
    }

    let next = match cur {
        CustomOrderStatus::Draft => CustomOrderStatus::LabDip,
        CustomOrderStatus::LabDip => CustomOrderStatus::Quotation,
        CustomOrderStatus::Quotation => CustomOrderStatus::YarnPurchasing,
        CustomOrderStatus::YarnPurchasing => CustomOrderStatus::Dyeing,
        CustomOrderStatus::Dyeing => CustomOrderStatus::Finishing,
        CustomOrderStatus::Finishing => CustomOrderStatus::Delivery,
        CustomOrderStatus::Delivery => CustomOrderStatus::AfterSales,
        CustomOrderStatus::AfterSales => CustomOrderStatus::Completed,
        // Cancelled 不会到达此处（is_terminal 已拦截）
        CustomOrderStatus::Completed | CustomOrderStatus::Cancelled => {
            return Err(StateMachineError::InvalidTransition {
                from: current.to_string(),
                to: "next".to_string(),
            });
        }
    };

    Ok(next)
}

/// 验证状态转换是否合法
pub fn can_transition(from: &str, to: &str) -> bool {
    let from_status = match from.parse::<CustomOrderStatus>() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let to_status = match to.parse::<CustomOrderStatus>() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if from_status == to_status {
        return !from_status.is_terminal();
    }

    if to_status == CustomOrderStatus::Cancelled && !from_status.is_terminal() {
        return true;
    }

    next_status(from).map(|n| n == to_status).unwrap_or(false)
}

/// 5 阶段工艺节点定义（用于自动生成节点）
pub fn default_process_nodes() -> Vec<(&'static str, &'static str, i32)> {
    vec![
        ("yarn_purchasing", "纱线采购", 1),
        ("dyeing", "染整", 2),
        ("finishing", "后整理", 3),
        ("delivery", "交付", 4),
        ("after_sales", "售后", 5),
    ]
}
