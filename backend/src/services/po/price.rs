//! 采购价格/采购建议服务（po/price）
//!
//! 包含预算检查与占用、采购建议（基于缺料预警、库存预警）等价格相关逻辑。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{product, purchase_order, purchase_order_item, supplier, warehouse};
// 批次 211 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use super::order::PurchaseOrderService;

/// 缺料预警参数对象
///
/// 批次 333 v10 复审 P3 修复：引入参数对象消除 create_purchase_suggestion_from_shortage
/// 的 too_many_arguments 警告。聚合缺料预警所需的全部字段，避免函数签名携带 8 个参数。
#[derive(Debug, Clone)]
pub struct ShortageAlertParams {
    /// 物料 ID
    pub material_id: i32,
    /// 物料名称
    pub material_name: String,
    /// 物料编码
    pub material_code: String,
    /// 需求数量
    pub required_quantity: Decimal,
    /// 可用数量
    pub available_quantity: Decimal,
    /// 缺口数量
    pub shortage_quantity: Decimal,
    /// 缺料级别（Critical / Severe / Warning）
    pub shortage_level: String,
    /// 受影响订单数
    pub affected_orders_count: i32,
}

impl PurchaseOrderService {
    /// 检查并占用预算（V15 P0-B06 强制拦截版本）
    ///
    /// 设计依据：审计报告 §17.7-D1 — 预算超支无拦截
    /// 修复前：返回 () 非阻断，预算不足仅记录 warn 日志，订单仍创建
    /// 修复后：返回 Result<(), AppError>，预算不足或无方案时返回错误阻断订单创建
    ///
    /// 业务流程：
    /// 1. 调用 BudgetManagementService::enforce_budget_available 强制校验
    ///    - 部门无预算方案 → 返回 Err 阻断
    ///    - 预算余额不足 → 返回 Err 阻断（含明细：可用/申请/已下达/已执行）
    /// 2. 校验通过后调用 occupy_budget 占用预算
    /// 3. 占用失败 → 返回 Err 阻断（避免订单创建但未关联预算的不一致状态）
    pub(crate) async fn check_and_occupy_budget(
        &self,
        order: &purchase_order::Model,
        department_id: i32,
        total_amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        let budget_service =
            crate::services::budget_management_service::BudgetManagementService::new(
                self.db.clone(),
            );

        // V15 P0-B06：强制校验预算可用性，不足时直接返回错误阻断订单创建
        let plan_id = budget_service
            .enforce_budget_available(department_id, total_amount)
            .await?;

        // 预算充足，占用预算
        budget_service
            .occupy_budget(
                department_id,
                plan_id,
                total_amount,
                "purchase_order".to_string(),
                order.id,
                user_id,
            )
            .await
            .map(|execution| {
                tracing::info!(
                    "订单 {} 预算占用成功，部门ID={}, 方案ID={}, 金额={}, 执行记录ID={}",
                    order.order_no,
                    department_id,
                    plan_id,
                    total_amount,
                    execution.id
                );
            })
    }

    /// 根据缺料预警创建采购建议
    ///
    /// 批次 333 v10 复审 P3 修复：签名从 8 参数改为单一参数对象 `ShortageAlertParams`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn create_purchase_suggestion_from_shortage(
        &self,
        params: ShortageAlertParams,
    ) -> Result<purchase_order::Model, AppError> {
        // 解构参数对象，便于函数体内按字段名访问
        let ShortageAlertParams {
            material_id,
            material_name,
            material_code,
            required_quantity,
            available_quantity,
            shortage_quantity,
            shortage_level,
            affected_orders_count,
        } = params;

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 1. 获取产品信息
        let product = product::Entity::find_by_id(material_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料 ID {} 不存在", material_id)))?;

        // 2. 查找默认供应商
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq(master_data::ACTIVE))
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("没有可用的活跃供应商"))?;

        // 3. 计算建议采购量（缺口数量 * 1.2，20%余量）
        let suggested_quantity = shortage_quantity * Decimal::new(12, 1);

        // 4. 生成采购订单号（使用事务连接）
        let order_no = self.generate_order_no_with_txn(&txn).await?;

        // 5. 根据缺料级别确定优先级
        let priority = match shortage_level.as_str() {
            "Critical" => "URGENT",
            "Severe" => "HIGH",
            "Warning" => "MEDIUM",
            _ => "LOW",
        };

        // 6. 创建采购订单
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(supplier.id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some(
                (Utc::now() + chrono::Duration::days(7)).date_naive(),
            )),
            warehouse_id: Set(crate::constants::DEFAULT_WAREHOUSE_ID),
            department_id: Set(crate::constants::DEFAULT_DEPARTMENT_ID),
            purchaser_id: Set(crate::constants::DEFAULT_PURCHASER_ID),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(Decimal::new(1, 0)),
            order_status: Set("DRAFT".to_string()),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 需求量: {} | 可用量: {} | 缺口: {} | 级别: {} | 受影响订单: {}",
                material_name, material_code, required_quantity, available_quantity, shortage_quantity, shortage_level, affected_orders_count
            ))),
            created_by: Set(1), // 系统用户
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 7. 创建订单明细
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = (suggested_quantity * unit_price).round_dp(2);
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = (amount * tax_rate / Decimal::new(100, 0)).round_dp(2);

        purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order.id),
            product_id: Set(material_id),
            quantity: Set(suggested_quantity),
            quantity_alt: Set(Decimal::ZERO),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(tax_rate),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(amount + tax_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 优先级: {}",
                material_name, material_code, priority
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 8. 提交事务
        txn.commit().await?;

        tracing::info!(
            "创建缺料预警采购建议: 订单号={}, 物料={} ({})，建议采购量={}, 优先级={}",
            order.order_no,
            material_name,
            material_code,
            suggested_quantity,
            priority
        );

        Ok(order)
    }

    /// 根据库存预警创建采购建议
    pub async fn create_purchase_suggestion(
        &self,
        product_id: i32,
        warehouse_id: i32,
        current_quantity: Decimal,
        reorder_point: Decimal,
        reorder_quantity: Decimal,
    ) -> Result<purchase_order::Model, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 1. 获取产品信息
        let product = product::Entity::find_by_id(product_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", product_id)))?;

        // 2. 获取仓库信息
        let warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", warehouse_id)))?;

        // 3. 查找默认供应商（这里简化处理，实际可能需要更复杂的供应商选择逻辑）
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq(master_data::ACTIVE))
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("没有可用的活跃供应商"))?;

        // 4. 生成采购订单号（使用事务连接）
        let order_no = self.generate_order_no_with_txn(&txn).await?;

        // 5. 创建采购订单
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(supplier.id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some(
                (Utc::now() + chrono::Duration::days(7)).date_naive(),
            )),
            warehouse_id: Set(warehouse_id),
            department_id: Set(crate::constants::DEFAULT_DEPARTMENT_ID),
            purchaser_id: Set(crate::constants::DEFAULT_PURCHASER_ID),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(Decimal::new(1, 0)),
            order_status: Set("DRAFT".to_string()),
            notes: Set(Some(format!(
                "库存预警自动生成，当前库存: {}, 补货点: {}, 建议补货量: {}",
                current_quantity, reorder_point, reorder_quantity
            ))),
            created_by: Set(1), // 系统用户
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 6. 创建订单明细
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let quantity = reorder_quantity;
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = (quantity * unit_price).round_dp(2);
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = (amount * tax_rate / Decimal::new(100, 0)).round_dp(2);

        purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order.id),
            product_id: Set(product_id),
            quantity: Set(quantity),
            quantity_alt: Set(Decimal::ZERO),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(Decimal::ZERO),
            tax_percent: Set(tax_rate),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(Decimal::ZERO),
            total_amount: Set(amount + tax_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!("库存预警自动生成，产品: {}", product.name))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 7. 提交事务
        txn.commit().await?;

        tracing::info!(
            "创建库存预警采购建议: 订单号={}, 产品={}, 仓库={}, 建议采购量={}",
            order.order_no,
            product.name,
            warehouse.name,
            quantity
        );

        Ok(order)
    }
}
