//! 客户管理服务 API 客户端
//! 提供客户管理相关的 API 调用方法

use crate::models::customer::{
    Customer, CustomerQuery, CustomerListResponse, CreateCustomerRequest, UpdateCustomerRequest,
};
use crate::services::crud_service::CrudService;

pub struct CustomerService;

impl CrudService for CustomerService {
    type Model = Customer;
    type ListResponse = CustomerListResponse;
    type CreateRequest = CreateCustomerRequest;
    type UpdateRequest = UpdateCustomerRequest;

    fn base_path() -> &'static str {
        "/customers"
    }
}
