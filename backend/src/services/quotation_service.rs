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
    use std::str::FromStr;
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

    /// 测试_ServiceError_Display_格式正确
    /// 验证 5 个 ServiceError 变体的 Display 实现返回中文错误信息
    #[test]
    fn 测试_ServiceError_Display_格式正确() {
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

    /// 测试_validate_create_空明细拒绝
    #[tokio::test]
    async fn 测试_validate_create_空明细拒绝() {
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

    /// 测试_validate_create_有效期早于报价日期拒绝
    #[tokio::test]
    async fn 测试_validate_create_有效期早于报价日期拒绝() {
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

    /// 测试_validate_create_非法贸易术语拒绝
    #[tokio::test]
    async fn 测试_validate_create_非法贸易术语拒绝() {
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

    /// 测试_validate_create_合法参数通过
    #[tokio::test]
    async fn 测试_validate_create_合法参数通过() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto();
        let result = svc.validate_create(&dto);
        assert!(result.is_ok());
    }

    // ============ calculate_totals 金额计算测试 ============

    /// 测试_calculate_totals_不含税金额计算正确
    #[tokio::test]
    async fn 测试_calculate_totals_不含税金额计算正确() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = sample_dto();
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, decs!(130));
        assert_eq!(total_amount, decs!(1130));
    }

    /// 测试_calculate_totals_含税金额税额为零
    #[tokio::test]
    async fn 测试_calculate_totals_含税金额税额为零() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let mut dto = sample_dto();
        dto.tax_inclusive = true;
        let (subtotal, tax_amount, total_amount) = svc.calculate_totals(&dto).unwrap();
        assert_eq!(subtotal, decs!(1000));
        assert_eq!(tax_amount, Decimal::ZERO);
        assert_eq!(total_amount, decs!(1000));
    }

    /// 测试_calculate_totals_多明细汇总正确
    #[tokio::test]
    async fn 测试_calculate_totals_多明细汇总正确() {
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

    /// 测试_calculate_totals_精度归一到2位小数
    /// 批次 87：33.333 * 3 = 99.999 → 100.00（round_dp(2)）
    #[tokio::test]
    async fn 测试_calculate_totals_精度归一到2位小数() {
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

    /// 测试_validate_price_terms_合法代码返回枚举
    #[test]
    fn 测试_validate_price_terms_合法代码返回枚举() {
        let valid_codes = [
            "EXW", "FCA", "CPT", "CIP", "DAP", "DPU", "DDP", "FAS", "FOB", "CFR", "CIF",
        ];
        for code in valid_codes {
            let result = QuotationService::validate_price_terms(code);
            assert!(result.is_ok(), "合法代码 {} 应通过校验", code);
        }
    }

    /// 测试_validate_price_terms_大小写不敏感
    #[test]
    fn 测试_validate_price_terms_大小写不敏感() {
        let lower = QuotationService::validate_price_terms("fob");
        let upper = QuotationService::validate_price_terms("FOB");
        assert!(lower.is_ok());
        assert!(upper.is_ok());
    }

    /// 测试_validate_price_terms_非法代码返回错误
    #[test]
    fn 测试_validate_price_terms_非法代码返回错误() {
        let result = QuotationService::validate_price_terms("XYZ");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("FOB"));
    }

    // ============ 状态常量值正确性测试 ============

    /// 测试_报价单状态常量_值正确性
    #[test]
    fn 测试_报价单状态常量_值正确性() {
        assert_eq!(quotation_status::DRAFT, "draft");
        assert_eq!(quotation_status::APPROVED, "approved");
        assert_eq!(quotation_status::REJECTED, "rejected");
        assert_eq!(quotation_status::CANCELLED, "cancelled");
    }

    /// 测试_报价单状态常量_互不相同
    #[test]
    fn 测试_报价单状态常量_互不相同() {
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

    /// 测试_QuotationService_new_正确持有数据库连接
    #[tokio::test]
    async fn 测试_QuotationService_new_正确持有数据库连接() {
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

    /// 测试_QuotationService_get_by_id_空数据库返回Err
    #[tokio::test]
    async fn 测试_QuotationService_get_by_id_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.get_by_id(9999).await;
        assert!(result.is_err());
    }

    /// 测试_QuotationService_list_空数据库返回Err
    #[tokio::test]
    async fn 测试_QuotationService_list_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let result = svc.list(1, 20, None, None, None, None).await;
        assert!(result.is_err());
    }

    /// 测试_QuotationService_cancel_不存在返回AppError
    #[tokio::test]
    async fn 测试_QuotationService_cancel_不存在返回AppError() {
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

    /// 测试_QuotationService_update_不存在返回AppError
    #[tokio::test]
    async fn 测试_QuotationService_update_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = QuotationService::new(Arc::new(db));
        let dto = UpdateQuotationDto::default();
        let result = svc.update(9999, dto, 1).await;
        assert!(result.is_err());
    }
}
