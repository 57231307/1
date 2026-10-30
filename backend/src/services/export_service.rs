//! 导出服务
//!
//! 提供 PDF、Excel、Word 格式的导出功能
//! v11 批次 161 CI2：移除 CSV 导出，规则 3 要求 xlsx 交付
//! V15 P1 batch-08 缺陷 8：新增 docx 导出，规则 3 要求合同/发票/报表支持 .docx

use crate::utils::docx_export::{DocxKeyValue, DocxPageLayout, build_docx_with_kv_and_image};
use crate::utils::error::AppError;
use crate::utils::xlsx_export::{XlsxTable, build_xlsx};
use serde::{Deserialize, Serialize};

/// 对账单 PDF 的版面参数，单位毫米，A4 横向为 297 乘 210
const RECON_PAGE_WIDTH_MM: f32 = 297.0;
const RECON_PAGE_HEIGHT_MM: f32 = 210.0;
const RECON_CONTENT_X_MM: f32 = 20.0;
const RECON_TITLE_Y_MM: f32 = 190.0;
const RECON_TABLE_BOTTOM_MM: f32 = 20.0;
const RECON_TABLE_COL_WIDTH_MM: f32 = 62.0;
const RECON_ROW_STEP_MM: f32 = 6.0;

/// 导出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub summary: Option<Vec<(String, String)>>,
}

/// printpdf 内建字体加载失败映射。
/// 调用方 本模块两个 PDF 生成器（通用导出与对账单），两者共用同一文案，
/// 避免同一条 internal 映射在每处 doc 构造里各抄一遍。
/// 入参 printpdf 字体错误，传给谁 上抛成 AppError::internal 出参，不落库。
fn font_load_error(e: impl std::fmt::Display) -> AppError {
    AppError::internal(format!("加载字体失败: {}", e))
}

/// PDF 字节流写缓冲失败映射。
/// 调用方 本模块两个 PDF 生成器，入参 printpdf 保存错误，
/// 传给谁 上抛成 AppError::internal 出参，不落库。
fn pdf_save_error(e: impl std::fmt::Display) -> AppError {
    AppError::internal(format!("PDF 保存失败: {}", e))
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
            .map_err(font_load_error)?;
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
            doc.save(&mut writer).map_err(pdf_save_error)?;
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

    /// 生成对账单 PDF（规则 3：禁止以 PDF 名义交付文本，此处走 printpdf 真件）
    ///
    /// 功能 把抬头信息、明细表格（表头行、明细行与合计行共用同一列坐标）与页脚说明排成
    /// A4 横向版面的真 PDF 字节流，翻页时新页句柄立刻接管后续绘制，不再画回上一页。
    /// 调用方 ArReconciliationService 的同名内部方法，它组装好明细项与期末余额后调用本函数。
    /// 入参 对账单号、客户名称、对账起止期间、对账状态、明细项列表与期末余额，均为上层定稿字符串。
    /// 传给谁 只写进本函数内的 printpdf 文档对象，字节流按返回值上抛给对账单导出 handler，
    /// handler 据此算 size 与 base64_content 两个出参，三者同源。
    /// 存什么 存 PDF 字节流，含文件头魔数、逐页内容流与文本算子。
    /// 存哪里 不落库，只作为返回值进入对账单 PDF 导出接口的响应体。
    ///
    /// 局限 字体沿用 printpdf 内建 Helvetica（WinAnsi 编码），中文不在该编码表内会被
    /// printpdf 静默忽略，因此产物上可见的字形只有拉丁字母与数字（单号、金额、日期、
    /// 期间与打印时间），中文文案与中文表头仍是无字形状态。中文字形要等字体资产裁定后
    /// 接外部字体才有，本处不预先改字体方案。
    pub fn generate_reconciliation_pdf(
        reconciliation_no: &str,
        customer_name: &str,
        period_start: &str,
        period_end: &str,
        status: &str,
        items: Vec<ReconciliationPdfItem>,
        closing_balance: &str,
    ) -> Result<Vec<u8>, AppError> {
        use printpdf::*;

        let (doc, page1, layer1) = PdfDocument::new(
            "应收账款对账单",
            Mm(RECON_PAGE_WIDTH_MM),
            Mm(RECON_PAGE_HEIGHT_MM),
            "Layer 1",
        );
        let layer = doc.get_page(page1).get_layer(layer1);
        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(font_load_error)?;

        // 标题与抬头信息
        let mut y_pos = RECON_TITLE_Y_MM;
        layer.use_text(
            "应收账款对账单",
            16.0,
            Mm(RECON_CONTENT_X_MM),
            Mm(y_pos),
            &font,
        );
        y_pos -= 10.0;
        for line in Self::reconciliation_info_lines(
            reconciliation_no,
            customer_name,
            period_start,
            period_end,
            status,
        ) {
            layer.use_text(
                line.as_str(),
                10.0,
                Mm(RECON_CONTENT_X_MM),
                Mm(y_pos),
                &font,
            );
            y_pos -= RECON_ROW_STEP_MM;
        }

        // 明细表格（表头行、明细行与合计行）
        y_pos -= 4.0;
        let headers = Self::reconciliation_table_headers();
        let rows: Vec<Vec<String>> = items
            .iter()
            .map(|item| {
                vec![
                    item.item_type.clone(),
                    item.document_no.clone(),
                    item.amount.clone(),
                    item.date.clone(),
                ]
            })
            .collect();
        let total_row = vec![
            "期末余额".to_string(),
            String::new(),
            closing_balance.to_string(),
            String::new(),
        ];
        let (layer, y_pos) =
            Self::draw_reconciliation_table(&doc, layer, &font, &headers, &rows, &total_row, y_pos);

        // 页脚说明与打印时间
        layer.use_text(
            "本对账单由系统自动生成，如有疑问请联系财务部门。",
            9.0,
            Mm(RECON_CONTENT_X_MM),
            Mm(y_pos - 8.0),
            &font,
        );
        layer.use_text(
            format!(
                "打印时间: {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            )
            .as_str(),
            9.0,
            Mm(RECON_CONTENT_X_MM),
            Mm(y_pos - 14.0),
            &font,
        );

        let mut buffer = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buffer);
            doc.save(&mut writer).map_err(pdf_save_error)?;
        }
        Ok(buffer)
    }

    /// 装配对账单 PDF 的抬头信息行（四行，随标题之下逐行下移）
    ///
    /// 功能 把对账单号、客户名称、对账期间与状态排成固定四行文本。
    /// 调用方 仅本文件的 generate_reconciliation_pdf。
    /// 入参 与 generate_reconciliation_pdf 同名的前五个字段。
    /// 传给谁 返回的行文本由调用方写进 PDF 当前页层。
    /// 存什么 存四行展示文本，内容与 docx 导出面同源同词。
    /// 存哪里 不落库，只在内存里向上传递。
    fn reconciliation_info_lines(
        reconciliation_no: &str,
        customer_name: &str,
        period_start: &str,
        period_end: &str,
        status: &str,
    ) -> Vec<String> {
        vec![
            format!("对账单号: {}", reconciliation_no),
            format!("客户名称: {}", customer_name),
            format!("对账期间: {} 至 {}", period_start, period_end),
            format!("状态: {}", status),
        ]
    }

    /// 对账单 PDF 明细表格的列标题（与明细行、合计行共用同一列坐标）
    ///
    /// 功能 给出表格四列的标题文本，列序与 ReconciliationPdfItem 的字段序一致。
    /// 调用方 仅本文件的 generate_reconciliation_pdf。
    /// 入参 无。
    /// 传给谁 交给 draw_reconciliation_table 排在表头行位置。
    /// 存什么 存四列标题文本，与 docx 导出面的明细表头同词。
    /// 存哪里 不落库，只在内存里向上传递。
    fn reconciliation_table_headers() -> Vec<String> {
        vec![
            "类型".to_string(),
            "单号".to_string(),
            "金额".to_string(),
            "日期".to_string(),
        ]
    }

    /// 绘制对账单表格（分隔线、表头行、明细行与合计行）
    ///
    /// 功能 在当前页层上按固定列宽排表格，明细行排到页底阈值以下时新建页，
    /// 并把绘制句柄换到新页后再继续排剩余行。
    /// 调用方 仅本文件的 generate_reconciliation_pdf。
    /// 入参 文档引用（新建页要用）、当前页层引用（按值接管）、字体、表头行、明细行、
    /// 合计行与起始 y 坐标（毫米，自页面底部往上计量）。
    /// 传给谁 文本算子写进 printpdf 的页层，页层与结束坐标按返回值交回调用方排页脚。
    /// 存什么 不存业务数据，只存已绘制的页层引用与最后一个可用 y 坐标。
    /// 存哪里 不落库，产物随调用方拿到的 PDF 字节流走。
    fn draw_reconciliation_table(
        doc: &printpdf::PdfDocumentReference,
        mut layer: printpdf::PdfLayerReference,
        font: &printpdf::IndirectFontRef,
        headers: &[String],
        rows: &[Vec<String>],
        total_row: &[String],
        mut y_pos: f32,
    ) -> (printpdf::PdfLayerReference, f32) {
        use printpdf::*;

        let rule = "-".repeat(headers.len() * 18);
        layer.use_text(rule.as_str(), 10.0, Mm(RECON_CONTENT_X_MM), Mm(y_pos), font);
        y_pos -= RECON_ROW_STEP_MM;

        let mut x_pos = RECON_CONTENT_X_MM;
        for header in headers {
            layer.use_text(header.as_str(), 10.0, Mm(x_pos), Mm(y_pos), font);
            x_pos += RECON_TABLE_COL_WIDTH_MM;
        }
        y_pos -= 7.0;

        for row in rows {
            if y_pos < RECON_TABLE_BOTTOM_MM {
                let (new_page, new_layer) =
                    doc.add_page(Mm(RECON_PAGE_WIDTH_MM), Mm(RECON_PAGE_HEIGHT_MM), "Layer 1");
                // 新页句柄必须立刻接管绘制，丢弃返回值会让翻页之后的行仍画回第一页
                layer = doc.get_page(new_page).get_layer(new_layer);
                y_pos = RECON_TITLE_Y_MM;
            }
            let mut x_pos = RECON_CONTENT_X_MM;
            for cell in row {
                layer.use_text(cell.as_str(), 8.0, Mm(x_pos), Mm(y_pos), font);
                x_pos += RECON_TABLE_COL_WIDTH_MM;
            }
            y_pos -= RECON_ROW_STEP_MM;
        }

        layer.use_text(rule.as_str(), 10.0, Mm(RECON_CONTENT_X_MM), Mm(y_pos), font);
        y_pos -= 7.0;
        let mut x_pos = RECON_CONTENT_X_MM;
        for cell in total_row {
            layer.use_text(cell.as_str(), 10.0, Mm(x_pos), Mm(y_pos), font);
            x_pos += RECON_TABLE_COL_WIDTH_MM;
        }
        (layer, y_pos - RECON_ROW_STEP_MM)
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

    /// 构造指定行数的 ExportData，内容只用 ASCII 避免内建字体编码限制
    fn make_data(row_count: usize) -> ExportData {
        ExportData {
            title: "Test Report".into(),
            headers: vec!["Col A".into(), "Col B".into(), "Col C".into()],
            rows: (0..row_count)
                .map(|i| vec![format!("A{}", i), format!("B{}", i), format!("C{}", i)])
                .collect(),
            summary: None,
        }
    }

    /// 在字节序列中查找子串首次出现的位置
    fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// 在字节序列中查找子串末次出现的位置
    fn rfind_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).rposition(|w| w == needle)
    }

    /// 按 endstream 切分 PDF 字节流，返回含 BT 文本算子的内容流段数。
    /// printpdf 0.7 的 save 默认不压缩内容流，因此 BT 可直接在原始字节中检索。
    fn streams_with_bt(pdf: &[u8]) -> usize {
        let endstream_kw = b"endstream";
        let stream_kw = b"stream";
        let bt_kw = b"BT";
        let mut count = 0;
        let mut pos = 0;
        while let Some(idx) = find_sub(&pdf[pos..], endstream_kw) {
            let segment = &pdf[pos..pos + idx];
            if let Some(spos) = rfind_sub(segment, stream_kw) {
                let after_stream = &segment[spos + stream_kw.len()..];
                if find_sub(after_stream, bt_kw).is_some() {
                    count += 1;
                }
            }
            pos += idx + endstream_kw.len();
        }
        count
    }

    /// 翻页句柄修复的自证用例：60 行触发一次翻页，断言每一页的内容流都非空。
    /// 若把 export_pdf 中翻页处的 `layer = ...` 改回 `let _layer = ...` 丢弃形态，
    /// 新页内容流为空、不含 BT 算子，bt_stream_count 会降到 1，断言必红。
    /// 读 PDF 页树的页数：页树根字典按 PDF 规范必带 `/Count N`，取所有 /Count 里的最大值
    /// （多级页树时父节点计数不小于子节点，最大值即总页数）。
    /// 入参：完整 PDF 字节流；传给谁：仅供本模块的翻页自证用例断言页数。
    /// 返回 None 表示字节流里找不到任何 /Count（写侧形态与本判据假设不符），
    /// 与「找到了但页数不足」分开报，避免把计数方式失效当成业务红。
    /// 为什么不用 /Type /Page 字面量计数：printpdf 出参里 /Type 与 /Page 之间的空格形态随版本变，
    /// 按字面量数会数出 0，两种红就混成一种了。
    fn pdf_page_count(pdf: &[u8]) -> Option<usize> {
        let token = b"/Count";
        let mut best: Option<usize> = None;
        let mut pos = 0usize;
        while let Some(idx) = find_sub(&pdf[pos..], token) {
            let after = &pdf[pos + idx + token.len()..];
            let digits: Vec<u8> = after
                .iter()
                .copied()
                .take_while(|b| b.is_ascii_whitespace() || b.is_ascii_digit())
                .filter(u8::is_ascii_digit)
                .collect();
            if let Some(parsed) = std::str::from_utf8(&digits)
                .ok()
                .and_then(|text| text.parse::<usize>().ok())
            {
                best = Some(best.map_or(parsed, |current| current.max(parsed)));
            }
            pos += idx + token.len();
        }
        best
    }

    #[test]
    fn export_pdf_page_break_produces_content_on_every_page() {
        let data = make_data(60);
        let bytes = ExportService::export_pdf(&data).expect("export_pdf 应成功返回字节流");

        // PDF 魔数
        assert!(bytes.starts_with(b"%PDF"), "返回值应以 PDF 魔数开头");

        // 页数断言：读页树 /Count，取不到与取到但不足分别报因
        let actual_pages = pdf_page_count(&bytes)
            .expect("PDF 页树应带 /Count 条目，未找到说明写侧形态与本判据假设不符");
        assert!(
            actual_pages >= 2,
            "60 行数据应触发翻页产生至少 2 页，实际页树 /Count {}",
            actual_pages
        );

        // 每页内容流都含 BT：把翻页句柄改回丢弃形态时新页流为空、此断言必红
        let bt_stream_count = streams_with_bt(&bytes);
        assert!(
            bt_stream_count >= 2,
            "每个翻页后的内容流都应含文本算子 BT，实际含 BT 的流段数 {}",
            bt_stream_count
        );
    }
}
