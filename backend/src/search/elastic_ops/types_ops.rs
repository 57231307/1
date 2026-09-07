//! DocType 与 SearchQuery 业务方法实现（构造器留在 facade）
use crate::search::elastic::{DocType, SearchQuery, indices};

impl DocType {
    /// 返回索引名
    pub fn index(&self) -> &'static str {
        match self {
            Self::SalesOrder => indices::SALES_ORDERS,
            Self::Customer => indices::CUSTOMERS,
            Self::Product => indices::PRODUCTS,
            Self::Supplier => indices::SUPPLIERS,
            Self::Warehouse => indices::WAREHOUSES,
            Self::PurchaseOrder => indices::PURCHASE_ORDERS,
            Self::InventoryBatch => indices::INVENTORY_BATCHES,
            Self::ProductionOrder => indices::PRODUCTION_ORDERS,
            Self::DyeBatch => indices::DYE_BATCHES,
            Self::DyeRecipe => indices::DYE_RECIPES,
            Self::ColorCard => indices::COLOR_CARDS,
            Self::ColorPrice => indices::COLOR_PRICES,
            Self::Bom => indices::BOMS,
            Self::CustomOrder => indices::CUSTOM_ORDERS,
            Self::Voucher => indices::VOUCHERS,
            Self::ArInvoice => indices::AR_INVOICES,
            Self::ApInvoice => indices::AP_INVOICES,
            Self::SalesContract => indices::SALES_CONTRACTS,
            Self::PurchaseContract => indices::PURCHASE_CONTRACTS,
            Self::AccountingPeriod => indices::ACCOUNTING_PERIODS,
        }
    }

    /// 返回中文描述
    pub fn desc_zh(&self) -> &'static str {
        match self {
            Self::SalesOrder => "销售订单",
            Self::Customer => "客户",
            Self::Product => "产品",
            Self::Supplier => "供应商",
            Self::Warehouse => "仓库",
            Self::PurchaseOrder => "采购订单",
            Self::InventoryBatch => "库存批次",
            Self::ProductionOrder => "生产工单",
            Self::DyeBatch => "染色批次",
            Self::DyeRecipe => "染色配方",
            Self::ColorCard => "色卡",
            Self::ColorPrice => "色号价格",
            Self::Bom => "BOM 清单",
            Self::CustomOrder => "定制订单",
            Self::Voucher => "财务凭证",
            Self::ArInvoice => "应收发票",
            Self::ApInvoice => "应付发票",
            Self::SalesContract => "销售合同",
            Self::PurchaseContract => "采购合同",
            Self::AccountingPeriod => "会计期间",
        }
    }

    /// 全部文档类型（/search/doc-types 公共 API 用）
    pub fn all() -> Vec<Self> {
        vec![
            Self::SalesOrder,
            Self::Customer,
            Self::Product,
            Self::Supplier,
            Self::Warehouse,
            Self::PurchaseOrder,
            Self::InventoryBatch,
            Self::ProductionOrder,
            Self::DyeBatch,
            Self::DyeRecipe,
            Self::ColorCard,
            Self::ColorPrice,
            Self::Bom,
            Self::CustomOrder,
            Self::Voucher,
            Self::ArInvoice,
            Self::ApInvoice,
            Self::SalesContract,
            Self::PurchaseContract,
            Self::AccountingPeriod,
        ]
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
