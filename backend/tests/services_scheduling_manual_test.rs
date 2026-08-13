use bingxi_backend::services::scheduling_manual::P92_MANUAL_MODULE;

#[test]
fn test_module_loaded() {
    assert_eq!(P92_MANUAL_MODULE, "scheduling_manual");
}