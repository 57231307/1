//! 销售发货-数据导出子模块（delivery_ops/export）
//!
//! 从 `so/delivery.rs` 拆出的销售订单导出腿。
//! 包含 3 个方法：
//! - export_orders_to_xlsx（公开 API，返回表头、行集与服务端全量总数）
//! - build_order_csv_headers / order_to_csv_row（私有辅助）
//!
//! 取数只走 order_query 子模块的导出专用全量腿，与列表共用同一套筛选条件构造器。

use crate::utils::error::AppError;

use super::super::order::SalesService;

impl SalesService {
    // ========== 数据导出方法 ==========

    /// 导出销售订单为结构化格式（表头、行集与服务端全量总数）
    pub async fn export_orders_to_xlsx(
        &self,
        filter: super::super::order_query::SalesOrderFilter,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        // 行级数据权限按无下推取全量，与既有行为逐字一致；该面单独登记为独立缺陷族，
        // 权限收紧不与行数口径的修复合并，避免一条改动同时污染两条验收。
        let (orders, server_total) = self.list_all_orders_for_export(filter, None).await?;

        let headers = Self::build_order_csv_headers();
        let rows: Vec<std::collections::HashMap<String, String>> =
            orders.into_iter().map(Self::order_to_csv_row).collect();

        // 将 HashMap 转为 Vec<Vec<String>>
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                headers
                    .iter()
                    .map(|h| row.get(h).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();

        Ok((headers, data, server_total))
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
    fn order_to_csv_row(
        o: super::super::SalesOrderDetail,
    ) -> std::collections::HashMap<String, String> {
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
            o.required_date
                .map(|d: chrono::DateTime<chrono::Utc>| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
        );
        row.insert(
            "发货日期".to_string(),
            o.ship_date
                .map(|d: chrono::DateTime<chrono::Utc>| d.format("%Y-%m-%d %H:%M:%S").to_string())
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
                .map(|d: chrono::DateTime<chrono::Utc>| d.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
        );
        row
    }
}
