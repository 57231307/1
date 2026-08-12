#[cfg(test)]
mod tests {
use bingxi_backend::services::scheduling_auto::*;


    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_AUTO_MODULE, "scheduling_auto");
    }
}