//! DocType 与 SearchQuery 业务方法实现（构造器留在 facade）
use crate::search::elastic::{indices, DocType, SearchQuery};

impl DocType {
    /// 返回索引名
    pub fn index(&self) -> &'static str {
        match self {
            Self::SalesOrder => indices::SALES_ORDERS,
            Self::Customer => indices::CUSTOMERS,
            Self::Product => indices::PRODUCTS,
        }
    }

    /// 返回中文描述
    pub fn desc_zh(&self) -> &'static str {
        match self {
            Self::SalesOrder => "销售订单",
            Self::Customer => "客户",
            Self::Product => "产品",
        }
    }
}

impl SearchQuery {
    /// 设置关键字
    pub fn with_keyword(mut self, q: impl Into<String>) -> Self {
        self.q = Some(q.into());
        self
    }

    /// 添加精确过滤条件
    pub fn with_filter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.insert(key.into(), value.into());
        self
    }

    /// 设置分页
    pub fn with_pagination(mut self, from: i64, size: i64) -> Self {
        self.from = from;
        self.size = size;
        self
    }

    /// 启用高亮
    pub fn with_highlight(mut self) -> Self {
        self.highlight = true;
        self
    }
}
