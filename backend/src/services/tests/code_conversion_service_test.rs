//! 编码转换服务测试
//! 测试编码转换的核心功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::CodeConversionService;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_convert_supplier_to_internal() {
        let db = setup_test_db().await;
        let service = CodeConversionService::new(Arc::new(db));
        
        let result = service.convert_supplier_to_internal(
            "SUP-PROD-001",
            "SUP-COLOR-001",
            "SUP-DL-001",
            &vec!["SUP-PIECE-001".to_string(), "SUP-PIECE-002".to_string()],
            1,
            1
        ).await;
        
        assert!(result.is_ok());
        let conversion = result.unwrap();
        assert!(!conversion.internal_product_code.is_empty());
        assert!(!conversion.internal_color_no.is_empty());
        assert!(!conversion.internal_dye_lot_no.is_empty());
        assert!(!conversion.internal_piece_nos.is_empty());
        assert_eq!(conversion.supplier_product_code, "SUP-PROD-001");
        assert_eq!(conversion.supplier_color_code, "SUP-COLOR-001");
        assert_eq!(conversion.supplier_dye_lot_no, "SUP-DL-001");
    }

    #[tokio::test]
    async fn test_convert_internal_to_supplier() {
        let db = setup_test_db().await;
        let service = CodeConversionService::new(Arc::new(db));
        
        let result = service.convert_internal_to_supplier(
            "INT-PROD-001",
            "INT-COLOR-001",
            "INT-DL-001",
            &vec!["INT-PIECE-001".to_string(), "INT-PIECE-002".to_string()],
            1,
            1
        ).await;
        
        assert!(result.is_ok());
        let conversion = result.unwrap();
        assert!(!conversion.supplier_product_code.is_empty());
        assert!(!conversion.supplier_color_code.is_empty());
        assert!(!conversion.supplier_dye_lot_no.is_empty());
        assert!(!conversion.supplier_piece_nos.is_empty());
        assert_eq!(conversion.internal_product_code, "INT-PROD-001");
        assert_eq!(conversion.internal_color_no, "INT-COLOR-001");
        assert_eq!(conversion.internal_dye_lot_no, "INT-DL-001");
    }

    #[tokio::test]
    async fn test_validate_conversion_success() {
        let db = setup_test_db().await;
        let service = CodeConversionService::new(Arc::new(db));
        
        let result = service.validate_conversion(
            "INT-PROD-001",
            "INT-COLOR-001",
            "INT-DL-001",
            &vec!["INT-PIECE-001".to_string()],
            "SUP-PROD-001",
            "SUP-COLOR-001",
            "SUP-DL-001",
            &vec!["SUP-PIECE-001".to_string()]
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_conversion_workflow() {
        let db = setup_test_db().await;
        let service = CodeConversionService::new(Arc::new(db));
        
        let supplier_product_code = "SUP-PROD-001";
        let supplier_color_code = "SUP-COLOR-001";
        let supplier_dye_lot_no = "SUP-DL-001";
        let supplier_piece_nos = vec!["SUP-PIECE-001".to_string()];
        
        let result = service.convert_supplier_to_internal(
            supplier_product_code,
            supplier_color_code,
            supplier_dye_lot_no,
            &supplier_piece_nos,
            1,
            1
        ).await;
        
        assert!(result.is_ok());
        let conversion_result = result.unwrap();
        
        assert_eq!(conversion_result.validation_result, "success");
        assert!(conversion_result.validation_message.is_empty());
    }
}
