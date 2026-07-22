//! 库存事务记录 + 汇总查询方法（list_transactions / summary / alerts）
//!
//! 拆分自 inventory_stock_service.rs：原 6 个事务记录与汇总方法独立成文件。
//! 批次 400 修复：移除 record_transaction 非事务版本（已被 inventory_stock_txn.rs 的 record_transaction_txn 事务版本取代）。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::handlers::inventory_stock_handler_dto::InventorySummaryItem;
use crate::models::{inventory_stock, inventory_transaction};
use crate::services::stock_alert::{
    AlertType, ALERT_TYPE_NORMAL, EXPIRING_THRESHOLD_DAYS, SLOW_MOVING_THRESHOLD_DAYS,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 根据库存 Model 派生计算 alert_type 字符串
///
/// 批次 126 v8 复审 P2 修复：替换原硬编码 "normal"。
///
/// v11 批次 144 P1-4 修复：扩展 OverStock（高于上限）和 SlowMoving（滞销）告警判定。
/// - OverStock: max_stock_point > 0 && quantity_available > max_stock_point
/// - SlowMoving: last_movement_date 距今 > SLOW_MOVING_THRESHOLD_DAYS 天
///
/// 判定优先级（高优先级先返回）：
/// 1. stock_status != "正常" → discrepancy（盘点差异/状态异常）
/// 2. quantity_available == 0 && reorder_point > 0 → out_of_stock（缺货）
/// 3. reorder_point > 0 && quantity_available < reorder_point → low_stock（低于下限）
/// 4. max_stock_point > 0 && quantity_available > max_stock_point → over_stock（高于上限）
/// 5. expiry_date 存在且距今 ≤ EXPIRING_THRESHOLD_DAYS 天 → expiring（即将过期）
/// 6. last_movement_date 距今 > SLOW_MOVING_THRESHOLD_DAYS 天 → slow_moving（滞销）
/// 7. 否则 → normal
pub(crate) fn compute_alert_type(s: &inventory_stock::Model) -> &'static str {
    // 1. 状态异常优先（冻结/待检等）
    if s.stock_status != "正常" {
        return AlertType::Discrepancy.code();
    }
    // 2. 缺货（可用量为 0 且设置了补货点）
    if s.reorder_point > Decimal::ZERO && s.quantity_available == Decimal::ZERO {
        return AlertType::OutOfStock.code();
    }
    // 3. 低于下限（可用量 < 补货点）
    if s.reorder_point > Decimal::ZERO && s.quantity_available < s.reorder_point {
        return AlertType::LowStock.code();
    }
    // 4. 高于上限（可用量 > 库存上限，且设置了上限）
    //
    // v11 批次 144 P1-4：接入 max_stock_point 字段，实现 OverStock 告警。
    if s.max_stock_point > Decimal::ZERO && s.quantity_available > s.max_stock_point {
        return AlertType::OverStock.code();
    }
    // 5. 即将过期（expiry_date 距今 ≤ 阈值天数）
    if let Some(expiry) = s.expiry_date {
        let now = Utc::now();
        if (expiry - now).num_days() <= EXPIRING_THRESHOLD_DAYS {
            return AlertType::Expiring.code();
        }
    }
    // 6. 滞销（最后一次库存变动距今 > 阈值天数）
    //
    // v11 批次 144 P1-4：接入 last_movement_date 字段，实现 SlowMoving 告警。
    // last_movement_date 为 NULL 表示从未发生过库存变动（视为滞销）。
    if let Some(last_move) = s.last_movement_date {
        let now = Utc::now();
        if (now - last_move).num_days() > SLOW_MOVING_THRESHOLD_DAYS {
            return AlertType::SlowMoving.code();
        }
    } else if s.quantity_available > Decimal::ZERO {
        // 有库存但从未发生过库存变动，视为滞销
        return AlertType::SlowMoving.code();
    }
    // 7. 正常
    ALERT_TYPE_NORMAL
}

use super::inventory_stock_service::{InventoryStockService, InventorySummaryQueryResult};

/// 库存流水查询参数对象
///
/// 批次 335 v10 复审 P3 修复：引入参数对象消除 list_transactions 的 too_many_arguments 警告。
/// 聚合分页与过滤条件，避免函数签名携带 9 个参数。
/// 与 handler 层的 `ListTransactionParams` 分离，service 层不依赖 axum 的 Deserialize。
#[derive(Debug, Clone)]
pub struct ListTransactionsQuery {
    /// 页码（1-based，service 内部转换为 0-based）
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
    /// 批次号过滤
    pub batch_no: Option<String>,
    /// 色号过滤
    pub color_no: Option<String>,
    /// 产品 ID 过滤
    pub product_id: Option<i32>,
    /// 仓库 ID 过滤
    pub warehouse_id: Option<i32>,
    /// 事务类型过滤
    pub transaction_type: Option<String>,
    /// 起始日期过滤
    pub start_date: Option<chrono::NaiveDateTime>,
    /// 结束日期过滤
    pub end_date: Option<chrono::NaiveDateTime>,
}

/// 库存流水记录参数对象
///
/// 批次 338 v10 复审 P3 修复：引入参数对象消除 record_transaction 的 too_many_arguments 警告。
/// 聚合库存流水记录所需的全部字段，避免函数签名携带 18 个参数。
/// 批次 400 修复：record_transaction 非事务版本已移除，此参数对象由 inventory_stock_txn.rs 的 record_transaction_txn 事务版本复用。
#[derive(Debug, Clone)]
pub struct RecordTransactionArgs {
    /// 交易类型
    pub transaction_type: String,
    /// 产品 ID
    pub product_id: i32,
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 染缸批号（可选）
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
    /// 来源单据类型（可选）
    pub source_bill_type: Option<String>,
    /// 来源单据号（可选）
    pub source_bill_no: Option<String>,
    /// 来源单据 ID（可选）
    pub source_bill_id: Option<i32>,
    /// 变更前数量（米，可选）
    pub quantity_before_meters: Option<Decimal>,
    /// 变更前数量（公斤，可选）
    pub quantity_before_kg: Option<Decimal>,
    /// 变更后数量（米，可选）
    pub quantity_after_meters: Option<Decimal>,
    /// 变更后数量（公斤，可选）
    pub quantity_after_kg: Option<Decimal>,
    /// 备注（可选）
    pub notes: Option<String>,
    /// 创建人 ID（可选）
    pub created_by: Option<i32>,
}

/// 库存汇总查询参数（service 层，page/page_size 已解析为非 Option）
#[derive(Debug, Clone)]
pub struct InventorySummaryQuery {
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
    pub page: u64,
    pub page_size: u64,
}

impl InventoryStockService {
    /// 查询库存流水
    ///
    /// 批次 335 v10 复审 P3 修复：签名从 9 参数改为单一参数对象 `ListTransactionsQuery`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn list_transactions(
        &self,
        query: ListTransactionsQuery,
    ) -> Result<(Vec<inventory_transaction::Model>, u64), AppError> {
        // 解构参数对象，便于函数体内按字段名访问
        let ListTransactionsQuery {
            page,
            page_size,
            batch_no,
            color_no,
            product_id,
            warehouse_id,
            transaction_type,
            start_date,
            end_date,
        } = query;

        let mut q = inventory_transaction::Entity::find()
            .order_by(inventory_transaction::Column::CreatedAt, Order::Asc);

        if let Some(batch) = batch_no {
            q = q.filter(inventory_transaction::Column::BatchNo.eq(batch));
        }

        if let Some(color) = color_no {
            q = q.filter(inventory_transaction::Column::ColorNo.eq(color));
        }

        if let Some(pid) = product_id {
            q = q.filter(inventory_transaction::Column::ProductId.eq(pid));
        }

        if let Some(wid) = warehouse_id {
            q = q.filter(inventory_transaction::Column::WarehouseId.eq(wid));
        }

        if let Some(transaction_type) = transaction_type {
            q = q.filter(inventory_transaction::Column::TransactionType.eq(transaction_type));
        }

        if let Some(start_date) = start_date {
            q = q.filter(inventory_transaction::Column::CreatedAt.gte(start_date));
        }

        if let Some(end_date) = end_date {
            q = q.filter(inventory_transaction::Column::CreatedAt.lte(end_date));
        }

        // 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
        // paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
        // 补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
        let paginator = q.paginate(&*self.db, page_size);
        let (transactions, total) =
            paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((transactions, total))
    }

    /// 获取库存汇总（按批次 + 色号）
    ///
    /// # 参数
    /// - `warehouse_id`: 仓库ID筛选
    /// - `product_id`: 产品ID筛选
    /// - `batch_no`: 批次号筛选
    /// - `color_no`: 色号筛选
    /// - `grade`: 等级筛选
    /// - `page`: 页码（从1开始）
    /// - `page_size`: 每页大小
    ///
    /// # 返回
    /// 返回分页结果，包含数据列表和总记录数
    pub async fn get_inventory_summary(
        &self,
        params: InventorySummaryQuery,
    ) -> Result<(Vec<InventorySummaryItem>, u64), AppError> {
        use sea_orm::QuerySelect;

        let mut query = inventory_stock::Entity::find()
            .inner_join(crate::models::product::Entity)
            .inner_join(crate::models::warehouse::Entity)
            .select_only()
            .column_as(inventory_stock::Column::ProductId, "product_id")
            .column_as(crate::models::product::Column::Name, "product_name")
            .column_as(inventory_stock::Column::WarehouseId, "warehouse_id")
            .column_as(crate::models::warehouse::Column::Name, "warehouse_name")
            .column_as(inventory_stock::Column::BatchNo, "batch_no")
            .column_as(inventory_stock::Column::ColorNo, "color_no")
            .column_as(inventory_stock::Column::Grade, "grade")
            .column_as(
                inventory_stock::Column::QuantityMeters.sum(),
                "total_quantity_meters",
            )
            .column_as(
                inventory_stock::Column::QuantityKg.sum(),
                "total_quantity_kg",
            )
            .group_by(inventory_stock::Column::ProductId)
            .group_by(crate::models::product::Column::Name)
            .group_by(inventory_stock::Column::WarehouseId)
            .group_by(crate::models::warehouse::Column::Name)
            .group_by(inventory_stock::Column::BatchNo)
            .group_by(inventory_stock::Column::ColorNo)
            .group_by(inventory_stock::Column::Grade)
            .order_by_asc(inventory_stock::Column::BatchNo)
            .order_by_asc(inventory_stock::Column::ColorNo);

        query = Self::apply_inventory_filters(query, &params);
        query = query
            .filter(inventory_stock::Column::StockStatus.eq("正常"))
            .filter(inventory_stock::Column::QualityStatus.eq("合格"));

        // 批次 266：接入 paginate_with_total，消除手写 count + fetch_page 重复
        // 聚合查询使用 into_model::<InventorySummaryQueryResult>，泛型 M = InventorySummaryQueryResult
        let paginator = query
            .into_model::<InventorySummaryQueryResult>()
            .paginate(&*self.db, params.page_size);
        let (result, total) =
            paginate_with_total(paginator, params.page.clamp(1, 1000)).await?;

        Ok((Self::map_summary_items(result), total))
    }

    /// 应用库存查询过滤条件（warehouse/product/batch/color/grade）
    fn apply_inventory_filters<Q: sea_orm::QueryFilter>(
        mut query: Q,
        params: &InventorySummaryQuery,
    ) -> Q {
        if let Some(wid) = params.warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }
        if let Some(pid) = params.product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }
        if let Some(batch) = params.batch_no.as_ref() {
            query = query.filter(inventory_stock::Column::BatchNo.eq(batch.as_str()));
        }
        if let Some(color) = params.color_no.as_ref() {
            query = query.filter(inventory_stock::Column::ColorNo.eq(color.as_str()));
        }
        if let Some(g) = params.grade.as_ref() {
            query = query.filter(inventory_stock::Column::Grade.eq(g.as_str()));
        }
        query
    }

    /// 将查询结果转换为库存汇总条目
    fn map_summary_items(
        result: Vec<InventorySummaryQueryResult>,
    ) -> Vec<InventorySummaryItem> {
        result
            .into_iter()
            .map(|r| InventorySummaryItem {
                product_id: r.product_id,
                product_name: r.product_name,
                batch_no: r.batch_no,
                color_no: r.color_no,
                grade: r.grade,
                warehouse_name: r.warehouse_name,
                total_quantity_meters: r.total_quantity_meters,
                total_quantity_kg: r.total_quantity_kg,
            })
            .collect()
    }

    /// 按产品查询库存
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，修复原 fetch_page(page) 未做
    /// saturating_sub(1) 偏移的 bug（page 为 1-based，fetch_page 接收 0-based，原实现跳过第一页）。
    /// 补 clamp(1, 1000) 防 DoS。
    pub async fn get_stock_by_product(
        &self,
        product_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<inventory_stock::Model>, u64), AppError> {
        let paginator = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .order_by_asc(inventory_stock::Column::Id)
            .paginate(&*self.db, page_size);

        let (stocks, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((stocks, total))
    }

    /// 获取库存告警
    ///
    /// 批次 126 v8 复审 P2 修复：alert_type 字段从硬编码 "normal" 改为派生计算。
    /// compute_alert_type 函数根据 stock_status / quantity_available / reorder_point /
    /// expiry_date / max_stock_point / last_movement_date 派生告警类型
    /// （discrepancy/out_of_stock/low_stock/over_stock/expiring/slow_moving/normal）。
    ///
    /// v11 批次 144 P1-4 修复：
    /// - 接入 max_stock_point 字段，支持 OverStock（高于上限）告警
    /// - 接入 last_movement_date 字段，支持 SlowMoving（滞销）告警
    ///
    /// 返回字段包含 reorder_point / max_stock_point / expiry_date / last_movement_date /
    /// stock_status，便于前端展示阈值上下文。
    pub async fn get_stock_alerts(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let warehouse_id = query
            .get("warehouse_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let product_id = query
            .get("product_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let mut stock_query = inventory_stock::Entity::find()
            .inner_join(crate::models::product::Entity)
            .inner_join(crate::models::warehouse::Entity);

        if let Some(wid) = warehouse_id {
            stock_query = stock_query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }
        if let Some(pid) = product_id {
            stock_query = stock_query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        let stocks = stock_query.all(&*self.db).await?;

        let alert_list: Vec<serde_json::Value> = stocks
            .into_iter()
            .map(|s| {
                // 批次 126 v8 复审 P2 修复：派生计算 alert_type（替换硬编码 "normal"）
                // v11 批次 144 P1-4：扩展 OverStock / SlowMoving 告警判定
                let alert_type = compute_alert_type(&s);
                serde_json::json!({
                    "id": s.id,
                    "product_id": s.product_id,
                    "warehouse_id": s.warehouse_id,
                    "quantity_on_hand": s.quantity_on_hand.to_string(),
                    "quantity_available": s.quantity_available.to_string(),
                    "quantity_reserved": s.quantity_reserved.to_string(),
                    "reorder_point": s.reorder_point.to_string(),
                    "max_stock_point": s.max_stock_point.to_string(),
                    "expiry_date": s.expiry_date.map(|d| d.to_rfc3339()),
                    "last_movement_date": s.last_movement_date.map(|d| d.to_rfc3339()),
                    "stock_status": s.stock_status,
                    "alert_type": alert_type,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "list": alert_list,
            "total": alert_list.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use chrono::{Duration, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// 构造测试用库存 Model（默认"正常"状态）
    fn make_stock_model() -> inventory_stock::Model {
        inventory_stock::Model {
            id: 1,
            warehouse_id: 1,
            product_id: 1,
            quantity_on_hand: decs!("100"),
            quantity_available: decs!("100"),
            quantity_reserved: decs!("0"),
            quantity_shipped: decs!("0"),
            quantity_incoming: decs!("0"),
            reorder_point: decs!("0"),
            max_stock_point: decs!("0"),
            reorder_quantity: decs!("0"),
            bin_location: None,
            last_count_date: None,
            last_movement_date: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            batch_no: "B001".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("DL001".to_string()),
            grade: "一等品".to_string(),
            production_date: None,
            expiry_date: None,
            quantity_meters: decs!("100"),
            quantity_kg: decs!("10"),
            gram_weight: Some(decs!("200")),
            width: Some(decs!("150")),
            location_id: Some(1),
            shelf_no: Some("A01".to_string()),
            layer_no: Some("1".to_string()),
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
            version: 0,
        }
    }

    // ========== 优先级 1: discrepancy（状态异常）==========

    #[test]
    fn 测试_库存告警_状态异常返回discrepancy() {
        let mut stock = make_stock_model();
        stock.stock_status = "冻结".to_string();
        assert_eq!(compute_alert_type(&stock), AlertType::Discrepancy.code());
    }

    #[test]
    fn 测试_库存告警_状态异常优先级最高_覆盖缺货() {
        // 状态异常 + 缺货条件同时满足，应返回 discrepancy（优先级 1 > 2）
        let mut stock = make_stock_model();
        stock.stock_status = "待检".to_string();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::Discrepancy.code());
    }

    // ========== 优先级 2: out_of_stock（缺货）==========

    #[test]
    fn 测试_库存告警_缺货返回out_of_stock() {
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::OutOfStock.code());
    }

    #[test]
    fn 测试_库存告警_缺货未设置补货点不触发告警() {
        // reorder_point == 0 时不判定缺货（未设置补货策略）
        let mut stock = make_stock_model();
        stock.reorder_point = Decimal::ZERO;
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 3: low_stock（低于下限）==========

    #[test]
    fn 测试_库存告警_低于下限返回low_stock() {
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = decs!("30");
        assert_eq!(compute_alert_type(&stock), AlertType::LowStock.code());
    }

    // ========== 优先级 4: over_stock（高于上限）==========

    #[test]
    fn 测试_库存告警_高于上限返回over_stock() {
        let mut stock = make_stock_model();
        stock.max_stock_point = decs!("200");
        stock.quantity_available = decs!("250");
        assert_eq!(compute_alert_type(&stock), AlertType::OverStock.code());
    }

    #[test]
    fn 测试_库存告警_高于上限未设置上限不触发告警() {
        // max_stock_point == 0 时不判定超储
        let mut stock = make_stock_model();
        stock.max_stock_point = Decimal::ZERO;
        stock.quantity_available = decs!("9999");
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 5: expiring（即将过期）==========

    #[test]
    fn 测试_库存告警_即将过期返回expiring() {
        let mut stock = make_stock_model();
        // 过期日期距今 10 天（≤ 30 天阈值）
        stock.expiry_date = Some(Utc::now() + Duration::days(10));
        assert_eq!(compute_alert_type(&stock), AlertType::Expiring.code());
    }

    #[test]
    fn 测试_库存告警_过期日期恰等于阈值返回expiring() {
        let mut stock = make_stock_model();
        // 过期日期距今 30 天（== EXPIRING_THRESHOLD_DAYS，边界 ≤ 判定）
        stock.expiry_date = Some(Utc::now() + Duration::days(30));
        assert_eq!(compute_alert_type(&stock), AlertType::Expiring.code());
    }

    // ========== 优先级 6: slow_moving（滞销）==========

    #[test]
    fn 测试_库存告警_滞销返回slow_moving() {
        let mut stock = make_stock_model();
        // 最后变动日期距今 100 天（> 90 天阈值）
        stock.last_movement_date = Some(Utc::now() - Duration::days(100));
        assert_eq!(compute_alert_type(&stock), AlertType::SlowMoving.code());
    }

    #[test]
    fn 测试_库存告警_从未变动且有库存返回slow_moving() {
        let mut stock = make_stock_model();
        // last_movement_date 为 None 且有库存 → 视为滞销
        stock.last_movement_date = None;
        stock.quantity_available = decs!("50");
        assert_eq!(compute_alert_type(&stock), AlertType::SlowMoving.code());
    }

    #[test]
    fn 测试_库存告警_从未变动且无库存不返回slow_moving() {
        // last_movement_date 为 None 但无库存 → 不判定滞销（无库存无所谓滞销）
        let mut stock = make_stock_model();
        stock.last_movement_date = None;
        stock.quantity_available = Decimal::ZERO;
        stock.reorder_point = Decimal::ZERO; // 避免触发 out_of_stock
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级 7: normal（正常）==========

    #[test]
    fn 测试_库存告警_正常库存返回normal() {
        let stock = make_stock_model();
        assert_eq!(compute_alert_type(&stock), ALERT_TYPE_NORMAL);
    }

    // ========== 优先级链路覆盖测试 ==========

    #[test]
    fn 测试_库存告警_优先级链路_缺货高于低于下限() {
        // quantity_available == 0 满足缺货条件，同时 < reorder_point 满足低于下限
        // 缺货（优先级 2）应先于低于下限（优先级 3）返回
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.quantity_available = Decimal::ZERO;
        assert_eq!(compute_alert_type(&stock), AlertType::OutOfStock.code());
    }

    #[test]
    fn 测试_库存告警_优先级链路_低于下限高于高于上限() {
        // 同时满足低于下限和高于上限不可能（quantity_available < reorder_point 且 > max_stock_point）
        // 此测试验证逻辑不冲突：reorder_point=50, max_stock_point=200, quantity=30 → low_stock
        let mut stock = make_stock_model();
        stock.reorder_point = decs!("50");
        stock.max_stock_point = decs!("200");
        stock.quantity_available = decs!("30");
        assert_eq!(compute_alert_type(&stock), AlertType::LowStock.code());
    }
}
