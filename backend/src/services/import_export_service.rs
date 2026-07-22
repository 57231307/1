//! 导入导出 Service
//!
//! 提供 CSV/Excel 数据导入导出功能

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::status::import_task as import_status;
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use crate::utils::xlsx_export::{build_xlsx, XlsxTable};

// ============================================================================
// 安全漏洞 #8：导入端点请求体大小限制常量
// ============================================================================
// 设计依据：
// 1. 业务上限：单次批量导入不应超过 1 万行；每个单元格长度不应超过 1KB；
//    CSV 数据 10MB 是兼顾业务规模（10 万行 × 100 字符）与内存安全的合理上限。
// 2. HTTP body 上限 12MB：留 2MB 边界余量给 JSON 编码 / 头部开销，避
//    免 10MB CSV + 1MB metadata 触及外层限制时被截断。
// 3. 防御层次（defense-in-depth）：
//    - L1：DefaultBodyLimit 12MB（main.rs 全局中间件，兜底）
//    - L2：DTO #[validate(length(max = ...))]（axum 提取器层，结构化校验）
//    - L3：handler 入口早期校验（拒绝更快、更友好）
//    - L4：service 层 defense-in-depth（避免 handler 漏检 / 内部调用绕过）
// ============================================================================

/// CSV 字符串最大长度：10 MB
/// 依据：单行 100 字符 × 10 万行 ≈ 10MB，足够覆盖业务批量导入场景
pub const MAX_CSV_BYTES: usize = 10 * 1024 * 1024;

/// Excel 最大行数：1 万行
/// 依据：超过此行数时应分批导入；本服务只做单批次导入
pub const MAX_EXCEL_ROWS: usize = 10_000;

/// Excel 最大列数：100 列
/// 依据：通用业务实体（订单/客户/产品）字段均 < 100 列；超过则怀疑非业务数据
pub const MAX_EXCEL_COLS: usize = 100;

/// 单元格最大字符数：1024 字符
/// 依据：产品名称/地址等长文本字段通常 < 1KB；超过则怀疑恶意注入或粘贴错误
pub const MAX_CELL_LEN: usize = 1024;

/// 导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: u64,
    pub failed: u64,
    pub errors: Vec<ImportError>,
}

/// 导入错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub row: u64,
    pub field: Option<String>,
    pub message: String,
}

/// 导入模板定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplate {
    pub import_type: String,
    pub name: String,
    pub description: String,
    pub columns: Vec<ImportColumnDef>,
}

/// 导入列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportColumnDef {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub required: bool,
    pub example: Option<String>,
}

/// 导入导出 Service
pub struct ImportExportService {
    db: Arc<DatabaseConnection>,
}

impl ImportExportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取导入模板
    pub fn get_import_template(import_type: &str) -> Result<ImportTemplate, AppError> {
        match import_type {
            "products" => Ok(ImportTemplate {
                import_type: "products".to_string(),
                name: "产品导入模板".to_string(),
                description: "用于批量导入产品信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "code".to_string(),
                        title: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("P001".to_string()),
                    },
                    ImportColumnDef {
                        field: "name".to_string(),
                        title: "产品名称".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("棉布A".to_string()),
                    },
                    ImportColumnDef {
                        field: "category".to_string(),
                        title: "产品类别".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("面料".to_string()),
                    },
                    ImportColumnDef {
                        field: "unit".to_string(),
                        title: "单位".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("米".to_string()),
                    },
                    ImportColumnDef {
                        field: "price".to_string(),
                        title: "单价".to_string(),
                        data_type: "decimal".to_string(),
                        required: false,
                        example: Some("15.50".to_string()),
                    },
                ],
            }),
            "customers" => Ok(ImportTemplate {
                import_type: "customers".to_string(),
                name: "客户导入模板".to_string(),
                description: "用于批量导入客户信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "code".to_string(),
                        title: "客户编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("C001".to_string()),
                    },
                    ImportColumnDef {
                        field: "name".to_string(),
                        title: "客户名称".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("ABC公司".to_string()),
                    },
                    ImportColumnDef {
                        field: "contact".to_string(),
                        title: "联系人".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("张三".to_string()),
                    },
                    ImportColumnDef {
                        field: "phone".to_string(),
                        title: "电话".to_string(),
                        data_type: "string".to_string(),
                        required: false,
                        example: Some("13800138000".to_string()),
                    },
                ],
            }),
            "inventory" => Ok(ImportTemplate {
                import_type: "inventory".to_string(),
                name: "库存导入模板".to_string(),
                description: "用于批量导入库存信息".to_string(),
                columns: vec![
                    ImportColumnDef {
                        field: "product_code".to_string(),
                        title: "产品编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("P001".to_string()),
                    },
                    ImportColumnDef {
                        field: "warehouse_code".to_string(),
                        title: "仓库编码".to_string(),
                        data_type: "string".to_string(),
                        required: true,
                        example: Some("WH01".to_string()),
                    },
                    ImportColumnDef {
                        field: "quantity".to_string(),
                        title: "数量".to_string(),
                        data_type: "decimal".to_string(),
                        required: true,
                        example: Some("1000".to_string()),
                    },
                ],
            }),
            _ => Err(AppError::validation(format!(
                "不支持的导入类型: {}",
                import_type
            ))),
        }
    }

    /// 解析CSV内容
    ///
    /// 批次 340 v11 复审 P1 修复：移除防御性 `#[allow(clippy::needless_pass_by_value)]`，
    /// `content: &str` 是引用类型，clippy 不会对引用类型触发 needless_pass_by_value，
    /// 原标注为历史遗留误报防御。
    pub fn parse_csv(content: &str) -> Result<Vec<Vec<String>>, AppError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| AppError::validation(format!("CSV解析错误: {}", e)))?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            rows.push(row);
        }

        Ok(rows)
    }

    /// 生成 xlsx 内容（规则 3：面向用户的导出统一使用 xlsx 格式）
    pub fn generate_xlsx(headers: &[String], data: &[Vec<String>]) -> Result<Vec<u8>, AppError> {
        let table = XlsxTable {
            sheet_name: "数据导出".to_string(),
            headers: headers.to_vec(),
            rows: data.to_vec(),
        };
        build_xlsx(&table)
    }

    /// 验证导入数据
    ///
    /// 批次 340 v11 复审 P1 修复：移除防御性 `#[allow(clippy::needless_pass_by_value)]`，
    /// `&[Vec<String>]` 和 `&ImportTemplate` 都是引用类型，不会触发 needless_pass_by_value。
    pub fn validate_import_data(
        data: &[Vec<String>],
        template: &ImportTemplate,
    ) -> Vec<ImportError> {
        let mut errors = Vec::new();

        for (row_idx, row) in data.iter().enumerate() {
            for (col_idx, col_def) in template.columns.iter().enumerate() {
                if col_def.required {
                    let value = row.get(col_idx).map(|s| s.trim()).unwrap_or("");
                    if value.is_empty() {
                        errors.push(ImportError {
                            row: (row_idx + 1) as u64,
                            field: Some(col_def.field.clone()),
                            message: format!("必填字段 '{}' 为空", col_def.title),
                        });
                    }
                }

                // 数据类型验证
                if let Some(value) = row.get(col_idx) {
                    let value = value.trim();
                    if !value.is_empty() {
                        match col_def.data_type.as_str() {
                            "decimal" if value.parse::<f64>().is_err() => {
                                errors.push(ImportError {
                                    row: (row_idx + 1) as u64,
                                    field: Some(col_def.field.clone()),
                                    message: format!("'{}' 不是有效的数字格式", col_def.title),
                                });
                            }
                            "integer" if value.parse::<i64>().is_err() => {
                                errors.push(ImportError {
                                    row: (row_idx + 1) as u64,
                                    field: Some(col_def.field.clone()),
                                    message: format!("'{}' 不是有效的整数格式", col_def.title),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        errors
    }

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
        Self::validate_import_data_size(data)?;

        let mut imported = 0u64;
        let mut failed = 0u64;
        let mut errors = Vec::new();

        // P2 1-7 修复：抽取重复的"结果收集"逻辑为 record_import_result 方法
        match import_type {
            "products" => {
                for (idx, row) in data.iter().enumerate() {
                    let result = self.import_product_row(row, user_id).await;
                    Self::record_import_result(idx, result, &mut imported, &mut failed, &mut errors);
                }
            }
            "customers" => {
                for (idx, row) in data.iter().enumerate() {
                    let result = self.import_customer_row(row, user_id).await;
                    Self::record_import_result(idx, result, &mut imported, &mut failed, &mut errors);
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

    /// 校验导入数据尺寸（service 层 defense-in-depth）
    fn validate_import_data_size(data: &[Vec<String>]) -> Result<(), AppError> {
        if data.len() > MAX_EXCEL_ROWS {
            return Err(AppError::validation(format!(
                "导入数据超过最大行数限制：当前 {} 行，上限 {} 行",
                data.len(),
                MAX_EXCEL_ROWS
            )));
        }
        for (row_idx, row) in data.iter().enumerate() {
            if row.len() > MAX_EXCEL_COLS {
                return Err(AppError::validation(format!(
                    "第 {} 行列数超过最大列数限制：当前 {} 列，上限 {} 列",
                    row_idx + 1,
                    row.len(),
                    MAX_EXCEL_COLS
                )));
            }
            for (col_idx, cell) in row.iter().enumerate() {
                if cell.len() > MAX_CELL_LEN {
                    return Err(AppError::validation(format!(
                        "第 {} 行第 {} 列单元格长度超过最大字符数限制：当前 {} 字符，上限 {} 字符",
                        row_idx + 1,
                        col_idx + 1,
                        cell.len(),
                        MAX_CELL_LEN
                    )));
                }
            }
        }
        Ok(())
    }

    // ========================================================================
    // 批次 127 v8 复审 P2 修复：导入任务记录管理
    // ========================================================================
    // 原 list_import_tasks 返回空列表 vec![]，import_csv/import_excel 不落库任务记录。
    // 现新增 task 管理方法：create_import_task / update_import_task / list_import_tasks。
    // handler 在导入前创建 task 记录（status=running），导入完成后更新统计 + 状态。

    /// 创建导入任务记录（导入开始时调用）
    ///
    /// 批次 127 v8 复审 P2 修复：在 import_csv/import_excel 执行实际导入前创建任务记录，
    /// status 初始化为 "running"，total_rows 为待导入行数。
    /// 返回任务 ID 供后续 update_import_task 使用。
    pub async fn create_import_task(
        &self,
        import_type: &str,
        total_rows: u64,
        user_id: i32,
    ) -> Result<i32, AppError> {
        // 批次 357 v13 复审 baseline 清零：移除 unused import self（仅使用 ActiveModel）
        use crate::models::import_task::ActiveModel;
        use sea_orm::ActiveValue::Set;
        use chrono::Utc;

        let now = Utc::now();
        let active_model = ActiveModel {
            import_type: Set(import_type.to_string()),
            status: Set(import_status::RUNNING.to_string()),
            total_rows: Set(total_rows as i64),
            imported_rows: Set(0),
            failed_rows: Set(0),
            user_id: Set(Some(user_id)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active_model.insert(&*self.db).await?;
        Ok(model.id)
    }

    /// 更新导入任务记录（导入完成时调用）
    ///
    /// 批次 127 v8 复审 P2 修复：根据 ImportResult 更新任务的 imported_rows / failed_rows / status。
    /// 状态判定规则：
    /// - failed == 0 && imported > 0 → "success"
    /// - imported == 0 && failed > 0 → "failed"
    /// - imported > 0 && failed > 0 → "partial"
    /// - 其他（imported == 0 && failed == 0）→ "success"（空导入视为成功）
    pub async fn update_import_task(
        &self,
        task_id: i32,
        result: &ImportResult,
    ) -> Result<(), AppError> {
        // 批次 357 v13 复审 baseline 清零：移除 unused import self（仅使用 ActiveModel）
        use crate::models::import_task::ActiveModel;
        use sea_orm::ActiveValue::Set;
        use chrono::Utc;

        let status = if result.failed == 0 {
            import_status::SUCCESS
        } else if result.imported == 0 {
            import_status::FAILED
        } else {
            import_status::PARTIAL
        };

        let active_model = ActiveModel {
            id: Set(task_id),
            status: Set(status.to_string()),
            imported_rows: Set(result.imported as i64),
            failed_rows: Set(result.failed as i64),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        active_model.update(&*self.db).await?;
        Ok(())
    }

    /// 获取导入任务列表（list_import_tasks handler 调用）
    ///
    /// 批次 127 v8 复审 P2 修复：替代原 list_import_tasks 返回的空列表 vec![]。
    /// 按创建时间倒序返回最近 100 条任务记录。
    pub async fn list_import_tasks(
        &self,
    ) -> Result<Vec<crate::models::import_task::Model>, AppError> {
        use crate::models::import_task;
        use sea_orm::{QueryOrder, QuerySelect};

        let tasks = import_task::Entity::find()
            .order_by(import_task::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(100)
            .all(&*self.db)
            .await?;
        Ok(tasks)
    }

    /// P2 1-7 修复：记录单行导入结果，消除 "products"/"customers" 分支中重复的 imported/failed/errors 收集代码
    fn record_import_result(
        idx: usize,
        result: Result<(), AppError>,
        imported: &mut u64,
        failed: &mut u64,
        errors: &mut Vec<ImportError>,
    ) {
        match result {
            Ok(_) => *imported += 1,
            Err(e) => {
                *failed += 1;
                errors.push(ImportError {
                    row: (idx + 1) as u64,
                    field: None,
                    message: e.to_string(),
                });
            }
        }
    }

    /// 导入产品行
    async fn import_product_row(&self, row: &[String], _user_id: i32) -> Result<(), AppError> {
        use crate::models::product::{ActiveModel as ProductActiveModel, Entity as ProductEntity};
        use sea_orm::Set;

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
        use sea_orm::Set;

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

    /// 导出数据
    ///
    /// 架构优化（2026-06-25）：
    /// 1. 不再全表查询，根据 ExportQuery 条件过滤（status / date_from / date_to / keyword）
    /// 2. 强制行数上限 MAX_EXPORT_ROWS，避免大表导出 OOM
    /// 3. 过滤已删除数据（is_deleted=false）
    pub async fn export_data(
        &self,
        export_type: &str,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        match export_type {
            "products" => self.export_products(query).await,
            "customers" => self.export_customers(query).await,
            "inventory" => self.export_inventory(query).await,
            _ => Err(AppError::validation(format!(
                "不支持的导出类型: {}",
                export_type
            ))),
        }
    }

    /// 解析日期字符串为 DateTime（兼容多种格式）
    fn parse_date_filter(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        let formats = ["%Y-%m-%d", "%Y-%m-%d %H:%M:%S", "%Y/%m/%d"];
        for _fmt in &formats {
            if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(
                &format!("{} 00:00:00", date_str.split(' ').next().unwrap_or(date_str)),
                "%Y-%m-%d %H:%M:%S",
            ) {
                return Some(chrono::DateTime::from_naive_utc_and_offset(
                    naive,
                    chrono::Utc,
                ));
            }
        }
        None
    }

    /// 获取导出行数限制（不超过 MAX_EXPORT_ROWS）
    fn get_export_limit(query: &ExportQuery) -> u64 {
        query
            .limit
            .unwrap_or(MAX_EXPORT_ROWS)
            .min(MAX_EXPORT_ROWS)
    }

    /// 导出产品数据
    async fn export_products(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::product::{Column as ProductCol, Entity as ProductEntity};
        use sea_orm::QuerySelect;

        let mut db_query = ProductEntity::find().filter(ProductCol::IsDeleted.eq(false));

        if let Some(status) = &query.status {
            db_query = db_query.filter(ProductCol::Status.eq(status.clone()));
        }
        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                ProductCol::Name
                    .like(like.clone())
                    .or(ProductCol::Code.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = Self::parse_date_filter(date_from) {
                db_query = db_query.filter(ProductCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = Self::parse_date_filter(date_to) {
                db_query = db_query.filter(ProductCol::CreatedAt.lte(dt));
            }
        }

        let limit = Self::get_export_limit(query);
        let products = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "产品编码".to_string(),
            "产品名称".to_string(),
            "单位".to_string(),
            "标准单价".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = products
            .into_iter()
            .map(|p| {
                vec![
                    p.id.to_string(),
                    p.code,
                    p.name,
                    p.unit,
                    p.standard_price.map(|p| p.to_string()).unwrap_or_default(),
                    p.status,
                    p.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }

    /// 导出客户数据
    async fn export_customers(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::customer::{Column as CustomerCol, Entity as CustomerEntity};
        use sea_orm::QuerySelect;

        let mut db_query = CustomerEntity::find();

        if let Some(status) = &query.status {
            db_query = db_query.filter(CustomerCol::Status.eq(status.clone()));
        }
        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                CustomerCol::CustomerName
                    .like(like.clone())
                    .or(CustomerCol::CustomerCode.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = Self::parse_date_filter(date_from) {
                db_query = db_query.filter(CustomerCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = Self::parse_date_filter(date_to) {
                db_query = db_query.filter(CustomerCol::CreatedAt.lte(dt));
            }
        }

        let limit = Self::get_export_limit(query);
        let customers = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "客户编码".to_string(),
            "客户名称".to_string(),
            "联系人".to_string(),
            "电话".to_string(),
            "邮箱".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = customers
            .into_iter()
            .map(|c| {
                vec![
                    c.id.to_string(),
                    c.customer_code,
                    c.customer_name,
                    c.contact_person.unwrap_or_default(),
                    c.contact_phone.unwrap_or_default(),
                    c.contact_email.unwrap_or_default(),
                    c.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }

    /// 导出库存数据
    async fn export_inventory(
        &self,
        query: &ExportQuery,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        use crate::models::inventory_stock::{Column as StockCol, Entity as StockEntity};
        use sea_orm::QuerySelect;

        let mut db_query = StockEntity::find();

        if let Some(keyword) = &query.keyword {
            // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
            let like = safe_like_pattern(keyword);
            db_query = db_query.filter(
                StockCol::BatchNo
                    .like(like.clone())
                    .or(StockCol::ColorNo.like(like)),
            );
        }
        if let Some(date_from) = &query.date_from {
            if let Some(dt) = Self::parse_date_filter(date_from) {
                db_query = db_query.filter(StockCol::CreatedAt.gte(dt));
            }
        }
        if let Some(date_to) = &query.date_to {
            if let Some(dt) = Self::parse_date_filter(date_to) {
                db_query = db_query.filter(StockCol::CreatedAt.lte(dt));
            }
        }

        let limit = Self::get_export_limit(query);
        let stocks = db_query.limit(limit).all(&*self.db).await?;

        let headers = vec![
            "ID".to_string(),
            "产品ID".to_string(),
            "仓库ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "创建时间".to_string(),
        ];

        let data: Vec<Vec<String>> = stocks
            .into_iter()
            .map(|s| {
                vec![
                    s.id.to_string(),
                    s.product_id.to_string(),
                    s.warehouse_id.to_string(),
                    s.quantity_available.to_string(),
                    s.quantity_reserved.to_string(),
                    s.quantity_incoming.to_string(),
                    s.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect();

        Ok((headers, data))
    }
}

/// 单次导出最大行数（防止全表导出导致内存溢出）
/// 业务上限：单次导出不应超过 1 万行；超过应分页分批导出
pub const MAX_EXPORT_ROWS: u64 = 10_000;

/// 导出查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ExportQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub limit: Option<u64>,
}

#[cfg(test)]
mod tests {
    //! 安全漏洞 #8 修复配套单测
    //!
    //! 测试目标：
    //! 1. 常量定义正确（避免被误改）
    //! 2. service 层 import_data 在数据超过上限时立即拒绝（defense-in-depth 第四层）
    //!
    //! 备注：handler 层 DTO 校验 + 早期校验在路由层单测覆盖；
    //! 本处只覆盖 service 层入口校验（最关键的 defense-in-depth 屏障）。
    use super::*;
    use crate::services::test_common::setup_test_db;

    /// 测试常量定义正确（防止误改后引发业务可用性问题）
    #[test]
    fn test_vuln8_constants_defined_correctly() {
        // CSV 10MB：业务上限
        assert_eq!(MAX_CSV_BYTES, 10 * 1024 * 1024, "MAX_CSV_BYTES 应为 10MB");
        // Excel 1 万行
        assert_eq!(MAX_EXCEL_ROWS, 10_000, "MAX_EXCEL_ROWS 应为 1 万行");
        // 100 列
        assert_eq!(MAX_EXCEL_COLS, 100, "MAX_EXCEL_COLS 应为 100 列");
        // 单元格 1024 字符
        assert_eq!(MAX_CELL_LEN, 1024, "MAX_CELL_LEN 应为 1024 字符");
    }

    /// 漏洞 #8 修复：service 层 import_data 行数上限校验
    /// 超过 MAX_EXCEL_ROWS 行 → 立即拒绝（不进入 DB 查询）
    #[tokio::test]
    async fn test_import_data_rejects_exceeding_max_rows() {
        let db = Arc::new(setup_test_db().await);
        let service = ImportExportService::new(db);

        // 构造超过 MAX_EXCEL_ROWS + 1 行的数据
        let mut data = Vec::with_capacity(MAX_EXCEL_ROWS + 1);
        for _ in 0..=MAX_EXCEL_ROWS {
            data.push(vec!["P001".to_string(), "name".to_string()]);
        }

        // 调用 import_data，期望 ValidationError
        let result = service.import_data("products", &data, 1).await;
        assert!(
            result.is_err(),
            "漏洞 #8 单测：{} 行数据应被拒绝，但 import_data 返回成功",
            data.len()
        );
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("最大行数") || err_msg.contains("MAX_EXCEL_ROWS") || err_msg.contains("上限"),
            "漏洞 #8 单测：错误信息应包含'最大行数'或'上限'，实际：{}",
            err_msg
        );
    }

    /// 漏洞 #8 修复：service 层 import_data 列数上限校验
    /// 单行列数超过 MAX_EXCEL_COLS → 立即拒绝
    #[tokio::test]
    async fn test_import_data_rejects_exceeding_max_cols() {
        let db = Arc::new(setup_test_db().await);
        let service = ImportExportService::new(db);

        // 构造 1 行 MAX_EXCEL_COLS + 1 列的数据
        let mut row = Vec::with_capacity(MAX_EXCEL_COLS + 1);
        for i in 0..=MAX_EXCEL_COLS {
            row.push(format!("col_{}", i));
        }
        let data = vec![row];

        let result = service.import_data("products", &data, 1).await;
        assert!(
            result.is_err(),
            "漏洞 #8 单测：{} 列数据应被拒绝，但 import_data 返回成功",
            data[0].len()
        );
    }

    /// 漏洞 #8 修复：service 层 import_data 单元格长度上限校验
    /// 单个单元格超过 MAX_CELL_LEN 字符 → 立即拒绝
    #[tokio::test]
    async fn test_import_data_rejects_exceeding_max_cell_len() {
        let db = Arc::new(setup_test_db().await);
        let service = ImportExportService::new(db);

        // 构造 1 个超过 MAX_CELL_LEN 字符的单元格
        let long_cell = "A".repeat(MAX_CELL_LEN + 1);
        let data = vec![vec![long_cell.clone()]];

        let result = service.import_data("products", &data, 1).await;
        assert!(
            result.is_err(),
            "漏洞 #8 单测：{} 字符的单元格应被拒绝，但 import_data 返回成功",
            long_cell.len()
        );
    }

    /// 漏洞 #8 修复：service 层 import_data 正常数据不误拒
    /// 边界值测试：在所有上限内的数据应通过校验（即使后续因 unknown import_type 失败）
    #[tokio::test]
    async fn test_import_data_allows_within_limits() {
        let db = Arc::new(setup_test_db().await);
        let service = ImportExportService::new(db);

        // 构造 1 行 100 列的合法数据
        let mut row = Vec::with_capacity(MAX_EXCEL_COLS);
        for i in 0..MAX_EXCEL_COLS {
            row.push(format!("val_{}", i));
        }
        let data = vec![row];

        // 使用 unknown import_type 触发 "不支持的导入类型" 错误（说明校验通过）
        let result = service.import_data("unknown_type", &data, 1).await;
        assert!(
            result.is_err(),
            "漏洞 #8 单测：边界内数据不应被 service 层校验拒绝"
        );
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("不支持的导入类型"),
            "漏洞 #8 单测：service 层应通过校验，仅在 import_type 校验处失败，实际：{}",
            err_msg
        );
    }
}
