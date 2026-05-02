use crate::models::product_category::{ProductCategory, CreateProductCategoryRequest, UpdateProductCategoryRequest};
use crate::services::crud_service::CrudService;

pub struct ProductCategoryService;

impl CrudService for ProductCategoryService {
    type Model = ProductCategory;
    type ListResponse = Vec<ProductCategory>;
    type CreateRequest = CreateProductCategoryRequest;
    type UpdateRequest = UpdateProductCategoryRequest;

    fn base_path() -> &'static str {
        "/products/categories"
    }
}
