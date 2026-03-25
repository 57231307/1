// P1/P2 模块统一导出
// 创建时间：2026-03-15

// P1 模块
pub mod fixed_asset;
pub mod purchase_contract;
pub mod sales_contract;
pub mod customer_credit;
pub mod fund_management;
pub mod budget_management;
pub mod quality_standard;

// P2 模块
pub mod financial_analysis;
pub mod supplier_evaluation;
pub mod purchase_price;
pub mod sales_price;
pub mod sales_analysis;
pub mod quality_inspection;

// P1 模块导出
pub use fixed_asset::Entity as FixedAsset;
pub use purchase_contract::Entity as PurchaseContract;
pub use sales_contract::Entity as SalesContract;
pub use customer_credit::Entity as CustomerCredit;
pub use fund_management::Entity as FundAccount;
pub use budget_management::Entity as BudgetItem;
pub use quality_standard::Entity as QualityStandard;

// P2 模块导出
pub use financial_analysis::Entity as FinancialIndicator;
pub use supplier_evaluation::Entity as SupplierEvaluationIndicator;
pub use purchase_price::Entity as PurchasePrice;
pub use sales_price::Entity as SalesPrice;
pub use sales_analysis::Entity as SalesStatistic;
pub use quality_inspection::Entity as QualityInspectionStandard;
