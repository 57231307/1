//! 采购订单-查询/导出子模块（order_ops/query）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块。
//! 包含明细查询与导出方法如下，
//! - list_order_items 获取订单明细列表（关联物料编码与名称）
//! - list_all_orders_for_export 导出专用全量取数，不分页且不加行数上限
//! - export_orders_to_xlsx 导出采购订单为结构化表格，返回表头、行集与服务端全量总数
//! - export_rows_complete 导出行集完整性判据，单点定义供调用方四处留痕共用
//! - csv_headers 与 build_csv_rows 导出 helper（私有 associated function）
//!
//! 依赖说明如下，
//! - 两条取数腿（列表分页 list_orders、导出全量 list_all_orders_for_export）共用
//!   crud 子模块的 build_orders_query，筛选条件与行级权限口径同源
//! - 跨 impl 块调用同类型的方法允许，只要可见性够

use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait,
};

use crate::models::{product, purchase_order, purchase_order_item};
use crate::services::po::order::{PurchaseOrderDto, PurchaseOrderItemDto, PurchaseOrderService};
use crate::utils::data_scope::DataScopeContext;
use crate::utils::error::AppError;

impl PurchaseOrderService {
    /// 获取订单明细列表
    pub async fn list_order_items(
        &self,
        order_id: i32,
    ) -> Result<Vec<PurchaseOrderItemDto>, AppError> {
        // amount 映射到实体 subtotal 列；returned_quantity 数据库无对应列，置 0
        // （若 FromQueryResult 按默认列名匹配不到列会 TryGetError 导致接口 500）
        // returned_quantity 的 0 必须写成 `CAST(0 AS NUMERIC)` 而非裸 `Expr::value(0)`：
        // 裸整数常量在 Postgres 结果集里带 INT4 OID，而 DTO 字段是 Decimal(NUMERIC)，
        // 解码期会因 INT4 与 Option<Decimal> 不匹配抛 500（CI #4653）；显式 CAST 令列 OID
        // 为 numeric，与 FromQueryResult 期望类型同源，勿改回整数字面量。
        let items = purchase_order_item::Entity::find()
            .column_as(product::Column::Code, "material_code")
            .column_as(product::Column::Name, "material_name")
            .column_as(purchase_order_item::Column::Subtotal, "amount")
            .column_as(Expr::cust("CAST(0 AS NUMERIC)"), "returned_quantity")
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

    /// 导出专用的全量取数：不分页、不加 LIMIT，返回行集与服务端全量总数。
    ///
    /// 形态选单发一条不分页查询，不做 offset 分批循环：导出行本就全量驻内存建字符串矩阵，
    /// 循环只是多打 SQL，且深翻页是平方级代价。
    /// 总数走 SeaORM 的 count，它把本查询包成 COUNT(*) 子查询并清掉 ORDER BY，
    /// 与列表分页的 num_items 同一形态，因此导出总数与列表 total 可比。
    /// 行集与总数是两次查询，之间发生并发写入时两者会不等，本函数如实返回两个值，
    /// 不在此吞差值；完整性由 `export_rows_complete` 判定，交由调用方留痕。
    pub async fn list_all_orders_for_export(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {
        let query = Self::build_orders_query(status, supplier_id, keyword, data_scope);

        let rows = query
            .clone()
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .all(&*self.db)
            .await?;
        let server_total = query.count(&*self.db).await?;

        Ok((rows, server_total))
    }

    /// 导出行集完整性判据：交付到文件的行数与服务端全量总数相等才算完整。
    ///
    /// 单点定义，调用方写审计描述、留痕快照、水印与服务端日志四处共用同一判据；
    /// 不等时如实标不完整，既不在导出侧补行，也不静默吞差值。
    pub fn export_rows_complete(returned_rows: usize, server_total: u64) -> bool {
        returned_rows as u64 == server_total
    }

    /// 导出采购订单为结构化格式（表头 + 行 + 服务端全量总数）
    pub async fn export_orders_to_xlsx(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<String>, Vec<Vec<String>>, u64), AppError> {
        // 关键字与行级数据权限当前按无下推取全量，与既有行为逐字一致；
        // 两处口径差异（列表带关键字、列表带行级权限）已单独登记为独立缺陷族，
        // 权限收紧不与此处的行数口径修复合并，避免一条 diff 污染两条验收。
        let (orders, server_total) = self
            .list_all_orders_for_export(status, supplier_id, None, None)
            .await?;

        let headers = Self::csv_headers();
        let rows = Self::build_csv_rows(orders);

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
