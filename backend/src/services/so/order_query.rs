//! P9-2 销售订单查询子模块（占位）
//!
//! 拆分自原 `services/so/order.rs`。
//!
//! ## 模块职责
//! - 销售订单分页查询（含客户、日期、状态等过滤）
//! - 销售订单统计（按客户/产品/月份）
//! - 销售订单导出（Excel/CSV）
//!
//! ## API 兼容
//! 通过 `crate::services::so::order` 路径访问。

#[allow(unused_imports)]
pub use crate::services::so::order::SalesService;

/// P9-2 标记：销售订单查询子模块路径
pub const P92_QRY_MODULE: &str = "sales_order_query";

/// 销售订单查询条件
#[derive(Debug, Clone, Default)]
pub struct OrderQuery {
    /// 客户 ID
    pub customer_id: Option<i32>,
    /// 订单状态
    pub status: Option<String>,
    /// 起始日期
    pub date_from: Option<chrono::NaiveDate>,
    /// 截止日期
    pub date_to: Option<chrono::NaiveDate>,
    /// 关键字
    pub keyword: Option<String>,
}

impl OrderQuery {
    /// 是否为空查询
    pub fn is_empty(&self) -> bool {
        self.customer_id.is_none()
            && self.status.is_none()
            && self.date_from.is_none()
            && self.date_to.is_none()
            && self.keyword.is_none()
    }

    /// 中文描述
    pub fn desc(&self) -> String {
        let mut parts = Vec::new();
        if let Some(cid) = self.customer_id {
            parts.push(format!("客户ID={cid}"));
        }
        if let Some(s) = &self.status {
            parts.push(format!("状态={s}"));
        }
        if let Some(d) = self.date_from {
            parts.push(format!("从 {d}"));
        }
        if let Some(d) = self.date_to {
            parts.push(format!("至 {d}"));
        }
        if let Some(k) = &self.keyword {
            parts.push(format!("关键字={k}"));
        }
        if parts.is_empty() {
            "无过滤条件".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_query_is_empty() {
        let q = OrderQuery::default();
        assert!(q.is_empty());
        assert_eq!(q.desc(), "无过滤条件");
    }

    #[test]
    fn test_order_query_with_filters() {
        let q = OrderQuery {
            customer_id: Some(100),
            status: Some("approved".to_string()),
            ..Default::default()
        };
        assert!(!q.is_empty());
        assert!(q.desc().contains("客户ID=100"));
        assert!(q.desc().contains("状态=approved"));
    }

    #[test]
    fn test_query_module_loaded() {
        assert_eq!(P92_QRY_MODULE, "sales_order_query");
    }
}
