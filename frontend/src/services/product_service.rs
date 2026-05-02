#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use crate::models::product::{
    CreateProductRequest, Product, ProductListResponse, UpdateProductRequest,
};
use crate::services::crud_service::CrudService;

pub struct ProductService;

impl CrudService for ProductService {
    type Model = Product;
    type ListResponse = ProductListResponse;
    type CreateRequest = CreateProductRequest;
    type UpdateRequest = UpdateProductRequest;

    fn base_path() -> &'static str {
        "/products"
    }
}
