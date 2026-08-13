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
use axum::http::{HeaderValue, header};
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

    set_watermark_column_widths(worksheet, &table.headers, &table.rows, &watermark_text)?;

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
