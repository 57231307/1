use rust_decimal::prelude::*;
// 新增服务 gRPC 实现
//
// 包含辅助核算、供应商评估、五维查询、库存预留、财务分析等 gRPC 服务实现

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::services::assist_accounting_service::AssistAccountingService;
use crate::services::financial_analysis_service::FinancialAnalysisService;
use crate::services::five_dimension_query_service::FiveDimensionQueryService;
use crate::services::inventory_reservation_service::InventoryReservationService;
use crate::services::supplier_evaluation_service::SupplierEvaluationService;

use crate::grpc::service::proto::{
    // 辅助核算服务
    assist_accounting_service_server::AssistAccountingService as AssistAccountingServiceTrait,
    // 财务分析服务
    financial_analysis_service_server::FinancialAnalysisService as FinancialAnalysisServiceTrait,
    // 五维查询服务
    five_dimension_query_service_server::FiveDimensionQueryService as FiveDimensionQueryServiceTrait,
    // 库存预留服务
    inventory_reservation_service_server::InventoryReservationService as InventoryReservationServiceTrait,
    // 供应商评估服务
    supplier_evaluation_service_server::SupplierEvaluationService as SupplierEvaluationServiceTrait,
    AssistAccountingDimension,

    AssistAccountingRecord,
    BatchCreateReservationsRequest,
    BatchCreateReservationsResponse,
    BatchLockReservationsRequest,
    BatchLockReservationsResponse,
    BatchReleaseReservationsRequest,
    BatchReleaseReservationsResponse,
    CreateAnalysisResultRequest,
    CreateAnalysisResultResponse,
    CreateAssistRecordRequest,
    CreateAssistRecordResponse,
    CreateEvaluationRecordRequest,
    CreateEvaluationRecordResponse,
    CreateFinancialIndicatorRequest,
    CreateFinancialIndicatorResponse,
    CreateIndicatorRequest,
    CreateIndicatorResponse,
    CreateReservationRequest,
    CreateReservationResponse,
    DeleteAssistRecordRequest,
    DeleteAssistRecordResponse,
    EvaluationIndicator,
    FinancialAnalysisResult,
    FinancialIndicator,
    FindByBusinessRequest,
    FindByBusinessResponse,
    FindByFiveDimensionRequest,
    FindByFiveDimensionResponse,
    FiveDimension,

    GenerateFiveDimensionIdRequest,
    GenerateFiveDimensionIdResponse,
    GenerateMonthlySummaryRequest,
    GenerateMonthlySummaryResponse,
    GetEvaluationRecordByIdRequest,
    GetEvaluationRecordByIdResponse,
    GetEvaluationRecordsRequest,
    GetEvaluationRecordsResponse,
    GetFinancialIndicatorsListRequest,
    GetFinancialIndicatorsListResponse,
    GetIndicatorsListRequest,
    GetIndicatorsListResponse,
    GetLockedReservationsByOrderRequest,
    GetLockedReservationsByOrderResponse,
    GetReservationsByOrderRequest,
    GetReservationsByOrderResponse,
    GetSupplierRankingsRequest,
    GetSupplierRankingsResponse,
    GetSupplierScoreRequest,
    GetSupplierScoreResponse,
    GetTrendsRequest,
    GetTrendsResponse,
    InitializeDimensionsRequest,
    InitializeDimensionsResponse,
    InventoryReservation,

    ListDimensionsRequest,
    ListDimensionsResponse,
    ListRatingsRequest,
    ListRatingsResponse,
    LockReservationRequest,
    LockReservationResponse,
    ParseFiveDimensionIdRequest,
    ParseFiveDimensionIdResponse,
    QueryAssistRecordsRequest,
    QueryAssistRecordsResponse,
    ReleaseReservationRequest,
    ReleaseReservationResponse,
    SupplierEvaluationRecord,
    SupplierScore,

    UseReservationRequest,
    UseReservationResponse,
};

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

    /// 将辅助核算记录转换为 gRPC 模型
    fn to_grpc_assist_record(
        record: crate::models::assist_accounting_record::Model,
    ) -> AssistAccountingRecord {
        AssistAccountingRecord {
            id: record.id,
            business_type: record.business_type,
            business_no: record.business_no,
            business_id: record.business_id,
            account_subject_id: record.account_subject_id,
            debit_amount: record.debit_amount.to_string(),
            credit_amount: record.credit_amount.to_string(),
            five_dimension_id: record.five_dimension_id,
            product_id: record.product_id,
            batch_no: record.batch_no,
            color_no: record.color_no,
            dye_lot_no: record.dye_lot_no.unwrap_or_default(),
            grade: record.grade,
            warehouse_id: record.warehouse_id,
            quantity_meters: record.quantity_meters.to_string(),
            quantity_kg: record.quantity_kg.to_string(),
            workshop_id: record.workshop_id.unwrap_or(0),
            customer_id: record.customer_id.unwrap_or(0),
            supplier_id: record.supplier_id.unwrap_or(0),
            remarks: record.remarks.unwrap_or_default(),
            created_at: record.created_at.timestamp(),
            created_by: record.created_by.unwrap_or(0),
        }
    }

    /// 将辅助核算维度转换为 gRPC 模型
    fn to_grpc_dimension(
        dimension: crate::models::assist_accounting_dimension::Model,
    ) -> AssistAccountingDimension {
        AssistAccountingDimension {
            id: dimension.id,
            dimension_code: dimension.dimension_code,
            dimension_name: dimension.dimension_name,
            description: dimension.description.unwrap_or_default(),
            is_active: dimension.is_active,
            sort_order: dimension.sort_order,
            created_at: dimension.created_at.timestamp(),
            updated_at: dimension.updated_at.timestamp(),
        }
    }

    /// 将评估指标转换为 gRPC 模型
    fn to_grpc_evaluation_indicator(
        indicator: crate::models::supplier_evaluation::Model,
    ) -> EvaluationIndicator {
        EvaluationIndicator {
            id: indicator.id,
            indicator_name: indicator.indicator_name,
            indicator_code: indicator.indicator_code,
            category: indicator.category,
            weight: indicator.weight.to_string(),
            max_score: indicator.max_score,
            evaluation_method: indicator.evaluation_method.unwrap_or_default(),
            status: indicator.status,
            created_at: indicator.created_at.timestamp(),
            updated_at: indicator.updated_at.timestamp(),
        }
    }

    /// 将评估记录转换为 gRPC 模型
    fn to_grpc_evaluation_record(
        record: crate::models::supplier_evaluation_record::Model,
    ) -> SupplierEvaluationRecord {
        SupplierEvaluationRecord {
            id: record.id,
            supplier_id: record.supplier_id,
            evaluation_period: record.evaluation_period,
            indicator_id: record.indicator_id,
            score: record.score.to_string(),
            max_score: record.max_score.unwrap_or(0),
            weighted_score: record
                .weighted_score
                .map(|s| s.to_string())
                .unwrap_or_default(),
            evaluator_id: record.evaluator_id.unwrap_or(0),
            evaluation_date: record
                .evaluation_date
                .map(|d| d.to_string())
                .unwrap_or_default(),
            remark: record.remark.unwrap_or_default(),
            created_at: record.created_at.timestamp(),
        }
    }

    /// 将库存预留转换为 gRPC 模型
    fn to_grpc_reservation(
        reservation: crate::models::inventory_reservation::Model,
    ) -> InventoryReservation {
        InventoryReservation {
            id: reservation.id,
            order_id: reservation.order_id,
            product_id: reservation.product_id,
            warehouse_id: reservation.warehouse_id,
            quantity: reservation.quantity.to_string(),
            status: reservation.status,
            reserved_at: reservation.reserved_at.timestamp(),
            released_at: reservation.released_at.map(|t| t.timestamp()).unwrap_or(0),
            notes: reservation.notes.unwrap_or_default(),
            created_by: reservation.created_by.unwrap_or(0),
            created_at: reservation.created_at.timestamp(),
            updated_at: reservation.updated_at.timestamp(),
        }
    }

    /// 将财务指标转换为 gRPC 模型
    fn to_grpc_financial_indicator(
        indicator: crate::models::financial_analysis::Model,
    ) -> FinancialIndicator {
        FinancialIndicator {
            id: indicator.id,
            indicator_name: indicator.indicator_name,
            indicator_code: indicator.indicator_code,
            indicator_type: indicator.indicator_type,
            formula: indicator.formula.unwrap_or_default(),
            unit: indicator.unit.unwrap_or_default(),
            status: indicator.status,
            remark: indicator.remark.unwrap_or_default(),
            created_at: indicator.created_at.timestamp(),
            updated_at: indicator.updated_at.timestamp(),
        }
    }

    /// 将财务分析结果转换为 gRPC 模型
    fn to_grpc_analysis_result(
        result: crate::models::financial_analysis_result::Model,
    ) -> FinancialAnalysisResult {
        FinancialAnalysisResult {
            id: result.id,
            analysis_type: result.analysis_type,
            period: result.period,
            indicator_id: result.indicator_id,
            indicator_value: result.indicator_value.to_string(),
            target_value: result
                .target_value
                .map(|v| v.to_string())
                .unwrap_or_default(),
            variance: result.variance.map(|v| v.to_string()).unwrap_or_default(),
            variance_rate: result
                .variance_rate
                .map(|v| v.to_string())
                .unwrap_or_default(),
            trend: result.trend.unwrap_or_default(),
            analysis_date: result
                .analysis_date
                .map(|d| d.to_string())
                .unwrap_or_default(),
            created_by: result.created_by.unwrap_or(0),
            created_at: result.created_at.timestamp(),
        }
    }
}

// ===================== 辅助核算服务实现 =====================
#[tonic::async_trait]
impl AssistAccountingServiceTrait for GrpcNewServices {
    async fn initialize_dimensions(
        &self,
        _request: Request<InitializeDimensionsRequest>,
    ) -> Result<Response<InitializeDimensionsResponse>, Status> {
        match self.assist_accounting_service.initialize_dimensions().await {
            Ok(_) => Ok(Response::new(InitializeDimensionsResponse {
                success: true,
                message: "辅助核算维度初始化成功".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("初始化辅助核算维度失败：{}", e))),
        }
    }

    async fn create_assist_record(
        &self,
        request: Request<CreateAssistRecordRequest>,
    ) -> Result<Response<CreateAssistRecordResponse>, Status> {
        let req = request.into_inner();

        // 解析金额
        let debit_amount = req
            .debit_amount
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("借方金额格式错误：{}", e)))?;
        let credit_amount = req
            .credit_amount
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("贷方金额格式错误：{}", e)))?;
        let quantity_meters = req
            .quantity_meters
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("米数格式错误：{}", e)))?;
        let quantity_kg = req
            .quantity_kg
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("公斤数格式错误：{}", e)))?;

        match self
            .assist_accounting_service
            .create_assist_record(
                req.business_type,
                req.business_no,
                req.business_id,
                req.account_subject_id,
                debit_amount,
                credit_amount,
                req.five_dimension_id,
                req.product_id,
                req.batch_no,
                req.color_no,
                if req.dye_lot_no.is_empty() {
                    None
                } else {
                    Some(req.dye_lot_no)
                },
                req.grade,
                req.warehouse_id,
                quantity_meters,
                quantity_kg,
                if req.workshop_id == 0 {
                    None
                } else {
                    Some(req.workshop_id)
                },
                if req.customer_id == 0 {
                    None
                } else {
                    Some(req.customer_id)
                },
                if req.supplier_id == 0 {
                    None
                } else {
                    Some(req.supplier_id)
                },
                if req.remarks.is_empty() {
                    None
                } else {
                    Some(req.remarks)
                },
                if req.created_by == 0 {
                    None
                } else {
                    Some(req.created_by)
                },
            )
            .await
        {
            Ok(record) => Ok(Response::new(CreateAssistRecordResponse {
                success: true,
                message: "辅助核算记录创建成功".to_string(),
                record: Some(Self::to_grpc_assist_record(record)),
            })),
            Err(e) => Err(Status::internal(format!("创建辅助核算记录失败：{}", e))),
        }
    }

    async fn find_by_business(
        &self,
        request: Request<FindByBusinessRequest>,
    ) -> Result<Response<FindByBusinessResponse>, Status> {
        let req = request.into_inner();

        match self
            .assist_accounting_service
            .find_by_business(&req.business_type, &req.business_no)
            .await
        {
            Ok(records) => {
                let grpc_records: Vec<AssistAccountingRecord> = records
                    .into_iter()
                    .map(Self::to_grpc_assist_record)
                    .collect();
                Ok(Response::new(FindByBusinessResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询辅助核算记录失败：{}", e))),
        }
    }

    async fn find_by_five_dimension(
        &self,
        request: Request<FindByFiveDimensionRequest>,
    ) -> Result<Response<FindByFiveDimensionResponse>, Status> {
        let req = request.into_inner();

        match self
            .assist_accounting_service
            .find_by_five_dimension(&req.five_dimension_id)
            .await
        {
            Ok(records) => {
                let grpc_records: Vec<AssistAccountingRecord> = records
                    .into_iter()
                    .map(Self::to_grpc_assist_record)
                    .collect();
                Ok(Response::new(FindByFiveDimensionResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询辅助核算记录失败：{}", e))),
        }
    }

    async fn generate_monthly_summary(
        &self,
        request: Request<GenerateMonthlySummaryRequest>,
    ) -> Result<Response<GenerateMonthlySummaryResponse>, Status> {
        let req = request.into_inner();

        match self
            .assist_accounting_service
            .generate_monthly_summary(req.year, req.month as u32)
            .await
        {
            Ok(_) => Ok(Response::new(GenerateMonthlySummaryResponse {
                success: true,
                message: "月度汇总生成成功".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("生成月度汇总失败：{}", e))),
        }
    }

    async fn query_assist_records(
        &self,
        request: Request<QueryAssistRecordsRequest>,
    ) -> Result<Response<QueryAssistRecordsResponse>, Status> {
        let req = request.into_inner();

        let page = req.page.max(1) as u64;
        let page_size = req.page_size.clamp(1, 100) as u64;

        let accounting_period = if req.accounting_period.is_empty() {
            None
        } else {
            Some(req.accounting_period.as_str())
        };
        let dimension_code = if req.dimension_code.is_empty() {
            None
        } else {
            Some(req.dimension_code.as_str())
        };
        let business_type = if req.business_type.is_empty() {
            None
        } else {
            Some(req.business_type.as_str())
        };
        let warehouse_id = if req.warehouse_id == 0 {
            None
        } else {
            Some(req.warehouse_id)
        };

        match self
            .assist_accounting_service
            .query_assist_records(
                accounting_period,
                dimension_code,
                business_type,
                warehouse_id,
                page,
                page_size,
            )
            .await
        {
            Ok((records, total)) => {
                let grpc_records: Vec<AssistAccountingRecord> = records
                    .into_iter()
                    .map(Self::to_grpc_assist_record)
                    .collect();
                Ok(Response::new(QueryAssistRecordsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                    total: total as i32,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询辅助核算记录失败：{}", e))),
        }
    }

    async fn delete_assist_record(
        &self,
        request: Request<DeleteAssistRecordRequest>,
    ) -> Result<Response<DeleteAssistRecordResponse>, Status> {
        let req = request.into_inner();

        match self
            .assist_accounting_service
            .delete_assist_record(req.record_id)
            .await
        {
            Ok(_) => Ok(Response::new(DeleteAssistRecordResponse {
                success: true,
                message: "辅助核算记录删除成功".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("删除辅助核算记录失败：{}", e))),
        }
    }

    async fn list_dimensions(
        &self,
        _request: Request<ListDimensionsRequest>,
    ) -> Result<Response<ListDimensionsResponse>, Status> {
        match self.assist_accounting_service.list_dimensions().await {
            Ok(dimensions) => {
                let grpc_dimensions: Vec<AssistAccountingDimension> = dimensions
                    .into_iter()
                    .map(Self::to_grpc_dimension)
                    .collect();
                Ok(Response::new(ListDimensionsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    dimensions: grpc_dimensions,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询辅助核算维度失败：{}", e))),
        }
    }
}

// ===================== 供应商评估服务实现 =====================
#[tonic::async_trait]
impl SupplierEvaluationServiceTrait for GrpcNewServices {
    async fn get_indicators_list(
        &self,
        request: Request<GetIndicatorsListRequest>,
    ) -> Result<Response<GetIndicatorsListResponse>, Status> {
        let req = request.into_inner();

        let params = crate::services::supplier_evaluation_service::EvaluationIndicatorQueryParams {
            category: if req.category.is_empty() {
                None
            } else {
                Some(req.category)
            },
            status: if req.status.is_empty() {
                None
            } else {
                Some(req.status)
            },
            page: req.page as i64,
            page_size: req.page_size as i64,
        };

        match self
            .supplier_evaluation_service
            .get_indicators_list(params)
            .await
        {
            Ok((indicators, total)) => {
                let grpc_indicators: Vec<EvaluationIndicator> = indicators
                    .into_iter()
                    .map(Self::to_grpc_evaluation_indicator)
                    .collect();
                Ok(Response::new(GetIndicatorsListResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    indicators: grpc_indicators,
                    total: total as i32,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询评估指标失败：{}", e))),
        }
    }

    async fn create_indicator(
        &self,
        request: Request<CreateIndicatorRequest>,
    ) -> Result<Response<CreateIndicatorResponse>, Status> {
        let req = request.into_inner();

        let weight = req
            .weight
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("权重格式错误：{}", e)))?;

        let create_req =
            crate::services::supplier_evaluation_service::CreateEvaluationIndicatorRequest {
                indicator_name: req.indicator_name,
                indicator_code: req.indicator_code,
                category: req.category,
                weight,
                max_score: req.max_score,
                evaluation_method: if req.evaluation_method.is_empty() {
                    None
                } else {
                    Some(req.evaluation_method)
                },
            };

        let user_id = 1;

        match self
            .supplier_evaluation_service
            .create_indicator(create_req, user_id)
            .await
        {
            Ok(indicator) => Ok(Response::new(CreateIndicatorResponse {
                success: true,
                message: "评估指标创建成功".to_string(),
                indicator: Some(Self::to_grpc_evaluation_indicator(indicator)),
            })),
            Err(e) => Err(Status::internal(format!("创建评估指标失败：{}", e))),
        }
    }

    async fn create_evaluation_record(
        &self,
        request: Request<CreateEvaluationRecordRequest>,
    ) -> Result<Response<CreateEvaluationRecordResponse>, Status> {
        let req = request.into_inner();

        let score = req
            .score
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("得分格式错误：{}", e)))?;

        let eval_req = crate::services::supplier_evaluation_service::SupplierEvaluationRequest {
            supplier_id: req.supplier_id,
            evaluation_period: req.evaluation_period,
            indicator_id: req.indicator_id,
            score,
            remark: if req.remark.is_empty() {
                None
            } else {
                Some(req.remark)
            },
        };

        let user_id = 1;

        match self
            .supplier_evaluation_service
            .create_evaluation_record(eval_req, user_id)
            .await
        {
            Ok(record) => Ok(Response::new(CreateEvaluationRecordResponse {
                success: true,
                message: "评估记录创建成功".to_string(),
                record: Some(Self::to_grpc_evaluation_record(record)),
            })),
            Err(e) => Err(Status::internal(format!("创建评估记录失败：{}", e))),
        }
    }

    async fn get_supplier_score(
        &self,
        request: Request<GetSupplierScoreRequest>,
    ) -> Result<Response<GetSupplierScoreResponse>, Status> {
        let req = request.into_inner();

        match self
            .supplier_evaluation_service
            .get_supplier_score(req.supplier_id)
            .await
        {
            Ok(score) => Ok(Response::new(GetSupplierScoreResponse {
                success: true,
                message: "查询成功".to_string(),
                score: Some(SupplierScore {
                    supplier_id: score.supplier_id,
                    average_score: score.average_score.to_string(),
                    total_records: score.total_records,
                    rating: score.rating,
                    latest_evaluation_date: score
                        .latest_evaluation_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                }),
            })),
            Err(e) => Err(Status::internal(format!("查询供应商评分失败：{}", e))),
        }
    }

    async fn list_ratings(
        &self,
        _request: Request<ListRatingsRequest>,
    ) -> Result<Response<ListRatingsResponse>, Status> {
        match self.supplier_evaluation_service.list_ratings().await {
            Ok(ratings) => {
                let grpc_ratings: Vec<EvaluationIndicator> = ratings
                    .into_iter()
                    .map(Self::to_grpc_evaluation_indicator)
                    .collect();
                Ok(Response::new(ListRatingsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    ratings: grpc_ratings,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询评级列表失败：{}", e))),
        }
    }

    async fn get_supplier_rankings(
        &self,
        request: Request<GetSupplierRankingsRequest>,
    ) -> Result<Response<GetSupplierRankingsResponse>, Status> {
        let req = request.into_inner();

        match self
            .supplier_evaluation_service
            .get_supplier_rankings(req.limit as i64)
            .await
        {
            Ok(rankings) => {
                let grpc_rankings: Vec<SupplierScore> = rankings
                    .into_iter()
                    .map(|s| SupplierScore {
                        supplier_id: s.supplier_id,
                        average_score: s.average_score.to_string(),
                        total_records: s.total_records,
                        rating: s.rating,
                        latest_evaluation_date: s
                            .latest_evaluation_date
                            .map(|d| d.to_string())
                            .unwrap_or_default(),
                    })
                    .collect();
                Ok(Response::new(GetSupplierRankingsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    rankings: grpc_rankings,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询供应商排名失败：{}", e))),
        }
    }

    async fn get_evaluation_records(
        &self,
        request: Request<GetEvaluationRecordsRequest>,
    ) -> Result<Response<GetEvaluationRecordsResponse>, Status> {
        let req = request.into_inner();

        let supplier_id = if req.supplier_id == 0 {
            None
        } else {
            Some(req.supplier_id)
        };
        let period = if req.period.is_empty() {
            None
        } else {
            Some(req.period)
        };

        match self
            .supplier_evaluation_service
            .get_evaluation_records(supplier_id, period, req.page as i64, req.page_size as i64)
            .await
        {
            Ok(records) => {
                let grpc_records: Vec<SupplierEvaluationRecord> = records
                    .into_iter()
                    .map(Self::to_grpc_evaluation_record)
                    .collect();
                Ok(Response::new(GetEvaluationRecordsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询评估记录失败：{}", e))),
        }
    }

    async fn get_evaluation_record_by_id(
        &self,
        request: Request<GetEvaluationRecordByIdRequest>,
    ) -> Result<Response<GetEvaluationRecordByIdResponse>, Status> {
        let req = request.into_inner();

        match self
            .supplier_evaluation_service
            .get_evaluation_record_by_id(req.record_id)
            .await
        {
            Ok(record) => Ok(Response::new(GetEvaluationRecordByIdResponse {
                success: true,
                message: "查询成功".to_string(),
                record: Some(Self::to_grpc_evaluation_record(record)),
            })),
            Err(e) => Err(Status::internal(format!("查询评估记录失败：{}", e))),
        }
    }
}

// ===================== 五维查询服务实现 =====================
#[tonic::async_trait]
impl FiveDimensionQueryServiceTrait for GrpcNewServices {
    async fn generate_five_dimension_id(
        &self,
        request: Request<GenerateFiveDimensionIdRequest>,
    ) -> Result<Response<GenerateFiveDimensionIdResponse>, Status> {
        let req = request.into_inner();

        let dye_lot_no = if req.dye_lot_no.is_empty() {
            None
        } else {
            Some(req.dye_lot_no.as_str())
        };

        let five_dimension_id = FiveDimensionQueryService::generate_five_dimension_id(
            req.product_id,
            &req.batch_no,
            &req.color_no,
            dye_lot_no,
            &req.grade,
        );

        Ok(Response::new(GenerateFiveDimensionIdResponse {
            success: true,
            message: "生成成功".to_string(),
            five_dimension_id,
        }))
    }

    async fn parse_five_dimension_id(
        &self,
        request: Request<ParseFiveDimensionIdRequest>,
    ) -> Result<Response<ParseFiveDimensionIdResponse>, Status> {
        let req = request.into_inner();

        match FiveDimensionQueryService::parse_five_dimension_id(&req.five_dimension_id) {
            Some(dimension) => Ok(Response::new(ParseFiveDimensionIdResponse {
                success: true,
                message: "解析成功".to_string(),
                dimension: Some(FiveDimension {
                    product_id: dimension.product_id,
                    batch_no: dimension.batch_no,
                    color_no: dimension.color_no,
                    dye_lot_no: dimension.dye_lot_no.unwrap_or_default(),
                    grade: dimension.grade,
                }),
            })),
            None => Err(Status::invalid_argument("五维ID格式错误")),
        }
    }
}

// ===================== 库存预留服务实现 =====================
#[tonic::async_trait]
impl InventoryReservationServiceTrait for GrpcNewServices {
    async fn create_reservation(
        &self,
        request: Request<CreateReservationRequest>,
    ) -> Result<Response<CreateReservationResponse>, Status> {
        let req = request.into_inner();

        let quantity = req
            .quantity
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("数量格式错误：{}", e)))?;

        let created_by = if req.created_by == 0 {
            None
        } else {
            Some(req.created_by)
        };
        let notes = if req.notes.is_empty() {
            None
        } else {
            Some(req.notes)
        };

        match self
            .inventory_reservation_service
            .create_reservation(
                req.order_id,
                req.product_id,
                req.warehouse_id,
                quantity,
                created_by,
                notes,
            )
            .await
        {
            Ok(reservation) => Ok(Response::new(CreateReservationResponse {
                success: true,
                message: "库存预留创建成功".to_string(),
                reservation: Some(Self::to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(Status::internal(format!("创建库存预留失败：{}", e))),
        }
    }

    async fn lock_reservation(
        &self,
        request: Request<LockReservationRequest>,
    ) -> Result<Response<LockReservationResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .lock_reservation(req.reservation_id)
            .await
        {
            Ok(reservation) => Ok(Response::new(LockReservationResponse {
                success: true,
                message: "库存预留锁定成功".to_string(),
                reservation: Some(Self::to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(Status::internal(format!("锁定库存预留失败：{}", e))),
        }
    }

    async fn release_reservation(
        &self,
        request: Request<ReleaseReservationRequest>,
    ) -> Result<Response<ReleaseReservationResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .release_reservation(req.reservation_id)
            .await
        {
            Ok(reservation) => Ok(Response::new(ReleaseReservationResponse {
                success: true,
                message: "库存预留释放成功".to_string(),
                reservation: Some(Self::to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(Status::internal(format!("释放库存预留失败：{}", e))),
        }
    }

    async fn use_reservation(
        &self,
        request: Request<UseReservationRequest>,
    ) -> Result<Response<UseReservationResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .use_reservation(req.reservation_id)
            .await
        {
            Ok(reservation) => Ok(Response::new(UseReservationResponse {
                success: true,
                message: "库存预留使用成功".to_string(),
                reservation: Some(Self::to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(Status::internal(format!("使用库存预留失败：{}", e))),
        }
    }

    async fn get_reservations_by_order(
        &self,
        request: Request<GetReservationsByOrderRequest>,
    ) -> Result<Response<GetReservationsByOrderResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .get_reservations_by_order(req.order_id)
            .await
        {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations
                    .into_iter()
                    .map(Self::to_grpc_reservation)
                    .collect();
                Ok(Response::new(GetReservationsByOrderResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询库存预留失败：{}", e))),
        }
    }

    async fn get_locked_reservations_by_order(
        &self,
        request: Request<GetLockedReservationsByOrderRequest>,
    ) -> Result<Response<GetLockedReservationsByOrderResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .get_locked_reservations_by_order(req.order_id)
            .await
        {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations
                    .into_iter()
                    .map(Self::to_grpc_reservation)
                    .collect();
                Ok(Response::new(GetLockedReservationsByOrderResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询库存预留失败：{}", e))),
        }
    }

    async fn batch_create_reservations(
        &self,
        request: Request<BatchCreateReservationsRequest>,
    ) -> Result<Response<BatchCreateReservationsResponse>, Status> {
        let req = request.into_inner();

        let mut items = Vec::new();
        for item in req.items {
            let quantity = item
                .quantity
                .parse::<rust_decimal::Decimal>()
                .map_err(|e| Status::invalid_argument(format!("数量格式错误：{}", e)))?;
            items.push((item.product_id, item.warehouse_id, quantity));
        }

        let created_by = if req.created_by == 0 {
            None
        } else {
            Some(req.created_by)
        };

        match self
            .inventory_reservation_service
            .batch_create_reservations(req.order_id, items, created_by)
            .await
        {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations
                    .into_iter()
                    .map(Self::to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchCreateReservationsResponse {
                    success: true,
                    message: "批量创建库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(Status::internal(format!("批量创建库存预留失败：{}", e))),
        }
    }

    async fn batch_lock_reservations(
        &self,
        request: Request<BatchLockReservationsRequest>,
    ) -> Result<Response<BatchLockReservationsResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .batch_lock_reservations(req.reservation_ids)
            .await
        {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations
                    .into_iter()
                    .map(Self::to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchLockReservationsResponse {
                    success: true,
                    message: "批量锁定库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(Status::internal(format!("批量锁定库存预留失败：{}", e))),
        }
    }

    async fn batch_release_reservations(
        &self,
        request: Request<BatchReleaseReservationsRequest>,
    ) -> Result<Response<BatchReleaseReservationsResponse>, Status> {
        let req = request.into_inner();

        match self
            .inventory_reservation_service
            .batch_release_reservations(req.reservation_ids)
            .await
        {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations
                    .into_iter()
                    .map(Self::to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchReleaseReservationsResponse {
                    success: true,
                    message: "批量释放库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(Status::internal(format!("批量释放库存预留失败：{}", e))),
        }
    }
}

// ===================== 财务分析服务实现 =====================
#[tonic::async_trait]
impl FinancialAnalysisServiceTrait for GrpcNewServices {
    async fn get_indicators_list(
        &self,
        request: Request<GetFinancialIndicatorsListRequest>,
    ) -> Result<Response<GetFinancialIndicatorsListResponse>, Status> {
        let req = request.into_inner();

        let params = crate::services::financial_analysis_service::IndicatorQueryParams {
            indicator_type: if req.indicator_type.is_empty() {
                None
            } else {
                Some(req.indicator_type)
            },
            status: if req.status.is_empty() {
                None
            } else {
                Some(req.status)
            },
            page: req.page as i64,
            page_size: req.page_size as i64,
        };

        match self
            .financial_analysis_service
            .get_indicators_list(params)
            .await
        {
            Ok((indicators, total)) => {
                let grpc_indicators: Vec<FinancialIndicator> = indicators
                    .into_iter()
                    .map(Self::to_grpc_financial_indicator)
                    .collect();
                Ok(Response::new(GetFinancialIndicatorsListResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    indicators: grpc_indicators,
                    total: total as i32,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询财务指标失败：{}", e))),
        }
    }

    async fn create_indicator(
        &self,
        request: Request<CreateFinancialIndicatorRequest>,
    ) -> Result<Response<CreateFinancialIndicatorResponse>, Status> {
        let req = request.into_inner();

        let create_req = crate::services::financial_analysis_service::CreateIndicatorRequest {
            indicator_name: req.indicator_name,
            indicator_code: req.indicator_code,
            indicator_type: req.indicator_type,
            formula: if req.formula.is_empty() {
                None
            } else {
                Some(req.formula)
            },
            unit: if req.unit.is_empty() {
                None
            } else {
                Some(req.unit)
            },
            remark: if req.remark.is_empty() {
                None
            } else {
                Some(req.remark)
            },
        };

        let user_id = 1;

        match self
            .financial_analysis_service
            .create_indicator(create_req, user_id)
            .await
        {
            Ok(indicator) => Ok(Response::new(CreateFinancialIndicatorResponse {
                success: true,
                message: "财务指标创建成功".to_string(),
                indicator: Some(Self::to_grpc_financial_indicator(indicator)),
            })),
            Err(e) => Err(Status::internal(format!("创建财务指标失败：{}", e))),
        }
    }

    async fn create_analysis_result(
        &self,
        request: Request<CreateAnalysisResultRequest>,
    ) -> Result<Response<CreateAnalysisResultResponse>, Status> {
        let req = request.into_inner();

        let indicator_value = req
            .indicator_value
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("指标值格式错误：{}", e)))?;
        let target_value = if req.target_value.is_empty() {
            None
        } else {
            Some(
                req.target_value
                    .parse::<rust_decimal::Decimal>()
                    .map_err(|e| Status::invalid_argument(format!("目标值格式错误：{}", e)))?,
            )
        };

        let analysis_req = crate::services::financial_analysis_service::FinancialAnalysisRequest {
            analysis_type: req.analysis_type,
            period: req.period,
            indicator_id: req.indicator_id,
            indicator_value,
            target_value,
        };

        let user_id = 1;

        match self
            .financial_analysis_service
            .create_analysis_result(analysis_req, user_id)
            .await
        {
            Ok(result) => Ok(Response::new(CreateAnalysisResultResponse {
                success: true,
                message: "财务分析结果创建成功".to_string(),
                result: Some(Self::to_grpc_analysis_result(result)),
            })),
            Err(e) => Err(Status::internal(format!("创建财务分析结果失败：{}", e))),
        }
    }

    async fn get_trends(
        &self,
        request: Request<GetTrendsRequest>,
    ) -> Result<Response<GetTrendsResponse>, Status> {
        let req = request.into_inner();

        match self
            .financial_analysis_service
            .get_trends(req.indicator_id, req.limit as i64)
            .await
        {
            Ok(results) => {
                let grpc_results: Vec<FinancialAnalysisResult> = results
                    .into_iter()
                    .map(Self::to_grpc_analysis_result)
                    .collect();
                Ok(Response::new(GetTrendsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    results: grpc_results,
                }))
            }
            Err(e) => Err(Status::internal(format!("查询趋势数据失败：{}", e))),
        }
    }
}
