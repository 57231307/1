//! 新增服务 gRPC 实现
//!
//! 包含辅助核算、供应商评估、五维查询、库存预留、财务分析等 gRPC 服务实现

pub mod support;
pub mod assist_accounting;
pub mod supplier_evaluation;
pub mod five_dimension_query;
pub mod inventory_reservation;
pub mod financial_analysis;

use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::services::assist_accounting_service::AssistAccountingService;
use crate::services::supplier_evaluation_service::SupplierEvaluationService;
use crate::services::five_dimension_query_service::FiveDimensionQueryService;
use crate::services::inventory_reservation_service::InventoryReservationService;
use crate::services::financial_analysis_service::FinancialAnalysisService;

/// gRPC 新增服务集合
#[derive(Clone)]
pub struct GrpcNewServices {
    assist_accounting_service: Arc<AssistAccountingService>,
    supplier_evaluation_service: Arc<SupplierEvaluationService>,
    #[allow(dead_code)]
    five_dimension_query_service: Arc<FiveDimensionQueryService>,
    inventory_reservation_service: Arc<InventoryReservationService>,
    financial_analysis_service: Arc<FinancialAnalysisService>,
}

impl GrpcNewServices {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            assist_accounting_service: Arc::new(AssistAccountingService::new(db.clone())),
            supplier_evaluation_service: Arc::new(SupplierEvaluationService::new(db.clone())),
            five_dimension_query_service: Arc::new(FiveDimensionQueryService::new()),
            inventory_reservation_service: Arc::new(InventoryReservationService::new(db.clone())),
            financial_analysis_service: Arc::new(FinancialAnalysisService::new(db)),
        }
    }
}
