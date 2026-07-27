//! xlsx 导出工具模块（v11 批次 142 新增）
//!
//! 规则 3 强制要求：所有数据导出功能必须使用 .xlsx 格式（Excel），
//! 禁止使用 CSV 作为最终交付格式。
//!
//! 本模块封装 rust_xlsxwriter 提供统一的导出接口：
//! - `build_xlsx`：从二维数据构建 xlsx 字节流
//! - `xlsx_response`：构造 axum Response（含正确 Content-Type 和 Content-Disposition）
//! - `build_xlsx_with_watermark`：V15 P0-S15 新增，水印版导出（操作员/IP/时间戳/防篡改）
//!
//! V15 P0-S15 修复（Batch 474）：导出文件无水印问题。新增 `WatermarkConfig`
//! 结构体与 `build_xlsx_with_watermark` 函数，在 xlsx 标题行上方插入水印行，
//! 记录操作员、客户端 IP、导出时间戳，作为合规审计与防篡改证据。
//! 设计采用"新增函数 + 保留原 build_xlsx 不变"的向后兼容方案，避免影响 19 个
//! 已有 XlsxTable 构造点（详见规则 13 步骤 4 自审）。

use crate::utils::error::AppError;
use axum::http::{header, HeaderValue};
use axum::response::Response;
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook, Worksheet};

/// xlsx 表格数据（标题行 + 数据行）
pub struct XlsxTable {
    /// 工作表名称（如 "线索列表" / "商机列表"）
    pub sheet_name: String,
    /// 标题行（第一行）
    pub headers: Vec<String>,
    /// 数据行（每行一个 Vec<String>，长度应与 headers 一致）
    pub rows: Vec<Vec<String>>,
}

/// 导出水印配置（合规审计与防篡改），字段全为 Option，任一为 None 则省略该维度
#[derive(Debug, Clone, Default)]
pub struct WatermarkConfig {
    /// 操作员用户名（来自 AuthContext.username）
    pub operator: Option<String>,
    /// 客户端 IP（来自请求 x-forwarded-for 或 socket_addr）
    pub ip_address: Option<String>,
    /// 导出时间戳（ISO8601 字符串，建议 chrono::Utc::now().to_rfc3339()）
    pub exported_at: Option<String>,
    /// 额外信息（如资源类型说明，可选）
    pub extra: Option<String>,
}

impl WatermarkConfig {
    /// 渲染为单行水印文本（用 4 空格分隔各维度），全为 None 时返回 None
    pub fn render(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(op) = &self.operator {
            parts.push(format!("操作员:{}", op));
        }
        if let Some(ip) = &self.ip_address {
            parts.push(format!("导出IP:{}", ip));
        }
        if let Some(ts) = &self.exported_at {
            parts.push(format!("导出时间:{}", ts));
        }
        if let Some(ex) = &self.extra {
            parts.push(ex.to_string());
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("    "))
        }
    }
}

/// 从 XlsxTable 构建 xlsx 字节流（标题加粗+冻结首行+列宽自适应+全表边框）
pub fn build_xlsx(table: &XlsxTable) -> Result<Vec<u8>, AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(&table.sheet_name)
        .map_err(|e| AppError::internal(format!("xlsx 工作表名称错误: {}", e)))?;

    let header_format = make_header_format();
    let data_format = make_data_format();

    write_header_row(worksheet, 0, &table.headers, &header_format)?;
    write_data_rows(worksheet, 1, &table.rows, &data_format)?;

    worksheet
        .set_freeze_panes(1, 0)
        .map_err(|e| AppError::internal(format!("xlsx 冻结首行失败: {}", e)))?;

    set_column_widths(worksheet, &table.headers, &table.rows)?;

    let bytes = workbook
        .save_to_buffer()
        .map_err(|e| AppError::internal(format!("xlsx 保存失败: {}", e)))?;
    Ok(bytes)
}

/// 构造 xlsx 下载响应（含 Content-Type 和 Content-Disposition 头）
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

/// 带水印的 xlsx 构建：标题行上方插入水印行，render() 为 None 时退化为 build_xlsx
pub fn build_xlsx_with_watermark(
    table: &XlsxTable,
    watermark: &WatermarkConfig,
) -> Result<Vec<u8>, AppError> {
    let watermark_text = match watermark.render() {
        Some(t) => t,
        None => return build_xlsx(table),
    };

    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(&table.sheet_name)
        .map_err(|e| AppError::internal(format!("xlsx 工作表名称错误: {}", e)))?;

    let watermark_format = make_watermark_format();
    let header_format = make_header_format();
    let data_format = make_data_format();

    write_watermark_row(
        worksheet,
        &watermark_text,
        table.headers.len(),
        &watermark_format,
    )?;
    write_header_row(worksheet, 1, &table.headers, &header_format)?;
    write_data_rows(worksheet, 2, &table.rows, &data_format)?;

    worksheet
        .set_freeze_panes(2, 0)
        .map_err(|e| AppError::internal(format!("xlsx 冻结前 2 行失败: {}", e)))?;

    set_watermark_column_widths(
        worksheet,
        &table.headers,
        &table.rows,
        &watermark_text,
    )?;

    let bytes = workbook
        .save_to_buffer()
        .map_err(|e| AppError::internal(format!("xlsx 保存失败: {}", e)))?;
    Ok(bytes)
}

/// 带水印的 xlsx 一站式响应构造（等价于 build_xlsx_with_watermark + xlsx_response）
pub fn build_xlsx_response_with_watermark(
    table: &XlsxTable,
    filename: &str,
    watermark: &WatermarkConfig,
) -> Result<Response, AppError> {
    let bytes = build_xlsx_with_watermark(table, watermark)?;
    Ok(xlsx_response(bytes, filename))
}

// ----------------------------------------------------------------------
// 内部辅助函数
// ----------------------------------------------------------------------

/// 构建标题行格式（加粗 + 浅灰背景 + 边框 + 居中）
fn make_header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color("#E0E0E0")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
}

/// 构建数据行格式（边框 + 垂直居中）
fn make_data_format() -> Format {
    Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::VerticalCenter)
}

/// 构建水印行格式（浅黄背景 + 红色字体 + 居中 + 加粗 + 边框）
fn make_watermark_format() -> Format {
    Format::new()
        .set_bold()
        .set_font_color("#CC0000")
        .set_background_color("#FFF7CC")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
}

/// 在指定行写入标题行
fn write_header_row(
    worksheet: &mut Worksheet,
    row: u32,
    headers: &[String],
    format: &Format,
) -> Result<(), AppError> {
    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(row, col as u16, header, format)
            .map_err(|e| AppError::internal(format!("xlsx 写入标题失败: {}", e)))?;
    }
    Ok(())
}

/// 从指定行起写入数据行
fn write_data_rows(
    worksheet: &mut Worksheet,
    start_row: u32,
    rows: &[Vec<String>],
    format: &Format,
) -> Result<(), AppError> {
    for (row_idx, row) in rows.iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            worksheet
                .write_with_format(start_row + row_idx as u32, col as u16, cell, format)
                .map_err(|e| AppError::internal(format!("xlsx 写入数据失败: {}", e)))?;
        }
    }
    Ok(())
}

/// 写入水印行（含占位边框与合并单元格）
fn write_watermark_row(
    worksheet: &mut Worksheet,
    watermark_text: &str,
    headers_len: usize,
    format: &Format,
) -> Result<(), AppError> {
    worksheet
        .write_with_format(0, 0, watermark_text, format)
        .map_err(|e| AppError::internal(format!("xlsx 写入水印失败: {}", e)))?;
    for col in 1..headers_len {
        worksheet
            .write_with_format(0, col as u16, "", format)
            .map_err(|e| AppError::internal(format!("xlsx 写入水印占位失败: {}", e)))?;
    }
    if headers_len > 1 {
        worksheet
            .merge_range(0, 0, 0, (headers_len - 1) as u16, watermark_text, format)
            .map_err(|e| AppError::internal(format!("xlsx 合并水印行失败: {}", e)))?;
    }
    Ok(())
}

/// 设置列宽自适应（基于内容长度估算，最大 50，最小 10）
fn set_column_widths(
    worksheet: &mut Worksheet,
    headers: &[String],
    rows: &[Vec<String>],
) -> Result<(), AppError> {
    for col in 0..headers.len() {
        let max_len = rows
            .iter()
            .map(|row| row.get(col).map(|s| s.chars().count()).unwrap_or(0))
            .max()
            .unwrap_or(0);
        let header_len = headers.get(col).map(|s| s.chars().count()).unwrap_or(0);
        let width = ((max_len.max(header_len) as f64) * 1.2 + 2.0).clamp(10.0, 50.0);
        worksheet
            .set_column_width(col as u16, width)
            .map_err(|e| AppError::internal(format!("xlsx 设置列宽失败: {}", e)))?;
    }
    Ok(())
}

/// 设置列宽自适应（水印文本长度参与估算，避免水印行被截断）
fn set_watermark_column_widths(
    worksheet: &mut Worksheet,
    headers: &[String],
    rows: &[Vec<String>],
    watermark_text: &str,
) -> Result<(), AppError> {
    for col in 0..headers.len() {
        let max_len = rows
            .iter()
            .map(|row| row.get(col).map(|s| s.chars().count()).unwrap_or(0))
            .max()
            .unwrap_or(0);
        let header_len = headers.get(col).map(|s| s.chars().count()).unwrap_or(0);
        let watermark_len = watermark_text.chars().count() / headers.len().max(1);
        let width =
            ((max_len.max(header_len).max(watermark_len) as f64) * 1.2 + 2.0).clamp(10.0, 50.0);
        worksheet
            .set_column_width(col as u16, width)
            .map_err(|e| AppError::internal(format!("xlsx 设置列宽失败: {}", e)))?;
    }
    Ok(())
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

    /// V15 P0-S15：WatermarkConfig::render 全字段填充应输出 4 段
    #[test]
    fn 测试_watermark_render_全字段() {
        let wm = WatermarkConfig {
            operator: Some("admin".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            exported_at: Some("2026-07-17T10:00:00Z".to_string()),
            extra: Some("客户列表导出".to_string()),
        };
        let rendered = wm.render().expect("应输出水印文本");
        assert!(rendered.contains("操作员:admin"));
        assert!(rendered.contains("导出IP:127.0.0.1"));
        assert!(rendered.contains("导出时间:2026-07-17T10:00:00Z"));
        assert!(rendered.contains("客户列表导出"));
    }

    /// V15 P0-S15：WatermarkConfig::render 全字段为 None 应返回 None
    #[test]
    fn 测试_watermark_render_全空() {
        let wm = WatermarkConfig::default();
        assert!(wm.render().is_none());
    }

    /// V15 P0-S15：带水印 xlsx 构建应成功且文件大小合理
    #[test]
    fn 测试_xlsx_带水印_构建() {
        let table = XlsxTable {
            sheet_name: "客户列表".to_string(),
            headers: vec!["编码".to_string(), "名称".to_string()],
            rows: vec![
                vec!["C001".to_string(), "客户A".to_string()],
                vec!["C002".to_string(), "客户B".to_string()],
            ],
        };
        let wm = WatermarkConfig {
            operator: Some("admin".to_string()),
            ip_address: Some("10.0.0.1".to_string()),
            exported_at: Some("2026-07-17T10:00:00Z".to_string()),
            extra: Some("合规导出".to_string()),
        };
        let result = build_xlsx_with_watermark(&table, &wm);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // xlsx 文件最小约 4KB（带水印应略大）
        assert!(bytes.len() > 4000, "xlsx 带水印文件大小异常: {}", bytes.len());
        assert_eq!(&bytes[0..2], b"PK");
    }

    /// V15 P0-S15：水印为空时退化为 build_xlsx 行为（向后兼容）
    #[test]
    fn 测试_xlsx_带水印_空水印应退化() {
        let table = XlsxTable {
            sheet_name: "测试".to_string(),
            headers: vec!["列1".to_string()],
            rows: vec![vec!["v1".to_string()]],
        };
        let wm = WatermarkConfig::default();
        let with_wm = build_xlsx_with_watermark(&table, &wm).expect("应成功");
        let without_wm = build_xlsx(&table).expect("应成功");
        // 退化路径直接调用 build_xlsx，字节流应完全一致
        assert_eq!(with_wm, without_wm);
    }
}
