use tonic::{Request, Response, Status};
use crate::grpc::new_services::GrpcNewServices;
use crate::grpc::new_services::support::{parse_decimal, empty_to_option, id_to_option, operator_id, handle_error};

use crate::grpc::service::proto::{
    supplier_evaluation_service_server::SupplierEvaluationService as SupplierEvaluationServiceTrait,
    GetIndicatorsListRequest, GetIndicatorsListResponse,
    CreateIndicatorRequest, CreateIndicatorResponse,
    CreateEvaluationRecordRequest, CreateEvaluationRecordResponse,
    GetSupplierScoreRequest, GetSupplierScoreResponse,
    ListRatingsRequest, ListRatingsResponse,
    GetSupplierRankingsRequest, GetSupplierRankingsResponse,
    GetEvaluationRecordsRequest, GetEvaluationRecordsResponse,
    GetEvaluationRecordByIdRequest, GetEvaluationRecordByIdResponse,
    EvaluationIndicator, SupplierEvaluationRecord, SupplierScore,
};

/// 将评估指标转换为 gRPC 模型
fn to_grpc_evaluation_indicator(indicator: crate::models::supplier_evaluation::Model) -> EvaluationIndicator {
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
fn to_grpc_evaluation_record(record: crate::models::supplier_evaluation_record::Model) -> SupplierEvaluationRecord {
    SupplierEvaluationRecord {
        id: record.id,
        supplier_id: record.supplier_id,
        evaluation_period: record.evaluation_period,
        indicator_id: record.indicator_id,
        score: record.score.to_string(),
        max_score: record.max_score.unwrap_or(0),
        weighted_score: record.weighted_score.map(|s| s.to_string()).unwrap_or_default(),
        evaluator_id: record.evaluator_id.unwrap_or(0),
        evaluation_date: record.evaluation_date.map(|d| d.to_string()).unwrap_or_default(),
        remark: record.remark.unwrap_or_default(),
        created_at: record.created_at.timestamp(),
    }
}

#[tonic::async_trait]
impl SupplierEvaluationServiceTrait for GrpcNewServices {
    async fn get_indicators_list(
        &self,
        request: Request<GetIndicatorsListRequest>,
    ) -> Result<Response<GetIndicatorsListResponse>, Status> {
        let req = request.into_inner();
        
        let params = crate::services::supplier_evaluation_service::EvaluationIndicatorQueryParams {
            category: empty_to_option(req.category),
            status: empty_to_option(req.status),
            page: req.page as i64,
            page_size: req.page_size as i64,
        };
        
        match self.supplier_evaluation_service.get_indicators_list(params).await {
            Ok((indicators, total)) => {
                let grpc_indicators: Vec<EvaluationIndicator> = indicators.into_iter()
                    .map(to_grpc_evaluation_indicator)
                    .collect();
                Ok(Response::new(GetIndicatorsListResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    indicators: grpc_indicators,
                    total: total as i32,
                }))
            }
            Err(e) => Err(handle_error("查询评估指标失败", e)),
        }
    }
    
    async fn create_indicator(
        &self,
        request: Request<CreateIndicatorRequest>,
    ) -> Result<Response<CreateIndicatorResponse>, Status> {
        let user_id = operator_id(&request)?;
        let req = request.into_inner();
        
        let weight = parse_decimal("权重", &req.weight)?;
        
        let create_req = crate::services::supplier_evaluation_service::CreateEvaluationIndicatorRequest {
            indicator_name: req.indicator_name,
            indicator_code: req.indicator_code,
            category: req.category,
            weight,
            max_score: req.max_score,
            evaluation_method: empty_to_option(req.evaluation_method),
        };
        
        match self.supplier_evaluation_service.create_indicator(create_req, user_id).await {
            Ok(indicator) => Ok(Response::new(CreateIndicatorResponse {
                success: true,
                message: "评估指标创建成功".to_string(),
                indicator: Some(to_grpc_evaluation_indicator(indicator)),
            })),
            Err(e) => Err(handle_error("创建评估指标失败", e)),
        }
    }
    
    async fn create_evaluation_record(
        &self,
        request: Request<CreateEvaluationRecordRequest>,
    ) -> Result<Response<CreateEvaluationRecordResponse>, Status> {
        let user_id = operator_id(&request)?;
        let req = request.into_inner();
        
        let score = parse_decimal("得分", &req.score)?;
        
        let eval_req = crate::services::supplier_evaluation_service::SupplierEvaluationRequest {
            supplier_id: req.supplier_id,
            evaluation_period: req.evaluation_period,
            indicator_id: req.indicator_id,
            score,
            remark: empty_to_option(req.remark),
        };
        
        match self.supplier_evaluation_service.create_evaluation_record(eval_req, user_id).await {
            Ok(record) => Ok(Response::new(CreateEvaluationRecordResponse {
                success: true,
                message: "评估记录创建成功".to_string(),
                record: Some(to_grpc_evaluation_record(record)),
            })),
            Err(e) => Err(handle_error("创建评估记录失败", e)),
        }
    }
    
    async fn get_supplier_score(
        &self,
        request: Request<GetSupplierScoreRequest>,
    ) -> Result<Response<GetSupplierScoreResponse>, Status> {
        let req = request.into_inner();
        
        match self.supplier_evaluation_service.get_supplier_score(req.supplier_id).await {
            Ok(score) => Ok(Response::new(GetSupplierScoreResponse {
                success: true,
                message: "查询成功".to_string(),
                score: Some(SupplierScore {
                    supplier_id: score.supplier_id,
                    average_score: score.average_score.to_string(),
                    total_records: score.total_records,
                    rating: score.rating,
                    latest_evaluation_date: score.latest_evaluation_date.map(|d| d.to_string()).unwrap_or_default(),
                }),
            })),
            Err(e) => Err(handle_error("查询供应商评分失败", e)),
        }
    }
    
    async fn list_ratings(
        &self,
        _request: Request<ListRatingsRequest>,
    ) -> Result<Response<ListRatingsResponse>, Status> {
        match self.supplier_evaluation_service.list_ratings().await {
            Ok(ratings) => {
                let grpc_ratings: Vec<EvaluationIndicator> = ratings.into_iter()
                    .map(to_grpc_evaluation_indicator)
                    .collect();
                Ok(Response::new(ListRatingsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    ratings: grpc_ratings,
                }))
            }
            Err(e) => Err(handle_error("查询评级列表失败", e)),
        }
    }
    
    async fn get_supplier_rankings(
        &self,
        request: Request<GetSupplierRankingsRequest>,
    ) -> Result<Response<GetSupplierRankingsResponse>, Status> {
        let req = request.into_inner();
        
        match self.supplier_evaluation_service.get_supplier_rankings(req.limit as i64).await {
            Ok(rankings) => {
                let grpc_rankings: Vec<SupplierScore> = rankings.into_iter()
                    .map(|s| SupplierScore {
                        supplier_id: s.supplier_id,
                        average_score: s.average_score.to_string(),
                        total_records: s.total_records,
                        rating: s.rating,
                        latest_evaluation_date: s.latest_evaluation_date.map(|d| d.to_string()).unwrap_or_default(),
                    })
                    .collect();
                Ok(Response::new(GetSupplierRankingsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    rankings: grpc_rankings,
                }))
            }
            Err(e) => Err(handle_error("查询供应商排名失败", e)),
        }
    }
    
    async fn get_evaluation_records(
        &self,
        request: Request<GetEvaluationRecordsRequest>,
    ) -> Result<Response<GetEvaluationRecordsResponse>, Status> {
        let req = request.into_inner();
        
        let supplier_id = id_to_option(req.supplier_id);
        let period = empty_to_option(req.period);
        
        match self.supplier_evaluation_service.get_evaluation_records(
            supplier_id,
            period,
            req.page as i64,
            req.page_size as i64,
        ).await {
            Ok(records) => {
                let grpc_records: Vec<SupplierEvaluationRecord> = records.into_iter()
                    .map(to_grpc_evaluation_record)
                    .collect();
                Ok(Response::new(GetEvaluationRecordsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(handle_error("查询评估记录失败", e)),
        }
    }
    
    async fn get_evaluation_record_by_id(
        &self,
        request: Request<GetEvaluationRecordByIdRequest>,
    ) -> Result<Response<GetEvaluationRecordByIdResponse>, Status> {
        let req = request.into_inner();
        
        match self.supplier_evaluation_service.get_evaluation_record_by_id(req.record_id).await {
            Ok(record) => Ok(Response::new(GetEvaluationRecordByIdResponse {
                success: true,
                message: "查询成功".to_string(),
                record: Some(to_grpc_evaluation_record(record)),
            })),
            Err(e) => Err(handle_error("查询评估记录失败", e)),
        }
    }
}
