//! 采购价格/采购建议服务（po/price）
//!
//! 包含预算检查与占用、采购建议（基于缺料预警、库存预警）等价格相关逻辑。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{product, purchase_order, purchase_order_item, supplier, warehouse};
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use super::order::PurchaseOrderService;
use sea_orm::DatabaseConnection;

impl PurchaseOrderService {
    /// 检查并占用预算（非阻断）
    pub(crate) async fn check_and_occupy_budget(
        &self,
        order: &purchase_order::Model,
        department_id: i32,
        total_amount: Decimal,
        user_id: i32,
    ) {
        let budget_service =
            crate::services::budget_management_service::BudgetManagementService::new(
                self.db.clone(),
            );

        // 查找部门对应的预算方案
        match budget_service
            .get_available_plan_by_department(department_id)
            .await
        {
            Ok(Some(plan)) => {
                // 检查预算是否可用
                match budget_service
                    .check_budget_available(department_id, plan.id, total_amount)
                    .await
                {
                    Ok(true) => {
                        // 预算充足，占用预算
                        match budget_service
                            .occupy_budget(
                                department_id,
                                plan.id,
                                total_amount,
                                "purchase_order".to_string(),
                                order.id,
                                user_id,
                            )
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "订单 {} 预算占用成功，部门ID={}, 方案ID={}, 金额={}",
                                    order.order_no,
                                    department_id,
                                    plan.id,
                                    total_amount
                                );
                            }
                            Err(e) => {
                                // 预算占用失败，记录警告但不阻断
                                tracing::warn!(
                                    "订单 {} 预算占用失败：{}，订单已创建但未关联预算",
                                    order.order_no,
                                    e
                                );
                            }
                        }
                    }
                    Ok(false) => {
                        // 预算不足，记录警告但不阻断
                        tracing::warn!(
                            "订单 {} 预算余额不足，部门ID={}, 方案ID={}, 订单金额={}, 订单已创建但未占用预算",
                            order.order_no, department_id, plan.id, total_amount
                        );
                    }
                    Err(e) => {
                        // 预算检查失败，记录警告但不阻断
                        tracing::warn!(
                            "订单 {} 预算检查失败：{}，订单已创建但未关联预算",
                            order.order_no,
                            e
                        );
                    }
                }
            }
            Ok(None) => {
                // 未找到预算方案，记录警告但不阻断
                tracing::warn!(
                    "订单 {} 未找到部门 {} 的预算方案，订单已创建但未关联预算",
                    order.order_no,
                    department_id
                );
            }
            Err(e) => {
                // 查询预算方案失败，记录警告但不阻断
                tracing::warn!(
                    "订单 {} 查询预算方案失败：{}，订单已创建但未关联预算",
                    order.order_no,
                    e
                );
            }
        }
    }

    /// 根据缺料预警创建采购建议
    #[allow(clippy::too_many_arguments)]
    pub async fn create_purchase_suggestion_from_shortage(
        &self,
        material_id: i32,
        material_name: String,
        material_code: String,
        required_quantity: Decimal,
        available_quantity: Decimal,
        shortage_quantity: Decimal,
        shortage_level: String,
        affected_orders_count: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 1. 获取产品信息
        let product = product::Entity::find_by_id(material_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料 ID {} 不存在", material_id)))?;

        // 2. 查找默认供应商
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq("active"))
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
            warehouse_id: Set(1), // 默认仓库
            department_id: Set(1), // 默认部门
            purchaser_id: Set(1), // 默认采购员
            currency: Set("CNY".to_string()),
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
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = suggested_quantity * unit_price;
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = amount * tax_rate / Decimal::new(100, 0);

        purchase_order_item::ActiveModel {
            id: Set(0),
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
            .filter(supplier::Column::Status.eq("active"))
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
            department_id: Set(1), // 默认部门
            purchaser_id: Set(1),  // 默认采购员
            currency: Set("CNY".to_string()),
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
        let quantity = reorder_quantity;
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let amount = quantity * unit_price;
        let tax_rate = Decimal::new(13, 2); // 13% 增值税
        let tax_amount = amount * tax_rate / Decimal::new(100, 0);

        purchase_order_item::ActiveModel {
            id: Set(0),
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
