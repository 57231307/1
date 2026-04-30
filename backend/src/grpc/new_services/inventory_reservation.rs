use tonic::{Request, Response, Status};
use crate::grpc::new_services::GrpcNewServices;
use crate::grpc::new_services::support::{parse_decimal, empty_to_option, id_to_option, operator_id, handle_error};

use crate::grpc::service::proto::{
    inventory_reservation_service_server::InventoryReservationService as InventoryReservationServiceTrait,
    CreateReservationRequest, CreateReservationResponse,
    LockReservationRequest, LockReservationResponse,
    ReleaseReservationRequest, ReleaseReservationResponse,
    UseReservationRequest, UseReservationResponse,
    GetReservationsByOrderRequest, GetReservationsByOrderResponse,
    GetLockedReservationsByOrderRequest, GetLockedReservationsByOrderResponse,
    BatchCreateReservationsRequest, BatchCreateReservationsResponse,
    BatchLockReservationsRequest, BatchLockReservationsResponse,
    BatchReleaseReservationsRequest, BatchReleaseReservationsResponse,
    InventoryReservation,
};

/// 将库存预留转换为 gRPC 模型
fn to_grpc_reservation(reservation: crate::models::inventory_reservation::Model) -> InventoryReservation {
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

#[tonic::async_trait]
impl InventoryReservationServiceTrait for GrpcNewServices {
    async fn create_reservation(
        &self,
        request: Request<CreateReservationRequest>,
    ) -> Result<Response<CreateReservationResponse>, Status> {
        // 对于创建操作，我们可以尝试获取上下文中的操作人ID，获取不到就 fallback 到请求里的 created_by (兼容原有行为或测试)
        let mut created_by = operator_id(&request).ok();
        
        let req = request.into_inner();
        let quantity = parse_decimal("数量", &req.quantity)?;
        
        if created_by.is_none() {
            created_by = id_to_option(req.created_by);
        }
        
        let notes = empty_to_option(req.notes);
        
        match self.inventory_reservation_service.create_reservation(
            req.order_id,
            req.product_id,
            req.warehouse_id,
            quantity,
            created_by,
            notes,
        ).await {
            Ok(reservation) => Ok(Response::new(CreateReservationResponse {
                success: true,
                message: "库存预留创建成功".to_string(),
                reservation: Some(to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(handle_error("创建库存预留失败", e)),
        }
    }
    
    async fn lock_reservation(
        &self,
        request: Request<LockReservationRequest>,
    ) -> Result<Response<LockReservationResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.lock_reservation(req.reservation_id).await {
            Ok(reservation) => Ok(Response::new(LockReservationResponse {
                success: true,
                message: "库存预留锁定成功".to_string(),
                reservation: Some(to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(handle_error("锁定库存预留失败", e)),
        }
    }
    
    async fn release_reservation(
        &self,
        request: Request<ReleaseReservationRequest>,
    ) -> Result<Response<ReleaseReservationResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.release_reservation(req.reservation_id).await {
            Ok(reservation) => Ok(Response::new(ReleaseReservationResponse {
                success: true,
                message: "库存预留释放成功".to_string(),
                reservation: Some(to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(handle_error("释放库存预留失败", e)),
        }
    }
    
    async fn use_reservation(
        &self,
        request: Request<UseReservationRequest>,
    ) -> Result<Response<UseReservationResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.use_reservation(req.reservation_id).await {
            Ok(reservation) => Ok(Response::new(UseReservationResponse {
                success: true,
                message: "库存预留使用成功".to_string(),
                reservation: Some(to_grpc_reservation(reservation)),
            })),
            Err(e) => Err(handle_error("使用库存预留失败", e)),
        }
    }
    
    async fn get_reservations_by_order(
        &self,
        request: Request<GetReservationsByOrderRequest>,
    ) -> Result<Response<GetReservationsByOrderResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.get_reservations_by_order(req.order_id).await {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations.into_iter()
                    .map(to_grpc_reservation)
                    .collect();
                Ok(Response::new(GetReservationsByOrderResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(handle_error("查询库存预留失败", e)),
        }
    }
    
    async fn get_locked_reservations_by_order(
        &self,
        request: Request<GetLockedReservationsByOrderRequest>,
    ) -> Result<Response<GetLockedReservationsByOrderResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.get_locked_reservations_by_order(req.order_id).await {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations.into_iter()
                    .map(to_grpc_reservation)
                    .collect();
                Ok(Response::new(GetLockedReservationsByOrderResponse {
                    success: true,
                    message: "查询成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(handle_error("查询库存预留失败", e)),
        }
    }
    
    async fn batch_create_reservations(
        &self,
        request: Request<BatchCreateReservationsRequest>,
    ) -> Result<Response<BatchCreateReservationsResponse>, Status> {
        let mut created_by = operator_id(&request).ok();
        let req = request.into_inner();
        
        let mut items = Vec::new();
        for item in req.items {
            let quantity = parse_decimal("数量", &item.quantity)?;
            items.push((item.product_id, item.warehouse_id, quantity));
        }
        
        if created_by.is_none() {
            created_by = id_to_option(req.created_by);
        }
        
        match self.inventory_reservation_service.batch_create_reservations(req.order_id, items, created_by).await {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations.into_iter()
                    .map(to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchCreateReservationsResponse {
                    success: true,
                    message: "批量创建库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(handle_error("批量创建库存预留失败", e)),
        }
    }
    
    async fn batch_lock_reservations(
        &self,
        request: Request<BatchLockReservationsRequest>,
    ) -> Result<Response<BatchLockReservationsResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.batch_lock_reservations(req.reservation_ids).await {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations.into_iter()
                    .map(to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchLockReservationsResponse {
                    success: true,
                    message: "批量锁定库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(handle_error("批量锁定库存预留失败", e)),
        }
    }
    
    async fn batch_release_reservations(
        &self,
        request: Request<BatchReleaseReservationsRequest>,
    ) -> Result<Response<BatchReleaseReservationsResponse>, Status> {
        let req = request.into_inner();
        
        match self.inventory_reservation_service.batch_release_reservations(req.reservation_ids).await {
            Ok(reservations) => {
                let grpc_reservations: Vec<InventoryReservation> = reservations.into_iter()
                    .map(to_grpc_reservation)
                    .collect();
                Ok(Response::new(BatchReleaseReservationsResponse {
                    success: true,
                    message: "批量释放库存预留成功".to_string(),
                    reservations: grpc_reservations,
                }))
            }
            Err(e) => Err(handle_error("批量释放库存预留失败", e)),
        }
    }
}
