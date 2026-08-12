    use bingxi_backend::services::so::order_crud::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_crud_module_loaded() {
        assert_eq!(P92_CRUD_MODULE, "sales_order_crud");
    }
}