//! docx 导出工具模块（V15 P1 batch-08 缺陷 8 新增）
//!
//! 规则 3 强制要求：所有报表/文档生成支持 .docx 格式（Word）：
//! 合同/发票/报表。禁止 .txt/.rtf/.html 等非标准格式作为成品文档。
//!
//! 本模块封装 docx-rs 提供统一的导出接口：
//! - `build_docx`：从 DocxTable 构建 docx 字节流（标题 + 表头 + 数据行）
//! - `docx_response`：构造 axum Response（含正确 Content-Type 和 Content-Disposition）
//! - `build_docx_with_kv`：键值对形式输出主表数据 + 明细表（适用于合同/对账单）

use crate::utils::error::AppError;
use axum::http::{header, HeaderValue};
use axum::response::Response;
use docx_rs::*;

/// docx 表格数据（标题 + 表头 + 数据行）
pub struct DocxTable {
    /// 文档主标题（如 "销售合同" / "应收账款对账单"）
    pub title: String,
    /// 工作表/段落副标题（可选）
    pub subtitle: Option<String>,
    /// 表头行
    pub headers: Vec<String>,
    /// 数据行（每行一个 Vec<String>，长度应与 headers 一致）
    pub rows: Vec<Vec<String>>,
}

/// 键值对明细（用于合同/对账单的主表字段展示）
pub struct DocxKeyValue {
    /// 键名列表（如 ["合同编号", "客户名称", "签订日期"]）
    pub keys: Vec<String>,
    /// 值列表（与 keys 一一对应）
    pub values: Vec<String>,
}

/// 从 DocxTable 构建 docx 字节流（标题段落 + 表头表格 + 数据行）
pub fn build_docx(table: &DocxTable) -> Result<Vec<u8>, AppError> {
    let mut docx = Docx::new();

    // 标题段落（居中加粗）
    let title_para = Paragraph::new()
        .add_run(Run::new().add_text(&table.title).bold().size(32))
        .align(AlignmentType::Center);
    docx = docx.add_paragraph(title_para);

    // 副标题（可选）
    if let Some(subtitle) = &table.subtitle {
        let sub_para = Paragraph::new()
            .add_run(Run::new().add_text(subtitle).size(24))
            .align(AlignmentType::Center);
        docx = docx.add_paragraph(sub_para);
    }

    // 空行
    docx = docx.add_paragraph(Paragraph::new());

    // 表格：表头 + 数据行
    if !table.headers.is_empty() {
        let header_row = build_table_row(&table.headers, true);
        let mut table_obj = Table::new(vec![]).add_row(header_row);

        for row in &table.rows {
            let data_row = build_table_row(row, false);
            table_obj = table_obj.add_row(data_row);
        }

        // 设置表格边框
        let table_obj = set_table_borders(table_obj);
        docx = docx.add_table(table_obj);
    }

    // 页脚段落：生成时间
    docx = docx.add_paragraph(Paragraph::new());
    let footer_text = format!(
        "生成时间：{}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let footer_para = Paragraph::new()
        .add_run(Run::new().add_text(&footer_text).italic().size(20))
        .align(AlignmentType::Right);
    docx = docx.add_paragraph(footer_para);

    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    docx.pack(&mut buf)
        .map_err(|e| AppError::internal(format!("docx 序列化失败: {}", e)))?;
    Ok(buf.into_inner())
}

/// 构造 docx 下载响应（含 Content-Type 和 Content-Disposition 头）
pub fn docx_response(bytes: Vec<u8>, filename: &str) -> Response {
    let mut response = Response::new(bytes.into());
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
    );
    let disposition = format!("attachment; filename=\"{}.docx\"", filename);
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

/// 一站式：从 DocxTable 直接构造 axum Response
pub fn build_docx_response(table: &DocxTable, filename: &str) -> Result<Response, AppError> {
    let bytes = build_docx(table)?;
    Ok(docx_response(bytes, filename))
}

/// 构建带键值对主表 + 明细表格的 docx（适用于合同/对账单；布局：标题→键值对表格(2列)→明细表格）
pub fn build_docx_with_kv(
    title: &str,
    kv: &DocxKeyValue,
    detail_headers: &[String],
    detail_rows: &[Vec<String>],
) -> Result<Vec<u8>, AppError> {
    let mut docx = Docx::new();

    // 标题段落
    let title_para = Paragraph::new()
        .add_run(Run::new().add_text(title).bold().size(32))
        .align(AlignmentType::Center);
    docx = docx.add_paragraph(title_para);

    docx = docx.add_paragraph(Paragraph::new());

    // 键值对表格（2 列：字段名 | 值）
    if !kv.keys.is_empty() {
        let header_row = build_table_row(&["字段".to_string(), "值".to_string()], true);
        let mut kv_table = Table::new(vec![]).add_row(header_row);

        for (k, v) in kv.keys.iter().zip(kv.values.iter()) {
            let row = build_table_row(&[k.clone(), v.clone()], false);
            kv_table = kv_table.add_row(row);
        }
        kv_table = set_table_borders(kv_table);
        docx = docx.add_table(kv_table);
    }

    // 明细表格
    if !detail_headers.is_empty() {
        docx = docx.add_paragraph(Paragraph::new());
        let detail_title = Paragraph::new().add_run(Run::new().add_text("明细").bold().size(24));
        docx = docx.add_paragraph(detail_title);

        let header_row = build_table_row(detail_headers, true);
        let mut detail_table = Table::new(vec![]).add_row(header_row);
        for row in detail_rows {
            let data_row = build_table_row(row, false);
            detail_table = detail_table.add_row(data_row);
        }
        detail_table = set_table_borders(detail_table);
        docx = docx.add_table(detail_table);
    }

    // 页脚
    docx = docx.add_paragraph(Paragraph::new());
    let footer_text = format!(
        "生成时间：{}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let footer_para = Paragraph::new()
        .add_run(Run::new().add_text(&footer_text).italic().size(20))
        .align(AlignmentType::Right);
    docx = docx.add_paragraph(footer_para);

    let mut buf = std::io::Cursor::new(Vec::<u8>::new());
    docx.pack(&mut buf)
        .map_err(|e| AppError::internal(format!("docx 序列化失败: {}", e)))?;
    Ok(buf.into_inner())
}

/// 构建表格行（cells 为单元格文本列表，is_header 标识是否为表头行）
fn build_table_row(cells: &[String], is_header: bool) -> TableRow {
    let mut cell_list: Vec<TableCell> = Vec::with_capacity(cells.len());
    for cell_text in cells {
        let run = if is_header {
            Run::new().add_text(cell_text).bold().size(22)
        } else {
            Run::new().add_text(cell_text).size(22)
        };
        let para = Paragraph::new().add_run(run);
        let cell = TableCell::new().add_paragraph(para);
        cell_list.push(cell);
    }
    TableRow::new(cell_list)
}

/// 设置表格四周边框（单线样式）
fn set_table_borders(table: Table) -> Table {
    table.set_borders(TableBorders::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 测试_docx_构建_基本表格() {
        let table = DocxTable {
            title: "测试文档".to_string(),
            subtitle: None,
            headers: vec!["编号".to_string(), "名称".to_string()],
            rows: vec![
                vec!["001".to_string(), "测试项目1".to_string()],
                vec!["002".to_string(), "测试项目2".to_string()],
            ],
        };
        let result = build_docx(&table);
        assert!(result.is_ok(), "docx 构建应成功");
        let bytes = result.unwrap();
        // docx 文件以 PK 开头（zip 格式）
        assert_eq!(&bytes[0..2], b"PK", "docx 文件应以 PK 开头（zip 格式）");
        assert!(bytes.len() > 1000, "docx 文件大小异常: {}", bytes.len());
    }

    #[test]
    fn 测试_docx_构建_空数据() {
        let table = DocxTable {
            title: "空表".to_string(),
            subtitle: None,
            headers: vec![],
            rows: vec![],
        };
        let result = build_docx(&table);
        assert!(result.is_ok(), "docx 空表构建应成功");
    }

    #[test]
    fn 测试_docx_响应_正确的_content_type() {
        let response = docx_response(vec![1, 2, 3], "test");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            content_type,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
    }

    #[test]
    fn 测试_docx_响应_正确的_content_disposition() {
        let response = docx_response(vec![1, 2, 3], "sales_contract_001");
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(disposition.contains("sales_contract_001.docx"));
        assert!(disposition.contains("attachment"));
    }

    #[test]
    fn 测试_docx_带键值对构建() {
        let kv = DocxKeyValue {
            keys: vec!["合同编号".to_string(), "客户名称".to_string()],
            values: vec!["HT-2026-001".to_string(), "客户A".to_string()],
        };
        let result = build_docx_with_kv(
            "销售合同",
            &kv,
            &["序号".to_string(), "产品".to_string()],
            &[
                vec!["1".to_string(), "面料A".to_string()],
                vec!["2".to_string(), "面料B".to_string()],
            ],
        );
        assert!(result.is_ok(), "带键值对 docx 构建应成功");
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..2], b"PK");
    }
}
