//! 销售报价单 → 销售订单 转换服务
//!
//! 业务功能：
//! - 校验报价单状态（仅 approved 可转）
//! - 事务化复制明细到 sales_order_items
//! - 创建 sales_orders 草稿
//! - 更新报价单状态为 converted，记录 converted_sales_order_id
//!
//! Week 2 任务 8 - 销售报价单模块
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 8

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::container::AppState;
use crate::models::product;
use crate::models::sales_order::{self, ActiveModel as OrderActive, Entity as OrderEntity};
use crate::models::sales_order_item::ActiveModel as OrderItemActive;
use crate::models::sales_quotation::{
    self, ActiveModel as QuotationActive, Entity as QuotationEntity,
};
use crate::models::sales_quotation_item::{self, Entity as QuotationItemEntity};
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;

/// 报价行交易单位的换算策略：仅作为「单位 token → dual_unit_converter 函数」的
/// 内部分派标签，不承载任何换算系数（系数与防御校验单一真源在 utils::dual_unit_converter）。
enum UnitStrategy {
    /// 米制：quantity 即米数
    Meters,
    /// 码制：yards_to_meters
    Yards,
    /// 匹制：pieces_to_meters(× meters_per_piece)
    Pieces,
    /// 卷制：rolls_to_meters(× meters_per_roll)
    Rolls,
    /// 公斤制：quantity 即公斤数，米数走 kg_to_meters
    Kilograms,
}

/// 转订单服务
pub struct QuotationConvertService {
    db: Arc<DatabaseConnection>,
}

/// 转出订单行的四值汇总，是销售订单表头金额回写的唯一来源。
///
/// 存什么，subtotal 是各行未折扣净额之和、discount_amount 是各行净额口径折扣之和、
/// tax_amount 是各行税额之和、total_amount 是各行成交含税额之和，四值都是 2 位小数。
/// 传给谁，QuotationConvertService::convert 的表头回写腿，分别写 sales_orders 的
/// subtotal、discount_amount、tax_amount、total_amount 列（balance_amount 同 total_amount）。
/// 本模块内私有，不对外暴露，避免绕过内核单源自行拼表头。
struct ConvertedLineTotals {
    subtotal: Decimal,
    discount_amount: Decimal,
    tax_amount: Decimal,
    total_amount: Decimal,
}

impl QuotationConvertService {
    /// 从数据库连接直接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 将已审批的报价单转换为销售订单草稿
    pub async fn convert(
        &self,
        quotation_id: i64,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let txn = self.db.begin().await?;

        // 1. 校验报价单存在且状态为 approved
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        Self::validate_quotation_for_convert(&quotation, &txn).await?;

        // 3. 加载报价单明细
        let items = QuotationItemEntity::find()
            .filter(sales_quotation_item::Column::QuotationId.eq(quotation_id))
            .all(&txn)
            .await?;

        // 4./5. 取号 + 创建销售订单草稿：统一走生成器（SO + YYYYMMDD + 3 位流水），
        // 在转单事务内取号（advisory 锁 + 占用探测），INSERT 撞 order_no 唯一约束时
        // 保存点回滚重新取号重试，其余 SQL 错误显式上抛（utils/number_generator.rs）。
        // 原私有 generate_order_no_static 自拼 `SO+YYYYMMDD+4位计数` 与销售订单域
        // 标准格式不一致，且计数口径不含旁路写入的号段，已删除。
        let now = Utc::now();
        let order = crate::utils::number_generator::DocumentNumberGenerator::insert_with_no_retry(
            &txn,
            "SO",
            OrderEntity,
            sales_order::Column::OrderNo,
            |order_no| Self::build_order_from_quotation(&quotation, user_id, order_no, now),
        )
        .await?;

        // 6. 复制明细，行金额一律由 utils tax 内核行级单源产出，并累计出表头四值
        let totals =
            Self::copy_quotation_items_to_order(&items, &quotation, order.id, now, &txn).await?;

        // 7. 表头金额按转出的订单行汇总回写，不照抄报价表头。
        // 报价表头是报价时刻的汇总值，转单腿的行金额改由内核重算后两者可能不同源
        // （历史行按正向公式落库的净额与今天的除法反算值本就不等），照抄会让
        // 表头钩稽与逐行相加两套口径同时存在。
        // 入参，totals 是本次转出订单行的四值汇总；传给谁，sales_orders 的
        // subtotal 与 discount_amount 与 tax_amount 与 total_amount 与 balance_amount 五列。
        crate::utils::tax::ensure_reconciled(
            totals.subtotal - totals.discount_amount,
            totals.tax_amount,
            totals.total_amount,
        )?;
        let order_id = order.id;
        let mut order_active: OrderActive = order.into();
        order_active.subtotal = Set(totals.subtotal);
        order_active.discount_amount = Set(totals.discount_amount);
        order_active.tax_amount = Set(totals.tax_amount);
        order_active.total_amount = Set(totals.total_amount);
        order_active.balance_amount = Set(totals.total_amount);
        order_active.updated_at = Set(now);
        let order = order_active.update(&txn).await?;

        // 8. 更新报价单状态为 converted
        Self::update_quotation_to_converted(&quotation, order_id, now, &txn).await?;

        txn.commit().await?;

        Ok(order)
    }

    async fn validate_quotation_for_convert<C>(
        quotation: &sales_quotation::Model,
        txn: &C,
    ) -> Result<(), AppError>
    where
        C: sea_orm::ConnectionTrait,
    {
        if quotation.status != "approved" {
            return Err(AppError::business(format!(
                "报价单状态不允许转订单：{}（仅 approved 状态可转换）",
                quotation.status
            )));
        }
        if quotation.valid_until < Utc::now().date_naive() {
            let mut active: QuotationActive = quotation.clone().into();
            active.status = Set("expired".to_string());
            active.updated_at = Set(Utc::now());
            active.update(txn).await?;
            return Err(AppError::business("报价单已过期，无法转订单"));
        }
        Ok(())
    }

    /// 构建报价单转销售订单的主表 ActiveModel，单号由生成器在事务内提供。
    ///
    /// 金额四列在这里只是插入所需的初值，明细行落库后由 convert 的表头回写腿按
    /// 内核重算的各行汇总覆盖（见 ConvertedLineTotals），本函数的报价表头照抄值不作为最终口径。
    fn build_order_from_quotation(
        quotation: &sales_quotation::Model,
        user_id: i32,
        order_no: String,
        now: chrono::DateTime<Utc>,
    ) -> OrderActive {
        OrderActive {
            id: Default::default(),
            approval_reason: Set(None),
            rejected_reason: Set(None),
            order_no: Set(order_no),
            customer_id: Set(quotation.customer_id),
            opportunity_id: Set(None),
            order_date: Set(now),
            // 报价单的 valid_until 是报价有效期、不是交期，不可挪用；转单不带交期（如实 NULL），
            // 由销售在订单表单补录（前端必填），禁止编造"当前+30 天"。
            required_date: Set(None),
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(quotation.subtotal),
            tax_amount: Set(quotation.tax_amount),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(quotation.total_amount),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(quotation.total_amount),
            shipping_address: Set(None),
            billing_address: Set(None),
            contact_person: Set(None),
            contact_phone: Set(None),
            notes: Set(Some(format!(
                "[源自报价单 {}]\n{}",
                quotation.quotation_no,
                quotation.notes.clone().unwrap_or_default()
            ))),
            // 面料行业追溯字段：报价单不含这些信息，用 NotSet 让 DB DEFAULT '' 生效
            batch_no: sea_orm::ActiveValue::NotSet,
            color_no: sea_orm::ActiveValue::NotSet,
            dye_lot_no: sea_orm::ActiveValue::NotSet,
            grade: sea_orm::ActiveValue::NotSet,
            packaging_requirement: sea_orm::ActiveValue::NotSet,
            quality_standard: sea_orm::ActiveValue::NotSet,
            created_by: Set(Some(user_id)),
            // m_rls_dept_domain：department_id 由 trg_sales_orders_dept 触发器自动维护
            department_id: sea_orm::ActiveValue::NotSet,
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 复制报价明细到订单行，行金额一律走 utils tax 内核行级单源。
    ///
    /// 入参，items 是报价明细、quotation 是报价单头（税率取本单的 tax_rate 百分数列）、
    /// order_id 是刚落库的订单 id、txn 是转单事务。
    /// 出参 ConvertedLineTotals 是本次转出行的四值汇总，传给表头回写腿落 sales_orders 四列。
    /// 为什么必须经内核，报价行的 discount_amount 是含税口径折扣，而订单行的折扣列是净额口径
    /// （内核 line_split 用折前净额减折后净额取差），照抄报价列会让订单行的
    /// subtotal 减 discount_amount 加 tax_amount 与 total_amount 差出一个折扣额，
    /// 收货、开票、按占比分摊税额的发货腿读这三列时会继承这个缺口。
    async fn copy_quotation_items_to_order<C>(
        items: &[sales_quotation_item::Model],
        quotation: &sales_quotation::Model,
        order_id: i32,
        now: chrono::DateTime<Utc>,
        txn: &C,
    ) -> Result<ConvertedLineTotals, AppError>
    where
        C: sea_orm::ConnectionTrait,
    {
        // 批量取回涉及产品（换算元数据真源），杜绝逐行查询 N+1；
        // products.id 为 SERIAL(i32)，报价行 product_id 为 BIGINT(i64)：与既有边界转换约定一致。
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id as i32).collect();
        let product_map: HashMap<i32, product::Model> = if product_ids.is_empty() {
            HashMap::new()
        } else {
            product::Entity::find()
                .filter(product::Column::Id.is_in(product_ids))
                .all(txn)
                .await?
                .into_iter()
                .map(|p| (p.id, p))
                .collect()
        };

        // 表头回写的四个累加器，逐行取内核出参相加，不在表头做第二次舍入
        let mut subtotal_sum = Decimal::ZERO;
        let mut discount_sum = Decimal::ZERO;
        let mut tax_sum = Decimal::ZERO;
        let mut inclusive_sum = Decimal::ZERO;
        for item in items {
            let product = product_map.get(&(item.product_id as i32)).ok_or_else(|| {
                AppError::validation(format!(
                    "报价单转订单失败：报价明细引用的产品 {} 不存在（悬挂引用不允许转为订单行）",
                    item.product_id
                ))
            })?;
            // 真实单位换算：报价行 unit + 产品换算元数据 → 米/公斤双计量（converter 单一真源）
            let (quantity_meters, quantity_kg) =
                Self::convert_item_quantity_to_dual_units(item, product)?;

            // 行级五值一律由内核单源产出，含税单价与数量是上游带入事实，
            // 净额走除法反算、税额取差额、折扣取净额口径差额，本文件不写第二条算式。
            let split = crate::utils::tax::line_split(
                item.unit_price_with_tax,
                item.quantity,
                item.discount_rate,
                quotation.tax_rate,
            );
            // 入库前逐分钩稽，历史报价行带着正向公式落库的脏三值在这里点名拒绝，
            // 而不是把不钩稽的行复制进订单继续往下游传。
            crate::utils::tax::ensure_reconciled(split.deal_net, split.tax, split.deal_inclusive)?;
            subtotal_sum += split.list_net;
            discount_sum += split.discount_net;
            tax_sum += split.tax;
            inclusive_sum += split.deal_inclusive;
            let new_item = OrderItemActive {
                id: Default::default(),
                order_id: Set(order_id),
                product_id: Set(item.product_id as i32),
                quantity: Set(item.quantity),
                unit_price: Set(split.unit_price),
                discount_percent: Set(item.discount_rate.unwrap_or(Decimal::ZERO)),
                tax_percent: Set(quotation.tax_rate),
                subtotal: Set(split.list_net),
                tax_amount: Set(split.tax),
                discount_amount: Set(split.discount_net),
                total_amount: Set(split.deal_inclusive),
                shipped_quantity: Set(Decimal::ZERO),
                notes: Set(item.notes.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                color_no: Set(Self::compose_color_no(item)),
                piece_no: Set(None),
                color_name: Set(None),
                pantone_code: Set(item.pantone_code.clone()),
                grade_required: Set(None),
                quantity_meters: Set(quantity_meters),
                quantity_kg: Set(quantity_kg),
                // 产品换算/计量元数据带入订单行（此前 Set(None) 丢失真实字段）
                gram_weight: Set(product.gram_weight),
                width: Set(product.width),
                batch_requirement: Set(None),
                dye_lot_requirement: Set(None),
                base_price: Set(Some(split.unit_price)),
                color_extra_cost: Set(Decimal::ZERO),
                grade_price_diff: Set(Decimal::ZERO),
                final_price: Set(Some(item.unit_price_with_tax)),
                shipped_quantity_meters: Set(Decimal::ZERO),
                shipped_quantity_kg: Set(Decimal::ZERO),
                paper_tube_weight: Set(None),
                is_net_weight: Set(None),
                // 报价实体无行级容差字段（已核 sales_quotation_item 全字段），无从带入；
                // 订单行 NULL = 交付时按 delivery_tolerance 既有链路解析（品类 > 全局），
                // 此处保持 NULL 不是吞值，而是「未显式指定」的既定语义。
                quantity_tolerance_pct: Set(None),
                // 报价行不承载流转链来源（已核 sales_quotation_item 全字段无该语义列），
                // 转出的订单行如实落 NULL =「未定档」，不得替客户在报价阶段编一个来源。
                fulfillment_source: Set(None),
            };
            new_item.insert(txn).await?;
        }
        Ok(ConvertedLineTotals {
            subtotal: subtotal_sum.round_dp(2),
            discount_amount: discount_sum.round_dp(2),
            tax_amount: tax_sum.round_dp(2),
            total_amount: inclusive_sum.round_dp(2),
        })
    }

    /// 报价行交易单位 → 换算策略（对应 dual_unit_converter 的换算函数选择）。
    ///
    /// 词表依据：全仓无「单位→换算类别」权威枚举（models/status/** 仅状态词表；
    /// quotation_ops::crud 明确约定报价单位逐字符等于产品主数据中文单位 token、
    /// 不引入第二套字典/枚举），故此处只做「token → converter 函数选择」的分派匹配；
    /// 换算系数与防御校验（负数/元数据 ≤0 拒绝）全部留在 utils::dual_unit_converter，
    /// 本文件零自造系数、零静默兜底。token 集与 dual_unit_converter 各函数文档口径一致。
    fn resolve_unit_conversion(unit: &str) -> Option<UnitStrategy> {
        match unit.trim().to_lowercase().as_str() {
            "米" | "公尺" | "m" | "meter" | "meters" => Some(UnitStrategy::Meters),
            "码" | "yd" | "yard" | "yards" => Some(UnitStrategy::Yards),
            "匹" | "piece" | "pieces" => Some(UnitStrategy::Pieces),
            "卷" | "roll" | "rolls" => Some(UnitStrategy::Rolls),
            "公斤" | "千克" | "kg" => Some(UnitStrategy::Kilograms),
            _ => None,
        }
    }

    /// 将 converter 的防御式 Err(String) 包装为带报价行上下文的 AppError（不吞错、不猜值）。
    fn converter_err(item: &sales_quotation_item::Model, source: String) -> AppError {
        AppError::validation(format!(
            "报价单转订单失败（产品 {}，单位「{}」）：{}",
            item.product_id, item.unit, source
        ))
    }

    /// 按报价行单位与产品换算元数据，把交易量换算为订单行米/公斤双计量。
    ///
    /// - 米：quantity 即米数；码：`yards_to_meters`（米↔码率真源在 converter，1 米 = 1.0936 码）；
    /// - 匹：`pieces_to_meters(× product.meters_per_piece)`；卷：`rolls_to_meters(× product.meters_per_roll)`；
    /// - 公斤：quantity 即公斤数，米数走 `kg_to_meters(克重×幅宽)`；
    /// - 米→公斤统一走 `meters_to_kg(gram_weight × width_cm ÷ 1000)`；
    /// - 元数据缺失（克重/幅宽/每匹米数/每卷米数）或不支持单位（件/条/吨等）：
    ///   明确 AppError 拒绝，绝不静默写 0 或猜测——与 converter 对 ≤0 入参直接 Err 的既有语义同源。
    fn convert_item_quantity_to_dual_units(
        item: &sales_quotation_item::Model,
        product: &product::Model,
    ) -> Result<(Decimal, Decimal), AppError> {
        let qty = item.quantity;
        let strategy = Self::resolve_unit_conversion(&item.unit).ok_or_else(|| {
            AppError::business(format!(
                "报价单转订单失败：产品 {} 的交易单位「{}」不在米/公斤可换算词表内（dual_unit_converter 无对应换算函数），不能猜测米/公斤数；请修正产品主数据交易单位后重试",
                item.product_id, item.unit
            ))
        })?;

        // 米↔公斤与米数→公斤数均强依赖克重×幅宽；缺失即明确报错（converter 对 ≤0 同样拒绝）
        let kg_meta = || -> Result<(Decimal, Decimal), AppError> {
            let gram_weight = product.gram_weight.ok_or_else(|| {
                AppError::validation(format!(
                    "报价单转订单失败：产品 {} 缺少克重(gram_weight)元数据，无法完成米↔公斤换算（拒绝以 0 兜底）",
                    item.product_id
                ))
            })?;
            let width = product.width.ok_or_else(|| {
                AppError::validation(format!(
                    "报价单转订单失败：产品 {} 缺少幅宽(width, cm)元数据，无法完成米↔公斤换算（拒绝以 0 兜底）",
                    item.product_id
                ))
            })?;
            Ok((gram_weight, width))
        };
        let meters_to_kg = |meters: Decimal| -> Result<Decimal, AppError> {
            let (gram_weight, width) = kg_meta()?;
            DualUnitConverter::meters_to_kg(meters, gram_weight, width)
                .map_err(|e| Self::converter_err(item, e))
        };

        let (quantity_meters, quantity_kg) = match strategy {
            UnitStrategy::Meters => (qty, meters_to_kg(qty)?),
            UnitStrategy::Yards => {
                let meters = DualUnitConverter::yards_to_meters(qty)
                    .map_err(|e| Self::converter_err(item, e))?;
                (meters, meters_to_kg(meters)?)
            }
            UnitStrategy::Pieces => {
                let meters_per_piece = product.meters_per_piece.ok_or_else(|| {
                    AppError::validation(format!(
                        "报价单转订单失败：产品 {} 按「匹」计价但缺少每匹米数(meters_per_piece)换算元数据，无法换算米数（拒绝以 0 或猜测值兜底）",
                        item.product_id
                    ))
                })?;
                let meters = DualUnitConverter::pieces_to_meters(qty, meters_per_piece)
                    .map_err(|e| Self::converter_err(item, e))?;
                (meters, meters_to_kg(meters)?)
            }
            UnitStrategy::Rolls => {
                let meters_per_roll = product.meters_per_roll.ok_or_else(|| {
                    AppError::validation(format!(
                        "报价单转订单失败：产品 {} 按「卷」计价但缺少每卷米数(meters_per_roll)换算元数据，无法换算米数（拒绝以 0 或猜测值兜底）",
                        item.product_id
                    ))
                })?;
                let meters = DualUnitConverter::rolls_to_meters(qty, meters_per_roll)
                    .map_err(|e| Self::converter_err(item, e))?;
                (meters, meters_to_kg(meters)?)
            }
            UnitStrategy::Kilograms => {
                let (gram_weight, width) = kg_meta()?;
                let meters = DualUnitConverter::kg_to_meters(qty, gram_weight, width)
                    .map_err(|e| Self::converter_err(item, e))?;
                (meters, qty)
            }
        };
        Ok((quantity_meters, quantity_kg))
    }

    async fn update_quotation_to_converted<C>(
        quotation: &sales_quotation::Model,
        order_id: i32,
        now: chrono::DateTime<Utc>,
        txn: &C,
    ) -> Result<(), AppError>
    where
        C: sea_orm::ConnectionTrait,
    {
        let mut active: QuotationActive = quotation.clone().into();
        active.status = Set("converted".to_string());
        active.converted_sales_order_id = Set(Some(order_id as i64));
        active.converted_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(txn).await?;
        Ok(())
    }

    /// 拼接色号（color_code / pantone_code / cncs_code）
    /// 公开为 `pub` 以便集成测试 `tests/quotation_convert_test.rs` 直接调用。；内部实现细节稳定，可安全作为测试入口暴露。
    pub fn compose_color_no(item: &sales_quotation_item::Model) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(c) = &item.color_code {
            if !c.is_empty() {
                parts.push(c.clone());
            }
        }
        if let Some(p) = &item.pantone_code {
            if !p.is_empty() {
                parts.push(format!("PANTONE:{}", p));
            }
        }
        if let Some(c) = &item.cncs_code {
            if !c.is_empty() {
                parts.push(format!("CNCS:{}", c));
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join("/")
        }
    }
}
