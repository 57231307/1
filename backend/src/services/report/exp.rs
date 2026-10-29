//! 报表导出器服务（report/exp）
//!
//! 包含报表的多格式导出：
//! - `export_report`   统一导出入口
//! - `export_pdf`      PDF 导出（printpdf 库）
//! - `export_excel`    Excel 导出（xlsxwriter）
//! - `export_csv`      CSV 导出
//! - `export_json`     JSON 导出（已序列化数据直接返回）
//! - 内部：`build_pdf_document` / `build_sheet_xml` / `build_shared_strings_xml` / `column_letter`
//! - 公开：`*_bytes` 版本（无文件元信息，纯字节流）
//!
//! 拆分自原 `report_engine_service.rs` 的"报表导出"段。

use chrono::Utc;
use std::io::Write;
use tracing::info;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::utils::error::AppError;
use crate::utils::xlsx_export::{XlsxTable, build_xlsx};

use super::{ExcelExportResult, ExportFormat, PdfExportResult, ReportData, ReportEngineService};

/// XLSX 命名空间
const XLSX_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

impl ReportEngineService {
    /// 统一报表导出入口
    pub async fn export_report(
        &self,
        data: &ReportData,
        format: &str,
        template_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        let export_format: ExportFormat = format
            .parse()
            .map_err(|e: String| AppError::bad_request(e))?;

        match export_format {
            ExportFormat::Pdf => {
                let result = self.export_pdf(data, template_name).await?;
                Ok(result.content)
            }
            ExportFormat::Excel => {
                let result = self.export_excel(data, template_name).await?;
                Ok(result.content)
            }
            ExportFormat::Csv => self.export_csv(data).await,
            ExportFormat::Json => self.export_json(data).await,
        }
    }

    /// 导出 PDF
    ///
    /// 调用方：`export_report` 与 `export_pdf_bytes`。
    /// 入参：报表数据与模板名。传给谁：`PdfExportResult`（文件名、字节流大小、
    /// 页数、字节流）回给出参的调用方。
    /// 存什么/存哪里：字节流留在返回值的 `content` 里，本次导出不落盘。
    pub async fn export_pdf(
        &self,
        data: &ReportData,
        template_name: &str,
    ) -> Result<PdfExportResult, AppError> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let (buffer, page_count) = Self::build_pdf_document(data, template_name, &now)?;
        info!(
            "PDF 导出成功: template={}, rows={}, size={}",
            template_name,
            data.total_rows,
            buffer.len()
        );
        Ok(PdfExportResult {
            file_name: format!("{}_{}.pdf", template_name, now),
            file_size: buffer.len() as u64,
            page_count,
            content: buffer,
        })
    }

    /// 构建报表 PDF 文档字节流（纯计算，不依赖服务实例、运行时与数据库）
    ///
    /// 入参：报表数据、模板名、已格式化好的生成时间文本（同时用于页头文案与文件名，
    /// 由调用方传入以保证两处一致）。
    /// 传给谁：`export_pdf`，以及不经过服务实例的纯计算调用方。
    /// 存什么/传什么数：返回 PDF 完整字节流，以及表格绘制实际占用的页数——页数由
    /// 翻页动作数出，不从外部猜测文档页数。
    /// 存哪里：字节流只随返回值留在内存。
    fn build_pdf_document(
        data: &ReportData,
        template_name: &str,
        now: &str,
    ) -> Result<(Vec<u8>, u32), AppError> {
        use printpdf::*;
        let (doc, page1, layer1) = PdfDocument::new(template_name, Mm(297.0), Mm(210.0), "Layer 1");
        let layer = doc.get_page(page1).get_layer(layer1);
        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| AppError::internal(format!("加载字体失败: {}", e)))?;
        Self::draw_pdf_header(&layer, &font, template_name, now);
        let (_last_y, page_count) = Self::draw_pdf_table(&doc, page1, layer1, &font, data);
        Self::draw_pdf_footer(&layer, &font, data.total_rows as usize);
        let buffer = Self::save_pdf_to_buffer(doc)?;
        Ok((buffer, page_count))
    }

    /// PDF 导出：绘制标题与生成时间
    fn draw_pdf_header(
        layer: &printpdf::PdfLayerReference,
        font: &printpdf::IndirectFontRef,
        template_name: &str,
        now: &str,
    ) {
        use printpdf::*;
        layer.use_text(template_name, 16.0, Mm(20.0), Mm(280.0), font);
        layer.use_text(
            format!("生成时间: {}", now),
            10.0,
            Mm(20.0),
            Mm(270.0),
            font,
        );
    }

    /// PDF 导出：绘制表头与表格内容（含分页）
    ///
    /// 入参：文档句柄、首页页号与首层层号、字体、报表数据。
    /// 返回二元组 `(最终 y 坐标, 页数)`：第一个元素是最后一行落笔后的 y 坐标（毫米）；
    /// 第二个元素告诉调用方本次绘制实际用掉了多少页——页数从首页起算，每执行一次翻页
    /// 动作加一枚，由本函数自己的翻页动作数出，不猜文档对象的页数。
    /// 传给谁：`build_pdf_document`，它取页数填导出结果。
    fn draw_pdf_table(
        doc: &printpdf::PdfDocumentReference,
        first_page: printpdf::PdfPageIndex,
        first_layer: printpdf::PdfLayerIndex,
        font: &printpdf::IndirectFontRef,
        data: &ReportData,
    ) -> (f32, u32) {
        use printpdf::*;
        let mut layer = doc.get_page(first_page).get_layer(first_layer);
        let mut y_pos = 250.0_f32;
        let mut page_count = 1_u32;
        let col_width = 35.0_f32;
        // 表头
        let mut x_pos = 20.0_f32;
        for column in &data.columns {
            layer.use_text(&column.label, 10.0, Mm(x_pos), Mm(y_pos), font);
            x_pos += col_width;
        }
        y_pos -= 8.0;
        // 内容
        for row in &data.rows {
            let mut x_pos = 20.0_f32;
            for column in &data.columns {
                let value = row
                    .get(&column.key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let display_value = if value.len() > 15 {
                    format!("{}...", &value[..15])
                } else {
                    value
                };
                layer.use_text(&display_value, 8.0, Mm(x_pos), Mm(y_pos), font);
                x_pos += col_width;
            }
            y_pos -= 6.0;
            // 分页
            if y_pos < 20.0 {
                let (new_page, new_layer) = doc.add_page(Mm(297.0), Mm(210.0), "Layer 1");
                layer = doc.get_page(new_page).get_layer(new_layer);
                y_pos = 280.0;
                page_count += 1;
            }
        }
        (y_pos, page_count)
    }

    /// PDF 导出：绘制页脚记录数
    fn draw_pdf_footer(
        layer: &printpdf::PdfLayerReference,
        font: &printpdf::IndirectFontRef,
        total_rows: usize,
    ) {
        use printpdf::*;
        layer.use_text(
            format!("共 {} 条记录", total_rows),
            10.0,
            Mm(20.0),
            Mm(10.0),
            font,
        );
    }

    /// PDF 导出：保存文档到内存缓冲区
    fn save_pdf_to_buffer(doc: printpdf::PdfDocumentReference) -> Result<Vec<u8>, AppError> {
        let mut buffer = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buffer);
            doc.save(&mut writer)
                .map_err(|e| AppError::internal(format!("PDF 保存失败: {}", e)))?;
        }
        Ok(buffer)
    }

    /// 导出 Excel
    pub async fn export_excel(
        &self,
        data: &ReportData,
        template_name: &str,
    ) -> Result<ExcelExportResult, AppError> {
        // 简化实现: 使用 zip 库手动构建 xlsx 文件
        let buffer = self.build_xlsx(data, template_name)?;
        let now = Utc::now().format("%Y%m%d_%H%M%S").to_string();

        info!(
            "Excel 导出成功: template={}, rows={}, size={}",
            template_name,
            data.total_rows,
            buffer.len()
        );

        Ok(ExcelExportResult {
            file_name: format!("{}_{}.xlsx", template_name, now),
            file_size: buffer.len() as u64,
            sheet_count: 1,
            row_count: data.total_rows,
            content: buffer,
        })
    }

    /// 构建 XLSX 文件（ZIP 容器）
    fn build_xlsx(&self, data: &ReportData, template_name: &str) -> Result<Vec<u8>, AppError> {
        let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        Self::write_content_types_xml(&mut zip, options)?;
        Self::write_rels_xml(&mut zip, options)?;
        Self::write_workbook_rels_xml(&mut zip, options)?;
        Self::write_workbook_xml(&mut zip, options, template_name)?;
        Self::write_styles_xml(&mut zip, options)?;
        self.write_shared_strings_and_sheet_xml(&mut zip, options, data)?;

        let cursor = zip
            .finish()
            .map_err(|e| AppError::internal(format!("ZIP 结束失败: {}", e)))?;
        Ok(cursor.into_inner())
    }

    /// 写入 [Content_Types].xml
    fn write_content_types_xml(
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
    ) -> Result<(), AppError> {
        zip.start_file("[Content_Types].xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>"#;
        zip.write_all(content_types.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 写入 _rels/.rels
    fn write_rels_xml(
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
    ) -> Result<(), AppError> {
        zip.start_file("_rels/.rels", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;
        zip.write_all(rels.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 写入 xl/_rels/workbook.xml.rels
    fn write_workbook_rels_xml(
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
    ) -> Result<(), AppError> {
        zip.start_file("xl/_rels/workbook.xml.rels", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let wb_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;
        zip.write_all(wb_rels.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 写入 xl/workbook.xml
    fn write_workbook_xml(
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
        template_name: &str,
    ) -> Result<(), AppError> {
        zip.start_file("xl/workbook.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let workbook = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="{}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="{}" sheetId="1" r:id="rId1"/>
</sheets>
</workbook>"#,
            XLSX_NS, template_name
        );
        zip.write_all(workbook.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 写入 xl/styles.xml
    fn write_styles_xml(
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
    ) -> Result<(), AppError> {
        zip.start_file("xl/styles.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts>
<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
<borders count="1"><border/></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/></cellXfs>
<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"#;
        zip.write_all(styles.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 写入 xl/sharedStrings.xml + xl/worksheets/sheet1.xml
    fn write_shared_strings_and_sheet_xml(
        &self,
        zip: &mut ZipWriter<std::io::Cursor<Vec<u8>>>,
        options: SimpleFileOptions,
        data: &ReportData,
    ) -> Result<(), AppError> {
        let (shared_strings, string_lookup) = self.build_shared_strings_xml(data);
        zip.start_file("xl/sharedStrings.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        zip.write_all(shared_strings.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;

        let sheet_xml = self.build_sheet_xml(data, &string_lookup);
        zip.start_file("xl/worksheets/sheet1.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        zip.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        Ok(())
    }

    /// 构建 sharedStrings.xml
    fn build_shared_strings_xml(
        &self,
        data: &ReportData,
    ) -> (String, std::collections::HashMap<String, usize>) {
        let mut strings = Vec::new();
        let mut lookup = std::collections::HashMap::new();

        // 添加列标签到共享字符串
        for column in &data.columns {
            let key = column.label.clone();
            lookup.insert(key.clone(), strings.len());
            strings.push(key);
        }

        // 添加所有行值到共享字符串
        for row in &data.rows {
            for column in &data.columns {
                let value = row
                    .get(&column.key)
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string().trim_matches('"').to_string(),
                    })
                    .unwrap_or_default();
                if !value.is_empty() && !lookup.contains_key(&value) {
                    lookup.insert(value.clone(), strings.len());
                    strings.push(value);
                }
            }
        }

        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count=""#,
        );
        xml.push_str(&strings.len().to_string());
        xml.push_str(r#"" uniqueCount=""#);
        xml.push_str(&strings.len().to_string());
        xml.push_str(r#"">"#);

        for s in strings {
            xml.push_str("<si><t xml:space=\"preserve\">");
            xml.push_str(
                &s.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            );
            xml.push_str("</t></si>");
        }

        xml.push_str("</sst>");
        (xml, lookup)
    }

    /// 构建 sheet1.xml
    fn build_sheet_xml(
        &self,
        data: &ReportData,
        string_lookup: &std::collections::HashMap<String, usize>,
    ) -> String {
        let mut xml = Self::build_sheet_xml_opening();
        xml.push_str(&Self::build_sheet_header_row(data, string_lookup));
        xml.push_str(&Self::build_sheet_data_rows(data, string_lookup));
        xml.push_str("</sheetData></worksheet>");
        xml
    }

    /// 构建 sheet XML 开头（声明 + worksheet + sheetData）
    fn build_sheet_xml_opening() -> String {
        String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
        )
    }

    /// 构建表头行 XML
    fn build_sheet_header_row(
        data: &ReportData,
        string_lookup: &std::collections::HashMap<String, usize>,
    ) -> String {
        let mut xml = String::from("<row r=\"1\">");
        for (idx, column) in data.columns.iter().enumerate() {
            let cell_ref = format!("{}{}", Self::column_letter(idx + 1), 1);
            let string_idx = string_lookup
                .get(&column.label)
                .copied()
                .unwrap_or_default();
            xml.push_str(&format!(
                r#"<c r="{}" t="s" s="1"><v>{}</v></c>"#,
                cell_ref, string_idx
            ));
        }
        xml.push_str("</row>");
        xml
    }

    /// 构建数据行 XML
    fn build_sheet_data_rows(
        data: &ReportData,
        string_lookup: &std::collections::HashMap<String, usize>,
    ) -> String {
        let mut xml = String::new();
        for (row_idx, row) in data.rows.iter().enumerate() {
            let row_num = row_idx + 2;
            xml.push_str(&format!("<row r=\"{}\">", row_num));
            for (col_idx, column) in data.columns.iter().enumerate() {
                let cell_ref = format!("{}{}", Self::column_letter(col_idx + 1), row_num);
                let value = row
                    .get(&column.key)
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => v.to_string().trim_matches('"').to_string(),
                    })
                    .unwrap_or_default();
                if let Some(&string_idx) = string_lookup.get(&value) {
                    xml.push_str(&format!(
                        r#"<c r="{}" t="s"><v>{}</v></c>"#,
                        cell_ref, string_idx
                    ));
                } else {
                    xml.push_str(&format!(r#"<c r="{}"><v>{}</v></c>"#, cell_ref, value));
                }
            }
            xml.push_str("</row>");
        }
        xml
    }

    /// 数字转 Excel 列字母 (1->A, 26->Z, 27->AA)
    fn column_letter(col_num: usize) -> String {
        let mut result = String::new();
        let mut n = col_num;
        while n > 0 {
            let rem = (n - 1) % 26;
            result.insert(0, (b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        result
    }

    /// 导出 CSV（规则 3：实际输出 xlsx 格式）
    pub async fn export_csv(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        // 规则 3：CSV 导出升级为 xlsx 格式，字段名与原 CSV 保持一致
        let headers: Vec<String> = data.columns.iter().map(|c| c.label.clone()).collect();
        let rows: Vec<Vec<String>> = data
            .rows
            .iter()
            .map(|row| {
                data.columns
                    .iter()
                    .map(|col| {
                        row.get(&col.key)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                _ => v.to_string().trim_matches('"').to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        let table = XlsxTable {
            sheet_name: "报表".to_string(),
            headers,
            rows,
        };
        let bytes = build_xlsx(&table)?;
        info!(
            "xlsx 导出成功: rows={}, size={}",
            data.total_rows,
            bytes.len()
        );
        Ok(bytes)
    }

    /// 导出 JSON
    pub async fn export_json(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let json = serde_json::to_vec_pretty(data)
            .map_err(|e| AppError::internal(format!("JSON 序列化失败: {}", e)))?;
        info!(
            "JSON 导出成功: rows={}, size={}",
            data.total_rows,
            json.len()
        );
        Ok(json)
    }

    // ==================================================
    // 公开字节流版本（无元信息，便于 API 直接返回）
    // ==================================================

    /// 导出 Excel 字节流
    pub async fn export_excel_bytes(
        &self,
        data: &ReportData,
        template_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        self.export_excel(data, template_name)
            .await
            .map(|r| r.content)
    }

    /// 导出 PDF 字节流
    pub async fn export_pdf_bytes(
        &self,
        data: &ReportData,
        template_name: &str,
    ) -> Result<Vec<u8>, AppError> {
        self.export_pdf(data, template_name)
            .await
            .map(|r| r.content)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ReportColumn, ReportData, ReportEngineService, ReportMetadata};
    use chrono::Utc;

    /// 构造单列定义
    ///
    /// 入参：字段名与显示名。传给谁：`sample_report_data`。
    /// 存什么：除 key、label、data_type 外其余展示属性一律留空，表格绘制只读前两枚。
    fn sample_column(key: &str, label: &str) -> ReportColumn {
        ReportColumn {
            key: key.to_string(),
            label: label.to_string(),
            field_alias: None,
            data_type: "string".to_string(),
            format: None,
            aggregation: None,
            sortable: false,
            filterable: false,
            width: None,
            alignment: None,
        }
    }

    /// 构造列定义与行字段同形的报表数据
    ///
    /// 入参：数据行数。传给谁：本模块两枚页数用例。
    /// 存什么：两列（生产单号与成品名称），行内字段名与列 key 一一对应；行值取 ASCII，
    /// 使单元格超长截断分支不会切在多字节字符边界上。
    fn sample_report_data(row_count: usize) -> ReportData {
        ReportData {
            columns: vec![
                sample_column("order_no", "生产单号"),
                sample_column("piece_name", "成品名称"),
            ],
            rows: (1..=row_count)
                .map(|idx| {
                    serde_json::json!({
                        "order_no": format!("MO-{:04}", idx),
                        "piece_name": format!("PIECE-{:04}", idx),
                    })
                })
                .collect(),
            total_rows: row_count as u64,
            generated_at: Utc::now(),
            summary: None,
            metadata: ReportMetadata {
                data_source: "unit-test".to_string(),
                query_time_ms: 0,
                cache_hit: false,
                parameters: None,
            },
        }
    }

    /// 表格行数超出首页容量时，页数必须等于翻页真实数出的页数
    ///
    /// 几何依据：表头画在 y=250，首行 y=242，每行下移 6，落笔后低于 20 即翻新页，
    /// 故首页画 38 行；60 行会翻出一枚新页，总数为 2。断言值 2 与硬编码 1 不同，
    /// 页数一旦被写回常量本用例即判红。
    #[test]
    fn page_count_counts_pages_created_by_table_paging() {
        let data = sample_report_data(60);
        let (content, page_count) =
            ReportEngineService::build_pdf_document(&data, "生产日报", "2026-01-01 08:00:00")
                .expect("PDF 文档构建应成功");

        assert_eq!(page_count, 2);
        assert!(!content.is_empty());
        assert!(content.starts_with(b"%PDF"));
    }

    /// 表格行数未超出首页容量时不得翻出新页，页数为 1
    #[test]
    fn page_count_stays_one_when_rows_fit_first_page() {
        let data = sample_report_data(5);
        let (content, page_count) =
            ReportEngineService::build_pdf_document(&data, "生产日报", "2026-01-01 08:00:00")
                .expect("PDF 文档构建应成功");

        assert_eq!(page_count, 1);
        assert!(!content.is_empty());
        assert!(content.starts_with(b"%PDF"));
    }
}
