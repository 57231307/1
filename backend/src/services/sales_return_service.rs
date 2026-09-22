//! 销售退货 Service
//!
//! 销售退货服务层，负责销售退货的核心业务逻辑

use crate::models::status::sales_return as sr_status;
use crate::models::{
    inventory_stock, product, sales_delivery, sales_delivery_item, sales_order_item, sales_return,
    sales_return_item,
};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{DataScopeContext, apply_data_scope, check_resource_owner};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;

use super::ar_invoice_service::{ArInvoiceService, CreateArInvoiceRequest};
use super::inventory_stock_query::RecordTransactionArgs;
use super::inventory_stock_service::InventoryStockService;
// 批次 358 v13 复审 B-P1-1 修复：导入 BusinessEvent 和 EVENT_BUS 用于事务安全的事件发布
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};

/// 创建销售退货请求
#[derive(Deserialize)]
pub struct CreateSalesReturnRequest {
    pub order_id: Option<i32>,
    pub customer_id: i32,
    pub return_date: chrono::NaiveDate,
    pub warehouse_id: i32,
    pub reason_type: String,
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}

/// 更新销售退货请求
#[derive(Deserialize)]
pub struct UpdateSalesReturnRequest {
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub return_date: Option<chrono::NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub reason_type: Option<String>,
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}

/// 添加退货明细项请求
#[derive(Deserialize)]
pub struct CreateSalesReturnItemRequest {
    pub line_no: Option<i32>,
    pub product_id: i32,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    /// 税率（百分比，如 13 表示 13%）。缺省时后端按关联销售订单同商品明细的权威税率回填；
    /// 无权威来源时拒绝写入（不静默默认为 0）。
    pub tax_percent: Option<Decimal>,
    /// 折扣率（百分比）。缺省按 0（无折扣为合法业务默认，非缺失字段掩盖）。
    pub discount_percent: Option<Decimal>,
    pub reason: Option<String>,
}

/// 销售退货明细金额计算结果：四个 NOT NULL 金额列（对应 sales_return_item.subtotal /
/// discount_amount / tax_amount / total_amount）。
struct ReturnItemAmounts {
    subtotal: Decimal,
    discount_amount: Decimal,
    tax_amount: Decimal,
    total_amount: Decimal,
}

/// 销售退货明细追溯三元组（色号 / 缸号 / 批次）。
///
/// 面料四维口径下，退货再入库必须命中真实的色号/缸号/批次行，故这三列必须由系统
/// 从权威来源回写，禁止落 DB 默认空串（空串会命中空色号行，造成库存错配）。
struct ReturnItemTrace {
    color_no: String,
    dye_lot_no: String,
    batch_no: String,
}

/// 销售退货服务
pub struct SalesReturnService {
    db: Arc<DatabaseConnection>,
}

impl SalesReturnService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 解析税率（不做静默默认 0）。优先级：请求显式提供 > 关联销售订单同商品明细税率。
    /// 两者皆无（退货单未关联销售订单，或订单内无同商品明细）时报业务错误，
    /// 由调用方在请求中补 tax_percent 或数据层修正来源。
    fn resolve_tax_percent(
        req_tax: Option<Decimal>,
        order_item_tax: Option<Decimal>,
        sales_order_id: Option<i32>,
        product_id: i32,
    ) -> Result<Decimal, AppError> {
        if let Some(t) = req_tax {
            return Ok(t);
        }
        if let Some(t) = order_item_tax {
            return Ok(t);
        }
        Err(AppError::business(format!(
            "退货明细缺税率 tax_percent：退货单关联销售订单 {:?}，商品 {} 无法从销售订单明细取得权威税率，请在请求中显式提供 tax_percent",
            sales_order_id, product_id
        )))
    }

    /// 从出库明细（实际发货的那一行）构造追溯三元组。
    ///
    /// 出库明细的色号/缸号/批次由发货四维扣减如实记录（`so/delivery_ops/ship.rs` 的
    /// `build_delivery_item`，写入实际被扣库存行的维度，跨缸回退时为其他缸），
    /// 因此是"实扣了哪一行/哪个缸"的真相来源，回写优先级最高。
    fn trace_from_delivery_item(item: &sales_delivery_item::Model) -> ReturnItemTrace {
        ReturnItemTrace {
            color_no: item.color_no.clone(),
            dye_lot_no: item.dye_lot_no.clone(),
            batch_no: item.batch_no.clone(),
        }
    }

    /// 从销售订单明细构造追溯三元组（次级来源）。
    ///
    /// 色号取订单行 `color_no`；缸号/批次取订单行的染色要求 / 批次要求。要求为空即
    /// 白坯布（本仓库统一口径"色号为空即白坯布"），此处如实回写来源的空值、不臆造，
    /// 白坯布判定不在此实现。
    fn trace_from_order_item(item: &sales_order_item::Model) -> ReturnItemTrace {
        ReturnItemTrace {
            color_no: item.color_no.clone(),
            dye_lot_no: match item.dye_lot_requirement.as_deref() {
                Some(s) => s.to_string(),
                None => String::new(),
            },
            batch_no: match item.batch_requirement.as_deref() {
                Some(s) => s.to_string(),
                None => String::new(),
            },
        }
    }

    /// 回写优先级：出库明细（实际发货行）→ 销售订单明细。
    ///
    /// 两处都取不到该商品的追溯来源时返回业务错误，绝不落空串 / 默认值。
    /// 纯函数，只依据调用方查到的来源候选值决策，便于单测三种路径。
    fn resolve_return_item_trace(
        delivery: Option<ReturnItemTrace>,
        order_item: Option<ReturnItemTrace>,
        sales_order_id: Option<i32>,
        product_id: i32,
    ) -> Result<ReturnItemTrace, AppError> {
        if let Some(trace) = delivery {
            return Ok(trace);
        }
        if let Some(trace) = order_item {
            return Ok(trace);
        }
        Err(match sales_order_id {
            None => AppError::business(format!(
                "退货明细缺追溯字段（色号/缸号/批次）：退货单未关联销售订单，无法从出库明细或销售订单明细定位商品 {} 的权威来源。请通过销售订单/出库单发起退货并在建单时填写关联订单号",
                product_id
            )),
            Some(order_id) => AppError::business(format!(
                "退货明细缺追溯字段（色号/缸号/批次）：销售订单 {} 下商品 {} 既无出库明细也无订单明细，无法回写。请确认退货商品与原出库单/销售订单一致，或先关联对应单据",
                order_id, product_id
            )),
        })
    }

    /// 按优先级从权威来源解析退货明细追溯三元组（出库明细 → 销售订单明细）。
    ///
    /// 出库明细优先：它是发货四维扣减的真实落点；无出库明细时回落到销售订单明细。
    /// 同一商品在来源表中命中多行（多次发货 / 多缸拆分 / 同商品多色号订单行）时，
    /// 因退货明细不携带原出库行 / 订单行引用而无法唯一定位，直接返回业务错误暴露
    /// 缺失的引用维度，禁止臆测任一行（不做兜底）。
    async fn fetch_return_item_trace(
        txn: &sea_orm::DatabaseTransaction,
        sales_order_id: Option<i32>,
        product_id: i32,
    ) -> Result<ReturnItemTrace, AppError> {
        let Some(order_id) = sales_order_id else {
            return Self::resolve_return_item_trace(None, None, sales_order_id, product_id);
        };

        // 优先级 1：该订单下所有发货单里该商品的出库行
        let delivery_ids: Vec<i32> = sales_delivery::Entity::find()
            .filter(sales_delivery::Column::OrderId.eq(order_id))
            .all(txn)
            .await?
            .into_iter()
            .map(|d| d.id)
            .collect();
        if !delivery_ids.is_empty() {
            let delivery_items = sales_delivery_item::Entity::find()
                .filter(sales_delivery_item::Column::DeliveryId.is_in(delivery_ids))
                .filter(sales_delivery_item::Column::ProductId.eq(product_id))
                .all(txn)
                .await?;
            match delivery_items.len() {
                0 => {}
                1 => return Ok(Self::trace_from_delivery_item(&delivery_items[0])),
                n => {
                    return Err(AppError::business(format!(
                        "退货明细追溯无法唯一定位：销售订单 {} 下商品 {} 存在 {} 条出库明细（多次发货或跨缸拆分），退货明细未携带原出库行引用无法判定回写哪一行。请补充出库行引用后重试",
                        order_id, product_id, n
                    )));
                }
            }
        }

        // 优先级 2：该订单该商品的订单明细
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .filter(sales_order_item::Column::ProductId.eq(product_id))
            .all(txn)
            .await?;
        match order_items.len() {
            0 => Self::resolve_return_item_trace(None, None, sales_order_id, product_id),
            1 => Ok(Self::trace_from_order_item(&order_items[0])),
            n => Err(AppError::business(format!(
                "退货明细追溯无法唯一定位：销售订单 {} 下商品 {} 存在 {} 条订单明细（同商品多色号），退货明细未携带订单行引用无法判定回写哪一行。请补充订单行引用后重试",
                order_id, product_id, n
            ))),
        }
    }

    /// 计算销售退货明细四个 NOT NULL 金额列。
    /// 算法与同族保持一致：采购退货 purchase_return_service::compute_item_amounts、
    /// 销售订单 so/order_crud.rs::calculate_sales_item_amounts——
    /// subtotal=qty*price；discount=subtotal*折扣率；taxable=subtotal-discount；
    /// tax=taxable*税率；total=taxable+tax，均 round_dp(2) 防精度漂移。
    fn compute_return_item_amounts(
        quantity: Decimal,
        unit_price: Decimal,
        discount_percent: Decimal,
        tax_percent: Decimal,
    ) -> ReturnItemAmounts {
        let subtotal = (quantity * unit_price).round_dp(2);
        let discount_amount = (subtotal * (discount_percent / Decimal::new(100, 0))).round_dp(2);
        let taxable_amount = (subtotal - discount_amount).round_dp(2);
        let tax_amount = (taxable_amount * (tax_percent / Decimal::new(100, 0))).round_dp(2);
        let total_amount = (taxable_amount + tax_amount).round_dp(2);
        ReturnItemAmounts {
            subtotal,
            discount_amount,
            tax_amount,
            total_amount,
        }
    }

    pub async fn update_return_totals(
        &self,
        return_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        use sea_orm::ColumnTrait;
        let items = crate::models::sales_return_item::Entity::find()
            .filter(crate::models::sales_return_item::Column::ReturnId.eq(return_id))
            .all(txn)
            .await?;

        let mut total = rust_decimal::Decimal::new(0, 0);
        for item in items {
            // Because sales_return_item might not have `amount`, we multiply quantity by a unit price or assume it's pre-calculated if the field exists.
            // Let's check what fields are actually in sales_return_item
            // Wait, sales_return_item doesn't have an `amount` field. We must use unit_price * quantity.
            let qty = item.quantity;
            let price = item.unit_price;
            // 批次 97 P1-7 修复（v5 复审）：金额累加补 round_dp(2) 防止精度漂移
            total += (qty * price).round_dp(2);
        }

        let return_order = crate::models::sales_return::Entity::find_by_id(return_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货单 {}", return_id)))?;

        let mut return_active: crate::models::sales_return::ActiveModel = return_order.into();
        return_active.total_amount = sea_orm::ActiveValue::Set(total);
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            return_active,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    // 生成退货单号
    // 格式：SR + 年月日 + 三位序号（SR20260315001）
    crate::impl_generate_no!(
        generate_return_no,
        "SR",
        sales_return::Entity,
        sales_return::Column::ReturnNo
    );

    /// 创建销售退货单
    pub async fn create_return(
        &self,
        req: CreateSalesReturnRequest,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_no = self.generate_return_no().await?;

        // 将 reason_type 和 reason_detail 组合成 reason 字段
        let reason = if let Some(detail) = &req.reason_detail {
            format!("{}: {}", req.reason_type, detail)
        } else {
            req.reason_type
        };

        let return_order = sales_return::ActiveModel {
            return_no: Set(return_no),
            sales_order_id: Set(req.order_id),
            customer_id: Set(req.customer_id),
            return_date: Set(req.return_date),
            warehouse_id: Set(req.warehouse_id),
            reason: Set(reason),
            status: Set(sr_status::DRAFT.to_string()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 添加退货明细项
    pub async fn add_return_item(
        &self,
        return_id: i32,
        req: CreateSalesReturnItemRequest,
        user_id: i32,
    ) -> Result<sales_return_item::Model, AppError> {
        // P1-6 修复（批次 79 v1 复审）：状态门 + insert 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门用 self.db 裸查询无锁、insert 用 txn，
        // 并发场景下可能在状态检查通过后、insert 之前发生 approve/submit 状态变更，
        // 导致已审批退货单被追加明细。
        let txn = (*self.db).begin().await?;

        // 验证退货单存在且为草稿状态（加 lock_exclusive 串行化并发状态变更）
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::DRAFT {
            return Err(AppError::business(format!(
                "退货单状态不允许添加明细，当前状态：{}",
                return_order.status
            )));
        }

        // line_no：请求传入或按现有明细数自增（NOT NULL 约束）
        let line_no = match req.line_no {
            Some(n) => n,
            None => {
                let existing = sales_return_item::Entity::find()
                    .filter(sales_return_item::Column::ReturnId.eq(return_id))
                    .all(&txn)
                    .await?;
                (existing.len() as i32) + 1
            }
        };
        // 税率权威来源解析（不做静默默认 0）：请求显式提供优先，否则取关联销售订单
        // 同商品明细的税率；两处都取不到则报错，暴露缺失字段由调用方/数据层修正。
        let order_item_tax = match return_order.sales_order_id {
            Some(order_id) => sales_order_item::Entity::find()
                .filter(sales_order_item::Column::OrderId.eq(order_id))
                .filter(sales_order_item::Column::ProductId.eq(req.product_id))
                .one(&txn)
                .await?
                .map(|oi| oi.tax_percent),
            None => None,
        };
        let tax_percent = Self::resolve_tax_percent(
            req.tax_percent,
            order_item_tax,
            return_order.sales_order_id,
            req.product_id,
        )?;
        // 折扣率缺省为 0（无折扣为合法业务默认），含税明细金额四元组按同族算法计算
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let amounts = Self::compute_return_item_amounts(
            req.quantity,
            req.unit_price,
            discount_percent,
            tax_percent,
        );
        // 追溯三列（色号/缸号/批次）从权威来源回写：取不到即业务错误，绝不落空串。
        let trace =
            Self::fetch_return_item_trace(&txn, return_order.sales_order_id, req.product_id)
                .await?;

        let item = sales_return_item::ActiveModel {
            return_id: Set(return_id),
            line_no: Set(line_no),
            product_id: Set(req.product_id),
            quantity: Set(req.quantity),
            unit_price: Set(req.unit_price),
            unit_price_foreign: Set(Decimal::ZERO),
            discount_percent: Set(discount_percent),
            tax_percent: Set(tax_percent),
            subtotal: Set(amounts.subtotal),
            tax_amount: Set(amounts.tax_amount),
            discount_amount: Set(amounts.discount_amount),
            total_amount: Set(amounts.total_amount),
            color_no: Set(trace.color_no),
            dye_lot_no: Set(trace.dye_lot_no),
            batch_no: Set(trace.batch_no),
            notes: Set(req.reason),
            quantity_alt: Set(Decimal::ZERO),
            ..Default::default()
        };

        let item = item.insert(&txn).await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 更新销售退货单
    pub async fn update_return(
        &self,
        return_id: i32,
        req: UpdateSalesReturnRequest,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // P1-7 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门用 self.db 裸查询、update_with_audit 也用 self.db，无事务边界，
        // 并发场景下可能在状态检查通过后、update 之前发生状态变更导致已审批单被篡改。
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::DRAFT {
            return Err(AppError::business(format!(
                "退货单状态不允许修改，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();

        if let Some(order_id) = req.order_id {
            active_model.sales_order_id = Set(Some(order_id));
        }
        if let Some(customer_id) = req.customer_id {
            active_model.customer_id = Set(customer_id);
        }
        if let Some(return_date) = req.return_date {
            active_model.return_date = Set(return_date);
        }
        if let Some(warehouse_id) = req.warehouse_id {
            active_model.warehouse_id = Set(warehouse_id);
        }
        if let Some(reason_type) = req.reason_type {
            let reason = if let Some(detail) = req.reason_detail {
                format!("{}: {}", reason_type, detail)
            } else {
                reason_type
            };
            active_model.reason = Set(reason);
        }
        if let Some(notes) = req.notes {
            active_model.remarks = Set(Some(notes));
        }

        active_model.updated_at = Set(Utc::now());
        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 提交销售退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 &*self.db 裸查询退货单状态，再 begin() 开启事务，
        // 并发 submit_return 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // 获取退货单（加 lock_exclusive 串行化并发状态变更）
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::DRAFT {
            return Err(AppError::business(format!(
                "退货单状态不允许提交，当前状态：{}",
                return_order.status
            )));
        }

        // 验证是否包含明细
        // 批次 27 v7 P1 修复：事务边界泄漏，原实现 count 用 &*self.db 裸查询
        // 存在 TOCTOU 风险（并发 submit + add_item 时计数读快照不一致，可绕过"明细非空"校验）
        let items_count = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .count(&txn)
            .await?;

        if items_count == 0 {
            return Err(AppError::business("退货单没有明细，无法提交".to_string()));
        }

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        // 更新状态为已提交
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set(sr_status::SUBMITTED.to_string());
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 审批销售退货单
    /// P2 1-5 修复：原函数 167 行混合 6 职责（lock+校验+总金额更新+批量库存入库+状态变更+AR 生成），；拆为 validate_and_lock_submitted_txn / apply_stock_inbound_txn / mark_approved_txn / generate_red_ar_txn 4 个私有方法
    pub async fn approve_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. lock_exclusive + 状态校验 + 获取明细
        let (return_order, items) = Self::validate_and_lock_submitted_txn(&txn, return_id).await?;

        // 2. 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(return_id, &txn, user_id).await?;

        // 3. 批量库存入库
        // 批次 358 v13 复审 B-P1-1 修复：接收待发布事件列表，commit 成功后统一 publish
        let pending_inventory_events = self
            .apply_stock_inbound_txn(&txn, &return_order, &items, user_id)
            .await?;

        // 4. 状态变更（APPROVED）
        let return_order = Self::mark_approved_txn(&txn, return_order, user_id).await?;

        // 5. P1 5-5/1-3 修复（批次 62）：红字应收单生成移入事务内，失败则整体回滚
        // 原实现在 commit 后调用 ar_invoice_service.create，但 create 强制 amount > 0，
        // 红字金额（负数）注定失败，且失败仅 tracing::error 不回滚，导致账实不符。
        // 改用 create_credit_memo（支持负金额 + 外部事务 + 幂等检查），在 commit 前调用。
        Self::generate_red_ar_txn(&self.db, &txn, &return_order, user_id).await?;

        txn.commit().await?;

        // 批次 358 v13 复审 B-P1-1 修复：commit 成功后统一 publish 库存流水事件，
        // 避免事务回滚时已发布事件造成的幻事件（订阅方库存财务桥接会基于不存在的流水生成凭证）
        for event in pending_inventory_events {
            EVENT_BUS.publish(event);
        }

        tracing::info!("成功自动生成红字应收单 (退货单 {})", return_order.return_no);

        Ok(return_order)
    }

    /// P2 1-5 修复：lock_exclusive + 状态校验 + 获取明细（从 approve_return 抽取）（批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更）
    async fn validate_and_lock_submitted_txn(
        txn: &sea_orm::DatabaseTransaction,
        return_id: i32,
    ) -> Result<(sales_return::Model, Vec<sales_return_item::Model>), AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::SUBMITTED {
            return Err(AppError::business(format!(
                "退货单状态不允许审批，当前状态：{}",
                return_order.status
            )));
        }

        // 获取明细记录
        let items = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .all(txn)
            .await?;

        Ok((return_order, items))
    }

    /// 批量获取商品信息和库存记录（优化N+1查询），返回 (product_map, stock_map)
    async fn load_stock_batch_maps(
        txn: &sea_orm::DatabaseTransaction,
        items: &[sales_return_item::Model],
        warehouse_id: i32,
    ) -> Result<
        (
            std::collections::HashMap<i32, product::Model>,
            std::collections::HashMap<
                (i32, String, String, Option<String>),
                inventory_stock::Model,
            >,
        ),
        AppError,
    > {
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.clone()))
            .all(txn)
            .await?;
        let product_map: std::collections::HashMap<i32, product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        // v14 批次 419 修复 T-P0-5：stock_map 改为四维索引 (product_id, color_no, batch_no, dye_lot_no)，
        // 避免同一产品多缸号库存时 HashMap 覆盖导致库存错配
        let stock_map: std::collections::HashMap<
            (i32, String, String, Option<String>),
            inventory_stock::Model,
        > = stocks
            .into_iter()
            .map(|s| {
                (
                    (
                        s.product_id,
                        s.color_no.clone(),
                        s.batch_no.clone(),
                        s.dye_lot_no.clone(),
                    ),
                    s,
                )
            })
            .collect();

        Ok((product_map, stock_map))
    }

    /// 处理单个退货明细的库存入库（更新/创建库存 + 记录流水）
    async fn process_inbound_item(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        item: &sales_return_item::Model,
        product_map: &std::collections::HashMap<i32, product::Model>,
        stock_map: &std::collections::HashMap<
            (i32, String, String, Option<String>),
            inventory_stock::Model,
        >,
        user_id: i32,
    ) -> Result<Option<BusinessEvent>, AppError> {
        // 获取商品信息
        let _product_info = product_map
            .get(&item.product_id)
            .ok_or_else(|| AppError::not_found(format!("商品 {} 不存在", item.product_id)))?;
        // v14 批次 419 修复 T-P0-5：从退货明细获取缸号/色号/批号，
        // 按四维 (product_id, color_no, batch_no, dye_lot_no) 查找匹配库存
        let item_color_no = item.color_no.clone();
        // 追溯字段不可空规范：sales_return_item.dye_lot_no 已 String 化，
        // 四维索引 key 的 dye_lot_no 仍为 Option<String>，此处包 Some
        let item_dye_lot_no = Some(item.dye_lot_no.clone());
        let item_batch_no = item.batch_no.clone();
        let stock = stock_map.get(&(
            item.product_id,
            item_color_no.clone(),
            item_batch_no.clone(),
            item_dye_lot_no.clone(),
        ));
        // grade 从库存获取，无库存时使用默认值 "A"
        let grade = stock
            .map(|s| s.grade.clone())
            .unwrap_or_else(|| String::from("A"));
        if let Some(s) = stock {
            Self::update_existing_stock_txn(txn, s, item, user_id).await?;
        } else {
            Self::create_new_stock_txn(
                txn,
                return_order,
                item,
                &item_color_no,
                &item_batch_no,
                &grade,
            )
            .await?;
        }
        // 批次 358 v13 复审 B-P1-1 修复：改用 record_transaction_txn 关联函数，
        // 流水写入与主事务同生共死，事件返回由调用方在 commit 后统一 publish
        let args = Self::build_inbound_txn_args(
            return_order,
            item,
            &item_color_no,
            &item_batch_no,
            &item_dye_lot_no,
            &grade,
            user_id,
        );
        let (_, txn_event) = InventoryStockService::record_transaction_txn(txn, args).await?;
        Ok(txn_event)
    }

    /// 退货入库：更新现有库存（quantity_on_hand + quantity_available + updated_at + 审计）
    async fn update_existing_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        stock: &inventory_stock::Model,
        item: &sales_return_item::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        let new_qty = stock.quantity_on_hand + item.quantity;
        let new_avail = stock.quantity_available + item.quantity;
        let mut stock_update: inventory_stock::ActiveModel = stock.clone().into();
        stock_update.quantity_on_hand = Set(new_qty);
        stock_update.quantity_available = Set(new_avail);
        stock_update.updated_at = Set(Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            stock_update,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 退货入库：创建新库存记录（使用退货明细的缸号/色号/批号）
    async fn create_new_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        item: &sales_return_item::Model,
        item_color_no: &str,
        item_batch_no: &str,
        grade: &str,
    ) -> Result<(), AppError> {
        let new_stock = inventory_stock::ActiveModel {
            warehouse_id: Set(return_order.warehouse_id),
            product_id: Set(item.product_id),
            batch_no: Set(item_batch_no.to_string()),
            color_no: Set(item_color_no.to_string()),
            grade: Set(grade.to_string()),
            quantity_on_hand: Set(item.quantity),
            quantity_available: Set(item.quantity),
            quantity_reserved: Set(Decimal::ZERO),
            version: Set(0),
            ..Default::default()
        };
        new_stock.insert(txn).await?;
        Ok(())
    }

    /// 退货入库：构造库存交易记录参数（流水写入与主事务同生共死）
    fn build_inbound_txn_args(
        return_order: &sales_return::Model,
        item: &sales_return_item::Model,
        item_color_no: &str,
        item_batch_no: &str,
        item_dye_lot_no: &Option<String>,
        grade: &str,
        user_id: i32,
    ) -> RecordTransactionArgs {
        RecordTransactionArgs {
            transaction_type: "SALES_RETURN".to_string(),
            product_id: item.product_id,
            warehouse_id: return_order.warehouse_id,
            batch_no: item_batch_no.to_string(),
            color_no: item_color_no.to_string(),
            dye_lot_no: item_dye_lot_no.clone(),
            grade: grade.to_string(),
            quantity_meters: item.quantity,
            quantity_kg: item.quantity_alt,
            source_bill_type: Some("SALES_RETURN".to_string()),
            source_bill_no: Some(return_order.return_no.clone()),
            source_bill_id: Some(return_order.id),
            quantity_before_meters: None,
            quantity_before_kg: None,
            quantity_after_meters: None,
            quantity_after_kg: None,
            notes: Some("销售退货入库".to_string()),
            created_by: Some(user_id),
        }
    }

    /// P2 1-5 修复：批量库存入库（从 approve_return 抽取）
    /// 改用 `InventoryStockService::record_transaction_txn(txn, ...)` 关联函数：流水写入与主事务同生共死，事件由调用方在 commit 成功后统一 publish。
    async fn apply_stock_inbound_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        items: &[sales_return_item::Model],
        user_id: i32,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        // 收集待发布事件，由调用方在 commit 成功后统一 publish
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        // 批量获取商品信息和库存记录（优化N+1查询）
        let (product_map, stock_map) =
            Self::load_stock_batch_maps(txn, items, return_order.warehouse_id).await?;

        for item in items {
            let txn_event = self
                .process_inbound_item(txn, return_order, item, &product_map, &stock_map, user_id)
                .await?;
            if let Some(ev) = txn_event {
                pending_events.push(ev);
            }
        }

        Ok(pending_events)
    }

    /// P2 1-5 修复：状态变更（APPROVED）（从 approve_return 抽取）
    async fn mark_approved_txn(
        txn: &sea_orm::DatabaseTransaction,
        return_order: sales_return::Model,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set(sr_status::APPROVED.to_string());
        active_model.approved_by = Set(Some(user_id));
        active_model.approved_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(return_order)
    }

    /// P2 1-5 修复：红字应收单生成（从 approve_return 抽取）
    /// P1 5-5/1-3 修复（批次 62）：红字应收单生成移入事务内，失败则整体回滚；使用 create_credit_memo（支持负金额 + 外部事务 + 幂等检查）
    async fn generate_red_ar_txn(
        db: &Arc<DatabaseConnection>,
        txn: &sea_orm::DatabaseTransaction,
        return_order: &sales_return::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        let ar_invoice_service = ArInvoiceService::new(db.clone());
        let invoice_date = Utc::now().date_naive();
        let due_date = invoice_date + chrono::Duration::days(30);
        let ar_request = CreateArInvoiceRequest {
            invoice_date: Some(invoice_date),
            due_date: Some(due_date),
            customer_id: Some(return_order.customer_id),
            customer_name: None,
            source_type: Some("SALES_RETURN".to_string()),
            source_bill_id: Some(return_order.id),
            source_bill_no: Some(return_order.return_no.clone()),
            invoice_amount: Some(-return_order.total_amount), // 红字应收单
            batch_no: None,
            color_no: None,
            sales_order_no: None,
        };

        ar_invoice_service
            .create_credit_memo(ar_request, user_id, txn)
            .await?;

        Ok(())
    }

    /// 获取退货单详情
    pub async fn get_return(
        &self,
        return_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<sales_return::Model, AppError> {
        let return_order = sales_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // sales_return 表 created_by 为 i32（非 Option），Dept 退化为 Self
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(return_order.created_by), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问销售退货单 {}（数据范围限制）",
                    return_id
                )));
            }
        }

        Ok(return_order)
    }

    /// 删除退货单
    // 批次 93 P1-7 修复：补 user_id 参数 + lock_exclusive + 状态门移入 txn + 审计 user_id
    pub async fn delete_return(&self, return_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-7 修复：状态门 + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 find_by_id 在 self.db → 状态门 → begin txn，
        // 状态门在事务外，并发 delete + submit 会竞态绕过 DRAFT 状态门控。
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        // 状态门在 txn 内，基于 lock_exclusive 读出的 model
        if return_order.status != sr_status::DRAFT {
            return Err(AppError::business(format!(
                "退货单状态不允许删除，当前状态：{}",
                return_order.status
            )));
        }

        // 先删除明细
        sales_return_item::Entity::delete_many()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .exec(&txn)
            .await?;

        // 再删除退货单（P0 8-3 修复：补审计日志；批次 93 P1-7：user_id 从 handler AuthContext 注入）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_return::Entity,
            _,
        >(&txn, "sales_return", return_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 拒绝退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发拒绝同一退货单导致状态不一致
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::SUBMITTED {
            return Err(AppError::business(format!(
                "退货单状态不允许拒绝，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set(sr_status::REJECTED.to_string());
        active_model.rejected_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 执行退货单（完成退货流程）
    pub async fn execute_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<sales_return::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发执行同一退货单导致状态不一致
        let txn = (*self.db).begin().await?;

        let return_order = sales_return::Entity::find_by_id(return_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售退货单 {}", return_id)))?;

        if return_order.status != sr_status::APPROVED {
            return Err(AppError::business(format!(
                "退货单状态不允许执行，当前状态：{}",
                return_order.status
            )));
        }

        let mut active_model: sales_return::ActiveModel = return_order.into();
        active_model.status = Set(sr_status::COMPLETED.to_string());
        active_model.updated_at = Set(Utc::now());

        let return_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 获取退货单明细列表
    pub async fn list_return_items(
        &self,
        return_id: i32,
    ) -> Result<Vec<sales_return_item::Model>, AppError> {
        let items = sales_return_item::Entity::find()
            .filter(sales_return_item::Column::ReturnId.eq(return_id))
            .order_by_asc(sales_return_item::Column::LineNo)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 更新退货单明细
    pub async fn update_return_item(
        &self,
        item_id: i32,
        quantity: Option<Decimal>,
        unit_price: Option<Decimal>,
        reason: Option<String>,
        user_id: i32,
    ) -> Result<sales_return_item::Model, AppError> {
        let item = sales_return_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        let txn = (*self.db).begin().await?;

        let mut active_model: sales_return_item::ActiveModel = item.into();
        if let Some(qty) = quantity {
            active_model.quantity = Set(qty);
        }
        if let Some(price) = unit_price {
            active_model.unit_price = Set(price);
        }
        if let Some(r) = reason {
            active_model.notes = Set(Some(r));
        }
        active_model.updated_at = Set(Utc::now());

        let item = active_model.insert(&txn).await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(item.return_id, &txn, user_id)
            .await?;

        txn.commit().await?;
        Ok(item)
    }

    /// 删除退货单明细
    // 批次 93 P1-8 修复：find + delete 移入同一事务，补 lock_exclusive 串行化并发
    pub async fn delete_return_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-8 修复：find 移入 txn + lock_exclusive，消除 TOCTOU 风险
        // 原实现 find_by_id 在 self.db → begin txn → delete_by_id 在 txn，
        // find 与 delete 跨事务边界，并发删除同一明细可能双写 / return_id 读取过期。
        let txn = (*self.db).begin().await?;

        let item = sales_return_item::Entity::find_by_id(item_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("退货明细 {}", item_id)))?;

        // 批次 94 P2-6 修复：用 delete_with_audit 记录审计日志（原 delete_by_id 无审计）
        // delete_with_audit 内部 find_by_id + delete + 写审计日志；行已被 lock_exclusive 锁定，重复查询安全
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            sales_return_item::Entity,
            _,
        >(&txn, "sales_return_item", item_id, Some(user_id))
        .await?;

        // 更新退货单总金额
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.update_return_totals(item.return_id, &txn, user_id)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 获取列表
    pub async fn list_returns(
        &self,
        return_no: Option<String>,
        status: Option<String>,
        customer_id: Option<i32>,
        page: u64,
        page_size: u64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<sales_return::Model>, u64), AppError> {
        let mut query = sales_return::Entity::find();

        // V15 P0-S01：行级数据权限过滤（sales_return 表 created_by 为 i32，Dept 退化为 Self）
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                sales_return::Column::CreatedBy,
                sales_return::Column::CreatedBy,
            );
        }

        if let Some(no) = return_no {
            query = query.filter(sales_return::Column::ReturnNo.contains(&no));
        }

        if let Some(s) = status {
            query = query.filter(sales_return::Column::Status.eq(s));
        }

        if let Some(id) = customer_id {
            query = query.filter(sales_return::Column::CustomerId.eq(id));
        }

        let paginator = query
            .order_by_desc(sales_return::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (items, total) = paginate_with_total(paginator, page).await?;

        Ok((items, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== compute_return_item_amounts：四个 NOT NULL 金额列的算法 =====
    // 依据同族口径：采购退货 compute_item_amounts、销售订单 calculate_sales_item_amounts。

    fn dec(s: &str) -> Decimal {
        s.parse::<Decimal>().expect("测试夹具：合法十进制字面量")
    }

    #[test]
    fn test_return_amounts_basic_tax_inclusive() {
        // 100m × 25.50 = 2550.00；税率 13%、无折扣 → tax=331.50、total=2881.50
        let a = SalesReturnService::compute_return_item_amounts(
            dec("100"),
            dec("25.50"),
            dec("0"),
            dec("13"),
        );
        assert_eq!(a.subtotal, dec("2550.00"));
        assert_eq!(a.discount_amount, dec("0.00"));
        assert_eq!(a.tax_amount, dec("331.50"));
        assert_eq!(a.total_amount, dec("2881.50"));
    }

    #[test]
    fn test_return_amounts_with_discount() {
        // 200 × 10 = 2000；折扣 5% → discount=100，taxable=1900；税 9% → tax=171；total=2071
        let a = SalesReturnService::compute_return_item_amounts(
            dec("200"),
            dec("10"),
            dec("5"),
            dec("9"),
        );
        assert_eq!(a.subtotal, dec("2000.00"));
        assert_eq!(a.discount_amount, dec("100.00"));
        assert_eq!(a.tax_amount, dec("171.00"));
        assert_eq!(a.total_amount, dec("2071.00"));
        // 守恒不变量：total = subtotal - discount + tax
        assert_eq!(
            a.total_amount,
            a.subtotal - a.discount_amount + a.tax_amount
        );
    }

    #[test]
    fn test_return_amounts_zero_tax_and_rounding() {
        // 税率 0（免税行合法）：total == subtotal；并验证 round_dp(2) 收敛
        // 7 × 3.333 = 23.331 → 23.33
        let a = SalesReturnService::compute_return_item_amounts(
            dec("7"),
            dec("3.333"),
            dec("0"),
            dec("0"),
        );
        assert_eq!(a.subtotal, dec("23.33"));
        assert_eq!(a.tax_amount, dec("0.00"));
        assert_eq!(a.total_amount, dec("23.33"));
    }

    // ===== resolve_tax_percent：权威来源解析（不静默默认 0）=====

    #[test]
    fn test_resolve_tax_percent_prefers_request() {
        let t =
            SalesReturnService::resolve_tax_percent(Some(dec("6")), Some(dec("13")), Some(1), 10)
                .unwrap();
        assert_eq!(t, dec("6"));
    }

    #[test]
    fn test_resolve_tax_percent_falls_back_to_order_item() {
        let t =
            SalesReturnService::resolve_tax_percent(None, Some(dec("13")), Some(1), 10).unwrap();
        assert_eq!(t, dec("13"));
    }

    #[test]
    fn test_resolve_tax_percent_errors_without_source() {
        // 无请求值 + 无订单税率（未关联或订单内无同商品明细）→ 业务错误，绝不默认 0
        let err = SalesReturnService::resolve_tax_percent(None, None, Some(7), 42).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        let msg = err.to_string();
        assert!(msg.contains("tax_percent"));
        assert!(msg.contains("42"));
    }

    // ===== 追溯三列回写：resolve_return_item_trace 三种路径 =====

    fn trace(color: &str, dye: &str, batch: &str) -> ReturnItemTrace {
        ReturnItemTrace {
            color_no: color.to_string(),
            dye_lot_no: dye.to_string(),
            batch_no: batch.to_string(),
        }
    }

    #[test]
    fn test_resolve_trace_prefers_delivery_item() {
        // 出库明细与订单明细同时存在时，出库明细（实际发货行）优先，字段值取出库行
        let delivery = trace("C-DEL", "DYE-DEL", "BATCH-DEL");
        let order = trace("C-ORD", "DYE-ORD", "BATCH-ORD");
        let t =
            SalesReturnService::resolve_return_item_trace(Some(delivery), Some(order), Some(9), 5)
                .unwrap();
        assert_eq!(t.color_no, "C-DEL");
        assert_eq!(t.dye_lot_no, "DYE-DEL");
        assert_eq!(t.batch_no, "BATCH-DEL");
    }

    #[test]
    fn test_resolve_trace_falls_back_to_order_item() {
        // 无出库明细时回落销售订单明细
        let order = trace("C-ORD", "DYE-ORD", "BATCH-ORD");
        let t =
            SalesReturnService::resolve_return_item_trace(None, Some(order), Some(9), 5).unwrap();
        assert_eq!(t.color_no, "C-ORD");
        assert_eq!(t.dye_lot_no, "DYE-ORD");
        assert_eq!(t.batch_no, "BATCH-ORD");
    }

    #[test]
    fn test_resolve_trace_errors_without_any_source() {
        // 两者皆无 + 已关联订单 → 业务错误，指向订单来源，不默认空串
        let err =
            SalesReturnService::resolve_return_item_trace(None, None, Some(11), 42).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        let msg = err.to_string();
        assert!(msg.contains("色号/缸号/批次"));
        assert!(msg.contains("11"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_resolve_trace_errors_without_order_link() {
        // 两者皆无 + 未关联订单 → 业务错误，引导补关联单据
        let err = SalesReturnService::resolve_return_item_trace(None, None, None, 42).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        let msg = err.to_string();
        assert!(msg.contains("未关联销售订单"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_trace_from_order_item_writes_blank_for_greige() {
        // 白坯布来源：订单行色号为空、要求字段为 NULL → 如实回写空串，不臆造
        let oi = sales_order_item::Model {
            color_no: String::new(),
            dye_lot_requirement: None,
            batch_requirement: None,
            ..test_order_item_base()
        };
        let t = SalesReturnService::trace_from_order_item(&oi);
        assert_eq!(t.color_no, "");
        assert_eq!(t.dye_lot_no, "");
        assert_eq!(t.batch_no, "");
    }

    #[test]
    fn test_trace_from_order_item_maps_real_values() {
        let oi = sales_order_item::Model {
            color_no: "RED-01".to_string(),
            dye_lot_requirement: Some("DYE-88".to_string()),
            batch_requirement: Some("B-77".to_string()),
            ..test_order_item_base()
        };
        let t = SalesReturnService::trace_from_order_item(&oi);
        assert_eq!(t.color_no, "RED-01");
        assert_eq!(t.dye_lot_no, "DYE-88");
        assert_eq!(t.batch_no, "B-77");
    }

    #[test]
    fn test_trace_from_delivery_item_maps_actual_shipped_dims() {
        let di = sales_delivery_item::Model {
            color_no: "BLU-02".to_string(),
            dye_lot_no: "DYE-100".to_string(),
            batch_no: "BATCH-200".to_string(),
            ..test_delivery_item_base()
        };
        let t = SalesReturnService::trace_from_delivery_item(&di);
        assert_eq!(t.color_no, "BLU-02");
        assert_eq!(t.dye_lot_no, "DYE-100");
        assert_eq!(t.batch_no, "BATCH-200");
    }

    // 追溯字段以 `..base` 方式覆盖，其余字段仅为满足结构体完整性
    fn test_order_item_base() -> sales_order_item::Model {
        let now = chrono::Utc::now();
        sales_order_item::Model {
            id: 1,
            order_id: 1,
            product_id: 5,
            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,
            discount_percent: Decimal::ZERO,
            tax_percent: Decimal::ZERO,
            subtotal: Decimal::ZERO,
            tax_amount: Decimal::ZERO,
            discount_amount: Decimal::ZERO,
            total_amount: Decimal::ZERO,
            shipped_quantity: Decimal::ZERO,
            notes: None,
            created_at: now,
            updated_at: now,
            color_no: String::new(),
            color_name: None,
            pantone_code: None,
            grade_required: None,
            quantity_meters: Decimal::ZERO,
            quantity_kg: Decimal::ZERO,
            gram_weight: None,
            width: None,
            batch_requirement: None,
            dye_lot_requirement: None,
            piece_no: None,
            base_price: None,
            color_extra_cost: Decimal::ZERO,
            grade_price_diff: Decimal::ZERO,
            final_price: None,
            shipped_quantity_meters: Decimal::ZERO,
            shipped_quantity_kg: Decimal::ZERO,
            paper_tube_weight: None,
            is_net_weight: None,
        }
    }

    fn test_delivery_item_base() -> sales_delivery_item::Model {
        sales_delivery_item::Model {
            id: 1,
            delivery_id: 1,
            product_id: 5,
            batch_no: String::new(),
            color_no: String::new(),
            dye_lot_id: None,
            dye_lot_no: String::new(),
            piece_no: None,
            stock_id: None,
            is_cross_dye_lot: false,
            quantity: Decimal::ZERO,
            unit_price: Decimal::ZERO,
            amount: Decimal::ZERO,
            remarks: None,
            created_at: chrono::Utc::now(),
        }
    }
}
