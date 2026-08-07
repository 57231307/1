//! 导出服务
//!
//! 提供 PDF、Excel、Word 格式的导出功能
//! v11 批次 161 CI2：移除 CSV 导出，规则 3 要求 xlsx 交付
//! V15 P1 batch-08 缺陷 8：新增 docx 导出，规则 3 要求合同/发票/报表支持 .docx

use crate::utils::docx_export::{build_docx_with_kv, DocxKeyValue};
use crate::utils::error::AppError;
use crate::utils::xlsx_export::{build_xlsx, XlsxTable};
use serde::{Deserialize, Serialize};

/// 导出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub summary: Option<Vec<(String, String)>>,
}

/// 导出服务
pub struct ExportService;

impl ExportService {
    /// 导出为 Excel 格式（xlsx）
    pub fn export_excel(data: &ExportData) -> Result<Vec<u8>, AppError> {
        // 规则 3：使用 rust_xlsxwriter 构建真正的 xlsx 文件
        let mut rows = data.rows.clone();
        // 追加汇总行（如有），与原 CSV 导出行为保持一致
        if let Some(summary) = &data.summary {
            rows.push(Vec::new()); // 空行分隔
            for (key, value) in summary {
                rows.push(vec![key.clone(), value.clone()]);
            }
        }
        let table = XlsxTable {
            sheet_name: data.title.clone(),
            headers: data.headers.clone(),
            rows,
        };
        build_xlsx(&table)
    }

    /// 导出为真实 PDF 格式（规则 3：禁止以 PDF 名义交付文本，使用 printpdf 生成真实 PDF 字节流）
    pub fn export_pdf(data: &ExportData) -> Result<Vec<u8>, AppError> {
        use printpdf::*;
        let (doc, page1, layer1) = PdfDocument::new(data.title.as_str(), Mm(297.0), Mm(210.0), "Layer 1");
        let layer = doc.get_page(page1).get_layer(layer1);
        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| AppError::internal(format!("加载字体失败: {}", e)))?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 标题与生成时间
        layer.use_text(data.title.as_str(), 16.0, Mm(20.0), Mm(280.0), &font);
        layer.use_text(format!("生成时间: {}", now).as_str(), 10.0, Mm(20.0), Mm(270.0), &font);

        // 表头与数据（含分页）
        let mut y_pos = 250.0_f32;
        let col_width = 35.0_f32;
        let mut x_pos = 20.0_f32;
        for h in &data.headers {
            layer.use_text(h.as_str(), 10.0, Mm(x_pos), Mm(y_pos), &font);
            x_pos += col_width;
        }
        y_pos -= 8.0;
        for row in &data.rows {
            let mut x_pos = 20.0_f32;
            for cell in row {
                let display = if cell.len() > 15 {
                    format!("{}...", &cell[..15])
                } else {
                    cell.clone()
                };
                layer.use_text(display.as_str(), 8.0, Mm(x_pos), Mm(y_pos), &font);
                x_pos += col_width;
            }
            y_pos -= 6.0;
            if y_pos < 20.0 {
                let (new_page, new_layer) = doc.add_page(Mm(297.0), Mm(210.0), "Layer 1");
                let _layer = doc.get_page(new_page).get_layer(new_layer);
                y_pos = 280.0;
            }
        }

        // 汇总
        if let Some(summary) = &data.summary {
            y_pos -= 6.0;
            for (k, v) in summary {
                layer.use_text(format!("{}: {}", k, v).as_str(), 10.0, Mm(20.0), Mm(y_pos), &font);
                y_pos -= 6.0;
            }
        }

        // 页脚记录数
        layer.use_text(format!("共 {} 条记录", data.rows.len()).as_str(), 10.0, Mm(20.0), Mm(10.0), &font);

        let mut buffer = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buffer);
            doc.save(&mut writer)
                .map_err(|e| AppError::internal(format!("PDF 保存失败: {}", e)))?;
        }
        Ok(buffer)
    }

    /// V15 P1 batch-08 缺陷 8：导出为 Word 格式（docx）
    /// 规则 3 强制要求所有报表/文档生成支持 .docx 格式（Word）。；布局：标题 → 主表键值对（summary）→ 明细表格（headers + rows）。
    pub fn export_docx(data: &ExportData) -> Result<Vec<u8>, AppError> {
        let kv = if let Some(summary) = &data.summary {
            let keys: Vec<String> = summary.iter().map(|(k, _)| k.clone()).collect();
            let values: Vec<String> = summary.iter().map(|(_, v)| v.clone()).collect();
            DocxKeyValue { keys, values }
        } else {
            DocxKeyValue {
                keys: vec![],
                values: vec![],
            }
        };

        build_docx_with_kv(&data.title, &kv, &data.headers, &data.rows)
    }

    /// 生成对账单文本（保留旧 generate_reconciliation_pdf 行为，仅用于内部调试）
    pub fn generate_reconciliation_pdf(
        reconciliation_no: &str,
        customer_name: &str,
        period_start: &str,
        period_end: &str,
        status: &str,
        items: Vec<ReconciliationPdfItem>,
        closing_balance: &str,
    ) -> Result<Vec<u8>, AppError> {
        let mut content = String::new();

        // 标题
        content.push_str("应收账款对账单\n");
        content.push_str(&"=".repeat(80));
        content.push_str("\n\n");

        // 基本信息
        content.push_str(&format!("对账单号: {}\n", reconciliation_no));
        content.push_str(&format!("客户名称: {}\n", customer_name));
        content.push_str(&format!("对账期间: {} 至 {}\n", period_start, period_end));
        content.push_str(&format!("状态: {}\n", status));
        content.push('\n');

        // 明细表头
        content.push_str(&format!(
            "{:<15} {:<20} {:<15} {:<15}\n",
            "类型", "单号", "金额", "日期"
        ));
        content.push_str(&"-".repeat(80));
        content.push('\n');

        // 明细数据
        for item in &items {
            content.push_str(&format!(
                "{:<15} {:<20} {:<15} {:<15}\n",
                item.item_type, item.document_no, item.amount, item.date
            ));
        }

        content.push_str(&"-".repeat(80));
        content.push('\n');

        // 汇总
        content.push_str(&format!("期末余额: {}\n", closing_balance));

        // 页脚
        content.push('\n');
        content.push_str(&"=".repeat(80));
        content.push('\n');
        content.push_str(&format!(
            "打印时间: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));
        content.push_str("本对账单由系统自动生成，如有疑问请联系财务部门。\n");

        Ok(content.into_bytes())
    }

    /// V15 P1 batch-08 缺陷 8：生成对账单 Word 文档（docx）（规则 3 强制要求合同/发票/报表支持 .docx 格式，禁止 .txt 作为成品文档。）
    pub fn generate_reconciliation_docx(
        reconciliation_no: &str,
        customer_name: &str,
        period_start: &str,
        period_end: &str,
        status: &str,
        items: Vec<ReconciliationPdfItem>,
        closing_balance: &str,
    ) -> Result<Vec<u8>, AppError> {
        let kv = DocxKeyValue {
            keys: vec![
                "对账单号".to_string(),
                "客户名称".to_string(),
                "对账期间".to_string(),
                "状态".to_string(),
                "期末余额".to_string(),
            ],
            values: vec![
                reconciliation_no.to_string(),
                customer_name.to_string(),
                format!("{} 至 {}", period_start, period_end),
                status.to_string(),
                closing_balance.to_string(),
            ],
        };

        let detail_headers: Vec<String> = vec![
            "类型".to_string(),
            "单号".to_string(),
            "金额".to_string(),
            "日期".to_string(),
        ];
        let detail_rows: Vec<Vec<String>> = items
            .into_iter()
            .map(|item| vec![item.item_type, item.document_no, item.amount, item.date])
            .collect();

        build_docx_with_kv("应收账款对账单", &kv, &detail_headers, &detail_rows)
    }
}

/// 对账单PDF明细项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationPdfItem {
    pub item_type: String,
    pub document_no: String,
    pub amount: String,
    pub date: String,
}
