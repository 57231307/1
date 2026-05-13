use tonic::{Request, Response, Status};
use crate::grpc::new_services::GrpcNewServices;
use crate::grpc::new_services::support::empty_to_option;
use crate::services::five_dimension_query_service::FiveDimensionQueryService;

use crate::grpc::service::proto::{
    five_dimension_query_service_server::FiveDimensionQueryService as FiveDimensionQueryServiceTrait,
    GenerateFiveDimensionIdRequest, GenerateFiveDimensionIdResponse,
    ParseFiveDimensionIdRequest, ParseFiveDimensionIdResponse,
    FiveDimension,
};

#[tonic::async_trait]
impl FiveDimensionQueryServiceTrait for GrpcNewServices {
    async fn generate_five_dimension_id(
        &self,
        request: Request<GenerateFiveDimensionIdRequest>,
    ) -> Result<Response<GenerateFiveDimensionIdResponse>, Status> {
        let req = request.into_inner();
        
        let dye_lot_no = empty_to_option(req.dye_lot_no);
        
        let five_dimension_id = FiveDimensionQueryService::generate_five_dimension_id(
            req.product_id,
            &req.batch_no,
            &req.color_no,
            dye_lot_no.as_deref(),
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
