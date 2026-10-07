//! 销售发货-发货主流程子模块（delivery_ops/ship）
//!
//! 职责：销售发货主流程及其下游（库存扣减、发货量回写、AR、收入凭证、事件发布）。
//! 包含 ship_order 及其 15 个辅助方法：
//! - ship_order（公开 API）
//! - validate_ship_preconditions / load_ship_order_context / create_shipment_delivery
//! - process_shipment_items / lookup_line_price / build_delivery_item
//! - compute_quantity_kg / build_record_transaction_args / update_order_item_shipped_qty
//! - update_order_after_shipment / post_commit_shipment_effects / check_order_fully_shipped
//! - create_revenue_voucher_for_delivery / build_revenue_voucher_request / build_revenue_voucher_item

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};

use crate::models::status::sales_delivery as delivery_status;
use crate::models::status::sales_order as so_status;
use crate::models::{
    sales_delivery, sales_delivery_item, sales_order, sales_order_item, warehouse,
};
use crate::utils::error::AppError;

use super::super::delivery::{ShipOrderItemRequest, ShipOrderRequest};
use super::super::order::SalesService;
use super::types::{ShipOrderContext, ShipPostCommitContext, ShipmentItemsResult};

impl SalesService {
    /// 销售订单发货
    pub async fn ship_order(
        &self,
        request: ShipOrderRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        self.validate_ship_preconditions(&request).await?;
        let txn = (*self.db).begin().await?;
        let ctx = self.load_ship_order_context(&request, &txn).await?;
        let delivery = self
            .create_shipment_delivery(&request, &ctx.order, &ctx.warehouse, user_id, &txn)
            .await?;
        let items_result = self
            .process_shipment_items(&request, &ctx, &delivery, user_id, &txn)
            .await?;
        let post_ctx = self
            .update_order_after_shipment(&request, ctx, user_id, &txn)
            .await?;
        txn.commit().await?;
        self.post_commit_shipment_effects(&delivery, post_ctx, items_result, user_id)
            .await;

        // 业务追溯 chain tail 接入（best-effort，失败不阻塞销售发货）
        let trace_service =
            crate::services::business_trace_service::BusinessTraceService::new(self.db.clone());
        let delivery_items: Vec<sales_delivery_item::Model> = sales_delivery_item::Entity::find()
            .filter(sales_delivery_item::Column::DeliveryId.eq(delivery.id))
            .all(&*self.db)
            .await
            .unwrap_or_default();
        trace_service
            .record_sales_delivery(&delivery, &delivery_items, user_id)
            .await;

        Ok(())
    }

    /// 发货前置校验：缸号一致性 + 大货批色门禁
    async fn validate_ship_preconditions(
        &self,
        request: &ShipOrderRequest,
    ) -> Result<(), AppError> {
        // 缸号同订单校验
        // 依据：.monkeycode/docs/research/fabric-industry-research.md §2.3 约束 5 - 同一订单同面料必须使用相同缸号
        // 必须在开启事务前校验，避免无效请求占用数据库事务资源
        super::super::delivery::validate_dye_lot_consistency(&request.items)?;
        // 发货前校验大货批色门禁
        // 业务规则：销售订单关联的所有 bulk_color_approval 记录必须全部为 approved 状态
        // 否则阻止发货（delivery_blocking=true 阻断）
        crate::services::bulk_color_approval_service::validate_bulk_color_approval(
            &self.db,
            request.order_id,
        )
        .await
        .map_err(|e| match e {
            crate::services::bulk_color_approval_service::BulkColorApprovalError::InvalidState(
                msg,
            ) => AppError::business(msg),
            crate::services::bulk_color_approval_service::BulkColorApprovalError::SalesOrderNotFound => {
                AppError::not_found("销售订单不存在")
            }
            crate::services::bulk_color_approval_service::BulkColorApprovalError::Database(e) => {
                AppError::database(e.to_string())
            }
            other => AppError::business(other.to_string()),
        })?;
        Ok(())
    }

    /// 发货明细的产品必须出现在该销售订单的明细行中（款号维度一致性门控）。
    ///
    /// 若订单外产品混入，会一路走到：
    /// - `lookup_line_price` 取不到订单行、按 `(0, 0)` 兜底 ⇒ 发货单金额恒 0、
    /// 收入凭证被 `create_revenue_voucher_for_delivery` 判「含税金额为 0」跳过；
    /// - `update_order_item_shipped_qty` 的 `WHERE order_id AND product_id` 匹配 0 行，
    ///   发货量一行都没回写 ⇒ `check_order_fully_shipped` 恒假，订单停在 `partial_shipped`。
    /// 两处静默兜底叠加即「发货成功、账实两头空」，属必须 fail-closed 的前置缺失，
    /// 故在扣库存/写流水之前整单拒绝。拒绝原因是公开业务规则（用户改一下发货明细即可自行处理）
    /// ⇒ 可外显；但产品 ID 属内部标识，只进日志不出参。
    fn ensure_ship_items_belong_to_order(
        order_id: i32,
        order_items: &[sales_order_item::Model],
        items: &[ShipOrderItemRequest],
    ) -> Result<(), AppError> {
        let mut out_of_order: Vec<i32> = items
            .iter()
            .filter(|it| !order_items.iter().any(|oi| oi.product_id == it.product_id))
            .map(|it| it.product_id)
            .collect();
        if out_of_order.is_empty() {
            return Ok(());
        }
        out_of_order.sort_unstable();
        out_of_order.dedup();
        tracing::warn!(
            "销售订单 {} 发货被拒：发货明细产品 {:?} 不在该订单明细行中（既无单价也无发货量可回写）",
            order_id,
            out_of_order
        );
        Err(AppError::business_displayable(
            "发货明细中存在该销售订单未包含的产品，请按订单明细选择产品后重新发货",
        ))
    }

    /// 事务内加载发货上下文：订单锁定 + 明细 + 产品 + 仓库 + 库存校验
    async fn load_ship_order_context(
        &self,
        request: &ShipOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipOrderContext, AppError> {
        // 检查订单状态（加 lock_exclusive 串行化并发发货）
        let order = sales_order::Entity::find_by_id(request.order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;
        if order.status != so_status::APPROVED {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }
        // 查询订单明细
        // 保留查询结果用于款号一致性门控与发货金额计算
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(txn)
            .await?;
        // 发货明细必须属于该订单的明细行（款号维度）：越界产品一律整单拒绝，
        // 不再让它流到下游被静默按 0 单价/0 发货量处理（见 ensure_ 方法注释）
        Self::ensure_ship_items_belong_to_order(request.order_id, &order_items, &request.items)?;
        // 批量查询产品获取 gram_weight/width，
        // 用于库存流水的 quantity_kg 双单位换算
        let product_ids: Vec<i32> = order_items.iter().map(|oi| oi.product_id).collect();
        let products = if product_ids.is_empty() {
            Vec::new()
        } else {
            crate::models::product::Entity::find()
                .filter(crate::models::product::Column::Id.is_in(product_ids))
                .all(txn)
                .await?
        };
        let product_map: std::collections::HashMap<i32, crate::models::product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();
        // 查询仓库
        let warehouse = warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(&request.warehouse_code))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("仓库不存在"))?;
        // 保存发货明细快照用于事件发布
        let shipped_items_snapshot: Vec<(i32, rust_decimal::Decimal)> = request
            .items
            .iter()
            .map(|i| (i.product_id, i.quantity))
            .collect();
        self.check_inventory(request.order_id, warehouse.id, &request.items, txn)
            .await?;
        Ok(ShipOrderContext {
            order,
            order_items,
            product_map,
            warehouse,
            shipped_items_snapshot,
        })
    }

    /// 创建发货单主记录
    async fn create_shipment_delivery(
        &self,
        request: &ShipOrderRequest,
        order: &sales_order::Model,
        warehouse: &warehouse::Model,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<sales_delivery::Model, AppError> {
        // 单据号由 DocumentNumberGenerator 生成，保证并发唯一（同秒并发不重复）
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            delivery_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    txn,
                    "DN",
                    sales_delivery::Entity,
                    sales_delivery::Column::DeliveryNo,
                )
                .await?,
            ),
            order_id: Set(request.order_id),
            customer_id: Set(order.customer_id),
            warehouse_id: Set(warehouse.id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set(delivery_status::SHIPPED.to_string()),
            total_quantity: Set(request.items.iter().map(|i| i.quantity).sum()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(request.remarks.clone()),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        Ok(delivery.insert(txn).await?)
    }

    /// 循环处理发货明细：四维扣减库存 + 生成库存流水 + 累加金额 + 批量 INSERT
    ///
    /// 出库四维规则（染色布强制 缸号/色号/批次/匹号，
    /// 款号由 product_id 承载；白坯免缸号免匹号）：一笔发货明细可能拆成多笔实际扣减
    /// （指定缸不足时显式跨缸回退），每一笔实际扣减单独生成一行出库明细与一条库存流水，
    /// 如实记录实际扣到的缸号/批次与该行前后数量；染色布指定匹在同一事务内 CAS 消耗。
    async fn process_shipment_items(
        &self,
        request: &ShipOrderRequest,
        ctx: &ShipOrderContext,
        delivery: &sales_delivery::Model,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipmentItemsResult, AppError> {
        // 超发交货容差门控：越界即整单拒绝（在扣减库存/写流水之前，事务回滚无副作用）
        Self::validate_shipment_within_tolerance(ctx, request)?;
        let order_item_map: std::collections::HashMap<i32, &sales_order_item::Model> = ctx
            .order_items
            .iter()
            .map(|oi| (oi.product_id, oi))
            .collect();
        let mut delivery_items_to_insert: Vec<sales_delivery_item::ActiveModel> = Vec::new();
        let mut pending_inventory_events: Vec<crate::services::event_bus::BusinessEvent> =
            Vec::new();
        let mut delivery_total_amount = Decimal::ZERO;
        let mut delivery_total_tax = Decimal::ZERO;
        for item in &request.items {
            let (unit_price, tax_percent) = Self::lookup_line_price(&order_item_map, item);
            let reductions = self
                .reduce_inventory_four_dim(item, ctx.warehouse.id, request.order_id, user_id, txn)
                .await?;
            for reduction in reductions {
                let line_amount = (reduction.quantity * unit_price).round_dp(2);
                let line_tax = (line_amount * tax_percent / Decimal::new(100, 0)).round_dp(2);
                delivery_total_amount += line_amount;
                delivery_total_tax += line_tax;
                delivery_items_to_insert.push(Self::build_delivery_item(
                    item,
                    &reduction,
                    delivery.id,
                    unit_price,
                    line_amount,
                ));
                let quantity_kg = Self::compute_quantity_kg(
                    &ctx.product_map,
                    item.product_id,
                    reduction.quantity,
                );
                let args = Self::build_record_transaction_args(
                    item,
                    request,
                    ctx,
                    &reduction,
                    quantity_kg,
                    user_id,
                );
                let (_, txn_event) =
                    crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
                        txn, args,
                    )
                    .await?;
                if let Some(ev) = txn_event {
                    pending_inventory_events.push(ev);
                }
            }
            Self::update_order_item_shipped_qty(
                txn,
                request.order_id,
                item.product_id,
                item.quantity,
                ctx.product_map
                    .get(&item.product_id)
                    .map(|p| p.unit.as_str()),
            )
            .await?;
        }
        if !delivery_items_to_insert.is_empty() {
            sales_delivery_item::Entity::insert_many(delivery_items_to_insert)
                .exec(txn)
                .await?;
        }
        Ok(ShipmentItemsResult {
            delivery_total_amount,
            delivery_total_tax,
            pending_inventory_events,
        })
    }

    /// 销售发货超发容差门控（纯校验，无 DB）。
    ///
    /// 规则：同一产品「累计已发 + 本次发货」不得超过该产品**全部订单行**的
    /// 「行数量×(1+容差)」上界之和；落在 `数量×(1±容差)` 区间内一律放行（不拒发），
    /// 仅越界时返回含上下界的业务错误。
    ///
    /// 同一面料常因跨缸/分色号被拆成多条订单明细行，故上下界与已发量按产品聚合**全部**订单行：
    /// 若只取**第一条**匹配 product_id 的订单行做上下界（`order_items.iter().find(product_id)`），
    /// 其余同产品行既不计入允许上界（把可发总量算少），
    /// 也不计入已发量 ⇒ 与按行回写的发货量无法对账。
    ///
    /// 容差解析优先级：行显式值 > 品类默认（按产品计量单位：面料/按量→5%、计件→0%）> 全局默认（5%）。
    fn validate_shipment_within_tolerance(
        ctx: &ShipOrderContext,
        request: &ShipOrderRequest,
    ) -> Result<(), AppError> {
        use std::collections::HashMap;
        let mut running: HashMap<i32, Decimal> = HashMap::new();
        for item in &request.items {
            let lines: Vec<&sales_order_item::Model> = ctx
                .order_items
                .iter()
                .filter(|oi| oi.product_id == item.product_id)
                .collect();
            if lines.is_empty() {
                // 订单行不存在：已由 ensure_ship_items_belong_to_order 在任何写操作前整单拒绝，
                // 此处不再重复判定，保留原分支作为纯函数的独立防御
                continue;
            }
            let unit = ctx
                .product_map
                .get(&item.product_id)
                .map(|p| p.unit.as_str());
            let mut lower = Decimal::ZERO;
            let mut upper = Decimal::ZERO;
            let mut already_shipped = Decimal::ZERO;
            for oi in &lines {
                let pct = crate::utils::delivery_tolerance::resolve_tolerance_pct(
                    oi.quantity_tolerance_pct,
                    unit,
                );
                let (line_lower, line_upper) =
                    crate::utils::delivery_tolerance::tolerance_bounds(oi.quantity, pct);
                lower += line_lower;
                upper += line_upper;
                already_shipped += oi.shipped_quantity;
            }
            let cumulative = already_shipped
                + running
                    .get(&item.product_id)
                    .copied()
                    .unwrap_or(Decimal::ZERO)
                + item.quantity;
            if cumulative > upper {
                return Err(AppError::business(format!(
                    "产品 {} 累计发货量 {} 超过销售订单行允许发货上界 {}（允许区间 [{}, {}]，含交货容差），拒绝发货",
                    item.product_id, cumulative, upper, lower, upper
                )));
            }
            *running.entry(item.product_id).or_insert(Decimal::ZERO) += item.quantity;
        }
        Ok(())
    }

    /// 取订单行的单价与税率（行缺失按 0 处理）
    fn lookup_line_price(
        order_item_map: &std::collections::HashMap<i32, &sales_order_item::Model>,
        item: &ShipOrderItemRequest,
    ) -> (Decimal, Decimal) {
        order_item_map
            .get(&item.product_id)
            .map(|oi| (oi.unit_price, oi.tax_percent))
            .unwrap_or((Decimal::ZERO, Decimal::ZERO))
    }

    /// 构建出库明细行：色号/缸号/批次记录**实际被扣库存行**的维度（跨缸回退时为其他缸）
    fn build_delivery_item(
        item: &ShipOrderItemRequest,
        reduction: &super::inventory::StockReduction,
        delivery_id: i32,
        unit_price: Decimal,
        line_amount: Decimal,
    ) -> sales_delivery_item::ActiveModel {
        sales_delivery_item::ActiveModel {
            id: Default::default(),
            delivery_id: Set(delivery_id),
            product_id: Set(item.product_id),
            quantity: Set(reduction.quantity),
            batch_no: Set(reduction.batch_no.clone()),
            color_no: Set(reduction.color_no.clone()),
            dye_lot_id: Set(None),
            dye_lot_no: Set(reduction.dye_lot_no.clone().unwrap_or_default()),
            piece_no: Set(item.piece_no.clone()),
            // 四维扣减落点：实际被扣库存行 ID 与是否跨缸回退，如实写入出库明细
            stock_id: Set(Some(reduction.stock_id)),
            is_cross_dye_lot: Set(reduction.is_cross_dye_lot()),
            remarks: Set(if reduction.is_cross_dye_lot() {
                Some(reduction.dye_lot_trace_note())
            } else {
                None
            }),
            unit_price: Set(unit_price),
            amount: Set(line_amount),
            created_at: Set(chrono::Utc::now()),
        }
    }

    fn compute_quantity_kg(
        product_map: &std::collections::HashMap<i32, crate::models::product::Model>,
        product_id: i32,
        quantity: Decimal,
    ) -> Decimal {
        product_map
            .get(&product_id)
            .and_then(|p| {
                let gram_weight = p.gram_weight?;
                let width = p.width?;
                crate::utils::dual_unit_converter::DualUnitConverter::meters_to_kg(
                    quantity,
                    gram_weight,
                    width,
                )
                .ok()
            })
            .unwrap_or(Decimal::ZERO)
    }

    fn build_record_transaction_args(
        item: &ShipOrderItemRequest,
        request: &ShipOrderRequest,
        ctx: &ShipOrderContext,
        reduction: &super::inventory::StockReduction,
        quantity_kg: Decimal,
        user_id: i32,
    ) -> crate::services::inventory_stock_query::RecordTransactionArgs {
        crate::services::inventory_stock_query::RecordTransactionArgs {
            transaction_type: "SALES_DELIVERY".to_string(),
            product_id: item.product_id,
            warehouse_id: ctx.warehouse.id,
            // 流水如实记录实际被扣库存行的批次/色号/缸号（跨缸回退时为其他缸）
            batch_no: reduction.batch_no.clone(),
            color_no: reduction.color_no.clone(),
            dye_lot_no: reduction.dye_lot_no.clone(),
            grade: String::new(),
            quantity_meters: reduction.quantity,
            quantity_kg,
            source_bill_type: Some("sales_order".to_string()),
            source_bill_no: Some(ctx.order.order_no.clone()),
            source_bill_id: Some(request.order_id),
            quantity_before_meters: Some(reduction.quantity_before),
            quantity_before_kg: None,
            quantity_after_meters: Some(reduction.quantity_after),
            quantity_after_kg: None,
            notes: Some(format!(
                "销售出库 - 订单 {}{}",
                ctx.order.order_no,
                reduction.dye_lot_trace_note()
            )),
            created_by: Some(user_id),
        }
    }

    async fn update_order_item_shipped_qty(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        product_id: i32,
        quantity: Decimal,
        unit: Option<&str>,
    ) -> Result<(), AppError> {
        // 同一产品的多条订单行（跨缸/分色号拆行）必须逐行分配，不能整笔加到每一行：
        // 若用 `update_many().filter(order_id).filter(product_id)` 把本次发货量同时累加到
        // 该产品**每一条**订单行（发 500 米 ⇒ 两行各 +500），行进度与
        // `check_order_fully_shipped` 的「所有行 shipped >= quantity」双双失真
        // （要么提前判成全额发货，要么与出库明细对不上账）。
        //
        // 分配口径：按订单行 ID 升序（建单次序，确定性可重放）逐行补到本行允许上界
        // `数量×(1+容差)` 为止；订单行已被订单头 `lock_exclusive` 串行化，事务内重查即最新值。
        let lines = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .filter(sales_order_item::Column::ProductId.eq(product_id))
            .order_by_asc(sales_order_item::Column::Id)
            .all(txn)
            .await?;
        let mut remaining = quantity;
        for line in lines {
            if remaining.is_zero() {
                break;
            }
            let line_shipped = line.shipped_quantity;
            let pct = crate::utils::delivery_tolerance::resolve_tolerance_pct(
                line.quantity_tolerance_pct,
                unit,
            );
            let (_line_lower, line_upper) =
                crate::utils::delivery_tolerance::tolerance_bounds(line.quantity, pct);
            let capacity = line_upper - line_shipped;
            if capacity <= Decimal::ZERO {
                continue;
            }
            let allocated = if capacity < remaining {
                capacity
            } else {
                remaining
            };
            let mut active: sales_order_item::ActiveModel = line.into();
            active.shipped_quantity = Set(line_shipped + allocated);
            active.updated_at = Set(chrono::Utc::now());
            active.update(txn).await?;
            remaining -= allocated;
        }
        // 分配不完 = 本次发货量超过该产品全部订单行的可发余量：整单回滚拒绝，
        // 严禁把余量静默丢弃（丢弃即「出库了但订单行没记」的账实脱节）
        if !remaining.is_zero() {
            tracing::warn!(
                "销售订单 {} 产品 {} 发货量回写失败：仍有 {} 无法分配到任何订单行（各行均已达允许上界或无订单行）",
                order_id,
                product_id,
                remaining
            );
            return Err(AppError::business(format!(
                "产品 {} 本次发货量 {} 无法分配到销售订单明细行（各行均已达到允许发货上界），拒绝发货",
                product_id, quantity
            )));
        }
        tracing::info!(
            "销售订单 {} 产品 {} 发货量回写完成：{}（按订单明细行逐行分配）",
            order_id,
            product_id,
            quantity
        );
        Ok(())
    }

    /// 更新订单状态 + 全额发货时生成 AR 应收
    async fn update_order_after_shipment(
        &self,
        request: &ShipOrderRequest,
        ctx: ShipOrderContext,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipPostCommitContext, AppError> {
        // 更新订单状态
        let order_items_total: Vec<sales_order_item::Model> = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(txn)
            .await?;
        // 全额发货判断提取到 check_order_fully_shipped
        let is_fully_shipped = Self::check_order_fully_shipped(&order_items_total);
        let new_status = if is_fully_shipped {
            so_status::SHIPPED
        } else {
            so_status::PARTIAL_SHIPPED
        };
        // 状态推进结论必须显式可查：partial_shipped 到底是「还有行没发满」还是
        // 「发货量根本没回写到行」，靠这条日志 + 上一行（逐行回写完成）即可二分定位
        tracing::info!(
            "销售订单 {} 发货后状态判定：{}（明细行数 {}，全额发货 {}）",
            request.order_id,
            new_status,
            order_items_total.len(),
            is_fully_shipped
        );
        // 保存发货上下文用于 AR 生成和事件发布
        // order 在下方 .into() 被消费，提前保存所需字段
        // 收入凭证源单号使用 delivery_no
        let ship_customer_id = ctx.order.customer_id;
        let ship_order_total = ctx.order.total_amount;
        let ship_order_id = request.order_id;
        let ship_items_for_event: Vec<crate::services::event_bus::ShippedItem> = ctx
            .shipped_items_snapshot
            .iter()
            .map(|(pid, qty)| crate::services::event_bus::ShippedItem {
                product_id: *pid,
                quantity: *qty,
            })
            .collect();
        let mut order_update: sales_order::ActiveModel = ctx.order.into();
        order_update.status = Set(new_status.to_string());
        order_update.ship_date = Set(Some(chrono::Utc::now()));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_update,
            // 审计记录真实操作人 user_id
            Some(user_id),
        )
        .await?;
        // 销售→AR 业务流：全额发货时在 commit 前调用 create_receivable 生成 AR（与订单状态更新共用事务）
        if is_fully_shipped {
            // 查询客户账期（payment_terms <= 0 时 create_receivable 内部回退 30 天）
            let customer = crate::models::customer::Entity::find_by_id(ship_customer_id)
                .one(txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", ship_customer_id)))?;
            let payment_terms = customer.payment_terms;
            let ar_service = crate::services::ar::ArReconciliationService::new(self.db.clone());
            ar_service
                .create_receivable(
                    ship_customer_id,
                    ship_order_id,
                    ship_order_total,
                    payment_terms,
                    user_id,
                    txn,
                )
                .await?;
        }
        Ok(ShipPostCommitContext {
            ship_customer_id,
            ship_order_id,
            ship_items_for_event,
        })
    }

    /// 提交事务后：收入凭证生成 + 库存流水事件 + 销售发货事件发布
    async fn post_commit_shipment_effects(
        &self,
        delivery: &sales_delivery::Model,
        post_ctx: ShipPostCommitContext,
        items_result: ShipmentItemsResult,
        user_id: i32,
    ) {
        // 收入凭证生成提取到 create_revenue_voucher_for_delivery
        // 每次发货都生成收入确认凭证
        // 借：应收账款（含税总额，挂客户辅助核算）
        // 贷：主营业务收入（不含税）/ 应交税费-销项税额
        // 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
        self.create_revenue_voucher_for_delivery(
            delivery,
            items_result.delivery_total_amount,
            items_result.delivery_total_tax,
            post_ctx.ship_customer_id,
            user_id,
        )
        .await;
        // commit 后统一发布库存流水事件
        // 触发 inventory_finance_bridge_service 自动生成销售出库凭证
        for ev in items_result.pending_inventory_events {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }
        // P1 5-1 修复（批次 62）：commit 后发布 SalesOrderShipped 事件
        // 事件发布必须在 commit 之后，避免消费者读到未提交数据。
        // 监听器（event_bus.rs）消费此事件触发财务指标刷新（5-2 修复）。
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::SalesOrderShipped {
                order_id: post_ctx.ship_order_id,
                customer_id: post_ctx.ship_customer_id,
                items: post_ctx.ship_items_for_event,
            },
        );
    }

    /// 判断订单是否全额发货（所有明细 shipped_quantity >= quantity）
    fn check_order_fully_shipped(order_items_total: &[sales_order_item::Model]) -> bool {
        for oi in order_items_total {
            if oi.shipped_quantity < oi.quantity {
                return false;
            }
        }
        true
    }

    /// 为发货单生成收入确认凭证（借应收/贷收入+销项税）
    /// 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
    async fn create_revenue_voucher_for_delivery(
        &self,
        delivery: &sales_delivery::Model,
        delivery_total_amount: Decimal,
        delivery_total_tax: Decimal,
        ship_customer_id: i32,
        user_id: i32,
    ) {
        let delivery_total_incl_tax = delivery_total_amount + delivery_total_tax;
        if delivery_total_incl_tax <= Decimal::ZERO {
            // 货物已出库却不确认收入，属账实脱节：金额为零必然订单明细缺单价，
            // 静默返回会让发货单长期没有收入凭证且无人知晓
            tracing::warn!(
                "发货单 {}（订单 {}）含税金额为 0，跳过收入确认凭证：请核对订单明细单价是否维护",
                delivery.delivery_no,
                delivery.order_id
            );
            return;
        }
        let voucher_req = Self::build_revenue_voucher_request(
            delivery,
            delivery_total_amount,
            delivery_total_tax,
            delivery_total_incl_tax,
            ship_customer_id,
        );
        let voucher_service =
            crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!("发货单 {} 收入凭证生成失败：{}", delivery.delivery_no, e);
        }
    }

    /// 构建销售出库收入确认凭证请求（借应收/贷收入/贷销项税 三行分录）
    fn build_revenue_voucher_request(
        delivery: &sales_delivery::Model,
        delivery_total_amount: Decimal,
        delivery_total_tax: Decimal,
        delivery_total_incl_tax: Decimal,
        ship_customer_id: i32,
    ) -> crate::services::voucher_service::CreateVoucherRequest {
        let summary = format!("销售出库收入确认-{}", delivery.delivery_no);
        crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: Some("SALES_DELIVERY".to_string()),
            source_module: Some("sales".to_string()),
            source_bill_id: Some(delivery.id),
            source_bill_no: Some(delivery.delivery_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                Self::build_revenue_voucher_item(
                    1,
                    "1122",
                    "应收账款",
                    delivery_total_incl_tax,
                    Decimal::ZERO,
                    &summary,
                    ship_customer_id,
                ),
                Self::build_revenue_voucher_item(
                    2,
                    "6001",
                    "主营业务收入",
                    Decimal::ZERO,
                    delivery_total_amount,
                    &summary,
                    ship_customer_id,
                ),
                Self::build_revenue_voucher_item(
                    3,
                    "222101",
                    "应交税费-应交增值税-销项税额",
                    Decimal::ZERO,
                    delivery_total_tax,
                    &summary,
                    ship_customer_id,
                ),
            ],
        }
    }

    /// 构建收入确认凭证的单行分录（客户辅助核算项固定，其余辅助核算项为 None）
    fn build_revenue_voucher_item(
        line_no: i32,
        subject_code: &str,
        subject_name: &str,
        debit: Decimal,
        credit: Decimal,
        summary: &str,
        ship_customer_id: i32,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(line_no),
            subject_id: None,
            subject_code: Some(subject_code.to_string()),
            subject_name: Some(subject_name.to_string()),
            debit,
            credit,
            summary: Some(summary.to_string()),
            assist_customer_id: Some(ship_customer_id),
            assist_supplier_id: None,
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: None,
            assist_color_no_id: None,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: None,
            quantity_kg: None,
            unit_price: None,
        }
    }
}
