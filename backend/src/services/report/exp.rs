//! 报表导出器服务（report/exp）
//!
//! 包含报表的多格式导出：
//! - `export_report`   统一导出入口
//! - `export_pdf`      PDF 导出（printpdf 库）
//! - `export_excel`    Excel 导出（xlsxwriter）
//! - `export_csv`      CSV 导出
//! - `export_json`     JSON 导出（已序列化数据直接返回）
//! - 内部：`build_sheet_xml` / `build_shared_strings_xml` / `column_letter`
//! - 公开：`*_bytes` 版本（无文件元信息，纯字节流）
//!
//! 拆分自原 `report_engine_service.rs` 的"报表导出"段。

use base64::Engine;
use chrono::Utc;
use std::io::Write;
use tracing::info;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::utils::error::AppError;

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
    pub async fn export_pdf(
        &self,
        data: &ReportData,
        template_name: &str,
    ) -> Result<PdfExportResult, AppError> {
        use printpdf::*;

        let (doc, page1, layer1) = PdfDocument::new(template_name, Mm(297.0), Mm(210.0), "Layer 1");
        let layer = doc.get_page(page1).get_layer(layer1);

        // 设置字体
        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| AppError::internal(format!("加载字体失败: {}", e)))?;

        // 标题
        layer.use_text(template_name, 16.0, Mm(20.0), Mm(280.0), &font);

        // 生成时间
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        layer.use_text(
            format!("生成时间: {}", now),
            10.0,
            Mm(20.0),
            Mm(270.0),
            &font,
        );

        // 表格头部
        let mut y_pos = 250.0_f32;
        let mut x_pos = 20.0_f32;
        let col_width = 35.0_f32;

        for column in &data.columns {
            layer.use_text(&column.label, 10.0, Mm(x_pos), Mm(y_pos), &font);
            x_pos += col_width;
        }

        y_pos -= 8.0;

        // 表格内容
        for row in &data.rows {
            x_pos = 20.0;
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
                layer.use_text(&display_value, 8.0, Mm(x_pos), Mm(y_pos), &font);
                x_pos += col_width;
            }
            y_pos -= 6.0;

            // 分页
            if y_pos < 20.0 {
                let (new_page, new_layer) = doc.add_page(Mm(297.0), Mm(210.0), "Layer 1");
                let _layer = doc.get_page(new_page).get_layer(new_layer);
                y_pos = 280.0;
                let _ = new_layer;
            }
        }

        // 页脚
        layer.use_text(
            format!("共 {} 条记录", data.total_rows),
            10.0,
            Mm(20.0),
            Mm(10.0),
            &font,
        );

        // 保存到内存
        let mut buffer = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buffer);
            doc.save(&mut writer)
                .map_err(|e| AppError::internal(format!("PDF 保存失败: {}", e)))?;
        }

        info!(
            "PDF 导出成功: template={}, rows={}, size={}",
            template_name,
            data.total_rows,
            buffer.len()
        );

        Ok(PdfExportResult {
            file_name: format!("{}_{}.pdf", template_name, now),
            file_size: buffer.len() as u64,
            page_count: 1,
            content: buffer,
        })
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

        // 1. [Content_Types].xml
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

        // 2. _rels/.rels
        zip.start_file("_rels/.rels", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;
        zip.write_all(rels.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;

        // 3. xl/_rels/workbook.xml.rels
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

        // 4. xl/workbook.xml
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

        // 5. xl/styles.xml
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

        // 6. xl/sharedStrings.xml
        let (shared_strings, string_lookup) = self.build_shared_strings_xml(data);
        zip.start_file("xl/sharedStrings.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        zip.write_all(shared_strings.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;

        // 7. xl/worksheets/sheet1.xml
        let sheet_xml = self.build_sheet_xml(data, &string_lookup);
        zip.start_file("xl/worksheets/sheet1.xml", options)
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;
        zip.write_all(sheet_xml.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP写入失败: {}", e)))?;

        let cursor = zip
            .finish()
            .map_err(|e| AppError::internal(format!("ZIP 结束失败: {}", e)))?;
        Ok(cursor.into_inner())
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
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>"#,
        );

        // 表头
        xml.push_str("<row r=\"1\">");
        for (idx, column) in data.columns.iter().enumerate() {
            let cell_ref = format!("{}{}", Self::column_letter(idx + 1), 1);
            // 字符串共享表索引未命中时默认 0（Excel 共享字符串索引）
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

        // 数据行
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

        xml.push_str("</sheetData></worksheet>");
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

    /// 导出 CSV
    pub async fn export_csv(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let mut output = Vec::new();

        // 写入 BOM 以支持 Excel 打开 UTF-8 CSV
        output.extend_from_slice(&[0xEF, 0xBB, 0xBF]);

        // 写入表头
        let headers: Vec<String> = data.columns.iter().map(|c| c.label.clone()).collect();
        writeln!(output, "{}", Self::encode_csv_row(&headers))
            .map_err(|e| AppError::internal(format!("CSV 写入失败: {}", e)))?;

        // 写入数据
        for row in &data.rows {
            let values: Vec<String> = data
                .columns
                .iter()
                .map(|col| {
                    row.get(&col.key)
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => v.to_string().trim_matches('"').to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            writeln!(output, "{}", Self::encode_csv_row(&values))
                .map_err(|e| AppError::internal(format!("CSV 写入失败: {}", e)))?;
        }

        info!(
            "CSV 导出成功: rows={}, size={}",
            data.total_rows,
            output.len()
        );
        Ok(output)
    }

    fn encode_csv_row(values: &[String]) -> String {
        values
            .iter()
            .map(|v| {
                if v.contains(',') || v.contains('"') || v.contains('\n') {
                    format!("\"{}\"", v.replace('"', "\"\""))
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",")
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

// 抑制未使用导入
#[allow(dead_code)]
fn _unused() {
    let _ = base64::engine::general_purpose::STANDARD.encode([0u8]);
}
