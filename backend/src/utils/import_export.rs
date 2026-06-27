//! 数据导入导出工具模块
//!
//! 提供通用的数据导入导出功能，支持 CSV 和 Excel 格式。
//!
//! # 主要功能
//! - CSV 数据解析和生成
//! - Excel 数据解析和生成
//! - 数据验证和错误报告
//! - 导入模板生成

use crate::utils::error::AppError;
use serde::Serialize;
use std::collections::HashMap;

/// 导入格式
#[allow(dead_code)] // TODO(tech-debt): Excel 导入功能接入后移除
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// CSV 格式
    Csv,
    /// Excel 格式
    Excel,
}

#[allow(dead_code)] // TODO(tech-debt): Excel 导入功能接入后移除
impl ImportFormat {
    /// 从文件扩展名解析格式
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "csv" => Some(ImportFormat::Csv),
            "xlsx" | "xls" => Some(ImportFormat::Excel),
            _ => None,
        }
    }

    /// 获取文件扩展名
    pub fn extension(&self) -> &'static str {
        match self {
            ImportFormat::Csv => "csv",
            ImportFormat::Excel => "xlsx",
        }
    }

    /// 获取 MIME 类型
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImportFormat::Csv => "text/csv",
            ImportFormat::Excel => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
        }
    }
}

/// 导入错误
#[derive(Debug, Clone, Serialize)]
pub struct ImportError {
    /// 行号
    pub row: usize,
    /// 列名
    pub column: String,
    /// 错误信息
    pub message: String,
    /// 原始值
    pub value: String,
}

/// 导入结果
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    /// 总行数
    pub total_count: usize,
    /// 成功行数
    pub success_count: usize,
    /// 错误行数
    pub error_count: usize,
    /// 错误详情
    pub errors: Vec<ImportError>,
}

impl ImportResult {
    /// 创建新的导入结果
    pub fn new() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            error_count: 0,
            errors: Vec::new(),
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, row: usize, column: String, message: String, value: String) {
        self.errors.push(ImportError {
            row,
            column,
            message,
            value,
        });
        self.error_count += 1;
    }

    /// 增加成功计数
    pub fn add_success(&mut self) {
        self.success_count += 1;
    }

    /// 增加总计数
    pub fn add_total(&mut self) {
        self.total_count += 1;
    }

    /// 是否全部成功
    pub fn is_all_success(&self) -> bool {
        self.error_count == 0 && self.success_count > 0
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// CSV 导入工具
pub struct CsvImporter;

impl CsvImporter {
    /// 解析 CSV 数据
    ///
    /// # 参数
    /// - `data`: CSV 字节数据
    ///
    /// # 返回
    /// - `Ok(Vec<HashMap<String, String>>)`: 解析后的数据，每行是一个键值对
    /// - `Err(AppError)`: 解析失败
    pub fn parse(data: &[u8]) -> Result<Vec<HashMap<String, String>>, AppError> {
        let content = String::from_utf8(data.to_vec())
            .map_err(|e| AppError::validation(format!("无效的 UTF-8 数据: {}", e)))?;

        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| AppError::validation(format!("CSV 头解析失败: {}", e)))?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<String>>();

        let mut records = Vec::new();

        for (row_idx, result) in reader.records().enumerate() {
            let record = result.map_err(|e| {
                AppError::validation(format!("第 {} 行解析失败: {}", row_idx + 2, e))
            })?;

            let mut row = HashMap::new();
            for (col_idx, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(col_idx) {
                    row.insert(header.clone(), field.to_string());
                }
            }
            records.push(row);
        }

        Ok(records)
    }

    /// 生成 CSV 数据
    ///
    /// # 参数
    /// - `headers`: 表头
    /// - `rows`: 数据行
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: CSV 字节数据
    /// - `Err(AppError)`: 生成失败
    pub fn generate(
        headers: &[String],
        rows: &[HashMap<String, String>],
    ) -> Result<Vec<u8>, AppError> {
        let mut writer = csv::Writer::from_writer(Vec::new());

        // 写入表头
        writer
            .write_record(headers)
            .map_err(|e| AppError::internal(format!("CSV 头写入失败: {}", e)))?;

        // 写入数据
        for row in rows {
            let record: Vec<String> = headers
                .iter()
                .map(|h| row.get(h).cloned().unwrap_or_default())
                .collect();
            writer
                .write_record(&record)
                .map_err(|e| AppError::internal(format!("CSV 数据写入失败: {}", e)))?;
        }

        writer
            .into_inner()
            .map_err(|e| AppError::internal(format!("CSV 生成失败: {}", e)))
    }

    /// 生成 CSV 模板
    ///
    /// # 参数
    /// - `headers`: 表头列表
    /// - `examples`: 示例数据（可选）
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: CSV 模板字节数据
    pub fn generate_template(
        headers: &[String],
        examples: Option<&[HashMap<String, String>]>,
    ) -> Result<Vec<u8>, AppError> {
        let mut writer = csv::Writer::from_writer(Vec::new());

        // 写入表头
        writer
            .write_record(headers)
            .map_err(|e| AppError::internal(format!("CSV 头写入失败: {}", e)))?;

        // 写入示例数据
        if let Some(examples) = examples {
            for row in examples {
                let record: Vec<String> = headers
                    .iter()
                    .map(|h| row.get(h).cloned().unwrap_or_default())
                    .collect();
                writer
                    .write_record(&record)
                    .map_err(|e| AppError::internal(format!("CSV 示例写入失败: {}", e)))?;
            }
        }

        writer
            .into_inner()
            .map_err(|e| AppError::internal(format!("CSV 模板生成失败: {}", e)))
    }
}

/// 字段验证器
pub struct FieldValidator;

impl FieldValidator {
    /// 验证必填字段
    pub fn required(value: &str, field_name: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            Err(format!("{} 不能为空", field_name))
        } else {
            Ok(())
        }
    }

    /// 验证整数
    pub fn integer(value: &str, field_name: &str) -> Result<i32, String> {
        value
            .parse::<i32>()
            .map_err(|_| format!("{} 必须是有效的整数", field_name))
    }

    /// 验证小数
    pub fn decimal(value: &str, field_name: &str) -> Result<rust_decimal::Decimal, String> {
        value
            .parse::<rust_decimal::Decimal>()
            .map_err(|_| format!("{} 必须是有效的数字", field_name))
    }

    /// 验证日期（YYYY-MM-DD 格式）
    pub fn date(value: &str, field_name: &str) -> Result<chrono::NaiveDate, String> {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| format!("{} 必须是有效的日期格式（YYYY-MM-DD）", field_name))
    }

    /// 验证布尔值
    pub fn boolean(value: &str, field_name: &str) -> Result<bool, String> {
        match value.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "是" => Ok(true),
            "false" | "0" | "no" | "否" => Ok(false),
            _ => Err(format!(
                "{} 必须是布尔值（true/false/1/0/是/否）",
                field_name
            )),
        }
    }

    /// 验证枚举值
    pub fn enum_value(value: &str, field_name: &str, allowed: &[&str]) -> Result<String, String> {
        let trimmed = value.trim();
        if allowed.contains(&trimmed) {
            Ok(trimmed.to_string())
        } else {
            Err(format!(
                "{} 必须是以下值之一: {}",
                field_name,
                allowed.join(", ")
            ))
        }
    }

    /// 验证最大长度
    #[allow(dead_code)] // TODO(tech-debt): Excel 导入功能接入后移除
    pub fn max_length(value: &str, field_name: &str, max_len: usize) -> Result<(), String> {
        if value.len() > max_len {
            Err(format!("{} 长度不能超过 {} 个字符", field_name, max_len))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parse() {
        let csv_data = b"name,age,city\nAlice,30,Beijing\nBob,25,Shanghai";
        let records = CsvImporter::parse(csv_data).expect("P9-1: CSV 解析失败");

        assert_eq!(records.len(), 2);
        // P9-1: 用 if let Some(...) 替代 .get(...).unwrap()，明确处理键不存在场景
        assert_eq!(records[0].get("name").map(String::as_str), Some("Alice"));
        assert_eq!(records[0].get("age").map(String::as_str), Some("30"));
        assert_eq!(records[1].get("city").map(String::as_str), Some("Shanghai"));
    }

    #[test]
    fn test_csv_generate() {
        let headers = vec!["name".to_string(), "age".to_string()];
        let mut row1 = HashMap::new();
        row1.insert("name".to_string(), "Alice".to_string());
        row1.insert("age".to_string(), "30".to_string());
        let rows = vec![row1];

        let data = CsvImporter::generate(&headers, &rows).expect("P9-1: CSV 生成失败");
        let content = String::from_utf8(data).expect("P9-1: UTF-8 解码失败");
        assert!(content.contains("name,age"));
        assert!(content.contains("Alice,30"));
    }

    #[test]
    fn test_field_validator_required() {
        assert!(FieldValidator::required("test", "名称").is_ok());
        assert!(FieldValidator::required("", "名称").is_err());
        assert!(FieldValidator::required("   ", "名称").is_err());
    }

    #[test]
    fn test_field_validator_integer() {
        assert_eq!(FieldValidator::integer("42", "数量").expect("P9-1: 整数校验"), 42);
        assert!(FieldValidator::integer("abc", "数量").is_err());
    }

    #[test]
    fn test_field_validator_decimal() {
        assert!(FieldValidator::decimal("99.99", "价格").is_ok());
        assert!(FieldValidator::decimal("abc", "价格").is_err());
    }

    #[test]
    fn test_field_validator_date() {
        assert!(FieldValidator::date("2024-01-15", "日期").is_ok());
        assert!(FieldValidator::date("2024/01/15", "日期").is_err());
    }

    #[test]
    fn test_field_validator_boolean() {
        // P9-1: 改用 expect 替代 unwrap，并明确中文失败原因
        assert!(FieldValidator::boolean("true", "启用").expect("P9-1: 布尔校验"));
        assert!(!FieldValidator::boolean("0", "启用").expect("P9-1: 布尔校验"));
        assert!(FieldValidator::boolean("是", "启用").expect("P9-1: 布尔校验"));
        assert!(FieldValidator::boolean("maybe", "启用").is_err());
    }

    #[test]
    fn test_field_validator_enum() {
        let allowed = &["A", "B", "C"];
        assert_eq!(
            FieldValidator::enum_value("B", "类型", allowed).expect("P9-1: 枚举校验"),
            "B"
        );
        assert!(FieldValidator::enum_value("D", "类型", allowed).is_err());
    }

    #[test]
    fn test_import_format_from_extension() {
        assert_eq!(
            ImportFormat::from_extension("csv").expect("P9-1: 扩展名解析"),
            ImportFormat::Csv
        );
        assert_eq!(
            ImportFormat::from_extension("xlsx").expect("P9-1: 扩展名解析"),
            ImportFormat::Excel
        );
        assert_eq!(
            ImportFormat::from_extension("XLS").expect("P9-1: 扩展名解析"),
            ImportFormat::Excel
        );
        assert!(ImportFormat::from_extension("pdf").is_none());
    }
}
