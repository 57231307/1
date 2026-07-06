//! xlsx 导出工具模块（v11 批次 142 新增）
//!
//! 规则 3 强制要求：所有数据导出功能必须使用 .xlsx 格式（Excel），
//! 禁止使用 CSV 作为最终交付格式。
//!
//! 本模块封装 rust_xlsxwriter 提供统一的导出接口：
//! - `build_xlsx`：从二维数据构建 xlsx 字节流
//! - `xlsx_response`：构造 axum Response（含正确 Content-Type 和 Content-Disposition）

use crate::utils::error::AppError;
use axum::http::{header, HeaderValue};
use axum::response::Response;
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};

/// xlsx 表格数据（标题行 + 数据行）
pub struct XlsxTable {
    /// 工作表名称（如 "线索列表" / "商机列表"）
    pub sheet_name: String,
    /// 标题行（第一行）
    pub headers: Vec<String>,
    /// 数据行（每行一个 Vec<String>，长度应与 headers 一致）
    pub rows: Vec<Vec<String>>,
}

/// 从 XlsxTable 构建 xlsx 字节流
///
/// 自动应用：
/// - 标题行加粗 + 浅灰背景
/// - 全表边框
/// - 冻结首行
/// - 列宽自适应（基于内容长度估算）
pub fn build_xlsx(table: &XlsxTable) -> Result<Vec<u8>, AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(&table.sheet_name)
        .map_err(|e| AppError::internal(format!("xlsx 工作表名称错误: {}", e)))?;

    // 标题行格式：加粗 + 浅灰背景 + 边框 + 居中对齐
    let header_format = Format::new()
        .set_bold()
        .set_background_color("#E0E0E0")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    // 数据行格式：边框 + 垂直居中
    let data_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::VerticalCenter);

    // 写入标题行
    for (col, header) in table.headers.iter().enumerate() {
        worksheet
            .write_with_format(0, col as u16, header, &header_format)
            .map_err(|e| AppError::internal(format!("xlsx 写入标题失败: {}", e)))?;
    }

    // 写入数据行
    for (row_idx, row) in table.rows.iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            worksheet
                .write_with_format((row_idx + 1) as u32, col as u16, cell, &data_format)
                .map_err(|e| AppError::internal(format!("xlsx 写入数据失败: {}", e)))?;
        }
    }

    // 冻结首行
    worksheet
        .set_freeze_panes(1, 0)
        .map_err(|e| AppError::internal(format!("xlsx 冻结首行失败: {}", e)))?;

    // 列宽自适应（基于内容长度估算，最大 50，最小 10）
    for col in 0..table.headers.len() {
        let max_len = table
            .rows
            .iter()
            .map(|row| {
                row.get(col)
                    .map(|s| s.chars().count())
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        let header_len = table.headers.get(col).map(|s| s.chars().count()).unwrap_or(0);
        let width = ((max_len.max(header_len) as f64) * 1.2 + 2.0).clamp(10.0, 50.0);
        worksheet
            .set_column_width(col as u16, width)
            .map_err(|e| AppError::internal(format!("xlsx 设置列宽失败: {}", e)))?;
    }

    // 保存到内存
    let bytes = workbook
        .save_to_buffer()
        .map_err(|e| AppError::internal(format!("xlsx 保存失败: {}", e)))?;
    Ok(bytes)
}

/// 构造 xlsx 下载响应
///
/// - Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
/// - Content-Disposition: attachment; filename="<filename>.xlsx"
pub fn xlsx_response(bytes: Vec<u8>, filename: &str) -> Response {
    let mut response = Response::new(bytes.into());
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    );
    let disposition = format!("attachment; filename=\"{}.xlsx\"", filename);
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

/// 一站式：从 XlsxTable 直接构造 axum Response
pub fn build_xlsx_response(table: &XlsxTable, filename: &str) -> Result<Response, AppError> {
    let bytes = build_xlsx(table)?;
    Ok(xlsx_response(bytes, filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 测试_xlsx_构建_基本表格() {
        let table = XlsxTable {
            sheet_name: "测试".to_string(),
            headers: vec!["编号".to_string(), "名称".to_string()],
            rows: vec![
                vec!["001".to_string(), "测试项目1".to_string()],
                vec!["002".to_string(), "测试项目2".to_string()],
            ],
        };
        let result = build_xlsx(&table);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // xlsx 文件最小大小约 4KB（zip 压缩格式）
        assert!(bytes.len() > 4000, "xlsx 文件大小异常: {}", bytes.len());
        // xlsx 文件以 PK 开头（zip 格式）
        assert_eq!(&bytes[0..2], b"PK", "xlsx 文件应以 PK 开头（zip 格式）");
    }

    #[test]
    fn 测试_xlsx_构建_空数据() {
        let table = XlsxTable {
            sheet_name: "空表".to_string(),
            headers: vec!["列1".to_string()],
            rows: vec![],
        };
        let result = build_xlsx(&table);
        assert!(result.is_ok());
    }

    #[test]
    fn 测试_xlsx_响应_正确的_content_type() {
        let response = xlsx_response(vec![1, 2, 3], "test");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            content_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
    }

    #[test]
    fn 测试_xlsx_响应_正确的_content_disposition() {
        let response = xlsx_response(vec![1, 2, 3], "crm_leads_export");
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(disposition.contains("crm_leads_export.xlsx"));
        assert!(disposition.contains("attachment"));
    }
}
