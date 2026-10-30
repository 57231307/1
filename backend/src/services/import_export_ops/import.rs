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
    /// 批次 327 v10 复审 P3 修复：移除误报的 #[allow]；too_many_arguments：仅 3 参数（import_type, data, user_id），远低于阈值 7；needless_pass_by_value：参数均为引用或 Copy 类型，不会触发；redundant_clone：签名层面无 clone 操作，不会触发
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
                return Err(AppError::validation_displayable(format!(
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

    /// 导入产品行（数据导入中心 /import/excel 的 products 腿，按位读列）
    /// 列序契约与列集合来自产品导入权威表（services::product_ops::import_export::PRODUCT_IMPORT_COLUMNS）：
    /// 模板面 元数据面 注册表面与本腿按同一张表的列序 zip（形态同 crm 线索腿「列序即导入器按位读列的次序」锁型），
    /// 增删列或调换顺序只改权威表一处，模板与读列自动同序，不可能错位。
    /// 落库前过与单条 HTTP 与产品 CSV 导入同一组建品判据（必录 分级 取值域 安全类别，
    /// validate_product_create_gates），面料类（坯布 成品布）缺 幅宽 克重 面料成分 任一整行拒绝；
    /// 不再把产品类型静默写成词表外的 "GENERAL" 硬编码绕过分级必填（该值不在 product_type::ALL_TYPES，
    /// 历史行为属把分级门改道，非兼容面）。ActiveModel 构造与类别外键预校验共用 crud 的同一份函数。
    /// ES 同步维持本腿既有形态（改动前后都不经 create_product 的 PG→ES 腿，缺口记交接单）。
    async fn import_product_row(&self, row: &[String], _user_id: i32) -> Result<(), AppError> {
        use crate::models::product::Column as ProductCol;
        use crate::models::product::Entity as ProductEntity;
        use crate::services::product_service::ProductService;

        let map = ProductService::product_import_map_from_positional(row);
        // 行级必填与形态校验：与 /products/import 同一实现（权威表 required 位驱动，文案同形态），
        // 首错即拒并点名真实列。
        let validated = ProductService::validate_import_row_fields(&map)
            .map_err(|issue| AppError::validation_displayable(issue.message))?;
        let args = ProductService::build_csv_create_args(&map, &validated);

        // 建品判据三连（必录 取值域 安全类别），与单条与 CSV 导入两路同门。
        ProductService::validate_product_create_gates(&args)?;
        // ActiveModel 共用构造（类型规范化与全列透传只此一份形态）。
        let active_model = ProductService::product_active_model_from_args(&args)?;

        // 检查编码是否已存在（本腿既有语义：业务错误点名，不让它撞库级唯一约束）
        let existing = ProductEntity::find()
            .filter(ProductCol::Code.eq(&args.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business(format!("产品编码 {} 已存在", args.code)));
        }

        // 类别外键预校验与单条建品同一份判据，非法类别点名拒绝不落 500。
        ProductService::require_product_category_exists(&*self.db, args.category_id).await?;

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
            return Err(AppError::validation_displayable(
                "客户编码和名称不能为空".to_string(),
            ));
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
            // 缺陷修（波0，非值域变更）：此前硬编码大写 "RETAIL"，而全部读侧对
            // customers.customer_type 是小写精确匹配（services/customer_ops/crud.rs:136、
            // query.rs:82）⇒ 导入客户的类型筛选永不命中。归一为唯一词表模块的真实 token。
            customer_type: Set(crate::constants::customer_type::RETAIL.to_string()),
            notes: Set(None),
            created_by: Set(Some(user_id)),
            // m_rls_dept_domain：department_id 由 trg_customers_dept 触发器自动维护
            department_id: sea_orm::ActiveValue::NotSet,
            // 客户分层列：导入通道不产生档位依据，保持未定档（NULL≠NORMAL），
            // 由评级审批或人工改档写入，语义见 constants/customer_tier.rs
            tier: sea_orm::ActiveValue::NotSet,
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
            // batch-13 P3：客户特殊工艺和来源字段
            special_process: Set(None),
            source: Set(Some("import".to_string())),
            pool_recycle_reason: Set(None),
        };

        active_model.insert(&*self.db).await?;

        Ok(())
    }
}
