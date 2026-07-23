//! 采购订单-查询/导出子模块（order_ops/query）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块。
//! 包含明细查询与 CSV 导出方法：
//! - `list_order_items` 获取订单明细列表（关联物料编码/名称）
//! - `export_orders_to_csv` 导出采购订单为 CSV（D08 Tier 4 子批次9：≤20 行主函数 + 2 helper）
//! - `csv_headers` / `build_csv_rows` 导出 helper（私有 associated function）
//!
//! 依赖说明：
//! - `export_orders_to_csv` 内部调用 `list_orders`（定义于 crud 子模块，`pub` 方法，跨 impl 块可直接调用）

use sea_orm::{ColumnTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait};

use crate::models::{product, purchase_order_item};
use crate::services::po::order::{PurchaseOrderDto, PurchaseOrderItemDto, PurchaseOrderService};
use crate::utils::error::AppError;

impl PurchaseOrderService {
    /// 获取订单明细列表
    pub async fn list_order_items(
        &self,
        order_id: i32,
    ) -> Result<Vec<PurchaseOrderItemDto>, AppError> {
        let items = purchase_order_item::Entity::find()
            .column_as(product::Column::Code, "material_code")
            .column_as(product::Column::Name, "material_name")
            .join(
                JoinType::LeftJoin,
                purchase_order_item::Relation::Product.def(),
            )
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .into_model::<PurchaseOrderItemDto>()
            .all(&*self.db)
            .await?;

        Ok(items)
    }

    // ========== 数据导出方法 ==========

    /// 导出采购订单为 CSV 格式
    ///
    /// D08 Tier 4 子批次9：拆分为 ≤20 行主函数 + 2 个 helper（csv_headers / build_csv_rows）
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<Vec<u8>, AppError> {
        // V15 P0-S01：内部调用传 None（导出由调用方决定权限范围，service 不再二次过滤）
        let (orders, _total) = self
            .list_orders(1, 10000, status, supplier_id, None)
            .await?;

        let headers = Self::csv_headers();
        let rows = Self::build_csv_rows(orders);

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::internal(format!("CSV 生成失败: {}", e)))
    }

    /// CSV 导出表头（21 列）
    fn csv_headers() -> Vec<String> {
        vec![
            "订单编号".to_string(),
            "供应商ID".to_string(),
            "供应商名称".to_string(),
            "订单日期".to_string(),
            "预计交货日期".to_string(),
            "实际交货日期".to_string(),
            "仓库ID".to_string(),
            "仓库名称".to_string(),
            "部门ID".to_string(),
            "部门名称".to_string(),
            "采购员ID".to_string(),
            "币种".to_string(),
            "汇率".to_string(),
            "总金额".to_string(),
            "总金额外币".to_string(),
            "总数量".to_string(),
            "总数量辅助".to_string(),
            "状态".to_string(),
            "付款条件".to_string(),
            "运输条款".to_string(),
            "备注".to_string(),
        ]
    }

    /// 将采购订单列表转换为 CSV 行数据
    fn build_csv_rows(
        orders: Vec<PurchaseOrderDto>,
    ) -> Vec<std::collections::HashMap<String, String>> {
        orders
            .into_iter()
            .map(|o| {
                let mut row = std::collections::HashMap::new();
                row.insert("订单编号".to_string(), o.order_no);
                row.insert("供应商ID".to_string(), o.supplier_id.to_string());
                row.insert(
                    "供应商名称".to_string(),
                    o.supplier_name.unwrap_or_default(),
                );
                row.insert("订单日期".to_string(), o.order_date.to_string());
                row.insert(
                    "预计交货日期".to_string(),
                    o.expected_delivery_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "实际交货日期".to_string(),
                    o.actual_delivery_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                );
                row.insert("仓库ID".to_string(), o.warehouse_id.to_string());
                row.insert("仓库名称".to_string(), o.warehouse_name.unwrap_or_default());
                row.insert("部门ID".to_string(), o.department_id.to_string());
                row.insert(
                    "部门名称".to_string(),
                    o.department_name.unwrap_or_default(),
                );
                row.insert("采购员ID".to_string(), o.purchaser_id.to_string());
                row.insert("币种".to_string(), o.currency);
                row.insert("汇率".to_string(), o.exchange_rate.to_string());
                row.insert("总金额".to_string(), o.total_amount.to_string());
                row.insert("总金额外币".to_string(), o.total_amount_foreign.to_string());
                row.insert("总数量".to_string(), o.total_quantity.to_string());
                row.insert("总数量辅助".to_string(), o.total_quantity_alt.to_string());
                row.insert("状态".to_string(), o.order_status);
                row.insert("付款条件".to_string(), o.payment_terms.unwrap_or_default());
                row.insert("运输条款".to_string(), o.shipping_terms.unwrap_or_default());
                row.insert("备注".to_string(), o.notes.unwrap_or_default());
                row
            })
            .collect()
    }
}
