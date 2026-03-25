use chrono::{Utc, NaiveDate};
// 四级批次管理服务测试
// 测试四级批次管理的核心功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::FourLevelBatchService;
    use chrono::NaiveDate;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;
    use rust_decimal::Decimal;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_dye_lot() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let req = crate::services::four_level_batch_service::CreateDyeLotRequest {
            product_id: 1,
            color_id: 1,
            supplier_dye_lot_no: "SUP-DL-001".to_string(),
            supplier_id: 1,
            production_date: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            machine_no: Some("M001".to_string()),
            batch_weight: Some(Decimal::new(1000, 2)),
            quality_grade: Some("A".to_string()),
            created_by: 1,
        };
        
        let result = service.create_dye_lot(req).await;
        
        assert!(result.is_ok());
        let dye_lot = result.unwrap();
        assert!(dye_lot.dye_lot_no.starts_with("DL"));
        assert_eq!(dye_lot.supplier_dye_lot_no, "SUP-DL-001");
    }

    #[tokio::test]
    async fn test_create_piece() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let req = crate::services::four_level_batch_service::CreatePieceRequest {
            dye_lot_id: 1,
            supplier_piece_no: "SUP-PIECE-001".to_string(),
            length: Decimal::new(100, 2),
            weight: Some(Decimal::new(50, 2)),
            quality_status: Some("合格".to_string()),
            production_date: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            created_by: 1,
        };
        
        let result = service.create_piece(req).await;
        
        assert!(result.is_ok());
        let piece = result.unwrap();
        assert!(piece.piece_no.starts_with("P"));
        assert_eq!(piece.supplier_piece_no, "SUP-PIECE-001");
    }

    #[tokio::test]
    async fn test_create_code_mapping() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let req = crate::services::four_level_batch_service::CreateCodeMappingRequest {
            mapping_type: "product".to_string(),
            internal_code: "INT-001".to_string(),
            supplier_code: "SUP-001".to_string(),
            supplier_id: 1,
            is_active: true,
            created_by: 1,
        };
        
        let result = service.create_code_mapping(req).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_supplier_code_by_internal() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let result = service.get_supplier_code_by_internal(
            "INT-001",
            "product",
            1
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_internal_code_by_supplier() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let result = service.get_internal_code_by_supplier(
            "SUP-001",
            "product",
            1
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_conversion() {
        let db = setup_test_db().await;
        let service = FourLevelBatchService::new(Arc::new(db));
        
        let req = crate::services::four_level_batch_service::BatchConversionRequest {
            supplier_product_code: "SUP-PROD-001".to_string(),
            supplier_color_code: "SUP-COLOR-001".to_string(),
            supplier_dye_lot_no: "SUP-DL-001".to_string(),
            supplier_piece_nos: vec!["SUP-PIECE-001".to_string()],
            supplier_id: 1,
            operator_id: 1,
        };
        
        let result = service.batch_conversion(req).await;
        
        assert!(result.is_ok());
        let conversion = result.unwrap();
        assert!(!conversion.internal_product_code.is_empty());
        assert!(!conversion.internal_color_no.is_empty());
        assert!(!conversion.internal_dye_lot_no.is_empty());
    }
}
