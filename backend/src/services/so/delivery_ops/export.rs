//! 销售发货-数据导出子模块（delivery_ops/export）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L1322-1443 迁移。
//! 包含 3 个 CSV 导出方法：
//! - export_orders_to_csv（公开 API）
//! - build_order_csv_headers / order_to_csv_row（私有辅助）

use crate::utils::error::AppError;

use super::super::order::SalesService;

impl SalesService {
    // ========== 数据导出方法 ==========

    /// 导出销售订单为 CSV 格式
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
    ) -> Result<Vec<u8>, AppError> {
        let page_req = crate::models::dto::PageRequest {
            page: 1,
            page_size: 10000,
        };
        let orders = self
            .list_orders(page_req, status, customer_id, order_no, None)
            .await?;

        let headers = Self::build_order_csv_headers();
        let rows: Vec<std::collections::HashMap<String, String>> = orders
            .items
            .into_iter()
            .map(Self::order_to_csv_row)
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::business(format!("CSV 生成失败: {}", e)))
    }

    /// 构建销售订单 CSV 导出表头
    fn build_order_csv_headers() -> Vec<String> {
        vec![
            "订单编号".to_string(),
            "客户ID".to_string(),
            "客户名称".to_string(),
            "商机ID".to_string(),
            "订单日期".to_string(),
            "要求交货日期".to_string(),
            "发货日期".to_string(),
            "状态".to_string(),
            "小计金额".to_string(),
            "税额".to_string(),
            "折扣金额".to_string(),
            "运费".to_string(),
            "总金额".to_string(),
            "已付金额".to_string(),
            "余额".to_string(),
            "送货地址".to_string(),
            "账单地址".to_string(),
            "备注".to_string(),
            "创建人ID".to_string(),
            "审批人ID".to_string(),
            "审批时间".to_string(),
        ]
    }

    /// 将销售订单转换为 CSV 行（HashMap<表头, 值>）
    fn order_to_csv_row(o: super::super::SalesOrderDetail) -> std::collections::HashMap<String, String> {
        let mut row = std::collections::HashMap::new();
        row.insert("订单编号".to_string(), o.order_no);
        row.insert("客户ID".to_string(), o.customer_id.to_string());
        row.insert("客户名称".to_string(), o.customer_name.unwrap_or_default());
        row.insert(
            "商机ID".to_string(),
            o.opportunity_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            "订单日期".to_string(),
            o.order_date.format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        row.insert(
            "要求交货日期".to_string(),
            o.required_date.format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        row.insert(
            "发货日期".to_string(),
            o.ship_date
                .map(|d: chrono::DateTime<chrono::Utc>| {
                    d.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default(),
        );
        row.insert("状态".to_string(), o.status);
        row.insert("小计金额".to_string(), o.subtotal.to_string());
        row.insert("税额".to_string(), o.tax_amount.to_string());
        row.insert("折扣金额".to_string(), o.discount_amount.to_string());
        row.insert("运费".to_string(), o.shipping_cost.to_string());
        row.insert("总金额".to_string(), o.total_amount.to_string());
        row.insert("已付金额".to_string(), o.paid_amount.to_string());
        row.insert("余额".to_string(), o.balance_amount.to_string());
        row.insert(
            "送货地址".to_string(),
            o.shipping_address.unwrap_or_default(),
        );
        row.insert(
            "账单地址".to_string(),
            o.billing_address.unwrap_or_default(),
        );
        row.insert("备注".to_string(), o.notes.unwrap_or_default());
        row.insert(
            "创建人ID".to_string(),
            o.created_by
                .map(|id: i32| id.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            "审批人ID".to_string(),
            o.approved_by
                .map(|id: i32| id.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            "审批时间".to_string(),
            o.approved_at
                .map(|d: chrono::DateTime<chrono::Utc>| {
                    d.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default(),
        );
        row
    }
}
