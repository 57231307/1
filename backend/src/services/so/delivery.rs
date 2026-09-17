//! 销售发货服务（facade，批次 488 D10-3 拆分）
//!
//! 本文件为 facade 入口，仅保留 DTO + `validate_dye_lot_consistency` + 小 impl 块
//! （订单号生成、发货记录查询、手动创建发货单）+ 单元测试。
//! 业务实现已按职责拆分到 `delivery_ops/` 子模块（与 `delivery` 同为 `crate::services::so` 下兄弟模块）：
//! - `delivery_ops::ship`：发货管理（ship_order 及 15 个辅助方法，原 L126-694）
//! - `delivery_ops::inventory`：库存辅助（check_inventory/lock_inventory/reduce_inventory/release_reservations，原 L747-1082）
//! - `delivery_ops::cancel`：取消发货（cancel_delivery 及 3 个辅助方法，原 L1084-1320）
//! - `delivery_ops::export`：CSV 导出（export_orders_to_csv 及 2 个辅助方法，原 L1322-1443）
//! - `delivery_ops::types`：内部聚合辅助 struct（ShipOrderContext/ShipmentItemsResult/ShipPostCommitContext）
//!
//! 设计要点（与拆分前一致）：
//! - 包含销售订单的发货、库存扣减/释放、订单号生成等
//! - `check_inventory`、`lock_inventory`、`reduce_inventory`、`release_reservations`
//!   这四个方法与发货/库存操作紧密相关，统一在 delivery_ops::inventory 中实现
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::so::delivery::ShipOrderRequest` 引用，路径不变
//! - `SalesService` struct 定义在 `crate::services::so::order`，impl 块分散到 delivery_ops 子模块
//! - impl 块分散在 delivery_ops 子模块，Rust 允许同一 crate 多文件多 impl 块

use crate::models::status::sales_delivery as delivery_status;
use crate::models::{sales_delivery, sales_order};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
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
    // v14 批次 421 T-P1-5：缸号同订单校验支持字段
    // 依据：fabric-industry-research.md §2.3 约束 5 - 同一订单同面料必须使用相同缸号
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    /// 染色匹号（匹号领域：出库使用染色匹号）
    pub piece_no: Option<String>,
}

// =====================================================
// 销售订单服务 impl 块
// =====================================================

/// v14 批次 421 T-P1-5：缸号同订单校验
/// 缸号一致性提示（2026-09-17 业务规则更新：同缸面料出完后允许使用其他缸面料）
/// 依据：fabric-industry-research.md §2.3 约束 5 + 用户业务指令（2026-09-17）
/// 业务规则演变：原"同一订单同面料必须同缸"硬性校验过严——同缸库存出完后
/// 应允许使用其他缸面料继续供货（有得发优先于色差零容忍）；
/// 色差风险保留为警告提示（裁床铺布时同床尽量同缸，多缸需分开裁剪）。
/// 校验逻辑：同 product_id 出现多个 dye_lot_no 时记录警告日志（提示色差风险），
/// 不再阻断发货；dye_lot_no 均为 None 跳过；单缸或空 → 正常通过
pub fn validate_dye_lot_consistency(items: &[ShipOrderItemRequest]) -> Result<(), AppError> {
    use std::collections::HashMap;

    // 按 product_id 分组收集 dye_lot_no
    let mut product_dye_lots: HashMap<i32, std::collections::HashSet<String>> = HashMap::new();
    for item in items {
        if let Some(dye_lot_no) = &item.dye_lot_no {
            if !dye_lot_no.is_empty() {
                product_dye_lots
                    .entry(item.product_id)
                    .or_default()
                    .insert(dye_lot_no.clone());
            }
        }
    }

    // 多缸号 → 警告（不阻断）：同缸出完允许换缸，提示裁床分缸裁剪避免色差
    for (product_id, dye_lots) in &product_dye_lots {
        if dye_lots.len() > 1 {
            let dye_lot_list: Vec<String> = dye_lots.iter().cloned().collect();
            tracing::warn!(
                product_id = %product_id,
                dye_lots = %dye_lot_list.join("/"),
                "【缸号提示】同一订单同产品使用多个缸号：同缸面料已出完或分配多缸供货，裁床请分缸裁剪避免色差"
            );
        }
    }

    Ok(())
}

impl SalesService {
    // 生成销售订单号
    // 格式：SO + 年月日 + 三位序号（SO20260315001）
    crate::impl_generate_no!(
        generate_order_no,
        "SO",
        sales_order::Entity,
        sales_order::Column::OrderNo
    );

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
        // P1 3-8 修复（批次 60）：包裹事务，确保单号生成的 advisory_xact_lock
        // 与 INSERT 在同一事务内，锁覆盖完整临界区
        let txn = (*self.db).begin().await?;
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            // P1 3-8 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
            delivery_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    &txn,
                    "DN",
                    sales_delivery::Entity,
                    sales_delivery::Column::DeliveryNo,
                )
                .await?,
            ),
            order_id: Set(order_id),
            customer_id: Set(0),
            warehouse_id: Set(warehouse_id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set(delivery_status::PENDING.to_string()),
            total_quantity: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(None),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let delivery = delivery.insert(&txn).await?;
        txn.commit().await?;
        Ok(delivery)
    }
}
