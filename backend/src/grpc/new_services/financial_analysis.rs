use tonic::{Request, Response, Status};
use crate::grpc::new_services::GrpcNewServices;
use crate::grpc::new_services::support::{parse_decimal, empty_to_option, operator_id, handle_error};

use crate::grpc::service::proto::{
    financial_analysis_service_server::FinancialAnalysisService as FinancialAnalysisServiceTrait,
    GetFinancialIndicatorsListRequest, GetFinancialIndicatorsListResponse,
    CreateFinancialIndicatorRequest, CreateFinancialIndicatorResponse,
    CreateAnalysisResultRequest, CreateAnalysisResultResponse,
    GetTrendsRequest, GetTrendsResponse,
    FinancialIndicator, FinancialAnalysisResult,
};

/// 将财务指标转换为 gRPC 模型
fn to_grpc_financial_indicator(indicator: crate::models::financial_analysis::Model) -> FinancialIndicator {
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
fn to_grpc_analysis_result(result: crate::models::financial_analysis_result::Model) -> FinancialAnalysisResult {
    FinancialAnalysisResult {
        id: result.id,
        analysis_type: result.analysis_type,
        period: result.period,
        indicator_id: result.indicator_id,
        indicator_value: result.indicator_value.to_string(),
        target_value: result.target_value.map(|v| v.to_string()).unwrap_or_default(),
        variance: result.variance.map(|v| v.to_string()).unwrap_or_default(),
        variance_rate: result.variance_rate.map(|v| v.to_string()).unwrap_or_default(),
        trend: result.trend.unwrap_or_default(),
        analysis_date: result.analysis_date.map(|d| d.to_string()).unwrap_or_default(),
        created_by: result.created_by.unwrap_or(0),
        created_at: result.created_at.timestamp(),
    }
}

#[tonic::async_trait]
impl FinancialAnalysisServiceTrait for GrpcNewServices {
    async fn get_indicators_list(
        &self,
        request: Request<GetFinancialIndicatorsListRequest>,
    ) -> Result<Response<GetFinancialIndicatorsListResponse>, Status> {
        let req = request.into_inner();
        
        let params = crate::services::financial_analysis_service::IndicatorQueryParams {
            indicator_type: empty_to_option(req.indicator_type),
            status: empty_to_option(req.status),
            page: req.page as i64,
            page_size: req.page_size as i64,
        };
        
        match self.financial_analysis_service.get_indicators_list(params).await {
            Ok((indicators, total)) => {
                let grpc_indicators: Vec<FinancialIndicator> = indicators.into_iter()
                    .map(to_grpc_financial_indicator)
                    .collect();
                Ok(Response::new(GetFinancialIndicatorsListResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    indicators: grpc_indicators,
                    total: total as i32,
                }))
            }
            Err(e) => Err(handle_error("查询财务指标失败", e)),
        }
    }
    
    async fn create_indicator(
        &self,
        request: Request<CreateFinancialIndicatorRequest>,
    ) -> Result<Response<CreateFinancialIndicatorResponse>, Status> {
        let user_id = operator_id(&request)?;
        let req = request.into_inner();
        
        let create_req = crate::services::financial_analysis_service::CreateIndicatorRequest {
            indicator_name: req.indicator_name,
            indicator_code: req.indicator_code,
            indicator_type: req.indicator_type,
            formula: empty_to_option(req.formula),
            unit: empty_to_option(req.unit),
            remark: empty_to_option(req.remark),
        };
        
        match self.financial_analysis_service.create_indicator(create_req, user_id).await {
            Ok(indicator) => Ok(Response::new(CreateFinancialIndicatorResponse {
                success: true,
                message: "财务指标创建成功".to_string(),
                indicator: Some(to_grpc_financial_indicator(indicator)),
            })),
            Err(e) => Err(handle_error("创建财务指标失败", e)),
        }
    }
    
    async fn create_analysis_result(
        &self,
        request: Request<CreateAnalysisResultRequest>,
    ) -> Result<Response<CreateAnalysisResultResponse>, Status> {
        let user_id = operator_id(&request)?;
        let req = request.into_inner();
        
        let indicator_value = parse_decimal("指标值", &req.indicator_value)?;
        
        let target_value = if req.target_value.is_empty() { 
            None 
        } else { 
            Some(parse_decimal("目标值", &req.target_value)?)
        };
        
        let analysis_req = crate::services::financial_analysis_service::FinancialAnalysisRequest {
            analysis_type: req.analysis_type,
            period: req.period,
            indicator_id: req.indicator_id,
            indicator_value,
            target_value,
        };
        
        match self.financial_analysis_service.create_analysis_result(analysis_req, user_id).await {
            Ok(result) => Ok(Response::new(CreateAnalysisResultResponse {
                success: true,
                message: "财务分析结果创建成功".to_string(),
                result: Some(to_grpc_analysis_result(result)),
            })),
            Err(e) => Err(handle_error("创建财务分析结果失败", e)),
        }
    }
    
    async fn get_trends(
        &self,
        request: Request<GetTrendsRequest>,
    ) -> Result<Response<GetTrendsResponse>, Status> {
        let req = request.into_inner();
        
        match self.financial_analysis_service.get_trends(req.indicator_id, req.limit as i64).await {
            Ok(results) => {
                let grpc_results: Vec<FinancialAnalysisResult> = results.into_iter()
                    .map(to_grpc_analysis_result)
                    .collect();
                Ok(Response::new(GetTrendsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    results: grpc_results,
                }))
            }
            Err(e) => Err(handle_error("查询趋势数据失败", e)),
        }
    }
}
