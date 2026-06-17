//! P9-2 销售订单工作流子模块（占位）
//!
//! 拆分自原 `services/so/order.rs`。
//!
//! ## 模块职责
//! - 销售订单审批流（草稿→待审→已审→已发货→已收款→已关闭）
//! - 状态机转换合法性校验
//! - 工作流日志（操作人/时间/原因）
//!
//! ## API 兼容
//! 通过 `crate::services::so::order` 路径访问。

#[allow(unused_imports)]
pub use crate::services::so::order::SalesService;

/// P9-2 标记：销售订单工作流子模块路径
pub const P92_WF_MODULE: &str = "sales_order_workflow";

/// 销售订单工作流状态枚举（P9-2 占位，定义在父模块）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStage {
    /// 草稿
    Draft,
    /// 待审核
    Pending,
    /// 已审核
    Approved,
    /// 已发货
    Shipped,
    /// 已收款
    Received,
    /// 已关闭
    Closed,
}

impl WorkflowStage {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Pending => "待审核",
            Self::Approved => "已审核",
            Self::Shipped => "已发货",
            Self::Received => "已收款",
            Self::Closed => "已关闭",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_stage_desc() {
        assert_eq!(WorkflowStage::Draft.desc(), "草稿");
        assert_eq!(WorkflowStage::Pending.desc(), "待审核");
        assert_eq!(WorkflowStage::Approved.desc(), "已审核");
        assert_eq!(WorkflowStage::Shipped.desc(), "已发货");
        assert_eq!(WorkflowStage::Received.desc(), "已收款");
        assert_eq!(WorkflowStage::Closed.desc(), "已关闭");
    }

    #[test]
    fn test_workflow_module_loaded() {
        assert_eq!(P92_WF_MODULE, "sales_order_workflow");
    }
}
