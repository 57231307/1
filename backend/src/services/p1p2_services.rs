// P1/P2 模块 Service 层统一导出
// 创建时间：2026-03-15

pub mod financial_analysis_service;
pub mod supplier_evaluation_service;
pub mod purchase_price_service;
pub mod sales_price_service;
pub mod sales_analysis_service;
pub mod quality_inspection_service;
pub mod fund_management_service;
pub mod budget_management_service;
pub mod quality_standard_service;

pub use financial_analysis_service::FinancialAnalysisService;
pub use supplier_evaluation_service::SupplierEvaluationService;
