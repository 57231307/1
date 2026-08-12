#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stage_name() {
        assert_eq!(get_stage_name("PURCHASE_RECEIPT"), "采购收货");
        assert_eq!(get_stage_name("SALES_DELIVERY"), "销售发货");
        assert!(get_stage_name("UNKNOWN").contains("未知"));
    }
}