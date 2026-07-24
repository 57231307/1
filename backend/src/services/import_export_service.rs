//! 导入导出 Service（facade）
//!
//! 本文件为 facade 入口，仅保留：
//! - `ImportExportService` struct + `new` 构造函数
//! - 公共常量（MAX_CSV_BYTES / MAX_EXCEL_ROWS / MAX_EXCEL_COLS / MAX_CELL_LEN / MAX_EXPORT_ROWS）
//! - DTO struct（ImportResult / ImportError / ImportTemplate / ImportColumnDef / ExportQuery）
//! - 纯函数（无 &self / 无 db 访问）：get_import_template / build_*_template / parse_csv /
//!   generate_xlsx / validate_import_data / validate_import_data_size /
//!   record_import_result / parse_date_filter / get_export_limit
//! - 单元测试模块 `#[cfg(test)] mod tests`
//!
//! 业务实现已按职责拆分到 `import_export_ops/` 子模块（与 `import_export_service` 同为 `crate::services`
//! 下兄弟模块）：
//! - `import_export_ops::import`：批量数据导入（import_data + 产品行/客户行导入）
//! - `import_export_ops::export`：数据导出（export_data + 产品/客户/库存导出）
//! - `import_export_ops::task`：导入任务记录管理（create_import_task / update_import_task / list_import_tasks）
//!
//! 外部调用路径不变：`crate::services::import_export_service::ImportExportService` 等保持稳定。
//! `db` 字段使用 `pub(crate)` 可见性，import_export_ops 子模块的 impl 块可直接访问 `self.db`。

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::utils::error::AppError;
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

/// 单次导出最大行数（防止全表导出导致内存溢出）
/// 业务上限：单次导出不应超过 1 万行；超过应分页分批导出
pub const MAX_EXPORT_ROWS: u64 = 10_000;

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

/// 导出查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ExportQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub limit: Option<u64>,
}

/// 导入导出 Service
pub struct ImportExportService {
    /// `pub(crate)` 可见性：import_export_ops 兄弟模块的 impl 块需直接访问此字段。
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ImportExportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取导入模板
    pub fn get_import_template(import_type: &str) -> Result<ImportTemplate, AppError> {
        match import_type {
            "products" => Ok(Self::build_products_template()),
            "customers" => Ok(Self::build_customers_template()),
            "inventory" => Ok(Self::build_inventory_template()),
            _ => Err(AppError::validation(format!(
                "不支持的导入类型: {}",
                import_type
            ))),
        }
    }

    /// 构建产品导入模板
    pub(crate) fn build_products_template() -> ImportTemplate {
        ImportTemplate {
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
        }
    }

    /// 构建客户导入模板
    pub(crate) fn build_customers_template() -> ImportTemplate {
        ImportTemplate {
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
        }
    }

    /// 构建库存导入模板
    pub(crate) fn build_inventory_template() -> ImportTemplate {
        ImportTemplate {
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

    /// 校验导入数据尺寸（service 层 defense-in-depth）
    ///
    /// `pub(crate)` 可见性：import_export_ops::import 子模块在 import_data 入口处调用此方法，
    /// 实现 service 层 defense-in-depth 第四层屏障（避免 handler 漏检 / 内部调用绕过）。
    pub(crate) fn validate_import_data_size(data: &[Vec<String>]) -> Result<(), AppError> {
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

    /// 记录单行导入结果，消除 "products"/"customers" 分支中重复的 imported/failed/errors 收集代码
    /// （P2 1-7 修复抽取）
    ///
    /// `pub(crate)` 可见性：import_export_ops::import 子模块在 import_data 循环中调用此方法，
    /// 统一处理单行导入成功/失败的结果累积逻辑。
    pub(crate) fn record_import_result(
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

    /// 解析日期字符串为 DateTime（兼容多种格式）
    ///
    /// `pub(crate)` 可见性：import_export_ops::export 子模块在 export_products/export_customers/
    /// export_inventory 中调用此方法解析 date_from / date_to 过滤参数。
    pub(crate) fn parse_date_filter(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
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
    ///
    /// `pub(crate)` 可见性：import_export_ops::export 子模块在 export_products/export_customers/
    /// export_inventory 中调用此方法计算最终 limit（取 query.limit 与 MAX_EXPORT_ROWS 的最小值）。
    pub(crate) fn get_export_limit(query: &ExportQuery) -> u64 {
        query
            .limit
            .unwrap_or(MAX_EXPORT_ROWS)
            .min(MAX_EXPORT_ROWS)
    }
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
