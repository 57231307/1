//! 报表引擎 Service
//!
//! 提供报表模板管理、数据导出、动态模板创建、PDF/Excel 导出、报表订阅与定时发送功能

#![allow(dead_code)]

use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use std::sync::Arc;
use std::collections::HashMap;
use std::io::Write;
use sea_orm::DatabaseConnection;
use chrono::{Utc, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::purchase_order::Entity as PurchaseOrderEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::utils::error::AppError;

/// 报表类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    Sales,
    Purchase,
    Inventory,
    Financial,
    Custom,
}

/// 报表模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub columns: Vec<ReportColumn>,
    pub filters: Vec<ReportFilter>,
    pub sort_by: Option<String>,
    pub sort_order: String,
}

/// 报表列定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportColumn {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub width: Option<i32>,
    pub format: Option<String>,
}

/// 报表筛选条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// 报表数据
#[derive(Debug, Clone)]
pub struct ReportData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

/// 导出格式
#[derive(Debug, Clone)]
pub enum ExportFormat {
    CSV,
    Excel,
    PDF,
    JSON,
}

/// 动态报表模板创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub report_type: String,
    pub columns: Vec<CreateColumnRequest>,
    pub filters: Vec<CreateFilterRequest>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// 动态列定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateColumnRequest {
    pub field: String,
    pub title: String,
    pub data_type: String,
    pub width: Option<i32>,
    pub format: Option<String>,
}

/// 动态筛选条件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFilterRequest {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// 报表订阅
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSubscription {
    pub id: String,
    pub user_id: i32,
    pub template_id: String,
    pub name: String,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub format: String,
    pub enabled: bool,
    pub created_at: NaiveDateTime,
    pub next_run_at: Option<NaiveDateTime>,
}

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub template_id: String,
    pub name: String,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub format: Option<String>,
}

/// PDF 导出结果
#[derive(Debug)]
pub struct PdfExportResult {
    pub data: Vec<u8>,
    pub filename: String,
}

/// Excel 导出结果
#[derive(Debug)]
pub struct ExcelExportResult {
    pub data: Vec<u8>,
    pub filename: String,
}

/// 报表引擎 Service
pub struct ReportEngineService {
    db: Arc<DatabaseConnection>,
}

impl ReportEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取预定义报表模板
    pub fn get_predefined_templates() -> Vec<ReportTemplate> {
        vec![
            ReportTemplate {
                id: "sales_daily".to_string(),
                name: "销售日报表".to_string(),
                report_type: ReportType::Sales,
                columns: vec![
                    ReportColumn { field: "order_no".to_string(), title: "订单编号".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "customer_name".to_string(), title: "客户名称".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "order_date".to_string(), title: "订单日期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                    ReportColumn { field: "total_amount".to_string(), title: "订单金额".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "status".to_string(), title: "状态".to_string(), data_type: "string".to_string(), width: Some(100), format: None },
                ],
                filters: vec![
                    ReportFilter { field: "order_date".to_string(), operator: ">=".to_string(), value: "today-30".to_string() },
                ],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "inventory_status".to_string(),
                name: "库存状态报表".to_string(),
                report_type: ReportType::Inventory,
                columns: vec![
                    ReportColumn { field: "product_code".to_string(), title: "产品编码".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "product_name".to_string(), title: "产品名称".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "quantity_available".to_string(), title: "可用库存".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "quantity_reserved".to_string(), title: "预留库存".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "warehouse".to_string(), title: "仓库".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                ],
                filters: vec![],
                sort_by: Some("quantity_available".to_string()),
                sort_order: "desc".to_string(),
            },
            ReportTemplate {
                id: "purchase_summary".to_string(),
                name: "采购汇总报表".to_string(),
                report_type: ReportType::Purchase,
                columns: vec![
                    ReportColumn { field: "order_no".to_string(), title: "采购单号".to_string(), data_type: "string".to_string(), width: Some(150), format: None },
                    ReportColumn { field: "supplier_name".to_string(), title: "供应商".to_string(), data_type: "string".to_string(), width: Some(200), format: None },
                    ReportColumn { field: "order_date".to_string(), title: "下单日期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                    ReportColumn { field: "total_amount".to_string(), title: "采购金额".to_string(), data_type: "decimal".to_string(), width: Some(120), format: Some("#.00".to_string()) },
                    ReportColumn { field: "delivery_date".to_string(), title: "交期".to_string(), data_type: "date".to_string(), width: Some(120), format: Some("YYYY-MM-DD".to_string()) },
                ],
                filters: vec![
                    ReportFilter { field: "order_date".to_string(), operator: ">=".to_string(), value: "today-30".to_string() },
                ],
                sort_by: Some("order_date".to_string()),
                sort_order: "desc".to_string(),
            },
        ]
    }

    /// 创建自定义报表模板
    pub fn create_custom_template(req: CreateTemplateRequest) -> Result<ReportTemplate, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::ValidationError("模板名称不能为空".to_string()));
        }
        if req.columns.is_empty() {
            return Err(AppError::ValidationError("至少需要定义一个列".to_string()));
        }

        let report_type = match req.report_type.as_str() {
            "sales" => ReportType::Sales,
            "purchase" => ReportType::Purchase,
            "inventory" => ReportType::Inventory,
            "financial" => ReportType::Financial,
            _ => ReportType::Custom,
        };

        let template_id = format!("custom_{}", Utc::now().timestamp_millis());

        let columns: Vec<ReportColumn> = req.columns.into_iter().map(|c| ReportColumn {
            field: c.field,
            title: c.title,
            data_type: c.data_type,
            width: c.width,
            format: c.format,
        }).collect();

        let filters: Vec<ReportFilter> = req.filters.into_iter().map(|f| ReportFilter {
            field: f.field,
            operator: f.operator,
            value: f.value,
        }).collect();

        Ok(ReportTemplate {
            id: template_id,
            name: req.name,
            report_type,
            columns,
            filters,
            sort_by: req.sort_by,
            sort_order: req.sort_order.unwrap_or_else(|| "asc".to_string()),
        })
    }

    /// 获取所有模板（预定义 + 自定义）
    pub fn get_all_templates(custom_templates: &[ReportTemplate]) -> Vec<ReportTemplate> {
        let mut all = Self::get_predefined_templates();
        all.extend(custom_templates.iter().cloned());
        all
    }

    /// 执行报表查询
    pub async fn execute_report(
        &self,
        template_id: &str,
        custom_filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        match template_id {
            "sales_daily" => self.query_sales_report(custom_filters, page, page_size).await,
            "inventory_status" => self.query_inventory_report(custom_filters, page, page_size).await,
            "purchase_summary" => self.query_purchase_report(custom_filters, page, page_size).await,
            id if id.starts_with("custom_") => {
                Err(AppError::NotFound("自定义模板需要通过列定义动态生成查询，暂不支持直接执行".to_string()))
            }
            _ => Err(AppError::NotFound(format!("报表模板 {} 不存在", template_id))),
        }
    }

    /// 销售报表查询
    async fn query_sales_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = SalesOrderEntity::find()
            .order_by_desc(crate::models::sales_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = SalesOrderEntity::find()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "订单编号".to_string(),
            "客户ID".to_string(),
            "订单金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.customer_id.to_string(),
                    o.total_amount.to_string(),
                    o.status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 库存报表查询
    async fn query_inventory_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = InventoryStockEntity::find()
            .order_by_desc(crate::models::inventory_stock::Column::QuantityAvailable)
            .paginate(&*self.db, page_size);

        let stocks = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = InventoryStockEntity::find()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "产品ID".to_string(),
            "可用库存".to_string(),
            "预留库存".to_string(),
            "在途库存".to_string(),
            "仓库ID".to_string(),
        ];

        let rows: Vec<Vec<String>> = stocks
            .into_iter()
            .map(|s| {
                vec![
                    s.product_id.to_string(),
                    s.quantity_available.to_string(),
                    s.quantity_reserved.to_string(),
                    s.quantity_incoming.to_string(),
                    s.warehouse_id.to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 采购报表查询
    async fn query_purchase_report(
        &self,
        _filters: Vec<ReportFilter>,
        page: u64,
        page_size: u64,
    ) -> Result<ReportData, AppError> {
        let paginator = PurchaseOrderEntity::find()
            .order_by_desc(crate::models::purchase_order::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let orders = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total = PurchaseOrderEntity::find()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let columns = vec![
            "采购单号".to_string(),
            "供应商ID".to_string(),
            "采购金额".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = orders
            .into_iter()
            .map(|o| {
                vec![
                    o.order_no,
                    o.supplier_id.to_string(),
                    o.total_amount.to_string(),
                    o.order_status,
                    o.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect();

        Ok(ReportData {
            columns,
            rows,
            total_count: total,
        })
    }

    /// 导出报表数据
    pub fn export_report(
        &self,
        data: &ReportData,
        format: ExportFormat,
    ) -> Result<Vec<u8>, AppError> {
        match format {
            ExportFormat::CSV => self.export_csv(data),
            ExportFormat::JSON => self.export_json(data),
            ExportFormat::Excel => self.export_excel_bytes(data),
            ExportFormat::PDF => self.export_pdf_bytes(data),
        }
    }

    /// PDF 导出
    pub fn export_pdf(&self, data: &ReportData, title: &str) -> Result<PdfExportResult, AppError> {
        let pdf_bytes = self.generate_pdf(data, title)?;
        let filename = format!("{}_{}.pdf", title, Utc::now().format("%Y%m%d%H%M%S"));
        Ok(PdfExportResult { data: pdf_bytes, filename })
    }

    /// Excel 导出
    pub fn export_excel(&self, data: &ReportData, title: &str) -> Result<ExcelExportResult, AppError> {
        let xlsx_bytes = self.generate_xlsx(data, title)?;
        let filename = format!("{}_{}.xlsx", title, Utc::now().format("%Y%m%d%H%M%S"));
        Ok(ExcelExportResult { data: xlsx_bytes, filename })
    }

    /// 生成 PDF 字节流（使用简单的 PDF 1.4 格式）
    fn generate_pdf(&self, data: &ReportData, title: &str) -> Result<Vec<u8>, AppError> {
        let mut pdf = Vec::new();

        // 计算页面尺寸和内容
        let page_width = 595.0_f64;
        let page_height = 842.0_f64;
        let margin = 50.0_f64;
        let line_height = 14.0_f64;
        let col_count = data.columns.len().max(1);
        let usable_width = page_width - 2.0 * margin;
        let col_width = usable_width / col_count as f64;

        // 构建文本内容
        let mut text_lines: Vec<String> = Vec::new();
        text_lines.push(title.to_string());
        text_lines.push(String::new());

        // 表头
        let header_line = data.columns.join("  |  ");
        let header_len = header_line.len().min(80);
        text_lines.push(header_line);
        text_lines.push("-".repeat(header_len));

        // 数据行
        for row in &data.rows {
            let line = row.join("  |  ");
            // 截断过长行
            if line.len() > 100 {
                text_lines.push(format!("{}...", &line[..97]));
            } else {
                text_lines.push(line);
            }
        }

        let total_lines = text_lines.len();
        let content_height = total_lines as f64 * line_height;
        let pages_needed = ((content_height / (page_height - 2.0 * margin)).ceil() as usize).max(1);

        // PDF Header
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let mut offsets = Vec::new();
        let mut page_refs = Vec::new();

        // Object 1: Catalog
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        offsets.push(pdf.len());
        let page_refs_str: String = (3..3 + pages_needed)
            .map(|i| format!("{} 0 R", i))
            .collect::<Vec<_>>()
            .join(" ");
        pdf.extend_from_slice(format!("2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n", page_refs_str, pages_needed).as_bytes());

        // Page objects and content streams
        let mut obj_num = 3;
        for page_idx in 0..pages_needed {
            page_refs.push(obj_num);
            let content_obj = obj_num + pages_needed;

            offsets.push(pdf.len());
            pdf.extend_from_slice(format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
                obj_num, page_width as u32, page_height as u32, content_obj
            ).as_bytes());
            obj_num += 1;
        }

        // Content streams
        for page_idx in 0..pages_needed {
            offsets.push(pdf.len());

            let start_line = page_idx * ((page_height - 2.0 * margin) / line_height) as usize;
            let end_line = ((start_line + ((page_height - 2.0 * margin) / line_height) as usize).min(total_lines));

            let mut stream_content = String::new();
            stream_content.push_str("BT\n");
            stream_content.push_str("/F1 10 Tf\n");

            let mut y_pos = page_height - margin;

            for line_idx in start_line..end_line {
                y_pos -= line_height;
                let y_str = format!("{:.1}", y_pos);
                let escaped = text_lines[line_idx]
                    .replace('\\', "\\\\")
                    .replace('(', "\\(")
                    .replace(')', "\\)");
                if line_idx == 0 {
                    stream_content.push_str(&format!("{} {} Td\n", margin, y_str));
                    stream_content.push_str("/F1 14 Tf\n");
                    stream_content.push_str(&format!("({}) Tj\n", escaped));
                    stream_content.push_str("/F1 10 Tf\n");
                } else {
                    stream_content.push_str(&format!("0 -{} Td\n", line_height));
                    stream_content.push_str(&format!("({}) Tj\n", escaped));
                }
            }

            stream_content.push_str("ET\n");

            let stream_bytes = stream_content.as_bytes();
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                obj_num + page_idx,
                stream_bytes.len(),
                stream_content
            ).as_bytes());
        }

        // Font object
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

        // Cross-reference table
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(format!("0 {}\n", offsets.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        }

        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(format!("<< /Size {} /Root 1 0 R >>\n", offsets.len() + 1).as_bytes());
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n%%EOF\n", xref_offset).as_bytes());

        Ok(pdf)
    }

    /// 生成 XLSX 字节流（OOXML 格式）
    fn generate_xlsx(&self, data: &ReportData, title: &str) -> Result<Vec<u8>, AppError> {
        let mut buf = Vec::new();
        {
            let writer = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(writer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            // [Content_Types].xml
            zip.start_file("[Content_Types].xml", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            zip.write_all(CONTENT_TYPES_XML.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // _rels/.rels
            zip.start_file("_rels/.rels", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            zip.write_all(RELS_XML.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // xl/workbook.xml
            zip.start_file("xl/workbook.xml", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            zip.write_all(WORKBOOK_XML.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // xl/_rels/workbook.xml.rels
            zip.start_file("xl/_rels/workbook.xml.rels", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            zip.write_all(WORKBOOK_RELS_XML.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // xl/styles.xml
            zip.start_file("xl/styles.xml", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            zip.write_all(STYLES_XML.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // xl/worksheets/sheet1.xml
            zip.start_file("xl/worksheets/sheet1.xml", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            let sheet_xml = self.build_sheet_xml(data, title);
            zip.write_all(sheet_xml.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            // xl/sharedStrings.xml
            zip.start_file("xl/sharedStrings.xml", options).map_err(|e| AppError::InternalError(e.to_string()))?;
            let strings_xml = self.build_shared_strings_xml(data, title);
            zip.write_all(strings_xml.as_bytes()).map_err(|e| AppError::InternalError(e.to_string()))?;

            zip.finish().map_err(|e| AppError::InternalError(e.to_string()))?;
        }
        Ok(buf)
    }

    /// 构建 Sheet XML
    fn build_sheet_xml(&self, data: &ReportData, title: &str) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);
        xml.push_str("\n<sheetData>\n");

        let mut string_index = 0usize;

        // Title row
        xml.push_str(&format!("<row r=\"1\"><c r=\"A1\" t=\"s\"><v>{}</v></c></row>\n", string_index));
        string_index += 1;

        // Header row
        xml.push_str("<row r=\"2\">");
        for (col_idx, col) in data.columns.iter().enumerate() {
            let col_letter = column_letter(col_idx);
            xml.push_str(&format!("<c r=\"{}2\" t=\"s\"><v>{}</v></c>", col_letter, string_index));
            string_index += 1;
        }
        xml.push_str("</row>\n");

        // Data rows
        for (row_idx, row) in data.rows.iter().enumerate() {
            let row_num = row_idx + 3;
            xml.push_str(&format!("<row r=\"{}\">", row_num));
            for (col_idx, cell) in row.iter().enumerate() {
                let col_letter = column_letter(col_idx);
                // Try to parse as number
                if let Ok(_num) = cell.parse::<f64>() {
                    xml.push_str(&format!("<c r=\"{}{}\"><v>{}</v></c>", col_letter, row_num, cell));
                } else {
                    xml.push_str(&format!("<c r=\"{}{}\" t=\"s\"><v>{}</v></c>", col_letter, row_num, string_index));
                    string_index += 1;
                }
            }
            xml.push_str("</row>\n");
        }

        xml.push_str("</sheetData>\n</worksheet>");
        xml
    }

    /// 构建 SharedStrings XML
    fn build_shared_strings_xml(&self, data: &ReportData, title: &str) -> String {
        let mut strings: Vec<String> = Vec::new();
        strings.push(title.to_string());
        for col in &data.columns {
            strings.push(col.clone());
        }
        for row in &data.rows {
            for cell in row {
                // Only add non-numeric strings
                if cell.parse::<f64>().is_err() {
                    strings.push(cell.clone());
                }
            }
        }

        let count = strings.len();
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
            count, count
        );
        for s in &strings {
            let escaped = s
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            xml.push_str(&format!("\n<si><t>{}</t></si>", escaped));
        }
        xml.push_str("\n</sst>");
        xml
    }

    /// 导出CSV格式
    fn export_csv(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(&data.columns).map_err(|e| AppError::InternalError(e.to_string()))?;
        for row in &data.rows {
            wtr.write_record(row).map_err(|e| AppError::InternalError(e.to_string()))?;
        }
        wtr.into_inner().map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 导出JSON格式
    fn export_json(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        let json_data: Vec<serde_json::Map<String, serde_json::Value>> = data
            .rows
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, col) in data.columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        map.insert(col.clone(), serde_json::Value::String(val.clone()));
                    }
                }
                map
            })
            .collect();

        let json = serde_json::to_string(&json_data)
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(json.into_bytes())
    }

    /// 导出 Excel 字节（内部方法）
    fn export_excel_bytes(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        self.generate_xlsx(data, "Report")
    }

    /// 导出 PDF 字节（内部方法）
    fn export_pdf_bytes(&self, data: &ReportData) -> Result<Vec<u8>, AppError> {
        self.generate_pdf(data, "Report")
    }

    /// 创建报表订阅
    pub fn create_subscription(
        user_id: i32,
        req: CreateSubscriptionRequest,
    ) -> Result<ReportSubscription, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::ValidationError("订阅名称不能为空".to_string()));
        }
        if req.template_id.trim().is_empty() {
            return Err(AppError::ValidationError("模板ID不能为空".to_string()));
        }
        if req.recipients.is_empty() {
            return Err(AppError::ValidationError("至少需要一个收件人".to_string()));
        }

        let valid_frequencies = ["daily", "weekly", "monthly", "quarterly"];
        if !valid_frequencies.contains(&req.frequency.as_str()) {
            return Err(AppError::ValidationError(
                format!("频率必须是以下之一: {:?}", valid_frequencies)
            ));
        }

        let now = Utc::now().naive_utc();
        let next_run = Self::calculate_next_run(&req.frequency, now);

        Ok(ReportSubscription {
            id: format!("sub_{}", now.timestamp_millis()),
            user_id,
            template_id: req.template_id,
            name: req.name,
            frequency: req.frequency,
            recipients: req.recipients,
            format: req.format.unwrap_or_else(|| "pdf".to_string()),
            enabled: true,
            created_at: now,
            next_run_at: Some(next_run),
        })
    }

    /// 计算下次执行时间
    fn calculate_next_run(frequency: &str, from: NaiveDateTime) -> NaiveDateTime {
        use chrono::Duration;
        match frequency {
            "daily" => from + Duration::days(1),
            "weekly" => from + Duration::weeks(1),
            "monthly" => from + Duration::days(30),
            "quarterly" => from + Duration::days(90),
            _ => from + Duration::days(1),
        }
    }
}

/// 生成 Excel 列字母（A, B, ..., Z, AA, AB, ...）
fn column_letter(index: usize) -> String {
    let mut col = String::new();
    let mut n = index;
    loop {
        col.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 { break; }
        n = n / 26 - 1;
    }
    col
}

// XLSX 模板常量
const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#;

const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

const WORKBOOK_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Sheet1" sheetId="1" r:id="rId1"/>
</sheets>
</workbook>"#;

const WORKBOOK_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#;

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="2">
<font><sz val="10"/><name val="Arial"/></font>
<font><b/><sz val="14"/><name val="Arial"/></font>
</fonts>
<fills count="3">
<fill><patternFill patternType="none"/></fill>
<fill><patternFill patternType="gray125"/></fill>
<fill><patternFill patternType="solid"><fgColor rgb="FF4472C4"/></patternFill></fill>
</fills>
<borders count="1">
<border><left/><right/><top/><bottom/><diagonal/></border>
</borders>
<cellStyleXfs count="1"><xf/></cellStyleXfs>
<cellXfs count="1">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
</cellXfs>
</styleSheet>"#;
