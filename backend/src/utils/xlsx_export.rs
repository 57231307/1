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

/// V15 P0-S15 新增：导出水印配置（合规审计与防篡改）
///
/// 字段全部为 `Option<String>`，允许调用方按需填充；任何字段为 `None` 时
/// 该维度水印信息将省略（不显示占位符）。水印行整体为空时跳过插入。
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
    /// 渲染为单行水印文本（用 4 空格分隔各维度）
    ///
    /// 任一字段存在即输出；全部为 None 时返回 None（调用方据此决定是否插入水印行）。
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

/// V15 P0-S15 新增：带水印的 xlsx 构建
///
/// 在 `build_xlsx` 基础上，于标题行上方插入 1 行水印信息（合并所有列），
/// 记录操作员、客户端 IP、导出时间戳。水印行格式：
/// - 浅黄色背景（#FFF7CC）
/// - 红色字体（#CC0000）
/// - 居中对齐 + 加粗
///
/// 布局调整：
/// - 第 0 行：水印信息（合并所有列）
/// - 第 1 行：标题行（原第 0 行）
/// - 第 2 行起：数据行（原第 1 行起）
/// - 冻结前 2 行（水印行 + 标题行）
///
/// 当 `watermark.render()` 返回 None 时退化为 `build_xlsx` 行为。
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

    // 水印行格式：浅黄背景 + 红色字体 + 居中 + 加粗 + 边框
    let watermark_format = Format::new()
        .set_bold()
        .set_font_color("#CC0000")
        .set_background_color("#FFF7CC")
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

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

    // 第 0 行：水印信息（写入 A1 单元格，其余列仍写入空字符串保持边框完整）
    worksheet
        .write_with_format(0, 0, &watermark_text, &watermark_format)
        .map_err(|e| AppError::internal(format!("xlsx 写入水印失败: {}", e)))?;
    // 水印行其余列写入空字符串以应用边框（视觉上一行完整）
    for col in 1..table.headers.len() {
        worksheet
            .write_with_format(0, col as u16, "", &watermark_format)
            .map_err(|e| AppError::internal(format!("xlsx 写入水印占位失败: {}", e)))?;
    }
    // 合并水印行所有列（视觉上一行合并单元格）
    if table.headers.len() > 1 {
        worksheet
            .merge_range(
                0,
                0,
                0,
                (table.headers.len() - 1) as u16,
                &watermark_text,
                &watermark_format,
            )
            .map_err(|e| AppError::internal(format!("xlsx 合并水印行失败: {}", e)))?;
    }

    // 第 1 行：标题行
    for (col, header) in table.headers.iter().enumerate() {
        worksheet
            .write_with_format(1, col as u16, header, &header_format)
            .map_err(|e| AppError::internal(format!("xlsx 写入标题失败: {}", e)))?;
    }

    // 第 2 行起：数据行
    for (row_idx, row) in table.rows.iter().enumerate() {
        for (col, cell) in row.iter().enumerate() {
            worksheet
                .write_with_format((row_idx + 2) as u32, col as u16, cell, &data_format)
                .map_err(|e| AppError::internal(format!("xlsx 写入数据失败: {}", e)))?;
        }
    }

    // 冻结前 2 行（水印行 + 标题行）
    worksheet
        .set_freeze_panes(2, 0)
        .map_err(|e| AppError::internal(format!("xlsx 冻结前 2 行失败: {}", e)))?;

    // 列宽自适应（基于内容长度估算，最大 50，最小 10）
    // V15 P0-S15：水印文本长度也参与列宽估算，避免水印行被截断
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
        // 水印文本总长度按列数均分估算（每列至少容纳平均长度）
        let watermark_len = watermark_text.chars().count() / table.headers.len().max(1);
        let width = ((max_len.max(header_len).max(watermark_len) as f64) * 1.2 + 2.0)
            .clamp(10.0, 50.0);
        worksheet
            .set_column_width(col as u16, width)
            .map_err(|e| AppError::internal(format!("xlsx 设置列宽失败: {}", e)))?;
    }

    let bytes = workbook
        .save_to_buffer()
        .map_err(|e| AppError::internal(format!("xlsx 保存失败: {}", e)))?;
    Ok(bytes)
}

/// V15 P0-S15 新增：带水印的 xlsx 一站式响应构造
///
/// 等价于 `build_xlsx_with_watermark` + `xlsx_response`。
pub fn build_xlsx_response_with_watermark(
    table: &XlsxTable,
    filename: &str,
    watermark: &WatermarkConfig,
) -> Result<Response, AppError> {
    let bytes = build_xlsx_with_watermark(table, watermark)?;
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
