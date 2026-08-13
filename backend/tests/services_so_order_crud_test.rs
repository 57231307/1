use bingxi_backend::services::so::order_crud::*;


#[test]
fn test_crud_module_loaded() {
    assert_eq!(P92_CRUD_MODULE, "sales_order_crud");
}