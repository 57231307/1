//! 导出服务
//!
//! 提供 PDF、Excel、Word 格式的导出功能
//! v11 批次 161 CI2：移除 CSV 导出，规则 3 要求 xlsx 交付
//! V15 P1 batch-08 缺陷 8：新增 docx 导出，规则 3 要求合同/发票/报表支持 .docx

use crate::utils::docx_export::{DocxKeyValue, DocxPageLayout, build_docx_with_kv_and_image};
use crate::utils::error::AppError;
use crate::utils::xlsx_export::{XlsxTable, build_xlsx};
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
        let (doc, page1, layer1) =
            PdfDocument::new(data.title.as_str(), Mm(297.0), Mm(210.0), "Layer 1");
        let mut layer = doc.get_page(page1).get_layer(layer1);
        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| AppError::internal(format!("加载字体失败: {}", e)))?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 标题与生成时间
        layer.use_text(data.title.as_str(), 16.0, Mm(20.0), Mm(280.0), &font);
        layer.use_text(
            format!("生成时间: {}", now).as_str(),
            10.0,
            Mm(20.0),
            Mm(270.0),
            &font,
        );

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
                layer = doc.get_page(new_page).get_layer(new_layer);
                y_pos = 280.0;
            }
        }

        // 汇总
        if let Some(summary) = &data.summary {
            y_pos -= 6.0;
            for (k, v) in summary {
                layer.use_text(
                    format!("{}: {}", k, v).as_str(),
                    10.0,
                    Mm(20.0),
                    Mm(y_pos),
                    &font,
                );
                y_pos -= 6.0;
            }
        }

        // 页脚记录数
        layer.use_text(
            format!("共 {} 条记录", data.rows.len()).as_str(),
            10.0,
            Mm(20.0),
            Mm(10.0),
            &font,
        );

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

        build_docx_with_kv_and_image(
            &data.title,
            &kv,
            &data.headers,
            &data.rows,
            None,
            &DocxPageLayout::default_a4_export(),
            // 导出面是否加公司抬头待裁,本波不动观感:显式传 None。
            None,
        )
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

        build_docx_with_kv_and_image(
            "应收账款对账单",
            &kv,
            &detail_headers,
            &detail_rows,
            None,
            &DocxPageLayout::default_a4_export(),
            // 导出面不加公司抬头（是否加抬头属后续待裁，本波不动观感）。
            None,
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    // 统计 PDF 字节里的内容流总数与含文本算子的内容流数。
    // 判据取自 printpdf 序列化事实：本用例只用内建字体且走默认合规级别，文档内不产出
    // XMP、ICC 或字体文件流，唯一的内容流对象就是逐页合并流，故内容流总数等于页数；
    // 每层无论有无内容都会被补 BDC 与 q … Q EMC 包裹，所以空页并非零字节流，
    // 但空页没有文本算子，而文本以十六进制写入（字母 T 非十六进制字符），
    // 故 "BT" 只可能来自 begin_text_section 落的算子字面量，据此可判某页是否真画了字。
    // 调用方 仅本测试模块。
    fn stream_text_stats(pdf: &[u8]) -> (usize, usize) {
        let open = b"stream\n";
        let close = b"\nendstream";
        let mut total = 0usize;
        let mut with_text = 0usize;
        let mut rest = pdf;
        while let Some(end) = find_seq(rest, close) {
            total += 1;
            let region = &rest[..end];
            if let Some(start) = rfind_seq(region, open) {
                let body = &region[start + open.len()..];
                if find_seq(body, b"BT").is_some() {
                    with_text += 1;
                }
            }
            rest = &rest[end + close.len()..];
        }
        (total, with_text)
    }

    fn find_seq(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).position(|w| w == needle)
    }

    fn rfind_seq(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).rposition(|w| w == needle)
    }

    #[test]
    fn export_pdf_paging_draws_rows_on_the_new_page() {
        let headers = vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()];
        let rows: Vec<Vec<String>> = (0..60)
            .map(|i| vec![format!("R{i}A"), format!("R{i}B"), format!("R{i}C")])
            .collect();
        let data = ExportData {
            title: "PagingCheck".to_string(),
            headers,
            rows,
            summary: None,
        };

        let pdf = ExportService::export_pdf(&data).expect("PDF 生成不应失败");
        let (total_streams, streams_with_text) = stream_text_stats(&pdf);

        // 60 行按每页 y 步长恰好触发一次分页，应产出两页内容流。
        assert_eq!(total_streams, 2, "60 行应恰好产出 2 页内容流");
        // 关键判据 每页内容流都必须含文本算子，缺陷里新页句柄被丢弃、后续行仍画回第一页，
        // 新页只剩 BDC 与 q … Q EMC 而无 BT，使含文本流数少于总页数。
        assert_eq!(
            streams_with_text, total_streams,
            "每页内容流都应含文本算子，出现无字页说明分页时把行画回了第一页"
        );
        assert!(
            streams_with_text >= 2,
            "除第一页外的第二页内容流须含文本算子"
        );
    }
}
