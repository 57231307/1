//! 导出服务
//!
//! 提供PDF、Excel、CSV等格式的导出功能

use crate::utils::error::AppError;
use serde::{Deserialize, Serialize};

/// 导出格式
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    PDF,
    Excel,
    CSV,
}

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
    /// 导出为CSV格式
    pub fn export_csv(data: &ExportData) -> Result<Vec<u8>, AppError> {
        let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);

        // 写入标题行
        wtr.write_record(&data.headers)
            .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;

        // 写入数据行
        for row in &data.rows {
            wtr.write_record(row)
                .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;
        }

        // 写入汇总
        if let Some(summary) = &data.summary {
            wtr.write_record(Vec::<String>::new())
                .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;
            for (key, value) in summary {
                wtr.write_record(vec![key.clone(), value.clone()])
                    .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;
            }
        }

        let bytes = wtr
            .into_inner()
            .map_err(|e| AppError::validation(format!("CSV序列化错误: {}", e)))?;

        Ok(bytes)
    }

    /// 导出为Excel格式（简化实现，使用CSV格式）
    pub fn export_excel(data: &ExportData) -> Result<Vec<u8>, AppError> {
        // 简化实现：返回CSV格式，实际项目中应使用xlsxwriter库
        Self::export_csv(data)
    }

    /// 导出为PDF格式（简化实现，生成文本格式）
    pub fn export_pdf(data: &ExportData) -> Result<Vec<u8>, AppError> {
        let mut content = String::new();

        // 标题
        content.push_str(&format!("{}\n", data.title));
        content.push_str(&"=".repeat(80));
        content.push('\n');
        content.push('\n');

        // 表头
        let header_line = data.headers.join(" | ");
        content.push_str(&header_line);
        content.push('\n');
        content.push_str(&"-".repeat(header_line.len()));
        content.push('\n');

        // 数据行
        for row in &data.rows {
            let row_line = row.join(" | ");
            content.push_str(&row_line);
            content.push('\n');
        }

        // 汇总
        if let Some(summary) = &data.summary {
            content.push('\n');
            content.push_str(&"=".repeat(80));
            content.push('\n');
            for (key, value) in summary {
                content.push_str(&format!("{}: {}\n", key, value));
            }
        }

        // 添加页脚
        content.push('\n');
        content.push_str(&"-".repeat(80));
        content.push('\n');
        content.push_str(&format!(
            "生成时间: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));

        Ok(content.into_bytes())
    }

    /// 根据格式导出
    pub fn export(data: &ExportData, format: &ExportFormat) -> Result<Vec<u8>, AppError> {
        match format {
            ExportFormat::PDF => Self::export_pdf(data),
            ExportFormat::Excel => Self::export_excel(data),
            ExportFormat::CSV => Self::export_csv(data),
        }
    }

    /// 生成对账单PDF
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
}

/// 对账单PDF明细项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationPdfItem {
    pub item_type: String,
    pub document_no: String,
    pub amount: String,
    pub date: String,
}
