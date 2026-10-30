//! 销售订单 → 销售合同 转换服务
//!
//! 业务功能：
//! - 回读指定销售订单表头与全部明细行，带出合同主体与行项的预填数据（preview）；
//! - 校验订单可转状态与重复转单门后，在同一事务内取号、建合同头、逐行落合同明细，
//!   行金额与税额由服务端回读订单行的折扣率与税率并经 services/so/amounts 统一重算，
//!   客户端只送数量与单价等可编辑字段，不申报折扣率/税率/金额（convert）。
//!
//! 幂等口径（裁定与依据）：
//! - 同一订单允许存在多条转换合同，但**非终态**（draft/active）合同至多一条：
//!   转换前在事务内对订单行加排他锁串行化并发转换，再查 source_order_id 命中且
//!   status 不在 (cancelled, rejected) 的合同，命中即业务拒绝并在文案中点名既有
//!   合同编号，前端据 preview 的 existing_contract（含 ID）提供跳转，不再生成。
//!   依据：作废/驳回合同不承载有效权利义务，重试转换是行业常态（重新签约）；
//!   部分成交拆签（一单两约）亦为纺织外贸常见诉求，故不做订单级硬唯一约束。
//!   回退：若业务裁定"一单一号"，只需在判据处把状态集合收窄或补部分唯一索引。
//! - 订单状态门（与销售订单生产入口门同源，见 ensure_convertible_status）：只放行
//!   已审核起的活跃态 approved/partial_shipped/shipped/completed，draft/pending（未审核）
//!   与 cancelled/rejected（失效）一律拒绝。依据：本仓订单进入下游履约的统一前置是
//!   「已审核」，发货门要 approved/partial_shipped、拒绝取消门只认活跃态，转合同与之
//!   同为审核通过后才允许推进的动作，门须逐态一致，避免未审核草稿已能签约却连货都不能发
//!   的自相矛盾；含 shipped/completed 是事后补签真实存在，合同仍走自身审批链无不可逆效果。
//!   回退：若业务裁定未审核也允许预签，只需放开 ensure_convertible_status 的放行集一处
//!   matches!，但须同步重开发货侧门以维持同源。
//!
//! 金额同源：行折算与表头合计一律走 `crate::services::so::amounts`，与订单建单
//! 链共用同一实现；本文件零自算金额。
//!
//! 参照范式：仓库既有报价转销售订单（services/quotation_convert_service.rs），
//! 取号复用 DocumentNumberGenerator::insert_with_no_retry（调用方事务内取号臂）。

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::models::product;
use crate::models::sales_contract;
use crate::models::sales_contract_item;
use crate::models::sales_order;
use crate::models::sales_order_item;
use crate::models::status::contract as contract_status;
use crate::models::status::sales::sales_order as so_status;
use crate::services::sales_contract_service::SalesContractService;
use crate::services::so::amounts::{self, SalesLineAmounts};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

/// 转换入参的单行（可编辑字段白名单）。
///
/// 折扣率与税率不在入参内：由服务端回读 sales_order_items 原值（单一来源），
/// 客户端申报任何折扣/税额数字都不会被采信（防前端两套算法漂移）。
#[derive(Debug, Clone)]
pub struct ConvertOrderItemInput {
    /// 来源订单明细行 ID（sales_order_items.id），必须属于本订单且全量覆盖
    pub order_item_id: i32,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    /// 行交期：None = 沿用订单表头要求交期（preview 已带出默认值）
    pub delivery_date: Option<chrono::NaiveDate>,
    /// 色号覆盖：None = 保持订单行色号原值；Some = 用户编辑后的值
    /// （空串 = 白坯布，与订单侧同一语义，长度上限与订单入参同口径 50）
    pub color_no: Option<String>,
    /// 交货数量允差百分比：None = 保持订单行值，Some = 覆盖（[0,100] 已在 DTO 校验）
    pub quantity_tolerance_pct: Option<Decimal>,
    /// 行备注：None = 不覆盖（落订单行 notes，超长由 DTO 校验拦截），Some = 覆盖
    pub remarks: Option<Option<String>>,
}

/// 转换入参（表头可编辑字段 + 明细行集合）。
///
/// 客户不在入参内：合同签约主体必须与订单一致，服务端直取订单 customer_id。
#[derive(Debug, Clone)]
pub struct ConvertFromOrderRequest {
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub signed_date: Option<chrono::NaiveDate>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 合同表头交期：None = 沿用订单要求交期
    pub delivery_date: Option<chrono::NaiveDate>,
    pub delivery_location: Option<String>,
    pub remark: Option<String>,
    pub items: Vec<ConvertOrderItemInput>,
}

/// preview 出参：订单带入合同的预填数据（表头默认值 + 行集合 + 既有非终态合同）。
#[derive(Debug, serde::Serialize)]
pub struct ContractPreview {
    pub order_id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    /// 表头默认预填：合同名（销售合同-{order_no}）、交期（订单要求交期）、
    /// 收货地点（来源订单收货地址 shipping_address，用户在合同表单可改）、
    /// 备注（[源自销售订单 {order_no}] + 订单备注）
    pub default_contract_name: String,
    pub default_delivery_date: Option<chrono::NaiveDate>,
    pub default_delivery_location: Option<String>,
    pub default_remark: Option<String>,
    pub items: Vec<ContractPreviewItem>,
    /// 该订单当前是否存在非终态（draft/active）转换合同：存在则前端给出跳转提示、
    /// 不再提交转换；含合同 ID 供导航
    pub existing_contract: Option<ExistingContract>,
}

/// preview 既有合同指针
#[derive(Debug, serde::Serialize)]
pub struct ExistingContract {
    pub id: i32,
    pub contract_no: String,
    pub status: String,
}

/// preview 单行预填。金额各列为订单行存量值（订单建单链经 amounts 模块产出），
/// 产品编码/幅宽/克重/成分自产品主档带出（合同行无这些快照列，仅界面展示与确认用）。
#[derive(Debug, serde::Serialize)]
pub struct ContractPreviewItem {
    pub order_item_id: i32,
    pub product_id: i32,
    pub product_code: String,
    pub product_name: String,
    pub product_spec: Option<String>,
    pub unit: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub discount_percent: Decimal,
    pub discount_amount: Decimal,
    pub tax_percent: Decimal,
    pub tax_amount: Decimal,
    pub subtotal: Decimal,
    pub total_amount: Decimal,
    pub color_no: String,
    pub width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    pub fabric_composition: Option<String>,
    pub quantity_tolerance_pct: Option<Decimal>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
}

/// 转换成功后返回合同头与全部明细行（金额以服务端重算为准）
#[derive(Debug, serde::Serialize)]
pub struct ConvertResult {
    pub contract: sales_contract::Model,
    pub items: Vec<sales_contract_item::Model>,
}

pub struct SalesContractConvertService {
    db: Arc<DatabaseConnection>,
}

impl SalesContractConvertService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::container::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 可转状态门（与销售订单生产入口门同源）。
    ///
    /// 存在理由：本仓销售订单进入下游履约的统一前置是「已审核」，发货门
    /// （services/so/delivery_ops/ship.rs）判定 order.status 为 approved 或
    /// partial_shipped 才放行，拒绝与取消门（services/so/order_workflow.rs）也只
    /// 认 draft/pending/approved/partial_shipped 这一组活跃态。转合同与发货同属
    /// 「订单审核通过之后才允许推进的下游动作」，前置门必须与之逐态一致，否则会出现
    /// 一张尚未审核通过的草稿订单已能签出合同、而它连货都还不能发的自相矛盾。
    ///
    /// 因此放行集取「已审核且尚未了结的活跃态」approved/partial_shipped/shipped/
    /// completed（含 shipped/completed 是纺织外贸事后补签合同真实存在，与发货门不冲突，
    /// 合同仍需走自身审批链）；draft/pending（未审核）与 cancelled/rejected（失效）一律
    /// 拒绝。取值只引用 sales_order 词表常量，门内不写第二套字面量。
    fn ensure_convertible_status(status: &str) -> Result<(), AppError> {
        if matches!(
            status,
            so_status::APPROVED
                | so_status::PARTIAL_SHIPPED
                | so_status::SHIPPED
                | so_status::COMPLETED
        ) {
            return Ok(());
        }
        if matches!(status, so_status::DRAFT | so_status::PENDING) {
            return Err(AppError::business(format!(
                "销售订单状态为 {}，尚未审核通过，不能转销售合同；请先完成订单审核",
                status
            )));
        }
        Err(AppError::business(format!(
            "销售订单状态为 {}，已了结或失效，不能转销售合同",
            status
        )))
    }

    /// 读取订单全部明细行并按主键升序返回（preview 与 convert 共用形态）
    async fn load_order_items<C: sea_orm::ConnectionTrait>(
        order_id: i32,
        conn: &C,
    ) -> Result<Vec<sales_order_item::Model>, AppError> {
        Ok(sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .order_by(sales_order_item::Column::Id, Order::Asc)
            .all(conn)
            .await?)
    }

    /// 批量取产品主档（编码/单位/规格/成分/幅宽/克重的唯一来源）
    async fn load_product_map<C: sea_orm::ConnectionTrait>(
        items: &[sales_order_item::Model],
        conn: &C,
    ) -> Result<HashMap<i32, product::Model>, AppError> {
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        if product_ids.is_empty() {
            return Ok(HashMap::new());
        }
        Ok(product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids))
            .all(conn)
            .await?
            .into_iter()
            .map(|p| (p.id, p))
            .collect())
    }

    /// 查询该订单的非终态转换合同（幂等判据），命中返回最早一条
    async fn find_effective_converted_contract<C: sea_orm::ConnectionTrait>(
        order_id: i32,
        conn: &C,
    ) -> Result<Option<sales_contract::Model>, AppError> {
        Ok(sales_contract::Entity::find()
            .filter(sales_contract::Column::SourceOrderId.eq(order_id))
            .filter(
                sales_contract::Column::Status
                    .is_in([contract_status::DRAFT, contract_status::ACTIVE]),
            )
            .order_by(sales_contract::Column::Id, Order::Asc)
            .one(conn)
            .await?)
    }

    /// 预填数据读取：订单表头 + 全部行（金额列原样带出） + 产品主档补充列 +
    /// 既有非终态合同指针。只读、不加锁（真正互斥在 convert 事务内）。
    /// 行级归属门由 handler 侧 `SalesService::assert_order_ownership` 前置（
    /// 与报价转订单 handler 门同型），本方法不重复判定。
    pub async fn preview(&self, order_id: i32) -> Result<ContractPreview, AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("销售订单不存在"))?;

        let items = Self::load_order_items(order_id, &*self.db).await?;
        let products = Self::load_product_map(&items, &*self.db).await?;
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?;
        let existing = Self::find_effective_converted_contract(order_id, &*self.db).await?;

        let header_delivery_date = order.required_date.map(|dt| dt.date_naive());
        let default_remark = Some(format!(
            "[源自销售订单 {}] {}",
            order.order_no,
            order.notes.clone().unwrap_or_default()
        ));

        let mut preview_items = Vec::with_capacity(items.len());
        for item in &items {
            let product = products.get(&item.product_id).ok_or_else(|| {
                AppError::validation(format!(
                    "订单明细 {} 引用的产品 {} 不存在（悬挂引用不允许转合同）",
                    item.id, item.product_id
                ))
            })?;
            preview_items.push(ContractPreviewItem {
                order_item_id: item.id,
                product_id: item.product_id,
                product_code: product.code.clone(),
                product_name: product.name.clone(),
                product_spec: product.specification.clone(),
                unit: product.unit.clone(),
                quantity: item.quantity,
                unit_price: item.unit_price,
                discount_percent: item.discount_percent,
                discount_amount: item.discount_amount,
                tax_percent: item.tax_percent,
                tax_amount: item.tax_amount,
                subtotal: item.subtotal,
                total_amount: item.total_amount,
                color_no: item.color_no.clone(),
                width: item.width,
                gram_weight: item.gram_weight,
                fabric_composition: product.fabric_composition.clone(),
                quantity_tolerance_pct: item.quantity_tolerance_pct,
                delivery_date: header_delivery_date,
                remarks: item.notes.clone(),
            });
        }

        Ok(ContractPreview {
            order_id: order.id,
            order_no: order.order_no.clone(),
            customer_id: order.customer_id,
            customer_name: customer.map(|c| c.customer_name),
            default_contract_name: format!("销售合同-{}", order.order_no),
            default_delivery_date: header_delivery_date,
            default_delivery_location: order.shipping_address.clone(),
            default_remark,
            items: preview_items,
            existing_contract: existing.map(|c| ExistingContract {
                id: c.id,
                contract_no: c.contract_no,
                status: c.status,
            }),
        })
    }

    /// 事务化转换：锁订单行 → 状态门 → 幂等门 → 取号建头 → 逐行落库。
    ///
    /// 明细行必须与订单行**全量一一对应**（不允许删行）：部分成交签约走手工建合同
    /// 链路，本入口的语义是"整单转合同"，保证 source_order_item_id 溯源链完整。
    /// 行级归属门由 handler 侧前置（与 preview 同口径）。
    pub async fn convert(
        &self,
        order_id: i32,
        req: ConvertFromOrderRequest,
        user_id: i32,
    ) -> Result<ConvertResult, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 订单行加排他锁：串行化同一订单的并发转换（幂等判据与插入同事务生效）
        let order = sales_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("销售订单不存在"))?;

        // 2. 状态门
        Self::ensure_convertible_status(&order.status)?;

        // 3. 幂等门：非终态合同已存在即拒绝（并发下由步骤 1 的行锁兜底）
        if let Some(exist) = Self::find_effective_converted_contract(order_id, &txn).await? {
            return Err(AppError::business_displayable(format!(
                "该销售订单已存在有效销售合同 {}，不能重复转换；请先处理既有合同（编辑或作废）或使用该合同",
                exist.contract_no
            )));
        }

        // 4. 订单全部行 + 产品主档
        let order_items = Self::load_order_items(order_id, &txn).await?;
        if order_items.is_empty() {
            return Err(AppError::business("销售订单没有明细行，无法转销售合同"));
        }
        let product_map = Self::load_product_map(&order_items, &txn).await?;
        let item_map: HashMap<i32, &sales_order_item::Model> =
            order_items.iter().map(|i| (i.id, i)).collect();

        // 5. 入参行校验：order_item_id 必须属于本订单、不得重复、必须全量覆盖
        Self::validate_item_coverage(&req.items, &item_map)?;

        // 6. 表头金额：逐行走 amounts 模块（折扣率/税率取自订单行存量值），
        //    合计口径与订单表头一致（含税总额 = Σ 行折后+税额）
        let lines: Vec<SalesLineAmounts> = req
            .items
            .iter()
            .map(|li| {
                let src = item_map[&li.order_item_id];
                amounts::calculate_line_amounts(
                    li.quantity,
                    li.unit_price,
                    Some(src.discount_percent),
                    Some(src.tax_percent),
                )
            })
            .collect();
        let totals = amounts::sum_line_amounts(&lines);

        // 7. 建合同头：编号走系统取号（调用方事务内臂），客户与订单一致不可改
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::business_displayable("转换失败：订单客户不存在"))?;

        let header_delivery_date = order.required_date.map(|dt| dt.date_naive());
        let create_req = crate::services::sales_contract_service::CreateSalesContractRequest {
            contract_no: None,
            contract_name: req.contract_name.clone(),
            customer_id: order.customer_id,
            total_amount: totals.total_amount,
            contract_type: req.contract_type.clone(),
            payment_terms: req.payment_terms.clone(),
            delivery_date: req.delivery_date.or(header_delivery_date),
            signed_date: req.signed_date,
            effective_date: req.effective_date,
            expiry_date: req.expiry_date,
            payment_method: req.payment_method.clone(),
            // 收货地点：用户编辑值优先，未编辑则沿用来源订单收货地址（与 preview 默认同源）
            delivery_location: req
                .delivery_location
                .clone()
                .or_else(|| order.shipping_address.clone()),
            remark: req.remark.clone(),
            source_order_id: Some(order_id),
            // 转合同入参 ConvertFromOrderRequest 由订单表头与订单行带出，不含模板主键、
            // 条款填充值、内外销标记与含税标记，四项如实保持未采集：条款固化腿因无模板
            // 而整段跳过（合同条款快照为空），sales_contracts 的 contract_market 与
            // tax_inclusive 落 NULL，详情与打印按未选模板签约的口径回显。
            template_id: None,
            clause_values: None,
            contract_market: None,
            tax_inclusive: None,
            items: None,
        };
        let stamp_tax = SalesContractService::calculate_stamp_tax(
            create_req.contract_type.as_deref(),
            create_req.total_amount,
        );

        let contract = DocumentNumberGenerator::insert_with_no_retry(
            &txn,
            crate::services::sales_contract_service::SALES_CONTRACT_NO_PREFIX,
            sales_contract::Entity,
            sales_contract::Column::ContractNo,
            |no| {
                SalesContractService::build_sales_contract_active_model(
                    no,
                    &create_req,
                    customer.customer_name.clone(),
                    stamp_tax,
                    user_id,
                )
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                prefix = crate::services::sales_contract_service::SALES_CONTRACT_NO_PREFIX,
                "销售订单转合同时取号失败（sales_contract_convert_service.convert）"
            );
            AppError::business_displayable("销售合同编号生成失败，请稍后重试")
        })?;

        // 8. 逐行落合同明细（金额列 = amounts 重算值；折扣率/税率/色号源自订单行）
        let mut saved_items = Vec::with_capacity(req.items.len());
        for (idx, li) in req.items.iter().enumerate() {
            let src = item_map[&li.order_item_id];
            let line = lines[idx];
            let product = product_map.get(&src.product_id).ok_or_else(|| {
                AppError::validation(format!(
                    "订单明细 {} 引用的产品 {} 不存在（悬挂引用不允许转合同）",
                    src.id, src.product_id
                ))
            })?;
            let now = chrono::Utc::now();
            let active = sales_contract_item::ActiveModel {
                contract_id: Set(contract.id),
                product_id: Set(Some(src.product_id)),
                product_name: Set(product.name.clone()),
                product_spec: Set(product.specification.clone()),
                unit: Set(product.unit.clone()),
                quantity: Set(li.quantity),
                quantity_tolerance_pct: Set(li
                    .quantity_tolerance_pct
                    .or(src.quantity_tolerance_pct)),
                unit_price: Set(li.unit_price),
                amount: Set(line.subtotal),
                delivery_date: Set(li.delivery_date.or(header_delivery_date)),
                color_no: Set(Some(
                    li.color_no.clone().unwrap_or_else(|| src.color_no.clone()),
                )),
                discount_percent: Set(Some(line.discount_percent)),
                discount_amount: Set(Some(line.discount_amount)),
                tax_percent: Set(Some(line.tax_percent)),
                tax_amount: Set(Some(line.tax_amount)),
                source_order_item_id: Set(Some(src.id)),
                remarks: Set(match &li.remarks {
                    Some(v) => v.clone(),
                    None => src.notes.clone(),
                }),
                sort_order: Set(idx as i32),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            saved_items.push(active.insert(&txn).await?);
        }

        txn.commit().await?;
        tracing::info!(
            "销售订单 {} 已转为销售合同 {}（{} 行，用户 {}）",
            order.order_no,
            contract.contract_no,
            saved_items.len(),
            user_id
        );
        Ok(ConvertResult {
            contract,
            items: saved_items,
        })
    }

    /// 入参行与订单行的对应关系校验：属于本订单、不重复、全量覆盖
    fn validate_item_coverage(
        items: &[ConvertOrderItemInput],
        item_map: &HashMap<i32, &sales_order_item::Model>,
    ) -> Result<(), AppError> {
        let mut seen = HashSet::with_capacity(items.len());
        for li in items {
            if !item_map.contains_key(&li.order_item_id) {
                return Err(AppError::validation(format!(
                    "转换明细包含非本订单的行 {}，已拒绝",
                    li.order_item_id
                )));
            }
            if !seen.insert(li.order_item_id) {
                return Err(AppError::validation(format!(
                    "转换明细包含重复的订单行 {}，已拒绝",
                    li.order_item_id
                )));
            }
        }
        let missing: Vec<i32> = item_map
            .keys()
            .filter(|id| !seen.contains(id))
            .copied()
            .collect();
        if !missing.is_empty() {
            return Err(AppError::validation(format!(
                "整单转合同要求携带全部订单明细行，缺少行 {:?}",
                missing
            )));
        }
        Ok(())
    }
}
