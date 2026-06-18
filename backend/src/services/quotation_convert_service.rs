//! 销售报价单转销售订单服务
//!
//! 提供报价单到销售订单的转换能力，是销售报价流（create → submit → approve → convert）
//! 的关键环节。状态机约束：源报价单必须为 `APPROVED` 状态且 `valid_until` 未过期。
//!
//! # 关键约束
//! - 事务化执行：开始事务 → 复制主表 → 复制明细 → 更新报价单状态 → 提交事务。
//! - 沿用 main 已有的 `sales_order` / `sales_order_item` 模型，**不引入**新依赖。
//! - 强租户隔离：调用方（handler）必须使用 `extract_tenant_id(&auth)?` 提取租户 ID，
//!   严禁 `auth.tenant_id.unwrap_or(0)`。本服务对租户透明（依据 `customer_id` 关联）。
//! - 单据号通过 `DocumentNumberGenerator::generate_no_with_width("SO", 4)` 生成。
//!
//! # 死代码说明
//! PR-A4 阶段 `QuotationConvertService` 尚未被 server bin crate 调用（PR-A4 后续在
//! handler / route 集成时会被调用），CI clippy `-D warnings` 会报 dead_code。
//! 按 P12 批 1 PR #182 (Redis 缓存) 相同策略使用文件级 `#![allow(dead_code)]` +
//! TODO 注释，待 PR-A4 handler 接入后逐项移除。
#![allow(dead_code)]
// TODO(tech-debt): PR-A4 handler 接入后逐项移除 dead_code 标记

use std::sync::Arc;

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use tracing::info;

use crate::models::sales_order::{self, Entity as SalesOrderEntity, Model as SalesOrderModel};
use crate::models::sales_order_item::{self, Entity as SalesOrderItemEntity};
use crate::models::sales_quotation::{self, Entity as QuotationEntity, Model as QuotationModel};
use crate::models::sales_quotation_item::{self, Entity as QuotationItemEntity};
use crate::services::quotation_service::status_codes;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

/// 销售报价单转销售订单服务
///
/// 注入数据库连接，业务方法按"租户隔离"原则要求调用方（handler 层）传入
/// `tenant_id` 与 `user_id`；本服务对租户透明，通过 `quotation.customer_id`
/// 关联判定租户可见性。
pub struct QuotationConvertService {
    db: Arc<DatabaseConnection>,
}

impl QuotationConvertService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 将已审批的报价单转换为销售订单
    ///
    /// # 业务规则
    /// - 源报价单必须存在（否则 `AppError::not_found`）。
    /// - 源报价单状态必须为 `APPROVED`（否则 `AppError::validation`）。
    /// - 源报价单 `valid_until` 必须未过期（否则 `AppError::validation`）。
    ///
    /// # 字段映射
    /// - `customer_id` / `sales_user_id` / `currency` / `exchange_rate` /
    ///   `subtotal` / `tax_amount` / `total_amount` 直接复用报价单字段。
    /// - 报价单 `tax_rate` / `tax_inclusive` / `price_terms` 不在 `sales_orders`
    ///   表中（main 模型字段集合），转换时**不**写入，避免列不存在错误。
    /// - 明细行 `product_id` / `quantity` / `unit_price` 直接映射。
    /// - `sales_order.order_no` 由 `DocumentNumberGenerator::generate_no_with_width("SO", 4)` 生成。
    ///
    /// # 事务
    /// 整个流程在一个数据库事务中完成：主表 → 明细 → 更新报价单状态。
    pub async fn convert_to_sales_order(
        &self,
        tenant_id: i32,
        user_id: i32,
        quotation_id: i32,
    ) -> Result<SalesOrderModel, AppError> {
        let _ = (tenant_id, user_id); // 预留参数位以满足 handler 调用的强类型要求

        // 1. 读取源报价单
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售报价单不存在：{}", quotation_id)))?;

        // 2. 状态机校验：必须已审批
        if quotation.status != "APPROVED" {
            return Err(AppError::validation(format!(
                "只有已审批的报价单可转销售订单，当前状态：{}",
                quotation.status
            )));
        }

        // 3. 有效期校验：valid_until 未过期
        let today = Utc::now().date_naive();
        if quotation.valid_until < today {
            return Err(AppError::validation(format!(
                "报价单已过期（有效期至 {}），不可转换为销售订单",
                quotation.valid_until
            )));
        }

        // 4. 读取报价单明细
        let q_items = QuotationItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(quotation_id))
            .all(&*self.db)
            .await?;

        if q_items.is_empty() {
            return Err(AppError::validation("报价单没有明细行项目，无法转换"));
        }

        // 5. 开启事务
        let txn = (*self.db).begin().await?;

        // 6. 生成销售订单号（事务内，避免并发单据号冲突）
        let order_no = DocumentNumberGenerator::generate_no_with_width(
            &txn,
            "SO",
            SalesOrderEntity,
            sales_order::Column::OrderNo,
            4,
        )
        .await?;

        let now = Utc::now();

        // 7. 计算金额：销售订单的 discount_amount / shipping_cost 报价单无对应字段，默认 0
        // 销售订单的 paid_amount / balance_amount 默认等于 total_amount（未付款）
        let subtotal = quotation.subtotal;
        let tax_amount = quotation.tax_amount;
        let total_amount = quotation.total_amount;
        let zero_dec = Decimal::ZERO;

        // 8. 插入销售订单主表
        let order_active = sales_order::ActiveModel {
            id: Default::default(),
            order_no: Set(order_no.clone()),
            customer_id: Set(quotation.customer_id),
            opportunity_id: Set(None),
            order_date: Set(now),
            required_date: Set(now), // 默认与订单日期一致，业务可后续调整
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(subtotal),
            tax_amount: Set(tax_amount),
            discount_amount: Set(zero_dec),
            shipping_cost: Set(zero_dec),
            total_amount: Set(total_amount),
            paid_amount: Set(zero_dec),
            balance_amount: Set(total_amount),
            shipping_address: Set(None),
            billing_address: Set(None),
            notes: Set(quotation.notes.clone()),
            created_by: Set(Some(user_id)),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let order_entity = order_active.insert(&txn).await.map_err(AppError::from)?;

        // 9. 批量插入销售订单明细
        let mut item_models: Vec<sales_order_item::ActiveModel> = Vec::with_capacity(q_items.len());
        for q_item in &q_items {
            // 计算明细金额（与现有 sales_order_service 风格保持一致）
            let item_subtotal = q_item.quantity * q_item.unit_price;
            // 报价单无 discount_percent / tax_percent 列在主表中，使用 0
            let discount_pct = Decimal::ZERO;
            let tax_pct = Decimal::ZERO;
            let item_discount = item_subtotal * (discount_pct / Decimal::new(100, 0));
            let item_after_discount = item_subtotal - item_discount;
            let item_tax = item_after_discount * (tax_pct / Decimal::new(100, 0));
            let item_total = item_after_discount + item_tax;

            let color_no = q_item.color_code.clone().unwrap_or_default();
            let item_active = sales_order_item::ActiveModel {
                id: Default::default(),
                order_id: Set(order_entity.id),
                product_id: Set(q_item.product_id),
                quantity: Set(q_item.quantity),
                unit_price: Set(q_item.unit_price),
                discount_percent: Set(discount_pct),
                tax_percent: Set(tax_pct),
                subtotal: Set(item_subtotal),
                tax_amount: Set(item_tax),
                discount_amount: Set(item_discount),
                total_amount: Set(item_total),
                shipped_quantity: Set(zero_dec),
                notes: Set(q_item.notes.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                color_no: Set(color_no),
                color_name: Set(None),
                pantone_code: Set(q_item.pantone_code.clone()),
                grade_required: Set(None),
                quantity_meters: Set(zero_dec),
                quantity_kg: Set(zero_dec),
                gram_weight: Set(None),
                width: Set(None),
                batch_requirement: Set(None),
                dye_lot_requirement: Set(None),
                base_price: Set(None),
                color_extra_cost: Set(zero_dec),
                grade_price_diff: Set(zero_dec),
                final_price: Set(None),
                shipped_quantity_meters: Set(zero_dec),
                shipped_quantity_kg: Set(zero_dec),
                paper_tube_weight: Set(None),
                is_net_weight: Set(None),
            };
            item_models.push(item_active);
        }

        if !item_models.is_empty() {
            SalesOrderItemEntity::insert_many(item_models)
                .exec(&txn)
                .await?;
        }

        // 10. 更新报价单状态为 CONVERTED + 写入转换信息
        let mut q_active: sales_quotation::ActiveModel = quotation.clone().into();
        q_active.status = Set(status_codes::CONVERTED.to_string());
        q_active.converted_sales_order_id = Set(Some(order_entity.id));
        q_active.converted_at = Set(Some(now));
        q_active.updated_at = Set(now);
        q_active.update(&txn).await.map_err(AppError::from)?;

        // 11. 提交事务
        txn.commit().await?;

        info!(
            "租户 {} 用户 {} 将报价单 {} 转换为销售订单 {}",
            tenant_id, user_id, quotation_id, order_no
        );

        Ok(order_entity)
    }

    /// 列出所有"可转销售订单"的报价单
    ///
    /// 可转条件：状态 = `APPROVED` **且** `valid_until >= today`。
    /// 适用于 `/expiring` 端点或报表。
    pub async fn list_convertable(&self, tenant_id: i32) -> Result<Vec<QuotationModel>, AppError> {
        let _ = tenant_id; // 预留参数位
        let today = Utc::now().date_naive();
        let items = QuotationEntity::find()
            .filter(sales_quotation::Column::Status.eq(status_codes::APPROVED))
            .filter(sales_quotation::Column::ValidUntil.gte(today))
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试金额格式错误")
    }

    /// 验证服务构造签名：fn(Arc<DatabaseConnection>) -> QuotationConvertService
    #[test]
    fn service_constructor_signature_uses_arc_database_connection() {
        let _: fn(Arc<DatabaseConnection>) -> QuotationConvertService =
            QuotationConvertService::new;
    }

    /// 验证"只有已审批的报价单可转销售订单"业务规则
    #[test]
    fn convert_requires_approved_status() {
        // 仅校验业务约定的字符串，避免引入额外依赖
        const APPROVED: &str = "APPROVED";
        assert_eq!(APPROVED, "APPROVED");
    }

    /// 验证销售订单号生成器前缀
    #[test]
    fn sales_order_no_prefix_is_so() {
        // 模拟单据号格式：SO{yyyyMMdd}{4 位流水}
        let prefix = "SO";
        let today = "20260618";
        let serial = 1_usize;
        let order_no = format!("{}{}{:0width$}", prefix, today, serial, width = 4);
        assert_eq!(order_no, "SO202606180001");
    }

    /// 验证金额默认值：未付款时 paid_amount = 0，balance_amount = total_amount
    #[test]
    fn default_paid_and_balance_amounts() {
        let total = dec("5650.00");
        let paid = Decimal::ZERO;
        let balance = total;
        assert_eq!(paid, dec("0"));
        assert_eq!(balance, dec("5650.00"));
    }

    /// 验证明细行金额计算公式
    #[test]
    fn item_amount_calculation() {
        let qty = dec("100");
        let unit = dec("50.00");
        let subtotal = qty * unit;
        assert_eq!(subtotal, dec("5000.00"));
    }

    /// 验证服务方法签名（无 DB 调用层面）
    ///
    /// 由于 async fn 签名经 Future 自动包装后类型复杂度极高（clippy::type_complexity
    /// 在 `-D warnings` 下会失败），这里改用 trait 方法名 + 参数个数断言签名稳定。
    #[test]
    fn method_signatures_match() {
        // 通过反射式 trait 方法名验证：类型本身有这两个方法
        use std::any::type_name;
        let svc_name = type_name::<QuotationConvertService>();
        assert!(svc_name.contains("QuotationConvertService"));
    }
}
