//! 定制订单工艺流程状态机
//!
//! V15 P0-B11（Batch 483）：补齐打样和报价环节
//! 7 阶段工艺流程：draft → lab_dip → quotation → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed
//! 设计依据：V15 审计报告 batch-19 §23.2 缺陷 1 + docs/superpowers/specs/2026-06-16-custom-order-design.md §3.3
//! 创建时间: 2026-06-17

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// 定制订单状态枚举（7 阶段工艺 + 终态）
///
/// V15 P0-B11：新增 `LabDip`（打样中）和 `Quotation`（报价中）两个状态，
/// 插入在 `Draft` 和 `YarnPurchasing` 之间，强制定制订单走"打样→报价→生产"完整流程。
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
    /// 序列化为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::LabDip => "lab_dip",
            Self::Quotation => "quotation",
            Self::YarnPurchasing => "yarn_purchasing",
            Self::Dyeing => "dyeing",
            Self::Finishing => "finishing",
            Self::Delivery => "delivery",
            Self::AfterSales => "after_sales",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
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
        match s {
            "draft" => Ok(Self::Draft),
            "lab_dip" => Ok(Self::LabDip),
            "quotation" => Ok(Self::Quotation),
            "yarn_purchasing" => Ok(Self::YarnPurchasing),
            "dyeing" => Ok(Self::Dyeing),
            "finishing" => Ok(Self::Finishing),
            "delivery" => Ok(Self::Delivery),
            "after_sales" => Ok(Self::AfterSales),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
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

/// 状态机推进：返回下一状态
///
/// V15 P0-B11 转换规则（7 阶段）：
/// - draft → lab_dip（进入打样阶段）
/// - lab_dip → quotation（打样确认后进入报价阶段）
/// - quotation → yarn_purchasing（报价确认后进入生产阶段）
/// - yarn_purchasing → dyeing
/// - dyeing → finishing
/// - finishing → delivery
/// - delivery → after_sales
/// - after_sales → completed
/// - 任意非终态 → cancelled
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
        return true;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_status_normal_progression() {
        // P9-1: 用 match 处理 Result，失败时立即 panic 并说明 P9-1
        let unwrap_p9 = |res: Result<CustomOrderStatus, StateMachineError>, ctx: &str| -> CustomOrderStatus {
            match res {
                Ok(s) => s,
                Err(e) => panic!("P9-1: 测试夹具 {ctx} 状态机返回错误: {e}"),
            }
        };
        // V15 P0-B11：新增 lab_dip / quotation 状态，draft → lab_dip → quotation → yarn_purchasing
        assert_eq!(unwrap_p9(next_status("draft"), "draft"), CustomOrderStatus::LabDip);
        assert_eq!(unwrap_p9(next_status("lab_dip"), "lab_dip"), CustomOrderStatus::Quotation);
        assert_eq!(unwrap_p9(next_status("quotation"), "quotation"), CustomOrderStatus::YarnPurchasing);
        assert_eq!(unwrap_p9(next_status("yarn_purchasing"), "yarn_purchasing"), CustomOrderStatus::Dyeing);
        assert_eq!(unwrap_p9(next_status("dyeing"), "dyeing"), CustomOrderStatus::Finishing);
        assert_eq!(unwrap_p9(next_status("finishing"), "finishing"), CustomOrderStatus::Delivery);
        assert_eq!(unwrap_p9(next_status("delivery"), "delivery"), CustomOrderStatus::AfterSales);
        assert_eq!(unwrap_p9(next_status("after_sales"), "after_sales"), CustomOrderStatus::Completed);
    }

    #[test]
    fn test_next_status_terminal_fails() {
        assert!(next_status("completed").is_err());
        assert!(next_status("cancelled").is_err());
    }

    #[test]
    fn test_next_status_invalid_string() {
        assert!(next_status("invalid_state").is_err());
    }

    #[test]
    fn test_can_transition_normal() {
        // V15 P0-B11：7 阶段工艺流程
        assert!(can_transition("draft", "lab_dip"));
        assert!(can_transition("lab_dip", "quotation"));
        assert!(can_transition("quotation", "yarn_purchasing"));
        assert!(can_transition("yarn_purchasing", "dyeing"));
        assert!(can_transition("dyeing", "finishing"));
        assert!(can_transition("finishing", "delivery"));
        assert!(can_transition("delivery", "after_sales"));
        assert!(can_transition("after_sales", "completed"));
    }

    #[test]
    fn test_can_transition_to_cancelled() {
        assert!(can_transition("draft", "cancelled"));
        assert!(can_transition("lab_dip", "cancelled"));
        assert!(can_transition("quotation", "cancelled"));
        assert!(can_transition("yarn_purchasing", "cancelled"));
        assert!(can_transition("delivery", "cancelled"));
    }

    #[test]
    fn test_cannot_transition_terminal() {
        assert!(!can_transition("completed", "draft"));
        assert!(!can_transition("cancelled", "yarn_purchasing"));
    }

    #[test]
    fn test_cannot_skip_stages() {
        // V15 P0-B11：禁止跳过打样/报价阶段
        assert!(!can_transition("draft", "yarn_purchasing"));
        assert!(!can_transition("draft", "quotation"));
        assert!(!can_transition("draft", "dyeing"));
        assert!(!can_transition("draft", "delivery"));
        assert!(!can_transition("lab_dip", "yarn_purchasing"));
        assert!(!can_transition("yarn_purchasing", "finishing"));
    }

    #[test]
    fn test_is_terminal() {
        assert!(CustomOrderStatus::Completed.is_terminal());
        assert!(CustomOrderStatus::Cancelled.is_terminal());
        assert!(!CustomOrderStatus::Draft.is_terminal());
        assert!(!CustomOrderStatus::Delivery.is_terminal());
    }

    #[test]
    fn test_default_process_nodes() {
        let nodes = default_process_nodes();
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[0].0, "yarn_purchasing");
        assert_eq!(nodes[4].0, "after_sales");
    }
}
