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

// 死代码清理（2026-06-26）：ImportFormat enum 及 impl 仅在测试中使用，无业务引用，已删除。
// 业务代码直接使用 import_export_handler 中的 import_csv/import_excel 函数。

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
    /// 解析 CSV 数据（data 为 CSV 字节数据；Ok(Vec<HashMap<String,String>>) 每行键值对，Err(AppError) 解析失败）
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

    /// 验证字段最大长度（v11 批次 156 P2-D：已被 product_service CSV 导入接入）
    pub fn max_length(value: &str, field_name: &str, max_len: usize) -> Result<(), String> {
        if value.len() > max_len {
            Err(format!("{} 长度不能超过 {} 个字符", field_name, max_len))
        } else {
            Ok(())
        }
    }
}
