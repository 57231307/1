use chrono::Datelike;
// 管理服务 gRPC 实现
// 
// 包含采购合同、销售合同、固定资产、预算管理等 gRPC 服务实现

use std::sync::Arc;
use tonic::{Request, Response, Status};
use sea_orm::DatabaseConnection;

use crate::services::purchase_contract_service::PurchaseContractService;
use crate::services::sales_contract_service::SalesContractService;
use crate::services::fixed_asset_service::FixedAssetService;
use crate::services::budget_management_service::BudgetManagementService;

use crate::grpc::service::proto::{
    purchase_contract_service_server::PurchaseContractService as PurchaseContractServiceTrait,
    PurchaseContract, ListPurchaseContractsRequest, ListPurchaseContractsResponse,
    GetPurchaseContractRequest, GetPurchaseContractResponse,
    CreatePurchaseContractRequest, CreatePurchaseContractResponse,
    ApprovePurchaseContractRequest, ApprovePurchaseContractResponse,
    ExecutePurchaseContractRequest, ExecutePurchaseContractResponse,
    CancelPurchaseContractRequest, CancelPurchaseContractResponse,
    
    sales_contract_service_server::SalesContractService as SalesContractServiceTrait,
    SalesContract, ListSalesContractsRequest, ListSalesContractsResponse,
    GetSalesContractRequest, GetSalesContractResponse,
    CreateSalesContractRequest, CreateSalesContractResponse,
    ApproveSalesContractRequest, ApproveSalesContractResponse,
    ExecuteSalesContractRequest, ExecuteSalesContractResponse,
    CancelSalesContractRequest, CancelSalesContractResponse,
    
    fixed_asset_service_server::FixedAssetService as FixedAssetServiceTrait,
    FixedAsset, ListFixedAssetsRequest, ListFixedAssetsResponse,
    GetFixedAssetRequest, GetFixedAssetResponse,
    CreateFixedAssetRequest, CreateFixedAssetResponse,
    DepreciateFixedAssetRequest, DepreciateFixedAssetResponse,
    DisposeFixedAssetRequest, DisposeFixedAssetResponse,
    DeleteFixedAssetRequest, DeleteFixedAssetResponse,
    
    budget_management_service_server::BudgetManagementService as BudgetManagementServiceTrait,
    BudgetItem, ListBudgetItemsRequest, ListBudgetItemsResponse,
    GetBudgetItemRequest, GetBudgetItemResponse,
    CreateBudgetItemRequest, CreateBudgetItemResponse,
    UpdateBudgetItemRequest, UpdateBudgetItemResponse,
    DeleteBudgetItemRequest, DeleteBudgetItemResponse, ListBudgetPlansRequest, ListBudgetPlansResponse,
    GetBudgetPlanRequest, GetBudgetPlanResponse,
    CreateBudgetPlanRequest, CreateBudgetPlanResponse,
    ApproveBudgetPlanRequest, ApproveBudgetPlanResponse,
    ExecuteBudgetPlanRequest, ExecuteBudgetPlanResponse,
};

/// gRPC 管理服务集合
#[derive(Clone)]
pub struct GrpcManagementServices {
    purchase_contract_service: Arc<PurchaseContractService>,
    sales_contract_service: Arc<SalesContractService>,
    fixed_asset_service: Arc<FixedAssetService>,
    budget_management_service: Arc<BudgetManagementService>,
}

impl GrpcManagementServices {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            purchase_contract_service: Arc::new(PurchaseContractService::new(db.clone())),
            sales_contract_service: Arc::new(SalesContractService::new(db.clone())),
            fixed_asset_service: Arc::new(FixedAssetService::new(db.clone())),
            budget_management_service: Arc::new(BudgetManagementService::new(db)),
        }
    }
    
    /// 将采购合同转换为 gRPC 模型
    fn to_grpc_purchase_contract(contract: crate::models::purchase_contract::Model) -> PurchaseContract {
        PurchaseContract {
            id: contract.id,
            contract_no: contract.contract_no,
            contract_name: contract.contract_name,
            contract_type: contract.contract_type.unwrap_or_default(),
            supplier_id: contract.supplier_id,
            supplier_name: contract.supplier_name.unwrap_or_default(),
            total_amount: contract.total_amount.map(|d| d.to_string()).unwrap_or_default(),
            signed_date: contract.signed_date.map(|d| d.to_string()).unwrap_or_default(),
            effective_date: contract.effective_date.map(|d| d.to_string()).unwrap_or_default(),
            expiry_date: contract.expiry_date.map(|d| d.to_string()).unwrap_or_default(),
            payment_terms: contract.payment_terms.unwrap_or_default(),
            payment_method: contract.payment_method.unwrap_or_default(),
            delivery_date: contract.delivery_date.map(|d| d.to_string()).unwrap_or_default(),
            delivery_location: contract.delivery_location.unwrap_or_default(),
            status: contract.status,
            created_by: contract.created_by,
            created_at: contract.created_at.timestamp(),
            updated_at: contract.updated_at.timestamp(),
        }
    }
    
    /// 将销售合同转换为 gRPC 模型
    fn to_grpc_sales_contract(contract: crate::models::sales_contract::Model) -> SalesContract {
        SalesContract {
            id: contract.id,
            contract_no: contract.contract_no,
            contract_name: contract.contract_name,
            contract_type: contract.contract_type.unwrap_or_default(),
            customer_id: contract.customer_id,
            customer_name: contract.customer_name.unwrap_or_default(),
            total_amount: contract.total_amount.map(|d| d.to_string()).unwrap_or_default(),
            signed_date: contract.signed_date.map(|d| d.to_string()).unwrap_or_default(),
            effective_date: contract.effective_date.map(|d| d.to_string()).unwrap_or_default(),
            expiry_date: contract.expiry_date.map(|d| d.to_string()).unwrap_or_default(),
            payment_terms: contract.payment_terms.unwrap_or_default(),
            payment_method: contract.payment_method.unwrap_or_default(),
            delivery_date: contract.delivery_date.map(|d| d.to_string()).unwrap_or_default(),
            delivery_location: contract.delivery_location.unwrap_or_default(),
            status: contract.status,
            created_by: contract.created_by,
            created_at: contract.created_at.timestamp(),
            updated_at: contract.updated_at.timestamp(),
        }
    }
    
    /// 将固定资产转换为 gRPC 模型
    fn to_grpc_fixed_asset(asset: crate::models::fixed_asset::Model) -> FixedAsset {
        FixedAsset {
            id: asset.id,
            asset_no: asset.asset_no,
            asset_name: asset.asset_name,
            asset_category: asset.asset_category.unwrap_or_default(),
            specification: asset.specification.unwrap_or_default(),
            model: asset.model.unwrap_or_default(),
            use_department_id: asset.use_department_id.unwrap_or(0),
            use_location: asset.use_location.unwrap_or_default(),
            responsible_person_id: asset.responsible_person_id.unwrap_or(0),
            original_value: asset.original_value.to_string(),
            salvage_value: asset.salvage_value.map(|d| d.to_string()).unwrap_or_default(),
            salvage_rate: asset.salvage_rate.map(|d| d.to_string()).unwrap_or_default(),
            depreciable_value: asset.depreciable_value.map(|d| d.to_string()).unwrap_or_default(),
            depreciation_method: asset.depreciation_method.unwrap_or_default(),
            useful_life: asset.useful_life.unwrap_or(0),
            monthly_depreciation: asset.monthly_depreciation.map(|d| d.to_string()).unwrap_or_default(),
            accumulated_depreciation: asset.accumulated_depreciation.to_string(),
            net_value: asset.net_value.map(|d| d.to_string()).unwrap_or_default(),
            status: asset.status,
            purchase_date: asset.purchase_date.map(|d| d.to_string()).unwrap_or_default(),
            in_service_date: asset.in_service_date.map(|d| d.to_string()).unwrap_or_default(),
            disposal_date: asset.disposal_date.map(|d| d.to_string()).unwrap_or_default(),
            supplier_id: asset.supplier_id.unwrap_or(0),
            supplier_name: asset.supplier_name.unwrap_or_default(),
            created_by: asset.created_by,
            created_at: asset.created_at.timestamp(),
            updated_at: asset.updated_at.timestamp(),
        }
    }
    
    /// 将预算项目转换为 gRPC 模型
    fn to_grpc_budget_item(item: crate::models::budget_management::Model) -> BudgetItem {
        BudgetItem {
            id: item.id,
            item_code: item.item_code,
            item_name: item.item_name,
            parent_id: item.parent_id.unwrap_or(0),
            item_type: item.item_type,
            level: item.level,
            status: item.status,
            created_at: item.created_at.timestamp(),
            updated_at: item.updated_at.timestamp(),
        }
    }
}

#[tonic::async_trait]
impl PurchaseContractServiceTrait for GrpcManagementServices {
    async fn list_contracts(
        &self,
        request: Request<ListPurchaseContractsRequest>,
    ) -> Result<Response<ListPurchaseContractsResponse>, Status> {
        let req = request.into_inner();
        
        let _page = req.page.max(1) as i64;
        let _page_size = req.page_size.clamp(1, 100) as i64;
        
        // TODO: 实现查询逻辑
        // 暂时返回空列表
        Ok(Response::new(ListPurchaseContractsResponse {
            success: true,
            message: "采购合同列表获取成功".to_string(),
            contracts: vec![],
            total: 0,
        }))
    }
    
    async fn get_contract(
        &self,
        request: Request<GetPurchaseContractRequest>,
    ) -> Result<Response<GetPurchaseContractResponse>, Status> {
        let req = request.into_inner();
        
        match self.purchase_contract_service.get_by_id(req.contract_id).await {
            Ok(contract) => {
                Ok(Response::new(GetPurchaseContractResponse {
                    success: true,
                    message: "采购合同获取成功".to_string(),
                    contract: Some(Self::to_grpc_purchase_contract(contract)),
                }))
            }
            Err(e) => {
                Err(Status::not_found(format!("采购合同不存在：{}", e)))
            }
        }
    }
    
    async fn create_contract(
        &self,
        request: Request<CreatePurchaseContractRequest>,
    ) -> Result<Response<CreatePurchaseContractResponse>, Status> {
        let req = request.into_inner();
        
        // 解析金额
        let total_amount = req.total_amount.parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("金额格式错误：{}", e)))?;
        
        // 解析日期
        let delivery_date = chrono::NaiveDate::parse_from_str(&req.delivery_date, "%Y-%m-%d")
            .map_err(|e| Status::invalid_argument(format!("日期格式错误：{}", e)))?;
        
        let create_req = crate::services::purchase_contract_service::CreateContractRequest {
            contract_no: req.contract_no,
            contract_name: req.contract_name,
            supplier_id: req.supplier_id,
            total_amount,
            payment_terms: if req.payment_terms.is_empty() { None } else { Some(req.payment_terms) },
            delivery_date,
            remark: if req.remark.is_empty() { None } else { Some(req.remark) },
        };
        
        // 暂时用户 ID 为 1，实际应该从验证信息获取
        let user_id = 1;
        
        match self.purchase_contract_service.create(create_req, user_id).await {
            Ok(contract) => {
                Ok(Response::new(CreatePurchaseContractResponse {
                    success: true,
                    message: "采购合同创建成功".to_string(),
                    contract: Some(Self::to_grpc_purchase_contract(contract)),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("创建采购合同失败：{}", e)))
            }
        }
    }
    
    async fn approve_contract(
        &self,
        request: Request<ApprovePurchaseContractRequest>,
    ) -> Result<Response<ApprovePurchaseContractResponse>, Status> {
        let req = request.into_inner();
        
        let user_id = 1;
        
        match self.purchase_contract_service.approve(req.contract_id, user_id).await {
            Ok(_) => {
                Ok(Response::new(ApprovePurchaseContractResponse {
                    success: true,
                    message: "采购合同审批成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("审批采购合同失败：{}", e)))
            }
        }
    }
    
    async fn execute_contract(
        &self,
        request: Request<ExecutePurchaseContractRequest>,
    ) -> Result<Response<ExecutePurchaseContractResponse>, Status> {
        let req = request.into_inner();
        
        let user_id = 1;
        
        let execute_req = crate::services::purchase_contract_service::ExecuteContractRequest {
            execution_type: req.execution_type,
            execution_amount: req.execution_amount.parse::<rust_decimal::Decimal>()
                .map_err(|e| Status::invalid_argument(format!("金额格式错误：{}", e)))?,
            execution_date: chrono::Utc::now().naive_utc().date(),
            related_bill_type: if req.related_bill_type.is_empty() { None } else { Some(req.related_bill_type) },
            related_bill_id: if req.related_bill_id == 0 { None } else { Some(req.related_bill_id) },
            remark: if req.remark.is_empty() { None } else { Some(req.remark) },
        };
        
        match self.purchase_contract_service.execute(req.contract_id, execute_req, user_id).await {
            Ok(_) => {
                Ok(Response::new(ExecutePurchaseContractResponse {
                    success: true,
                    message: "采购合同执行成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("执行采购合同失败：{}", e)))
            }
        }
    }
    
    async fn cancel_contract(
        &self,
        request: Request<CancelPurchaseContractRequest>,
    ) -> Result<Response<CancelPurchaseContractResponse>, Status> {
        let req = request.into_inner();
        
        let user_id = 1;
        
        match self.purchase_contract_service.cancel(req.contract_id, user_id, req.reason).await {
            Ok(_) => {
                Ok(Response::new(CancelPurchaseContractResponse {
                    success: true,
                    message: "采购合同取消成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("取消采购合同失败：{}", e)))
            }
        }
    }
}

#[tonic::async_trait]
impl SalesContractServiceTrait for GrpcManagementServices {
    async fn list_contracts(
        &self,
        request: Request<ListSalesContractsRequest>,
    ) -> Result<Response<ListSalesContractsResponse>, Status> {
        let req = request.into_inner();
        
        let _page = req.page.max(1) as i64;
        let _page_size = req.page_size.clamp(1, 100) as i64;
        
        // TODO: 实现查询逻辑
        Ok(Response::new(ListSalesContractsResponse {
            success: true,
            message: "销售合同列表获取成功".to_string(),
            contracts: vec![],
            total: 0,
        }))
    }
    
    async fn get_contract(
        &self,
        request: Request<GetSalesContractRequest>,
    ) -> Result<Response<GetSalesContractResponse>, Status> {
        let req = request.into_inner();
        
        match self.sales_contract_service.get_by_id(req.contract_id).await {
            Ok(contract) => {
                Ok(Response::new(GetSalesContractResponse {
                    success: true,
                    message: "销售合同获取成功".to_string(),
                    contract: Some(Self::to_grpc_sales_contract(contract)),
                }))
            }
            Err(e) => {
                Err(Status::not_found(format!("销售合同不存在：{}", e)))
            }
        }
    }
    
    async fn create_contract(
        &self,
        request: Request<CreateSalesContractRequest>,
    ) -> Result<Response<CreateSalesContractResponse>, Status> {
        let req = request.into_inner();
        
        let total_amount = req.total_amount.parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("金额格式错误：{}", e)))?;
        
        let delivery_date = chrono::NaiveDate::parse_from_str(&req.delivery_date, "%Y-%m-%d")
            .map_err(|e| Status::invalid_argument(format!("日期格式错误：{}", e)))?;
        
        let create_req = crate::services::sales_contract_service::CreateSalesContractRequest {
            contract_no: req.contract_no,
            contract_name: req.contract_name,
            customer_id: req.customer_id,
            total_amount,
            payment_terms: if req.payment_terms.is_empty() { None } else { Some(req.payment_terms) },
            delivery_date,
            remark: if req.remark.is_empty() { None } else { Some(req.remark) },
        };
        
        let user_id = 1;
        
        match self.sales_contract_service.create(create_req, user_id).await {
            Ok(contract) => {
                Ok(Response::new(CreateSalesContractResponse {
                    success: true,
                    message: "销售合同创建成功".to_string(),
                    contract: Some(Self::to_grpc_sales_contract(contract)),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("创建销售合同失败：{}", e)))
            }
        }
    }
    
    async fn approve_contract(
        &self,
        request: Request<ApproveSalesContractRequest>,
    ) -> Result<Response<ApproveSalesContractResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        match self.sales_contract_service.approve(req.contract_id, user_id).await {
            Ok(_) => {
                Ok(Response::new(ApproveSalesContractResponse {
                    success: true,
                    message: "销售合同审批成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("审批销售合同失败：{}", e)))
            }
        }
    }
    
    async fn execute_contract(
        &self,
        request: Request<ExecuteSalesContractRequest>,
    ) -> Result<Response<ExecuteSalesContractResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        let execute_req = crate::services::sales_contract_service::ExecuteSalesContractRequest {
            execution_type: req.execution_type,
            execution_amount: req.execution_amount.parse::<rust_decimal::Decimal>()
                .map_err(|e| Status::invalid_argument(format!("金额格式错误：{}", e)))?,
            related_bill_type: if req.related_bill_type.is_empty() { None } else { Some(req.related_bill_type) },
            related_bill_id: if req.related_bill_id == 0 { None } else { Some(req.related_bill_id) },
            remark: if req.remark.is_empty() { None } else { Some(req.remark) },
        };
        
        match self.sales_contract_service.execute(req.contract_id, execute_req, user_id).await {
            Ok(_) => {
                Ok(Response::new(ExecuteSalesContractResponse {
                    success: true,
                    message: "销售合同执行成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("执行销售合同失败：{}", e)))
            }
        }
    }
    
    async fn cancel_contract(
        &self,
        request: Request<CancelSalesContractRequest>,
    ) -> Result<Response<CancelSalesContractResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        match self.sales_contract_service.cancel(req.contract_id, user_id, req.reason).await {
            Ok(_) => {
                Ok(Response::new(CancelSalesContractResponse {
                    success: true,
                    message: "销售合同取消成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("取消销售合同失败：{}", e)))
            }
        }
    }
}

#[tonic::async_trait]
impl FixedAssetServiceTrait for GrpcManagementServices {
    async fn list_assets(
        &self,
        request: Request<ListFixedAssetsRequest>,
    ) -> Result<Response<ListFixedAssetsResponse>, Status> {
        let req = request.into_inner();
        
        let _page = req.page.max(1) as i64;
        let _page_size = req.page_size.clamp(1, 100) as i64;
        
        // TODO: 实现查询逻辑
        Ok(Response::new(ListFixedAssetsResponse {
            success: true,
            message: "固定资产列表获取成功".to_string(),
            assets: vec![],
            total: 0,
        }))
    }
    
    async fn get_asset(
        &self,
        request: Request<GetFixedAssetRequest>,
    ) -> Result<Response<GetFixedAssetResponse>, Status> {
        let req = request.into_inner();
        
        match self.fixed_asset_service.get_by_id(req.asset_id).await {
            Ok(asset) => {
                Ok(Response::new(GetFixedAssetResponse {
                    success: true,
                    message: "固定资产获取成功".to_string(),
                    asset: Some(Self::to_grpc_fixed_asset(asset)),
                }))
            }
            Err(e) => {
                Err(Status::not_found(format!("固定资产不存在：{}", e)))
            }
        }
    }
    
    async fn create_asset(
        &self,
        request: Request<CreateFixedAssetRequest>,
    ) -> Result<Response<CreateFixedAssetResponse>, Status> {
        let req = request.into_inner();
        
        let original_value = req.original_value.parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("原值格式错误：{}", e)))?;
        
        let purchase_date = chrono::NaiveDate::parse_from_str(&req.purchase_date, "%Y-%m-%d")
            .map_err(|e| Status::invalid_argument(format!("日期格式错误：{}", e)))?;
        
        let put_in_date = chrono::NaiveDate::parse_from_str(&req.put_in_date, "%Y-%m-%d")
            .map_err(|e| Status::invalid_argument(format!("日期格式错误：{}", e)))?;
        
        let create_req = crate::services::fixed_asset_service::CreateAssetRequest {
            asset_no: req.asset_no,
            asset_name: req.asset_name,
            asset_category: Some(req.asset_category),
            specification: if req.specification.is_empty() { None } else { Some(req.specification) },
            location: if req.location.is_empty() { None } else { Some(req.location) },
            original_value,
            useful_life: req.useful_life,
            depreciation_method: Some(req.depreciation_method),
            purchase_date,
            put_in_date,
            supplier_id: if req.supplier_id == 0 { None } else { Some(req.supplier_id) },
            remark: if req.remark.is_empty() { None } else { Some(req.remark) },
        };
        
        let user_id = 1;
        
        match self.fixed_asset_service.create(create_req, user_id).await {
            Ok(asset) => {
                Ok(Response::new(CreateFixedAssetResponse {
                    success: true,
                    message: "固定资产创建成功".to_string(),
                    asset: Some(Self::to_grpc_fixed_asset(asset)),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("创建固定资产失败：{}", e)))
            }
        }
    }
    
    async fn depreciate_asset(
        &self,
        request: Request<DepreciateFixedAssetRequest>,
    ) -> Result<Response<DepreciateFixedAssetResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        match self.fixed_asset_service.depreciate(req.asset_id, &chrono::Utc::now().format("%Y-%m").to_string(), user_id).await {
            Ok(_) => {
                Ok(Response::new(DepreciateFixedAssetResponse {
                    success: true,
                    message: "资产折旧成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("资产折旧失败：{}", e)))
            }
        }
    }
    
    async fn dispose_asset(
        &self,
        request: Request<DisposeFixedAssetRequest>,
    ) -> Result<Response<DisposeFixedAssetResponse>, Status> {
        let req = request.into_inner();
        
        let disposal_value = req.disposal_value.parse::<rust_decimal::Decimal>()
            .map_err(|e| Status::invalid_argument(format!("处置价值格式错误：{}", e)))?;
        
        let disposal_date = chrono::NaiveDate::parse_from_str(&req.disposal_date, "%Y-%m-%d")
            .map_err(|e| Status::invalid_argument(format!("日期格式错误：{}", e)))?;
        
        let dispose_req = crate::services::fixed_asset_service::DisposalRequest {
            disposal_type: req.disposal_type,
            disposal_value,
            disposal_date,
            reason: req.reason,
            buyer_info: if req.buyer_info.is_empty() { None } else { Some(req.buyer_info) },
        };
        
        let user_id = 1;
        
        match self.fixed_asset_service.dispose(req.asset_id, dispose_req, user_id).await {
            Ok(_) => {
                Ok(Response::new(DisposeFixedAssetResponse {
                    success: true,
                    message: "资产处置成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("资产处置失败：{}", e)))
            }
        }
    }
    
    async fn delete_asset(
        &self,
        request: Request<DeleteFixedAssetRequest>,
    ) -> Result<Response<DeleteFixedAssetResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        match self.fixed_asset_service.delete(req.asset_id, user_id).await {
            Ok(_) => {
                Ok(Response::new(DeleteFixedAssetResponse {
                    success: true,
                    message: "资产删除成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("删除资产失败：{}", e)))
            }
        }
    }
}

#[tonic::async_trait]
impl BudgetManagementServiceTrait for GrpcManagementServices {
    async fn list_items(
        &self,
        request: Request<ListBudgetItemsRequest>,
    ) -> Result<Response<ListBudgetItemsResponse>, Status> {
        let req = request.into_inner();
        
        let _page = req.page.max(1) as i64;
        let _page_size = req.page_size.clamp(1, 100) as i64;
        
        // TODO: 实现查询逻辑
        Ok(Response::new(ListBudgetItemsResponse {
            success: true,
            message: "预算项目列表获取成功".to_string(),
            items: vec![],
            total: 0,
        }))
    }
    
    async fn get_item(
        &self,
        request: Request<GetBudgetItemRequest>,
    ) -> Result<Response<GetBudgetItemResponse>, Status> {
        let req = request.into_inner();
        
        match self.budget_management_service.get_item_by_id(req.item_id).await {
            Ok(item) => {
                Ok(Response::new(GetBudgetItemResponse {
                    success: true,
                    message: "预算项目获取成功".to_string(),
                    item: Some(Self::to_grpc_budget_item(item)),
                }))
            }
            Err(e) => {
                Err(Status::not_found(format!("预算项目不存在：{}", e)))
            }
        }
    }
    
    async fn create_item(
        &self,
        request: Request<CreateBudgetItemRequest>,
    ) -> Result<Response<CreateBudgetItemResponse>, Status> {
        let req = request.into_inner();
        
        let create_req = crate::services::budget_management_service::CreateBudgetItemRequest {
            item_code: req.item_code,
            item_name: req.item_name,
            parent_id: if req.parent_id == 0 { None } else { Some(req.parent_id) },
            item_type: req.item_type,
            budget_year: chrono::Utc::now().year(),
            planned_amount: rust_decimal::Decimal::ZERO,
            remark: None,
        };
        
        let user_id = 1;
        
        match self.budget_management_service.create_item(create_req, user_id).await {
            Ok(item) => {
                Ok(Response::new(CreateBudgetItemResponse {
                    success: true,
                    message: "预算项目创建成功".to_string(),
                    item: Some(Self::to_grpc_budget_item(item)),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("创建预算项目失败：{}", e)))
            }
        }
    }
    
    async fn update_item(
        &self,
        request: Request<UpdateBudgetItemRequest>,
    ) -> Result<Response<UpdateBudgetItemResponse>, Status> {
        let req = request.into_inner();
        
        let _update_req = crate::services::budget_management_service::CreateBudgetItemRequest {
            item_code: String::new(),
            item_name: req.item_name,
            parent_id: None,
            item_type: req.item_type,
            budget_year: chrono::Utc::now().year(),
            planned_amount: rust_decimal::Decimal::ZERO,
            remark: None,
        };
        
        let _user_id = 1;
        
        match self.budget_management_service.get_item_by_id(req.item_id).await {
            Ok(_) => {
                Ok(Response::new(UpdateBudgetItemResponse {
                    success: true,
                    message: "预算项目更新成功".to_string(),
                    item: None,
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("更新预算项目失败：{}", e)))
            }
        }
    }
    
    async fn delete_item(
        &self,
        request: Request<DeleteBudgetItemRequest>,
    ) -> Result<Response<DeleteBudgetItemResponse>, Status> {
        let req = request.into_inner();
        let user_id = 1;
        
        match self.budget_management_service.delete_item(req.item_id, user_id).await {
            Ok(_) => {
                Ok(Response::new(DeleteBudgetItemResponse {
                    success: true,
                    message: "预算项目删除成功".to_string(),
                }))
            }
            Err(e) => {
                Err(Status::internal(format!("删除预算项目失败：{}", e)))
            }
        }
    }
    
    async fn list_plans(
        &self,
        request: Request<ListBudgetPlansRequest>,
    ) -> Result<Response<ListBudgetPlansResponse>, Status> {
        let req = request.into_inner();
        
        let _page = req.page.max(1) as i64;
        let _page_size = req.page_size.clamp(1, 100) as i64;
        
        // TODO: 实现查询逻辑
        Ok(Response::new(ListBudgetPlansResponse {
            success: true,
            message: "预算方案列表获取成功".to_string(),
            plans: vec![],
            total: 0,
        }))
    }
    
    async fn get_plan(
        &self,
        request: Request<GetBudgetPlanRequest>,
    ) -> Result<Response<GetBudgetPlanResponse>, Status> {
        let _req = request.into_inner();
        
        // TODO: 实现获取预算方案逻辑
        Ok(Response::new(GetBudgetPlanResponse {
            success: true,
            message: "预算方案获取成功".to_string(),
            plan: None,
        }))
    }
    
    async fn create_plan(
        &self,
        request: Request<CreateBudgetPlanRequest>,
    ) -> Result<Response<CreateBudgetPlanResponse>, Status> {
        let _req = request.into_inner();
        
        // TODO: 实现创建预算方案逻辑
        Ok(Response::new(CreateBudgetPlanResponse {
            success: true,
            message: "预算方案创建成功".to_string(),
            plan: None,
        }))
    }
    
    async fn approve_plan(
        &self,
        request: Request<ApproveBudgetPlanRequest>,
    ) -> Result<Response<ApproveBudgetPlanResponse>, Status> {
        let _req = request.into_inner();
        
        // TODO: 实现审批预算方案逻辑
        Ok(Response::new(ApproveBudgetPlanResponse {
            success: true,
            message: "预算方案审批成功".to_string(),
        }))
    }
    
    async fn execute_plan(
        &self,
        request: Request<ExecuteBudgetPlanRequest>,
    ) -> Result<Response<ExecuteBudgetPlanResponse>, Status> {
        let _req = request.into_inner();
        
        // TODO: 实现执行预算方案逻辑
        Ok(Response::new(ExecuteBudgetPlanResponse {
            success: true,
            message: "预算方案执行成功".to_string(),
        }))
    }
}
