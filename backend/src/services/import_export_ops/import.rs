//! 批量数据导入子模块（import_export_ops::import）
//!
//! 从原 `import_export_service.rs` 迁移 3 个方法：
//! - `import_data`：导入入口（分发到产品/客户行导入）
//! - `import_product_row`：单行产品数据导入
//! - `import_customer_row`：单行客户数据导入
//!
//! 调用 facade 中的纯函数（`pub(crate)` 可见性）：
//! - `ImportExportService::validate_import_data_size`（service 层 defense-in-depth 第四层）
//! - `ImportExportService::record_import_result`（统一记录单行导入结果）

use crate::models::status::master_data;
use crate::services::import_export_service::{ImportExportService, ImportResult};
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

impl ImportExportService {
    /// 执行数据导入
    ///
    /// 批次 327 v10 复审 P3 修复：移除误报的 #[allow]
    /// - too_many_arguments：仅 3 参数（import_type, data, user_id），远低于阈值 7
    /// - needless_pass_by_value：参数均为引用或 Copy 类型，不会触发
    /// - redundant_clone：签名层面无 clone 操作，不会触发
    pub async fn import_data(
        &self,
        import_type: &str,
        data: &[Vec<String>],
        user_id: i32,
    ) -> Result<ImportResult, AppError> {
        ImportExportService::validate_import_data_size(data)?;

        let mut imported = 0u64;
        let mut failed = 0u64;
        let mut errors = Vec::new();

        // P2 1-7 修复：抽取重复的"结果收集"逻辑为 record_import_result 方法
        match import_type {
            "products" => {
                for (idx, row) in data.iter().enumerate() {
                    let result = self.import_product_row(row, user_id).await;
                    ImportExportService::record_import_result(
                        idx,
                        result,
                        &mut imported,
                        &mut failed,
                        &mut errors,
                    );
                }
            }
            "customers" => {
                for (idx, row) in data.iter().enumerate() {
                    let result = self.import_customer_row(row, user_id).await;
                    ImportExportService::record_import_result(
                        idx,
                        result,
                        &mut imported,
                        &mut failed,
                        &mut errors,
                    );
                }
            }
            _ => {
                return Err(AppError::validation(format!(
                    "不支持的导入类型: {}",
                    import_type
                )));
            }
        }

        Ok(ImportResult {
            imported,
            failed,
            errors,
        })
    }

    /// 导入产品行
    async fn import_product_row(&self, row: &[String], _user_id: i32) -> Result<(), AppError> {
        use crate::models::product::{ActiveModel as ProductActiveModel, Entity as ProductEntity};

        let code = row
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let name = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let _category = row.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
        let unit = row
            .get(3)
            .map(|s| s.trim().to_string())
            .unwrap_or("个".to_string());
        // 批次 403 修复：价格列非空时必须可解析为 f64，失败时返回验证错误而非静默写 0。
        // 原 unwrap_or(0.0) 会让 "abc" 等非法价格静默变成 0，导致产品以错误成本价入库。
        let price = match row.get(4) {
            Some(s) if !s.trim().is_empty() => s.trim().parse::<f64>().map_err(|_| {
                AppError::validation(format!("产品 {} 的价格列无法解析为数字: {}", code, s))
            })?,
            _ => 0.0,
        };

        if code.is_empty() || name.is_empty() {
            return Err(AppError::validation("产品编码和名称不能为空".to_string()));
        }

        // 检查编码是否已存在
        let existing = ProductEntity::find()
            .filter(crate::models::product::Column::Code.eq(&code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!("产品编码 {} 已存在", code)));
        }

        let now = chrono::Utc::now();
        let active_model = ProductActiveModel {
            id: Default::default(),
            code: Set(code),
            name: Set(name),
            category_id: Set(None),
            specification: Set(None),
            unit: Set(unit),
            standard_price: Set(Some(
                rust_decimal::Decimal::from_f64_retain(price).unwrap_or_default(),
            )),
            cost_price: Set(None),
            description: Set(None),
            status: Set(master_data::ACTIVE.to_string()),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            product_type: Set("GENERAL".to_string()),
            fabric_composition: Set(None),
            yarn_count: Set(None),
            density: Set(None),
            width: Set(None),
            gram_weight: Set(None),
            structure: Set(None),
            finish: Set(None),
            min_order_quantity: Set(None),
            lead_time: Set(None),
            supplier_product_code: Set(None),
            supplier_id: Set(None),
            is_batch_managed: Set(None),
            batch_level: Set(None),
        };

        active_model.insert(&*self.db).await?;

        Ok(())
    }

    /// 导入客户行
    async fn import_customer_row(&self, row: &[String], user_id: i32) -> Result<(), AppError> {
        use crate::models::customer::{
            ActiveModel as CustomerActiveModel, Entity as CustomerEntity,
        };

        let code = row
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let name = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let contact = row.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
        let phone = row.get(3).map(|s| s.trim().to_string()).unwrap_or_default();

        if code.is_empty() || name.is_empty() {
            return Err(AppError::validation("客户编码和名称不能为空".to_string()));
        }

        // 检查编码是否已存在
        let existing = CustomerEntity::find()
            .filter(crate::models::customer::Column::CustomerCode.eq(&code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!("客户编码 {} 已存在", code)));
        }

        let now = chrono::Utc::now();
        let active_model = CustomerActiveModel {
            id: Default::default(),
            customer_code: Set(code),
            customer_name: Set(name),
            contact_person: Set(Some(contact)),
            contact_phone: Set(Some(phone)),
            contact_email: Set(None),
            address: Set(None),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS),
            tax_id: Set(None),
            bank_name: Set(None),
            bank_account: Set(None),
            status: Set(master_data::ACTIVE.to_string()),
            customer_type: Set("RETAIL".to_string()),
            notes: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            customer_industry: Set(None),
            main_products: Set(None),
            annual_purchase: Set(None),
            quality_requirement: Set(None),
            inspection_standard: Set(None),
            // V15 P0-S08 修复：导入客户时业务负责人默认为操作人
            owner_id: Set(user_id),
            owner_assigned_at: Set(Some(now)),
        };

        active_model.insert(&*self.db).await?;

        Ok(())
    }
}
