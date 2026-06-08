//! 销售发货服务（so/delivery）
//!
//! 包含销售订单的发货、库存扣减/释放、订单号生成等。
//! 拆分自原 `sales_service.rs`。
//! 由于 `check_inventory`、`lock_inventory`、`reduce_inventory`、`release_reservations`
//! 这四个方法与发货/库存操作紧密相关，统一在 delivery.rs 中实现。

use crate::models::{
    inventory_reservation, inventory_stock, product, sales_delivery, sales_delivery_item,
    sales_order, sales_order_item, warehouse,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::sea_query::ExprTrait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

use super::order::SalesService;

// =====================================================
// 发货请求 DTO
// =====================================================

#[derive(Debug, Validate, Deserialize)]
pub struct ShipOrderRequest {
    #[validate(range(min = 1, message = "订单ID必须大于0"))]
    pub order_id: i32,
    #[validate(length(max = 50, message = "仓库编号长度不能超过50个字符"))]
    pub warehouse_code: String,
    pub items: Vec<ShipOrderItemRequest>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct ShipOrderItemRequest {
    pub product_id: i32,
    pub quantity: Decimal,
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,
}

// =====================================================
// 销售订单服务 impl 块
// =====================================================

impl SalesService {
    // 生成销售订单号
    // 格式：SO + 年月日 + 三位序号（SO20260315001）
    crate::impl_generate_no!(
        generate_order_no,
        "SO",
        sales_order::Entity,
        sales_order::Column::OrderNo
    );

    /// 销售订单发货
    pub async fn ship_order(
        &self,
        request: ShipOrderRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 检查订单状态
        let order = sales_order::Entity::find_by_id(request.order_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "approved" {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }

        // 查询订单明细
        let _order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(&txn)
            .await?;

        // 查询仓库
        let warehouse = warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(&request.warehouse_code))
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("仓库不存在"))?;

        // 检查库存是否充足
        self.check_inventory(request.order_id, &request.items, &txn)
            .await?;

        // 创建发货单
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            delivery_no: Set(format!("DN{}", chrono::Utc::now().format("%Y%m%d%H%M%S"))),
            order_id: Set(request.order_id),
            customer_id: Set(order.customer_id),
            warehouse_id: Set(warehouse.id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set("shipped".to_string()),
            total_quantity: Set(request.items.iter().map(|i| i.quantity).sum()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(request.remarks),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let delivery = delivery.insert(&txn).await?;

        // 创建发货单明细并扣减库存
        for item in request.items {
            // 创建发货明细
            let delivery_item = sales_delivery_item::ActiveModel {
                id: Default::default(),
                delivery_id: Set(delivery.id),
                product_id: Set(item.product_id),
                quantity: Set(item.quantity),
                batch_no: Set(item.batch_no),
                color_no: Set(None),
                remarks: Set(None),
                unit_price: Set(Decimal::ZERO),
                amount: Set(Decimal::ZERO),
                created_at: Set(chrono::Utc::now()),
            };
            delivery_item.insert(&txn).await?;

            // 扣减库存
            self.reduce_inventory(
                item.product_id,
                warehouse.id,
                item.quantity,
                request.order_id,
                &txn,
            )
            .await?;

            // 使用 update_many 批量更新订单明细已发货数量
            sales_order_item::Entity::update_many()
                .filter(sales_order_item::Column::OrderId.eq(request.order_id))
                .filter(sales_order_item::Column::ProductId.eq(item.product_id))
                .col_expr(
                    sales_order_item::Column::ShippedQuantity,
                    sea_orm::sea_query::Expr::col(sales_order_item::Column::ShippedQuantity)
                        .add(item.quantity),
                )
                .col_expr(
                    sales_order_item::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                )
                .exec(&txn)
                .await?;
        }

        // 更新订单状态
        let order_items_total: Vec<sales_order_item::Model> = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(&txn)
            .await?;

        let mut is_fully_shipped = true;
        for oi in &order_items_total {
            if oi.shipped_quantity < oi.quantity {
                is_fully_shipped = false;
                break;
            }
        }

        let new_status = if is_fully_shipped {
            "shipped"
        } else {
            "partial_shipped"
        };

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set(new_status.to_string());
        order_update.ship_date = Set(Some(chrono::Utc::now()));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;
        Ok(())
    }

    /// 获取订单发货记录
    pub async fn get_order_deliveries(
        &self,
        order_id: i32,
    ) -> Result<Vec<sales_delivery::Model>, AppError> {
        let deliveries = sales_delivery::Entity::find()
            .filter(sales_delivery::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await?;
        Ok(deliveries)
    }

    /// 创建发货单（手动创建）
    pub async fn create_delivery(
        &self,
        order_id: i32,
        warehouse_id: i32,
        user_id: i32,
    ) -> Result<sales_delivery::Model, AppError> {
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            delivery_no: Set(format!("DN{}", chrono::Utc::now().format("%Y%m%d%H%M%S"))),
            order_id: Set(order_id),
            customer_id: Set(0),
            warehouse_id: Set(warehouse_id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set("pending".to_string()),
            total_quantity: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(None),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let delivery = delivery.insert(&*self.db).await?;
        Ok(delivery)
    }

    // ========== 库存辅助方法（私有） ==========

    /// 检查库存是否充足
    pub(crate) async fn check_inventory(
        &self,
        order_id: i32,
        items: &[ShipOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        for item in items {
            // 优先从预留记录查询
            let reservation = inventory_reservation::Entity::find()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                .filter(inventory_reservation::Column::Status.eq("pending"))
                .one(txn)
                .await?;

            if let Some(res) = reservation {
                if res.quantity < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 预留数量 {} 小于发货数量 {}",
                        item.product_id, res.quantity, item.quantity
                    )));
                }
                continue;
            }

            // 没有预留记录时直接查询库存
            let stock = inventory_stock::Entity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 库存 {} 小于发货数量 {}",
                        item.product_id, s.quantity_available, item.quantity
                    )));
                }
            } else {
                return Err(AppError::business(format!(
                    "产品 {} 库存不存在",
                    item.product_id
                )));
            }
        }
        Ok(())
    }

    /// 锁定库存（创建预留记录）
    pub(crate) async fn lock_inventory(
        &self,
        order_id: i32,
        items: &[super::SalesOrderItemRequest],
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        for item in items {
            let existing_reservation = inventory_reservation::Entity::find()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                .filter(inventory_reservation::Column::Status.eq("pending"))
                .one(txn)
                .await?;

            if existing_reservation.is_some() {
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }

            let stock = inventory_stock::Entity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足，无法锁定",
                        item.product_id
                    )));
                }

                let reservation = inventory_reservation::ActiveModel {
                    id: Default::default(),
                    order_id: Set(order_id),
                    product_id: Set(item.product_id),
                    warehouse_id: Set(s.warehouse_id),
                    quantity: Set(item.quantity),
                    status: Set("pending".to_string()),
                    reserved_at: Set(chrono::Utc::now()),
                    released_at: Set(None),
                    notes: Set(None),
                    created_by: Set(Some(user_id)),
                    created_at: Set(chrono::Utc::now()),
                    updated_at: Set(chrono::Utc::now()),
                };
                reservation.insert(txn).await?;

                inventory_stock::Entity::update_many()
                    .filter(inventory_stock::Column::Id.eq(s.id))
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                            .sub(item.quantity),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                    )
                    .exec(txn)
                    .await?;
            } else {
                return Err(AppError::business(format!(
                    "产品 {} 没有库存记录，无法锁定",
                    item.product_id
                )));
            }
        }
        Ok(())
    }

    /// 扣减库存
    pub(crate) async fn reduce_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        if stock.quantity_available < quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock.quantity_available, quantity
            )));
        }

        inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        // 标记预留为已完成
        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.eq(product_id))
            .filter(inventory_reservation::Column::Status.eq("pending"))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val("consumed".to_string()),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        Ok(())
    }

    /// 释放订单的库存预留记录
    pub(crate) async fn release_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq("pending"))
            .all(txn)
            .await?;

        for res in reservations {
            inventory_stock::Entity::update_many()
                .filter(inventory_stock::Column::ProductId.eq(res.product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(res.warehouse_id))
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                        .add(res.quantity),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                )
                .exec(txn)
                .await?;
        }

        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq("pending"))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val("cancelled".to_string()),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
            )
            .exec(txn)
            .await?;

        Ok(())
    }

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
            .list_orders(page_req, status, customer_id, order_no)
            .await?;

        let headers = vec![
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
        ];

        let rows: Vec<std::collections::HashMap<String, String>> = orders
            .items
            .into_iter()
            .map(|o| {
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
            })
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::business(format!("CSV 生成失败: {}", e)))
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

// 解决未使用导入告警
#[allow(dead_code)]
type _ProductModel = product::Model;
