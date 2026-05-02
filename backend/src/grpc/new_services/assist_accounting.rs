use tonic::{Request, Response, Status};
use crate::grpc::new_services::GrpcNewServices;
use crate::grpc::new_services::support::{parse_decimal, empty_to_option, id_to_option, handle_error};

use crate::grpc::service::proto::{
    assist_accounting_service_server::AssistAccountingService as AssistAccountingServiceTrait,
    InitializeDimensionsRequest, InitializeDimensionsResponse,
    CreateAssistRecordRequest, CreateAssistRecordResponse,
    FindByBusinessRequest, FindByBusinessResponse,
    FindByFiveDimensionRequest, FindByFiveDimensionResponse,
    GenerateMonthlySummaryRequest, GenerateMonthlySummaryResponse,
    QueryAssistRecordsRequest, QueryAssistRecordsResponse,
    DeleteAssistRecordRequest, DeleteAssistRecordResponse,
    ListDimensionsRequest, ListDimensionsResponse,
    AssistAccountingRecord, AssistAccountingDimension,
};

/// 将辅助核算记录转换为 gRPC 模型
fn to_grpc_assist_record(record: crate::models::assist_accounting_record::Model) -> AssistAccountingRecord {
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
fn to_grpc_dimension(dimension: crate::models::assist_accounting_dimension::Model) -> AssistAccountingDimension {
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
            Err(e) => Err(handle_error("初始化辅助核算维度失败", e)),
        }
    }
    
    async fn create_assist_record(
        &self,
        request: Request<CreateAssistRecordRequest>,
    ) -> Result<Response<CreateAssistRecordResponse>, Status> {
        let req = request.into_inner();
        
        let debit_amount = parse_decimal("借方金额", &req.debit_amount)?;
        let credit_amount = parse_decimal("贷方金额", &req.credit_amount)?;
        let quantity_meters = parse_decimal("米数", &req.quantity_meters)?;
        let quantity_kg = parse_decimal("公斤数", &req.quantity_kg)?;
        
        let dye_lot_no = empty_to_option(req.dye_lot_no);
        let workshop_id = id_to_option(req.workshop_id);
        let customer_id = id_to_option(req.customer_id);
        let supplier_id = id_to_option(req.supplier_id);
        let remarks = empty_to_option(req.remarks);
        let created_by = id_to_option(req.created_by);
        
        match self.assist_accounting_service.create_assist_record(
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
            dye_lot_no,
            req.grade,
            req.warehouse_id,
            quantity_meters,
            quantity_kg,
            workshop_id,
            customer_id,
            supplier_id,
            remarks,
            created_by,
        ).await {
            Ok(record) => Ok(Response::new(CreateAssistRecordResponse {
                success: true,
                message: "辅助核算记录创建成功".to_string(),
                record: Some(to_grpc_assist_record(record)),
            })),
            Err(e) => Err(handle_error("创建辅助核算记录失败", e)),
        }
    }
    
    async fn find_by_business(
        &self,
        request: Request<FindByBusinessRequest>,
    ) -> Result<Response<FindByBusinessResponse>, Status> {
        let req = request.into_inner();
        
        match self.assist_accounting_service.find_by_business(&req.business_type, &req.business_no).await {
            Ok(records) => {
                let grpc_records: Vec<AssistAccountingRecord> = records.into_iter()
                    .map(to_grpc_assist_record)
                    .collect();
                Ok(Response::new(FindByBusinessResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(handle_error("查询辅助核算记录失败", e)),
        }
    }
    
    async fn find_by_five_dimension(
        &self,
        request: Request<FindByFiveDimensionRequest>,
    ) -> Result<Response<FindByFiveDimensionResponse>, Status> {
        let req = request.into_inner();
        
        match self.assist_accounting_service.find_by_five_dimension(&req.five_dimension_id).await {
            Ok(records) => {
                let grpc_records: Vec<AssistAccountingRecord> = records.into_iter()
                    .map(to_grpc_assist_record)
                    .collect();
                Ok(Response::new(FindByFiveDimensionResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                }))
            }
            Err(e) => Err(handle_error("查询辅助核算记录失败", e)),
        }
    }
    
    async fn generate_monthly_summary(
        &self,
        request: Request<GenerateMonthlySummaryRequest>,
    ) -> Result<Response<GenerateMonthlySummaryResponse>, Status> {
        let req = request.into_inner();
        
        match self.assist_accounting_service.generate_monthly_summary(req.year, req.month as u32).await {
            Ok(_) => Ok(Response::new(GenerateMonthlySummaryResponse {
                success: true,
                message: "月度汇总生成成功".to_string(),
            })),
            Err(e) => Err(handle_error("生成月度汇总失败", e)),
        }
    }
    
    async fn query_assist_records(
        &self,
        request: Request<QueryAssistRecordsRequest>,
    ) -> Result<Response<QueryAssistRecordsResponse>, Status> {
        let req = request.into_inner();
        
        let page = req.page.max(1) as u64;
        let page_size = req.page_size.clamp(1, 100) as u64;
        
        let accounting_period = empty_to_option(req.accounting_period);
        let dimension_code = empty_to_option(req.dimension_code);
        let business_type = empty_to_option(req.business_type);
        let warehouse_id = id_to_option(req.warehouse_id);
        
        match self.assist_accounting_service.query_assist_records(
            accounting_period.as_deref(),
            dimension_code.as_deref(),
            business_type.as_deref(),
            warehouse_id,
            page,
            page_size,
        ).await {
            Ok((records, total)) => {
                let grpc_records: Vec<AssistAccountingRecord> = records.into_iter()
                    .map(to_grpc_assist_record)
                    .collect();
                Ok(Response::new(QueryAssistRecordsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    records: grpc_records,
                    total: total as i32,
                }))
            }
            Err(e) => Err(handle_error("查询辅助核算记录失败", e)),
        }
    }
    
    async fn delete_assist_record(
        &self,
        request: Request<DeleteAssistRecordRequest>,
    ) -> Result<Response<DeleteAssistRecordResponse>, Status> {
        let req = request.into_inner();
        
        match self.assist_accounting_service.delete_assist_record(req.record_id).await {
            Ok(_) => Ok(Response::new(DeleteAssistRecordResponse {
                success: true,
                message: "辅助核算记录删除成功".to_string(),
            })),
            Err(e) => Err(handle_error("删除辅助核算记录失败", e)),
        }
    }
    
    async fn list_dimensions(
        &self,
        _request: Request<ListDimensionsRequest>,
    ) -> Result<Response<ListDimensionsResponse>, Status> {
        match self.assist_accounting_service.list_dimensions().await {
            Ok(dimensions) => {
                let grpc_dimensions: Vec<AssistAccountingDimension> = dimensions.into_iter()
                    .map(to_grpc_dimension)
                    .collect();
                Ok(Response::new(ListDimensionsResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    dimensions: grpc_dimensions,
                }))
            }
            Err(e) => Err(handle_error("查询辅助核算维度失败", e)),
        }
    }
}
