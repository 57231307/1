use crate::models::warehouse::{
    CreateWarehouseRequest, Warehouse, WarehouseListResponse, UpdateWarehouseRequest,
};
use crate::services::crud_service::CrudService;

pub struct WarehouseService;

impl CrudService for WarehouseService {
    type Model = Warehouse;
    type ListResponse = WarehouseListResponse;
    type CreateRequest = CreateWarehouseRequest;
    type UpdateRequest = UpdateWarehouseRequest;

    fn base_path() -> &'static str {
        "/warehouses"
    }
}
