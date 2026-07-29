//! 销售报价单服务层（facade）
//!
//! D11 拆分：保留 QuotationService struct/构造函数/ServiceError/单元测试，
//! 业务方法（create_draft/list/get_by_id/update/cancel + helpers）迁移至 quotation_ops 子模块。

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::utils::error::AppError;

/// 业务错误
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("报价单不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 265：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 销售报价单服务
pub struct QuotationService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl QuotationService {
    /// 从数据库连接直接构造（与项目其他服务保持一致）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::models::quotation_create_dto::{CreateQuotationDto, CreateQuotationItemDto};
    use crate::models::quotation_update_dto::UpdateQuotationDto;
    use crate::models::status::quotation as quotation_status;
    use crate::services::test_common::setup_test_db;
    use crate::ymd;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    /// 构造合法的 CreateQuotationItemDto（单条明细）
    fn sample_item() -> CreateQuotationItemDto {
        CreateQuotationItemDto {
            product_id: 1001,
            color_id: Some(2001),
            specification: Some("规格 A".to_string()),
            unit: "M".to_string(),
            quantity: decs!(100),
            unit_price: decs!(10),
            unit_price_with_tax: decs!(11.3),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        }
    }

    /// 构造合法的 CreateQuotationDto（默认 FOB + 不含税 + 13% 税率）
    fn sample_dto() -> CreateQuotationDto {
        CreateQuotationDto {
            customer_id: 1,
            sales_user_id: 10,
            quotation_date: ymd!(2026, 7, 19),
            valid_until: ymd!(2026, 8, 19),
            currency: "CNY".to_string(),
            exchange_rate: Decimal::ONE,
            base_currency: "CNY".to_string(),
            price_terms: "FOB".to_string(),
            incoterms_version: Some("2020".to_string()),
            incoterm_location: Some("Shanghai".to_string()),
            tax_inclusive: false,
            tax_rate: decs!(13),
            moq: Some(decs!(50)),
            lead_time_days: Some(30),
            customer_level: Some("A".to_string()),
            notes: Some("测试报价单".to_string()),
            items: vec![sample_item()],
            terms: None,
        }
    }

    // ============ ServiceError 枚举值正确性测试 ============

    /// test_serviceerror_display_gszq
    /// 验证 5 个 ServiceError 变体的 Display 实现返回中文错误信息
    #[test]
    fn test_serviceerror_display_gszq() {
        assert_eq!(ServiceError::NotFound.to_string(), "报价单不存在");
        assert_eq!(
            ServiceError::InvalidState.to_string(),
            "当前状态不允许此操作"
        );
        assert_eq!(
            ServiceError::Validation("明细至少 1 条".to_string()).to_string(),
            "参数校验失败: 明细至少 1 条"
        );
        let db_err = ServiceError::Database(sea_orm::DbErr::RecordNotFound("test".to_string()));
        assert!(db_err.to_string().starts_with("数据库错误:"));
    }

    // ============ validate_create 业务校验测试 ============

    /// test_validate_create_kmxjj
    #[tokio::test]
    async fn test_validate_create_kmxjj() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items.clear();
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            assert!(msg.contains("明细至少 1 条"));
        }
    }

    /// test_validate_create_yxqzybjrqjj
    #[tokio::test]
    async fn test_validate_create_yxqzybjrqjj() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.valid_until = ymd!(2026, 6, 19);
        dto.quotation_date = ymd!(2026, 7, 19);
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            assert!(msg.contains("有效期截止必须不早于报价日期"));
        }
    }

    /// test_validate_create_ffmysyjj
    #[tokio::test]
    async fn test_validate_create_ffmysyjj() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.price_terms = "XYZ".to_string();
        let result = svc.validate_create(&dto);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
        if let Err(ServiceError::Validation(msg)) = result {
            assert!(msg.contains("FOB") || msg.contains("合法取值"));
        }
    }

    /// test_validate_create_hfcstg
    #[tokio::test]
    async fn test_validate_create_hfcstg() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto();
        let result = svc.validate_create(&dto);
        assert!(result.is_ok());
    }

    // ============ calculate_totals 金额计算测试 ============

    /// test_calculate_totals_bhsjejszq
    #[tokio::test]
    async fn test_calculate_totals_bhsjejszq() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto();
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, decs!(130));
        assert_eq!(total_amount, decs!(1130));
    }

    /// test_calculate_totals_hsjesewl
    #[tokio::test]
    async fn test_calculate_totals_hsjesewl() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.tax_inclusive = true;
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, Decimal::ZERO);
        assert_eq!(total_amount, decs!(1000));
    }

    /// test_calculate_totals_dmxhzzq
    #[tokio::test]
    async fn test_calculate_totals_dmxhzzq() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items.push(CreateQuotationItemDto {
            product_id: 1002,
            color_id: None,
            specification: None,
            unit: "M".to_string(),
            quantity: decs!(200),
            unit_price: decs!(20),
            unit_price_with_tax: decs!(22.6),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        });
        let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(5000));
    }

    /// test_calculate_totals_jdgyd2wxs
    /// 批次 87：33.333 * 3 = 99.999 → 100.00（round_dp(2)）
    #[tokio::test]
    async fn test_calculate_totals_jdgyd2wxs() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.items = vec![CreateQuotationItemDto {
            product_id: 1,
            color_id: None,
            specification: None,
            unit: "M".to_string(),
            quantity: decs!(3),
            unit_price: decs!(33.333),
            unit_price_with_tax: decs!(33.333),
            tier_pricing: None,
            discount_rate: None,
            notes: None,
        }];
        let (subtotal, _, _) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(100));
    }

    // ============ validate_price_terms 贸易术语校验测试 ============

    /// test_validate_price_terms_hfdmfhmj
    #[test]
    fn test_validate_price_terms_hfdmfhmj() {
        let valid_codes = [
            "EXW", "FCA", "CPT", "CIP", "DAP", "DPU", "DDP", "FAS", "FOB", "CFR", "CIF",
        ];
        for code in valid_codes {
            let result = QuotationService::validate_price_terms(code);
            assert!(result.is_ok(), "合法代码 {} 应通过校验", code);
        }
    }

    /// test_validate_price_terms_dxxbmg
    #[test]
    fn test_validate_price_terms_dxxbmg() {
        let lower = QuotationService::validate_price_terms("fob");
        let upper = QuotationService::validate_price_terms("FOB");
        assert!(lower.is_ok());
        assert!(upper.is_ok());
    }

    /// test_validate_price_terms_ffdmfhcw
    #[test]
    fn test_validate_price_terms_ffdmfhcw() {
        let result = QuotationService::validate_price_terms("XYZ");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("FOB"));
    }

    // ============ 状态常量值正确性测试 ============

    /// test_bjdztcl_zzqx
    #[test]
    fn test_bjdztcl_zzqx() {
        assert_eq!(quotation_status::DRAFT, "draft");
        assert_eq!(quotation_status::APPROVED, "approved");
        assert_eq!(quotation_status::REJECTED, "rejected");
        assert_eq!(quotation_status::CANCELLED, "cancelled");
    }

    /// test_bjdztcl_hbxt
    #[test]
    fn test_bjdztcl_hbxt() {
        let states = [
            quotation_status::DRAFT,
            quotation_status::APPROVED,
            quotation_status::REJECTED,
            quotation_status::CANCELLED,
        ];
        let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
        assert_eq!(unique.len(), 4);
    }

    // ============ QuotationService 构造与 DB 连接测试 ============

    /// test_quotationservice_new_zqcysjklj
    #[tokio::test]
    async fn test_quotationservice_new_zqcysjklj() {
        let db = Arc::new(setup_test_db().await);
        let svc = QuotationService::new(db.clone());
        use sea_orm::ConnectionTrait;
        let _ = svc
            .db
            .execute(sea_orm::Statement::from_sql_and_values(
                svc.db.get_database_backend(),
                "SELECT 1",
                Vec::new(),
            ))
            .await
            .expect("数据库连接应可用");
    }

    /// test_quotationservice_get_by_id_ksjkfherr
    #[tokio::test]
    async fn test_quotationservice_get_by_id_ksjkfherr() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.get_by_id(9999).await;
        assert!(result.is_err());
    }

    /// test_quotationservice_list_ksjkfherr
    #[tokio::test]
    async fn test_quotationservice_list_ksjkfherr() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.list(1, 20, None, None, None, None).await;
        assert!(result.is_err());
    }

    /// test_quotationservice_cancel_bczfhapperror
    #[tokio::test]
    async fn test_quotationservice_cancel_bczfhapperror() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.cancel(9999, 1).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("报价单不存在") || msg.contains("not found") || msg.contains("不存在")
        );
    }

    // ============ update 状态机校验测试 ============

    /// test_quotationservice_update_bczfhapperror
    #[tokio::test]
    async fn test_quotationservice_update_bczfhapperror() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = UpdateQuotationDto::default();
        let result = svc.update(9999, dto, 1).await;
        assert!(result.is_err());
    }
}
