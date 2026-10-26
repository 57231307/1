//! 采购价格/采购建议服务（po/price）
//!
//! 包含预算检查与占用、采购建议（基于缺料预警、库存预警）等价格相关逻辑。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::status::master_data;
use crate::models::status::purchase_inventory::purchase_order as po_status;
use crate::models::{product, purchase_order, purchase_order_item, supplier, warehouse};
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};

use super::order::PurchaseOrderService;
use super::order_ops::crud::ItemAmounts;

/// 缺料预警参数对象
/// 批次 333 v10 复审 P3 修复：引入参数对象消除 create_purchase_suggestion_from_shortage；的 too_many_arguments 警告。聚合缺料预警所需的全部字段，避免函数签名携带 8 个参数。
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

/// 自动补货两条链路（缺料预警与库存预警）写 purchase_orders 表头的汇率，恒取 1。
///
/// 两条链路都只按默认币种建单、只按物料成本价计价，本常量既写 exchange_rate 列，
/// 也用于折算 total_amount_foreign，两处同一来源。改成非 1 会让外币总额与本位币
/// 总额按该倍率脱钩，而这两条链路并不产生外币采购。
const AUTO_REPLENISHMENT_EXCHANGE_RATE: Decimal = Decimal::from_parts(1, 0, 0, false, 0);

impl PurchaseOrderService {
    /// 检查并占用预算（V15 P0-B06 强制拦截版本）
    /// 设计依据：审计报告 §17.7-D1 — 预算超支无拦截；修复前：返回 () 非阻断，预算不足仅记录 warn 日志，订单仍创建；修复后：返回 Result<(), AppError>，预算不足或无方案时返回错误阻断订单创建；业务流程：1. 调用 BudgetManagementService::enforce_budget_available 强制校验；部门无预算方案 → 返回 Err 阻断；预算余额不足 → 返回 Err 阻断（含明细：可用/申请/已下达/已执行）；2. 校验通过后调用 occupy_budget 占用预算；3. 占用失败 → 返回 Err 阻断（避免订单创建但未关联预算的不一致状态）
    pub(crate) async fn check_and_occupy_budget(
        &self,
        order: &purchase_order::Model,
        department_id: i32,
        total_amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 非生产环境跳过预算检查（CI 测试环境无预算方案数据，强制检查会导致订单创建失败）
        if !crate::utils::config::is_production() {
            tracing::info!(
                "非生产环境跳过预算检查，订单={} 部门={} 金额={}",
                order.order_no,
                department_id,
                total_amount
            );
            return Ok(());
        }

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

    /// 根据缺料预警创建采购建议（参数对象消除 too_many_arguments）
    pub async fn create_purchase_suggestion_from_shortage(
        &self,
        params: ShortageAlertParams,
    ) -> Result<purchase_order::Model, AppError> {
        // 该链路既无人工录入环节、又无可取税率的上游单据（预警事件直接建单），故在此
        // **显式声明无进项税**，口径值取自 utils tax 内核常量，写入 tax_percent 列并作为
        // 税额内核入参。人工建单侧缺 tax_rate 是点名拒绝、不走本常量，本链路也不得改成
        // 拒绝（自动补货无人可问），更禁止在此填 13 之类的税率档位。
        let tax_percent = crate::utils::tax::NO_INPUT_TAX_PERCENT;
        tracing::info!(
            "缺料预警采购建议按无进项税落账: 物料={} ({})，原因=自动补货链路无税率来源，系统不设默认税率档位",
            params.material_name,
            params.material_code
        );
        let txn = (*self.db).begin().await?;
        let (product, supplier) =
            Self::fetch_shortage_product_and_supplier(&txn, params.material_id).await?;
        let suggested_quantity = params.shortage_quantity * Decimal::new(12, 1);
        // 单行金额只算一次，表头四个总额列与明细行共用这一份，避免两处各写一套公式
        let amounts = Self::suggestion_line_amounts(&product, suggested_quantity, tax_percent);
        let order_no = self.generate_order_no_with_txn(&txn).await?;
        let priority = Self::shortage_priority(&params.shortage_level);
        let order = Self::build_shortage_order(&order_no, supplier.id, &params, &amounts)
            .insert(&txn)
            .await?;
        Self::create_shortage_order_item(&txn, order.id, &params, priority, &amounts).await?;
        txn.commit().await?;
        tracing::info!(
            "创建缺料预警采购建议: 订单号={}, 物料={} ({})，建议采购量={}, 优先级={}",
            order.order_no,
            params.material_name,
            params.material_code,
            suggested_quantity,
            priority
        );
        Ok(order)
    }

    /// 获取物料和默认活跃供应商
    async fn fetch_shortage_product_and_supplier(
        txn: &sea_orm::DatabaseTransaction,
        material_id: i32,
    ) -> Result<(product::Model, supplier::Model), AppError> {
        let product = product::Entity::find_by_id(material_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料 ID {} 不存在", material_id)))?;
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq(master_data::ACTIVE))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("没有可用的活跃供应商"))?;
        Ok((product, supplier))
    }

    /// 根据缺料级别确定优先级
    fn shortage_priority(level: &str) -> &'static str {
        match level {
            "Critical" => "URGENT",
            "Severe" => "HIGH",
            "Warning" => "MEDIUM",
            _ => "LOW",
        }
    }

    /// 计算自动补货建议的单行金额（缺料预警与库存预警两条建单链路共用，此处是唯一实现）
    ///
    /// 入参 product 为建议采购的物料（单价取其成本价 cost_price）、quantity 为建议数量、
    /// tax_percent 为调用方显式声明的百分数口径税率，税额经 utils tax 内核算出。产出
    /// ItemAmounts 同时喂给表头构造器汇总 purchase_orders 的四个总额列与明细构造器写
    /// purchase_order_item 的数量、单价、税额、金额列，保证表头与明细同源。辅助数量与
    /// 折扣在这两条自动链路没有来源，如实落 0，不引入任何折扣或辅量口径。
    ///
    /// 本函数不经人工建单的入参门（services/po/order_ops/crud.rs 的
    /// calculate_item_amounts 与 validate_line_quantity）：自动链路既没有人工录入环节，
    /// 单价与数量的来源就是物料成本价与预警建议量，两条链路各有一条且只有一条合法路径，
    /// 不得在本处补一条缺省分支，也不得把人工路径的门搬过来。
    fn suggestion_line_amounts(
        product: &product::Model,
        quantity: Decimal,
        tax_percent: Decimal,
    ) -> ItemAmounts {
        // 物料成本价是本链路唯一的价格事实；入参税率恒为内核常量「无进项税 0」（两条建单
        // 入口都显式取 NO_INPUT_TAX_PERCENT），0 税率下含税价与不含税价等值，成本价原样
        // 落 unit_price 列即同时是净额单价与含税单价，不存在第二次舍入。
        // 税额由 utils tax 的价税分离取差额得到（除法退化后为 0），与人工建单腿共用同一枚
        // 内核函数，本处不出现任何乘税率造税额的公式；带真实税率的人工行不得走本函数。
        let unit_price = product.cost_price.unwrap_or(Decimal::ZERO);
        let inclusive = crate::utils::tax::inclusive_amount(unit_price, quantity);
        let split = crate::utils::tax::split_from_inclusive(inclusive, tax_percent);
        ItemAmounts {
            quantity_ordered: quantity,
            unit_price,
            quantity_alt_ordered: Decimal::ZERO,
            tax_percent,
            discount_percent: Decimal::ZERO,
            amount: split.net,
            tax_amount: split.tax,
            discount_amount: Decimal::ZERO,
        }
    }

    /// 单行含税合计，表头四个总额列与明细 total_amount 列共用同一公式
    ///
    /// 与人工建单链路汇总明细行金额的表达式同一口径（采购订单建单与入库重算表头总额
    /// 时都按明细行的含税合计累加），入参 amounts 为 suggestion_line_amounts 的产出，
    /// 产出值写 purchase_orders 的 total_amount 与折算前的基数、以及
    /// purchase_order_item 的 total_amount 列。
    fn suggestion_line_total(amounts: &ItemAmounts) -> Decimal {
        amounts.amount + amounts.tax_amount - amounts.discount_amount
    }

    /// 构建缺料预警采购订单 ActiveModel
    ///
    /// 入参 amounts 是本链路已经算好的单行金额，与 create_shortage_order_item 写
    /// purchase_order_item 的是同一份；由它汇总出的 total_amount、total_amount_foreign、
    /// total_quantity、total_quantity_alt 落 purchase_orders 同名列。这四列必须显式写值：
    /// 后三列是迁移脚本 ADD COLUMN 追加的既可空又无默认值的列，依赖 Default 会让插入
    /// 落 NULL 后读不回非 Option 的 Model，total_amount 则依赖表默认值掩盖真实金额。
    fn build_shortage_order(
        order_no: &str,
        supplier_id: i32,
        params: &ShortageAlertParams,
        amounts: &ItemAmounts,
    ) -> purchase_order::ActiveModel {
        let line_total = Self::suggestion_line_total(amounts);
        purchase_order::ActiveModel {
            order_no: Set(order_no.to_string()),
            supplier_id: Set(supplier_id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some(
                (Utc::now() + chrono::Duration::days(7)).date_naive(),
            )),
            warehouse_id: Set(crate::constants::DEFAULT_WAREHOUSE_ID),
            department_id: Set(crate::constants::DEFAULT_DEPARTMENT_ID),
            purchaser_id: Set(crate::constants::DEFAULT_PURCHASER_ID),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(AUTO_REPLENISHMENT_EXCHANGE_RATE),
            order_status: Set(po_status::DRAFT.to_string()),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 需求量: {} | 可用量: {} | 缺口: {} | 级别: {} | 受影响订单: {}",
                params.material_name,
                params.material_code,
                params.required_quantity,
                params.available_quantity,
                params.shortage_quantity,
                params.shortage_level,
                params.affected_orders_count
            ))),
            created_by: Set(1),
            total_amount: Set(line_total),
            total_amount_foreign: Set(line_total * AUTO_REPLENISHMENT_EXCHANGE_RATE),
            total_quantity: Set(amounts.quantity_ordered),
            total_quantity_alt: Set(amounts.quantity_alt_ordered),
            ..Default::default()
        }
    }

    /// 创建采购订单明细（金额与表头同源，均取自 amounts 这一份单行计算结果）
    ///
    /// 入参 amounts 为 suggestion_line_amounts 的产出，其中 tax_percent 是调用方
    /// （create_purchase_suggestion_from_shortage）显式声明的百分数口径税率（13 表示
    /// 13%，0 表示无进项税），税额经 utils tax 内核算出；本函数把它写进
    /// purchase_order_item 的 quantity、unit_price、subtotal、tax_percent、tax_amount、
    /// total_amount 列。
    async fn create_shortage_order_item(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        params: &ShortageAlertParams,
        priority: &str,
        amounts: &ItemAmounts,
    ) -> Result<(), AppError> {
        purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            // 自动补货单恒为一行建议，行号落 1，purchase_order_item.line_no 是 NOT NULL
            // 且无 DB 默认值的列，不显式写值会让整张建议单在 insert 阶段失败
            line_no: Set(1),
            product_id: Set(params.material_id),
            quantity: Set(amounts.quantity_ordered),
            quantity_alt: Set(amounts.quantity_alt_ordered),
            unit_price: Set(amounts.unit_price),
            unit_price_foreign: Set(amounts.unit_price),
            discount_percent: Set(amounts.discount_percent),
            tax_percent: Set(amounts.tax_percent),
            subtotal: Set(amounts.amount),
            tax_amount: Set(amounts.tax_amount),
            discount_amount: Set(amounts.discount_amount),
            total_amount: Set(Self::suggestion_line_total(amounts)),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!(
                "缺料预警自动生成 | 物料: {} ({}) | 优先级: {}",
                params.material_name, params.material_code, priority
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(())
    }

    /// 获取补货所需的产品、仓库和默认活跃供应商
    async fn fetch_replenishment_entities(
        txn: &sea_orm::DatabaseTransaction,
        product_id: i32,
        warehouse_id: i32,
    ) -> Result<(product::Model, warehouse::Model, supplier::Model), AppError> {
        let product = product::Entity::find_by_id(product_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", product_id)))?;
        let warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库 ID {} 不存在", warehouse_id)))?;
        let supplier = supplier::Entity::find()
            .filter(supplier::Column::Status.eq(master_data::ACTIVE))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("没有可用的活跃供应商"))?;
        Ok((product, warehouse, supplier))
    }

    /// 构建库存预警采购订单 ActiveModel
    ///
    /// 入参 amounts 是本链路已经算好的单行金额，与 create_replenishment_order_item 写
    /// purchase_order_item 的是同一份；由它汇总出的 total_amount、total_amount_foreign、
    /// total_quantity、total_quantity_alt 落 purchase_orders 同名列，四列必须显式写值的
    /// 理由与 build_shortage_order 相同。
    fn build_replenishment_order(
        order_no: String,
        supplier_id: i32,
        warehouse_id: i32,
        current_quantity: Decimal,
        reorder_point: Decimal,
        reorder_quantity: Decimal,
        amounts: &ItemAmounts,
    ) -> purchase_order::ActiveModel {
        let line_total = Self::suggestion_line_total(amounts);
        purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(supplier_id),
            order_date: Set(Utc::now().date_naive()),
            expected_delivery_date: Set(Some(
                (Utc::now() + chrono::Duration::days(7)).date_naive(),
            )),
            warehouse_id: Set(warehouse_id),
            department_id: Set(crate::constants::DEFAULT_DEPARTMENT_ID),
            purchaser_id: Set(crate::constants::DEFAULT_PURCHASER_ID),
            currency: Set(crate::constants::DEFAULT_CURRENCY.to_string()),
            exchange_rate: Set(AUTO_REPLENISHMENT_EXCHANGE_RATE),
            order_status: Set(po_status::DRAFT.to_string()),
            notes: Set(Some(format!(
                "库存预警自动生成，当前库存: {}, 补货点: {}, 建议补货量: {}",
                current_quantity, reorder_point, reorder_quantity
            ))),
            created_by: Set(1),
            total_amount: Set(line_total),
            total_amount_foreign: Set(line_total * AUTO_REPLENISHMENT_EXCHANGE_RATE),
            total_quantity: Set(amounts.quantity_ordered),
            total_quantity_alt: Set(amounts.quantity_alt_ordered),
            ..Default::default()
        }
    }

    /// 创建库存预警订单明细(金额与表头同源，均取自 amounts 这一份单行计算结果)
    ///
    /// 入参 amounts 为 suggestion_line_amounts 的产出，其中 tax_percent 是调用方
    /// （create_purchase_suggestion）显式声明的百分数口径税率（13 表示 13%，0 表示
    /// 无进项税），税额经 utils tax 内核算出；本函数把它写进 purchase_order_item 的
    /// quantity、unit_price、subtotal、tax_percent、tax_amount、total_amount 列。
    async fn create_replenishment_order_item(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        product: &product::Model,
        amounts: &ItemAmounts,
    ) -> Result<(), AppError> {
        purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            // 同上，purchase_order_item.line_no 为 NOT NULL 且无 DB 默认值，
            // 库存预警补货建议恒一行，行号落 1
            line_no: Set(1),
            product_id: Set(product.id),
            quantity: Set(amounts.quantity_ordered),
            quantity_alt: Set(amounts.quantity_alt_ordered),
            unit_price: Set(amounts.unit_price),
            unit_price_foreign: Set(amounts.unit_price),
            discount_percent: Set(amounts.discount_percent),
            tax_percent: Set(amounts.tax_percent),
            subtotal: Set(amounts.amount),
            tax_amount: Set(amounts.tax_amount),
            discount_amount: Set(amounts.discount_amount),
            total_amount: Set(Self::suggestion_line_total(amounts)),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(Some(format!("库存预警自动生成，产品: {}", product.name))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(())
    }

    /// 根据库存预警创建采购建议(事务内创建订单与明细,commit 后返回)
    pub async fn create_purchase_suggestion(
        &self,
        product_id: i32,
        warehouse_id: i32,
        current_quantity: Decimal,
        reorder_point: Decimal,
        reorder_quantity: Decimal,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let (product, warehouse, supplier) =
            Self::fetch_replenishment_entities(&txn, product_id, warehouse_id).await?;
        // 同缺料预警链路，无税率来源即按无进项税落账（口径值来自 utils tax 内核），
        // 不填任何税率档位
        let tax_percent = crate::utils::tax::NO_INPUT_TAX_PERCENT;
        tracing::info!(
            "库存预警采购建议按无进项税落账: 产品={}，原因=该链路无税率来源且系统不设默认税率档位",
            product.name
        );
        let order_no = self.generate_order_no_with_txn(&txn).await?;
        // 单行金额只算一次，表头四个总额列与明细行共用这一份，避免两处各写一套公式
        let amounts = Self::suggestion_line_amounts(&product, reorder_quantity, tax_percent);
        let order = Self::build_replenishment_order(
            order_no,
            supplier.id,
            warehouse_id,
            current_quantity,
            reorder_point,
            reorder_quantity,
            &amounts,
        )
        .insert(&txn)
        .await?;
        Self::create_replenishment_order_item(&txn, order.id, &product, &amounts).await?;
        txn.commit().await?;
        tracing::info!(
            "创建库存预警采购建议: 订单号={}, 产品={}, 仓库={}, 建议采购量={}",
            order.order_no,
            product.name,
            warehouse.name,
            reorder_quantity
        );
        Ok(order)
    }
}
