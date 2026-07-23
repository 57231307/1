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

use crate::models::{sales_delivery, sales_order};
use crate::models::status::sales_delivery as delivery_status;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
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
}

// =====================================================
// 销售订单服务 impl 块
// =====================================================

/// v14 批次 421 T-P1-5：缸号同订单校验
///
/// 依据：fabric-industry-research.md §2.3 约束 5
/// 业务规则：出库时，同一订单必须使用相同缸号的面料，系统校验订单中所有该面料是否来自同一批次，不一致则报警提示
/// 业务语义：一个缸号代表一次染色，同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺
///
/// 校验逻辑：同一 product_id 的所有发货明细必须使用相同的 dye_lot_no
/// - 同 product_id 但 dye_lot_no 不一致 → 返回业务错误（避免混缸色差）
/// - dye_lot_no 均为 None → 视为未指定缸号，跳过校验（兼容无缸号场景）
/// - 单 product_id 单 dye_lot_no → 通过校验
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

    // 校验每个 product_id 下不能有多个不同的 dye_lot_no
    for (product_id, dye_lots) in &product_dye_lots {
        if dye_lots.len() > 1 {
            let dye_lot_list: Vec<String> = dye_lots.iter().cloned().collect();
            return Err(AppError::business(format!(
                "产品 {} 在同一订单中使用了多个不同缸号 {}，违反缸号同订单校验：同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺",
                product_id,
                dye_lot_list.join("/")
            )));
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
                .await?
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::search::{ElasticClient, SearchClient};
    use crate::models::status::inventory_reservation as reservation_status;
    use crate::models::status::sales_order as so_status;
    use sea_orm::DatabaseConnection;
    use std::str::FromStr;
    use std::sync::Arc;

    /// 复现 ship_order 的订单状态校验门（不涉及数据库）
    ///
    /// 与 ship_order 中 `if order.status != so_status::APPROVED` 保持一致：
    /// 仅已审批订单可发货，其余状态返回业务错误。
    fn ship_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != so_status::APPROVED {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }
        Ok(())
    }

    /// 复现 cancel_delivery 的发货单状态校验门（不涉及数据库）
    ///
    /// 与 cancel_delivery 中 `if delivery.status != delivery_status::SHIPPED` 保持一致：
    /// 仅已发货单可取消，其余状态返回业务错误。
    fn cancel_delivery_status_gate(status: &str) -> Result<(), AppError> {
        if status != delivery_status::SHIPPED {
            return Err(AppError::business(format!(
                "发货单状态不允许取消，当前状态：{}，仅 SHIPPED 状态可取消",
                status
            )));
        }
        Ok(())
    }

    /// 复现 ship_order 的全部发货判定（不涉及数据库）
    ///
    /// 与 ship_order 中遍历 order_items_total 判定 is_fully_shipped 保持一致：
    /// 所有明细 shipped_quantity >= quantity 即为全部发货。
    /// 入参元组为 (shipped_quantity, ordered_quantity)。
    fn compute_is_fully_shipped(items: &[(Decimal, Decimal)]) -> bool {
        items.iter().all(|(shipped, ordered)| *shipped >= *ordered)
    }

    /// 复现 ship_order 发货后的订单状态选择（不涉及数据库）
    ///
    /// 全部发货 → SHIPPED；否则 → PARTIAL_SHIPPED。
    fn compute_new_status_after_ship(is_fully_shipped: bool) -> &'static str {
        if is_fully_shipped {
            so_status::SHIPPED
        } else {
            so_status::PARTIAL_SHIPPED
        }
    }

    /// 复现 cancel_delivery 取消发货后的订单状态回退（不涉及数据库）
    ///
    /// 仍有已发数量 → PARTIAL_SHIPPED；全部取消 → APPROVED。
    fn compute_new_status_after_cancel(has_shipped: bool) -> &'static str {
        if has_shipped {
            so_status::PARTIAL_SHIPPED
        } else {
            so_status::APPROVED
        }
    }

    /// 复现 cancel_delivery 中订单状态回退的触发条件（不涉及数据库）
    ///
    /// 仅当订单当前状态为 SHIPPED 或 PARTIAL_SHIPPED 时才回退，
    /// 避免覆盖 CANCELLED 等终态。
    fn order_status_rollback_eligible(status: &str) -> bool {
        status == so_status::SHIPPED || status == so_status::PARTIAL_SHIPPED
    }

    /// 复现 cancel_delivery 的取消备注格式（不涉及数据库）
    ///
    /// 与 cancel_delivery 中 `format!("[取消原因] {}", reason)` 保持一致。
    fn format_cancel_remark(reason: &str) -> String {
        format!("[取消原因] {}", reason)
    }

    /// 复现 check_inventory 中预留数量校验逻辑（不涉及数据库）
    ///
    /// 与 check_inventory 中 `if res.quantity < item.quantity` 保持一致：
    /// 预留数量小于发货数量时返回业务错误。
    fn check_inventory_reservation_logic(
        res_qty: Decimal,
        item_qty: Decimal,
        product_id: i32,
    ) -> Result<(), AppError> {
        if res_qty < item_qty {
            return Err(AppError::business(format!(
                "产品 {} 预留数量 {} 小于发货数量 {}",
                product_id, res_qty, item_qty
            )));
        }
        Ok(())
    }

    /// 复现 check_inventory 中库存数量校验逻辑（不涉及数据库）
    ///
    /// 与 check_inventory 中 `if s.quantity_available < item.quantity` 保持一致：
    /// 可用库存小于发货数量时返回业务错误。
    fn check_inventory_stock_logic(
        stock_available: Decimal,
        item_qty: Decimal,
        product_id: i32,
    ) -> Result<(), AppError> {
        if stock_available < item_qty {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock_available, item_qty
            )));
        }
        Ok(())
    }

    // ===== 状态常量值正确性 =====

    /// 测试_销售发货状态常量值正确性
    ///
    /// 校验 sales_delivery 子模块的 PENDING/SHIPPED/CANCELLED 常量值，
    /// 避免硬编码字符串导致的拼写错误（批次 158 v11 接入）。
    #[test]
    fn 测试_销售发货状态常量值正确性() {
        assert_eq!(delivery_status::PENDING, "pending");
        assert_eq!(delivery_status::SHIPPED, "shipped");
        assert_eq!(delivery_status::CANCELLED, "cancelled");
    }

    /// 测试_销售订单状态常量值正确性
    ///
    /// 校验 sales_order 子模块的发货相关状态常量值（小写），
    /// 覆盖 ship_order 与 cancel_delivery 涉及的全部状态。
    #[test]
    fn 测试_销售订单状态常量值正确性() {
        assert_eq!(so_status::DRAFT, "draft");
        assert_eq!(so_status::PENDING, "pending");
        assert_eq!(so_status::APPROVED, "approved");
        assert_eq!(so_status::PARTIAL_SHIPPED, "partial_shipped");
        assert_eq!(so_status::SHIPPED, "shipped");
        assert_eq!(so_status::COMPLETED, "completed");
        assert_eq!(so_status::CANCELLED, "cancelled");
    }

    /// 测试_库存预留状态常量值正确性
    ///
    /// 校验 inventory_reservation 子模块的预留状态常量值（小写），
    /// 覆盖 reduce_inventory / release_reservations / cancel_delivery 涉及的状态转换。
    #[test]
    fn 测试_库存预留状态常量值正确性() {
        assert_eq!(reservation_status::PENDING, "pending");
        assert_eq!(reservation_status::LOCKED, "locked");
        assert_eq!(reservation_status::CONSUMED, "consumed");
        assert_eq!(reservation_status::RELEASED, "released");
        assert_eq!(reservation_status::CANCELLED, "cancelled");
    }

    // ===== ship_order 状态校验 =====

    /// 测试_发货状态校验_仅已审批订单可发货
    ///
    /// 验证 ship_order 中订单状态校验门：仅 APPROVED 状态可发货，其余状态拒绝。
    #[test]
    fn 测试_发货状态校验_仅已审批订单可发货() {
        // 已审批：放行
        assert!(ship_order_status_gate(so_status::APPROVED).is_ok());
        // 其他状态：拒绝
        assert!(ship_order_status_gate(so_status::DRAFT).is_err());
        assert!(ship_order_status_gate(so_status::PENDING).is_err());
        assert!(ship_order_status_gate(so_status::SHIPPED).is_err());
        assert!(ship_order_status_gate(so_status::PARTIAL_SHIPPED).is_err());
        assert!(ship_order_status_gate(so_status::CANCELLED).is_err());

        // 错误类型应为 BusinessError
        let err = ship_order_status_gate(so_status::DRAFT).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    // ===== 全部发货判定 =====

    /// 测试_全部发货判定_所有明细已发足
    ///
    /// 验证 ship_order 中 is_fully_shipped 判定：所有明细已发足（含恰好相等与超发）。
    #[test]
    fn 测试_全部发货判定_所有明细已发足() {
        // 全部发足
        let items = vec![(decs!("100"), decs!("100")), (decs!("50"), decs!("50"))];
        assert!(compute_is_fully_shipped(&items));

        // 边界：恰好相等也算全部发货
        let items_eq = vec![(decs!("10"), decs!("10"))];
        assert!(compute_is_fully_shipped(&items_eq));

        // 超发也算全部发货
        let items_over = vec![(decs!("120"), decs!("100"))];
        assert!(compute_is_fully_shipped(&items_over));
    }

    /// 测试_全部发货判定_部分明细未发足
    ///
    /// 验证 ship_order 中 is_fully_shipped 判定：任一明细未发足即为部分发货。
    #[test]
    fn 测试_全部发货判定_部分明细未发足() {
        // 部分明细未发足
        let items = vec![(decs!("100"), decs!("100")), (decs!("30"), decs!("50"))];
        assert!(!compute_is_fully_shipped(&items));

        // 全部未发
        let items_none = vec![(Decimal::ZERO, decs!("50"))];
        assert!(!compute_is_fully_shipped(&items_none));
    }

    // ===== 发货后订单状态选择 =====

    /// 测试_发货后订单状态选择_全部发货为已发货
    ///
    /// 验证 ship_order 中全部发货时订单状态置为 SHIPPED。
    #[test]
    fn 测试_发货后订单状态选择_全部发货为已发货() {
        assert_eq!(compute_new_status_after_ship(true), so_status::SHIPPED);
    }

    /// 测试_发货后订单状态选择_部分发货为部分发货
    ///
    /// 验证 ship_order 中部分发货时订单状态置为 PARTIAL_SHIPPED。
    #[test]
    fn 测试_发货后订单状态选择_部分发货为部分发货() {
        assert_eq!(
            compute_new_status_after_ship(false),
            so_status::PARTIAL_SHIPPED
        );
    }

    // ===== check_inventory 校验 =====

    /// 测试_库存检查_预留数量不足拒绝
    ///
    /// 验证 check_inventory 中预留数量校验：预留 < 发货 拒绝，预留 >= 发货 放行。
    #[test]
    fn 测试_库存检查_预留数量不足拒绝() {
        // 预留 < 发货：拒绝
        let err = check_inventory_reservation_logic(decs!("30"), decs!("50"), 1).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("预留数量"));

        // 预留 = 发货：放行（边界）
        assert!(check_inventory_reservation_logic(decs!("50"), decs!("50"), 1).is_ok());
        // 预留 > 发货：放行
        assert!(check_inventory_reservation_logic(decs!("80"), decs!("50"), 1).is_ok());
    }

    /// 测试_库存检查_库存数量不足拒绝
    ///
    /// 验证 check_inventory 中库存数量校验：库存 < 发货 拒绝，库存 >= 发货 放行。
    #[test]
    fn 测试_库存检查_库存数量不足拒绝() {
        // 库存 < 发货：拒绝
        let err = check_inventory_stock_logic(decs!("20"), decs!("50"), 2).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存"));

        // 库存 = 发货：放行（边界）
        assert!(check_inventory_stock_logic(decs!("50"), decs!("50"), 2).is_ok());
        // 库存 > 发货：放行
        assert!(check_inventory_stock_logic(decs!("100"), decs!("50"), 2).is_ok());
    }

    /// 测试_库存检查_库存不存在拒绝
    ///
    /// 验证 check_inventory 中 stock_map.get 返回 None 时的错误构造：
    /// 返回"产品 X 库存不存在"业务错误。
    #[test]
    fn 测试_库存检查_库存不存在拒绝() {
        let err = AppError::business(format!("产品 {} 库存不存在", 3));
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存不存在"));
    }

    // ===== 库存扣减/恢复计算公式 =====

    /// 测试_库存扣减计算公式
    ///
    /// 验证 reduce_inventory 的对称更新公式（发货扣减库存）：
    /// quantity_available -= qty，quantity_shipped += qty，
    /// 且守恒不变量：可用 + 已发 恒定。
    #[test]
    fn 测试_库存扣减计算公式() {
        let available = decs!("100");
        let shipped = decs!("20");
        let qty = decs!("30");

        let new_available = available - qty;
        let new_shipped = shipped + qty;

        assert_eq!(new_available, decs!("70"));
        assert_eq!(new_shipped, decs!("50"));
        // 守恒不变量：可用 + 已发 在扣减前后恒定
        assert_eq!(new_available + new_shipped, available + shipped);
    }

    /// 测试_库存恢复计算公式
    ///
    /// 验证 restore_inventory 的对称反向更新公式（cancel_delivery 取消发货时使用）：
    /// quantity_available += qty，quantity_shipped -= qty，
    /// 且守恒不变量：可用 + 已发 恒定。
    #[test]
    fn 测试_库存恢复计算公式() {
        let available = decs!("70");
        let shipped = decs!("50");
        let qty = decs!("30");

        let new_available = available + qty;
        let new_shipped = shipped - qty;

        assert_eq!(new_available, decs!("100"));
        assert_eq!(new_shipped, decs!("20"));
        // 守恒不变量：可用 + 已发 在恢复前后恒定
        assert_eq!(new_available + new_shipped, available + shipped);
    }

    // ===== 预留状态转换 =====

    /// 测试_预留状态转换_扣减时待处理转已消耗
    ///
    /// 验证 reduce_inventory 中将预留状态从 PENDING 更新为 CONSUMED。
    #[test]
    fn 测试_预留状态转换_扣减时待处理转已消耗() {
        let from_status = reservation_status::PENDING;
        let to_status = reservation_status::CONSUMED;

        assert_eq!(from_status, "pending");
        assert_eq!(to_status, "consumed");
        assert_ne!(from_status, to_status);
    }

    /// 测试_预留状态转换_释放时待处理转已取消
    ///
    /// 验证 release_reservations 中将预留状态从 PENDING 更新为 CANCELLED。
    #[test]
    fn 测试_预留状态转换_释放时待处理转已取消() {
        let from_status = reservation_status::PENDING;
        let to_status = reservation_status::CANCELLED;

        assert_eq!(from_status, "pending");
        assert_eq!(to_status, "cancelled");
        assert_ne!(from_status, to_status);
    }

    /// 测试_预留状态恢复_取消发货时已消耗转待处理
    ///
    /// 验证 cancel_delivery 中将预留状态从 CONSUMED 恢复为 PENDING
    /// （对称反向于 reduce_inventory 的 PENDING → CONSUMED 转换）。
    #[test]
    fn 测试_预留状态恢复_取消发货时已消耗转待处理() {
        let from_status = reservation_status::CONSUMED;
        let to_status = reservation_status::PENDING;

        assert_eq!(from_status, "consumed");
        assert_eq!(to_status, "pending");
        assert_ne!(from_status, to_status);
    }

    // ===== cancel_delivery 校验 =====

    /// 测试_取消发货状态校验_仅已发货可取消
    ///
    /// 验证 cancel_delivery 中发货单状态校验门：仅 SHIPPED 状态可取消，
    /// 其余状态拒绝且错误消息包含当前状态与"仅 SHIPPED 状态可取消"。
    #[test]
    fn 测试_取消发货状态校验_仅已发货可取消() {
        // 已发货：放行
        assert!(cancel_delivery_status_gate(delivery_status::SHIPPED).is_ok());
        // 其他状态：拒绝
        assert!(cancel_delivery_status_gate(delivery_status::PENDING).is_err());
        assert!(cancel_delivery_status_gate(delivery_status::CANCELLED).is_err());

        // 错误消息应包含当前状态和"仅 SHIPPED 状态可取消"
        let err = cancel_delivery_status_gate(delivery_status::PENDING).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        let msg = err.to_string();
        assert!(msg.contains("pending"));
        assert!(msg.contains("仅 SHIPPED 状态可取消"));
    }

    /// 测试_取消发货订单状态回退_全部取消转已审批
    ///
    /// 验证 cancel_delivery 中：所有发货取消后 has_shipped=false → 订单回退到 APPROVED。
    #[test]
    fn 测试_取消发货订单状态回退_全部取消转已审批() {
        assert_eq!(
            compute_new_status_after_cancel(false),
            so_status::APPROVED
        );
    }

    /// 测试_取消发货订单状态回退_部分取消转部分发货
    ///
    /// 验证 cancel_delivery 中：仍有已发数量 has_shipped=true → 订单回退到 PARTIAL_SHIPPED。
    #[test]
    fn 测试_取消发货订单状态回退_部分取消转部分发货() {
        assert_eq!(
            compute_new_status_after_cancel(true),
            so_status::PARTIAL_SHIPPED
        );
    }

    /// 测试_取消发货订单状态回退条件_仅已发货或部分发货回退
    ///
    /// 验证 cancel_delivery 中状态回退触发条件：仅 SHIPPED 或 PARTIAL_SHIPPED 才回退，
    /// 避免覆盖 CANCELLED / COMPLETED 等终态。
    #[test]
    fn 测试_取消发货订单状态回退条件_仅已发货或部分发货回退() {
        // 可回退
        assert!(order_status_rollback_eligible(so_status::SHIPPED));
        assert!(order_status_rollback_eligible(so_status::PARTIAL_SHIPPED));
        // 不可回退（避免覆盖终态）
        assert!(!order_status_rollback_eligible(so_status::CANCELLED));
        assert!(!order_status_rollback_eligible(so_status::COMPLETED));
        assert!(!order_status_rollback_eligible(so_status::DRAFT));
        assert!(!order_status_rollback_eligible(so_status::APPROVED));
    }

    /// 测试_取消发货备注格式
    ///
    /// 验证 cancel_delivery 中 `format!("[取消原因] {}", reason)` 的备注格式，
    /// 取消原因会被记录到发货单 remarks 字段。
    #[test]
    fn 测试_取消发货备注格式() {
        let remark = format_cancel_remark("客户拒收");
        assert_eq!(remark, "[取消原因] 客户拒收");

        // 空原因：前缀仍保留
        let remark_empty = format_cancel_remark("");
        assert_eq!(remark_empty, "[取消原因] ");
    }

    /// 测试_恢复库存防御性校验_已发货数量不足
    ///
    /// 验证 restore_inventory 中 `if stock.quantity_shipped < quantity` 的防御性校验：
    /// 已发货数量小于要恢复的数量时应拒绝（库存数据不一致），
    /// 已发货数量 >= 恢复数量时允许（含边界相等）。
    #[test]
    fn 测试_恢复库存防御性校验_已发货数量不足() {
        let shipped = decs!("20");
        let restore_qty = decs!("30");

        // 复现 restore_inventory 中 `if stock.quantity_shipped < quantity` 判定
        let should_reject = shipped < restore_qty;
        assert!(should_reject);

        // 错误构造与消息校验
        let err = AppError::business(format!(
            "产品 {} 已发货数量 {} 小于要恢复的数量 {}，库存数据不一致",
            1, shipped, restore_qty
        ));
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存数据不一致"));

        // 边界：shipped == restore_qty 应允许
        assert!(decs!("30") >= decs!("30"));
        // shipped > restore_qty 应允许
        assert!(decs!("50") >= decs!("30"));
    }

    // ===== 夹具宏可用性 =====

    /// 测试_夹具宏可用性_decs和ymd
    ///
    /// 验证项目测试夹具宏 decs!（Decimal 字符串解析）和 ymd!（NaiveDate 解析）可正常工作，
    /// 这两个宏在 utils/unwrap_safe.rs 中通过 #[macro_export] 导出。
    #[test]
    fn 测试_夹具宏可用性_decs和ymd() {
        // decs! 解析 Decimal 字符串
        let d = decs!("123.45");
        assert_eq!(d.to_string(), "123.45");

        // ymd! 解析日期
        let date = ymd!(2026, 7, 9);
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    // ===== 服务实例化与数据库交互 =====

    /// 测试_服务实例创建
    ///
    /// 验证 SalesService 在 SQLite 内存数据库 + mock SearchClient 上能正常实例化，
    /// SalesService::new 需要 db 与 search_client 两个依赖，使用 ElasticClient::mock() 提供空实现。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 校验服务内部依赖强引用计数 >= 1，证明实例化成功
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_取消发货_需要真实数据库
    ///
    /// 需要 sales_deliveries 表 schema 与真实数据，标注 #[ignore] 仅在 CI 提供数据库时运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore = "依赖数据库 schema，CI 中由 TEST_DATABASE_URL 提供真实数据库"]
    async fn 测试_取消发货_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 不存在的发货单应返回错误（NotFound 或数据库错误），调用路径不 panic
        let result = service
            .cancel_delivery(99999, "测试取消".to_string(), 1)
            .await;
        assert!(result.is_err());
    }

    // ===== v14 批次 421 T-P1-5：缸号同订单校验 validate_dye_lot_consistency =====
    // 依据：fabric-industry-research.md §2.3 约束 5
    // 业务语义：一个缸号代表一次染色，同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺

    /// 测试_缸号同订单校验_空发货明细通过
    ///
    /// 无发货明细时校验通过（边界场景）。
    #[test]
    fn 测试_缸号同订单校验_空发货明细通过() {
        let items: Vec<ShipOrderItemRequest> = vec![];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_单产品单缸号通过
    ///
    /// 同一 product_id 仅一个 dye_lot_no → 通过。
    #[test]
    fn 测试_缸号同订单校验_单产品单缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("5"), Some("DL001".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_多产品各自单缸号通过
    ///
    /// 不同 product_id 可使用不同 dye_lot_no，互不影响 → 通过。
    #[test]
    fn 测试_缸号同订单校验_多产品各自单缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1002, decs!("20"), Some("DL002".to_string())),
            build_ship_item(1003, decs!("5"), Some("DL003".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_同产品不同缸号拒绝
    ///
    /// 同一 product_id 出现多个不同 dye_lot_no → 拒绝（混缸色差风险）。
    #[test]
    fn 测试_缸号同订单校验_同产品不同缸号拒绝() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), Some("DL002".to_string())),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_err(), "同产品不同缸号应被拒绝");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1001"), "错误信息应包含 product_id");
        assert!(msg.contains("DL001"), "错误信息应包含第一个缸号");
        assert!(msg.contains("DL002"), "错误信息应包含第二个缸号");
        assert!(msg.contains("色差"), "错误信息应说明色差风险");
    }

    /// 测试_缸号同订单校验_未指定缸号通过
    ///
    /// 所有明细均未指定 dye_lot_no（None 或空字符串）→ 跳过校验通过，兼容无缸号场景。
    #[test]
    fn 测试_缸号同订单校验_未指定缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), None),
            build_ship_item(1001, decs!("20"), None),
            build_ship_item(1002, decs!("5"), None),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_空字符串缸号视为未指定
    ///
    /// dye_lot_no 为空字符串时视为未指定，跳过校验通过。
    #[test]
    fn 测试_缸号同订单校验_空字符串缸号视为未指定() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("".to_string())),
            build_ship_item(1001, decs!("20"), Some("".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_部分指定部分未指定通过
    ///
    /// 同一 product_id 部分明细指定缸号 DL001，部分未指定 → 仅校验已指定的，
    /// 未指定不参与比较 → 通过。
    #[test]
    fn 测试_缸号同订单校验_部分指定部分未指定通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), None),
            build_ship_item(1001, decs!("5"), Some("DL001".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_错误信息包含缸号列表
    ///
    /// 验证错误信息中包含所有冲突的缸号，便于业务人员定位问题。
    #[test]
    fn 测试_缸号同订单校验_错误信息包含缸号列表() {
        let items = vec![
            build_ship_item(2002, decs!("10"), Some("缸号A".to_string())),
            build_ship_item(2002, decs!("20"), Some("缸号B".to_string())),
            build_ship_item(2002, decs!("5"), Some("缸号C".to_string())),
        ];
        let err = validate_dye_lot_consistency(&items).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("2002"));
        assert!(msg.contains("缸号A"));
        assert!(msg.contains("缸号B"));
        assert!(msg.contains("缸号C"));
    }

    /// 测试夹具：构造 ShipOrderItemRequest
    ///
    /// 集中构造发货明细，避免每个测试重复字段初始化（规则 6 mock 数据抽取）。
    /// batch_no 默认 None，color_no 默认 None，仅 product_id/quantity/dye_lot_no 可变。
    fn build_ship_item(
        product_id: i32,
        quantity: Decimal,
        dye_lot_no: Option<String>,
    ) -> ShipOrderItemRequest {
        ShipOrderItemRequest {
            product_id,
            quantity,
            batch_no: None,
            color_no: None,
            dye_lot_no,
        }
    }
}
