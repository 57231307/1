//! 缺料预警 Service
//!
//! 提供缺料检测、预警阈值配置、缺料清单生成等功能
//!
//! V15 P0-B15（Batch 484）：修复审计报告 batch-18 §8.1 缺陷
//! 缺料预警状态持久化 — save/load/update_status 三个方法从桩实现改为真实 DB 读写，
//! detect_shortages 检测到缺料时持久化 alert 快照到 material_shortage_alerts 表，
//! 支持识别→采购申请→采购订单→入库→解除闭环。

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::bom::{self, Column as BomColumn, Entity as BomEntity};
use crate::models::bom_item::{self, Column as BomItemColumn, Entity as BomItemEntity};
use crate::models::inventory_stock::{Column as StockColumn, Entity as InventoryStockEntity};
use crate::models::material_shortage as alert_model;
use crate::models::material_shortage::threshold_config as threshold_model;
use crate::models::product::{Column as ProductColumn, Entity as ProductEntity};
use crate::models::production_order::{
    self, Column as ProductionOrderColumn, Entity as ProductionOrderEntity,
};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;

/// 缺料预警级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShortageLevel {
    /// 紧急：库存为零
    Critical,
    /// 严重：缺口 > 50%
    Severe,
    /// 一般：缺口 <= 50%
    Warning,
    /// 正常：无缺口
    Normal,
}

impl ShortageLevel {
    pub fn from_deficit_rate(rate: Decimal) -> Self {
        if rate >= Decimal::from(100) {
            ShortageLevel::Critical
        } else if rate > Decimal::from(50) {
            ShortageLevel::Severe
        } else if rate > Decimal::ZERO {
            ShortageLevel::Warning
        } else {
            ShortageLevel::Normal
        }
    }
}

/// 缺料预警项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialShortageItem {
    pub material_id: i32,
    pub material_name: String,
    pub material_code: String,
    pub required_quantity: Decimal,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub deficit_rate: Decimal,
    pub level: ShortageLevel,
    pub affected_orders: Vec<AffectedOrder>,
    pub unit: Option<String>,
}

/// 受影响的生产订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedOrder {
    pub order_id: i32,
    pub order_no: String,
    pub demand_quantity: Decimal,
    pub planned_end_date: Option<NaiveDate>,
}

/// 缺料汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortageSummary {
    pub total_materials_checked: i64,
    pub shortage_count: i64,
    pub critical_count: i64,
    pub severe_count: i64,
    pub warning_count: i64,
    pub affected_orders_count: i64,
    pub items: Vec<MaterialShortageItem>,
}

/// 预警阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortageThresholdConfig {
    /// 安全库存倍率（低于安全库存 * 此倍率时触发预警）
    pub safety_factor: Decimal,
    /// 紧急阈值：缺口百分比 >= 此值为紧急
    pub critical_threshold: Decimal,
    /// 严重阈值：缺口百分比 >= 此值为严重
    pub severe_threshold: Decimal,
}

impl Default for ShortageThresholdConfig {
    fn default() -> Self {
        Self {
            safety_factor: Decimal::from(1),
            critical_threshold: Decimal::from(100),
            severe_threshold: Decimal::from(50),
        }
    }
}

/// 手动检查请求
#[derive(Debug, Clone, Deserialize)]
pub struct ShortageCheckRequest {
    pub product_ids: Option<Vec<i32>>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub threshold: Option<ShortageThresholdConfig>,
}

// 批次 326 v10 复审 P2 修复：提取类型别名消除 type_complexity 警告
type MaterialReq = (Decimal, Option<String>, Vec<(i32, Decimal)>);

/// 缺料统计计数（内部传递用）
struct ShortageCounts {
    critical: i64,
    severe: i64,
    warning: i64,
    affected_order_ids: std::collections::HashSet<i32>,
}

impl ShortageCounts {
    fn new() -> Self {
        Self {
            critical: 0,
            severe: 0,
            warning: 0,
            affected_order_ids: std::collections::HashSet::new(),
        }
    }

    fn update(&mut self, level: &ShortageLevel, affected: &[AffectedOrder]) {
        match level {
            ShortageLevel::Critical => self.critical += 1,
            ShortageLevel::Severe => self.severe += 1,
            ShortageLevel::Warning => self.warning += 1,
            _ => {}
        }
        for ao in affected {
            self.affected_order_ids.insert(ao.order_id);
        }
    }
}

/// 缺料预警 Service
pub struct MaterialShortageService {
    db: Arc<DatabaseConnection>,
}

impl MaterialShortageService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 执行缺料检测
    pub async fn detect_shortages(
        &self,
        request: ShortageCheckRequest,
    ) -> Result<ShortageSummary, AppError> {
        let _threshold = request.threshold.unwrap_or_default();

        // 1. 查询活跃生产订单
        let orders = self.query_active_orders(&request).await?;
        if orders.is_empty() {
            return Ok(Self::empty_summary());
        }

        // 2. 按产品聚合需求
        let (product_demands, product_orders) = Self::aggregate_product_demands(&orders);

        // 3. 查询默认 BOM 及物料
        let product_ids: Vec<i32> = product_demands.keys().cloned().collect();
        let boms = self.query_default_boms(&product_ids).await?;
        let bom_ids: Vec<i32> = boms.iter().map(|b| b.id).collect();
        let product_to_bom: HashMap<i32, i32> =
            boms.iter().map(|b| (b.product_id, b.id)).collect();
        let bom_items = self.query_bom_items(&bom_ids).await?;

        // 4. 计算物料总需求
        let material_requirements =
            Self::compute_material_requirements(&bom_items, &product_to_bom, &product_demands);

        // 5. 查询库存和名称
        let material_ids: Vec<i32> = material_requirements.keys().cloned().collect();
        let stock_map = self.get_material_stock_map(&material_ids).await?;
        let material_names = self.get_product_names(&material_ids).await?;

        // 6. 汇总受影响订单
        let material_affected_orders = Self::aggregate_material_affected_orders(
            &bom_items,
            &product_to_bom,
            &product_orders,
        );

        // 7. 生成缺料清单并排序
        let (mut items, counts) = Self::build_shortage_items(
            &material_requirements,
            &stock_map,
            &material_names,
            &material_affected_orders,
        );
        Self::sort_items_by_level(&mut items);

        // 8. 持久化 alert（失败降级 warn，不阻断检测）
        if let Err(e) = self.persist_alerts(&items).await {
            tracing::warn!(
                error = %e,
                "persist_alerts 持久化缺料预警失败（不阻断检测，降级为 warn）"
            );
        }

        Ok(Self::build_summary(items, &material_requirements, counts))
    }

    /// 查询活跃生产订单（含产品/日期过滤）
    async fn query_active_orders(
        &self,
        request: &ShortageCheckRequest,
    ) -> Result<Vec<production_order::Model>, AppError> {
        let mut query = ProductionOrderEntity::find()
            .filter(ProductionOrderColumn::Status.is_in(vec!["SCHEDULED", "IN_PROGRESS"]));
        if let Some(ref product_ids) = request.product_ids {
            query = query.filter(ProductionOrderColumn::ProductId.is_in(product_ids.clone()));
        }
        if let Some(from) = request.date_from {
            query = query.filter(ProductionOrderColumn::PlannedEndDate.gte(from));
        }
        if let Some(to) = request.date_to {
            query = query.filter(ProductionOrderColumn::PlannedStartDate.lte(to));
        }
        Ok(query.all(&*self.db).await?)
    }

    /// 构建空汇总（无活跃订单时返回）
    fn empty_summary() -> ShortageSummary {
        ShortageSummary {
            total_materials_checked: 0,
            shortage_count: 0,
            critical_count: 0,
            severe_count: 0,
            warning_count: 0,
            affected_orders_count: 0,
            items: vec![],
        }
    }

    /// 按产品聚合需求量和受影响订单
    fn aggregate_product_demands(
        orders: &[production_order::Model],
    ) -> (HashMap<i32, Decimal>, HashMap<i32, Vec<AffectedOrder>>) {
        let mut product_demands: HashMap<i32, Decimal> = HashMap::new();
        let mut product_orders: HashMap<i32, Vec<AffectedOrder>> = HashMap::new();
        for order in orders {
            *product_demands
                .entry(order.product_id)
                .or_insert(Decimal::ZERO) += order.planned_quantity;
            product_orders
                .entry(order.product_id)
                .or_default()
                .push(AffectedOrder {
                    order_id: order.id,
                    order_no: order.order_no.clone(),
                    demand_quantity: order.planned_quantity,
                    planned_end_date: order.planned_end_date,
                });
        }
        (product_demands, product_orders)
    }

    /// 查询产品默认 BOM
    async fn query_default_boms(&self, product_ids: &[i32]) -> Result<Vec<bom::Model>, AppError> {
        if product_ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(BomEntity::find()
            .filter(BomColumn::ProductId.is_in(product_ids.to_vec()))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq("ACTIVE"))
            .all(&*self.db)
            .await?)
    }

    /// 查询 BOM 明细
    async fn query_bom_items(&self, bom_ids: &[i32]) -> Result<Vec<bom_item::Model>, AppError> {
        if bom_ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(BomItemEntity::find()
            .filter(BomItemColumn::BomId.is_in(bom_ids.to_vec()))
            .all(&*self.db)
            .await?)
    }

    /// 计算每种物料的总需求
    fn compute_material_requirements(
        bom_items: &[bom_item::Model],
        product_to_bom: &HashMap<i32, i32>,
        product_demands: &HashMap<i32, Decimal>,
    ) -> HashMap<i32, MaterialReq> {
        let mut material_requirements: HashMap<i32, MaterialReq> = HashMap::new();
        for item in bom_items {
            for (product_id, bom_id) in product_to_bom {
                if *bom_id != item.bom_id {
                    continue;
                }
                if let Some(&demand) = product_demands.get(product_id) {
                    let scrap_rate = item.scrap_rate.unwrap_or(Decimal::ZERO);
                    // 批次 97 P1-9 修复（v5 复审）：数量计算补 round_dp(4) 防止精度漂移
                    let qty_per_unit = (item.quantity * (Decimal::ONE + scrap_rate)).round_dp(4);
                    let total_for_product = (qty_per_unit * demand).round_dp(4);
                    let entry = material_requirements
                        .entry(item.material_id)
                        .or_insert((Decimal::ZERO, item.unit.clone(), vec![]));
                    entry.0 += total_for_product;
                    entry.2.push((*product_id, qty_per_unit));
                }
            }
        }
        material_requirements
    }

    /// 汇总每个物料受影响的订单
    fn aggregate_material_affected_orders(
        bom_items: &[bom_item::Model],
        product_to_bom: &HashMap<i32, i32>,
        product_orders: &HashMap<i32, Vec<AffectedOrder>>,
    ) -> HashMap<i32, Vec<AffectedOrder>> {
        let mut material_affected_orders: HashMap<i32, Vec<AffectedOrder>> = HashMap::new();
        for item in bom_items {
            for (product_id, bom_id) in product_to_bom {
                if *bom_id != item.bom_id {
                    continue;
                }
                if let Some(orders) = product_orders.get(product_id) {
                    material_affected_orders
                        .entry(item.material_id)
                        .or_default()
                        .extend(orders.clone());
                }
            }
        }
        material_affected_orders
    }

    /// 计算缺口率
    fn compute_deficit_rate(required: Decimal, shortage: Decimal) -> Decimal {
        if required > Decimal::ZERO {
            ((shortage / required) * Decimal::from(100))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        } else {
            Decimal::ZERO
        }
    }

    /// 发布缺料预警事件
    fn publish_shortage_event(item: &MaterialShortageItem) {
        EVENT_BUS.publish(BusinessEvent::MaterialShortageAlert {
            material_id: item.material_id,
            material_name: item.material_name.clone(),
            material_code: item.material_code.clone(),
            required_quantity: item.required_quantity,
            available_quantity: item.available_quantity,
            shortage_quantity: item.shortage_quantity,
            shortage_level: format!("{:?}", item.level),
            affected_orders_count: item.affected_orders.len() as i32,
        });
    }

    /// 生成缺料清单
    fn build_shortage_items(
        material_requirements: &HashMap<i32, MaterialReq>,
        stock_map: &HashMap<i32, Decimal>,
        material_names: &HashMap<i32, (String, String)>,
        material_affected_orders: &HashMap<i32, Vec<AffectedOrder>>,
    ) -> (Vec<MaterialShortageItem>, ShortageCounts) {
        let mut items = Vec::new();
        let mut counts = ShortageCounts::new();
        for (material_id, (required, unit, _)) in material_requirements {
            let available = stock_map.get(material_id).copied().unwrap_or(Decimal::ZERO);
            let shortage = if required > &available {
                *required - available
            } else {
                Decimal::ZERO
            };
            let deficit_rate = Self::compute_deficit_rate(*required, shortage);
            let level = ShortageLevel::from_deficit_rate(deficit_rate);
            if level == ShortageLevel::Normal {
                continue;
            }
            let affected = material_affected_orders
                .get(material_id)
                .cloned()
                .unwrap_or_default();
            let (material_name, material_code) = material_names
                .get(material_id)
                .cloned()
                .unwrap_or_else(|| (format!("物料#{}", material_id), String::new()));
            let item = MaterialShortageItem {
                material_id: *material_id,
                material_name,
                material_code,
                required_quantity: *required,
                available_quantity: available,
                shortage_quantity: shortage,
                deficit_rate,
                level: level.clone(),
                affected_orders: affected.clone(),
                unit: unit.clone(),
            };
            Self::publish_shortage_event(&item);
            counts.update(&level, &affected);
            items.push(item);
        }
        (items, counts)
    }

    /// 按严重程度排序缺料清单
    fn sort_items_by_level(items: &mut [MaterialShortageItem]) {
        let order = |l: &ShortageLevel| match l {
            ShortageLevel::Critical => 0,
            ShortageLevel::Severe => 1,
            ShortageLevel::Warning => 2,
            ShortageLevel::Normal => 3,
        };
        items.sort_by(|a, b| order(&a.level).cmp(&order(&b.level)));
    }

    /// 构建缺料汇总
    fn build_summary(
        items: Vec<MaterialShortageItem>,
        material_requirements: &HashMap<i32, MaterialReq>,
        counts: ShortageCounts,
    ) -> ShortageSummary {
        ShortageSummary {
            total_materials_checked: material_requirements.len() as i64,
            shortage_count: counts.critical + counts.severe + counts.warning,
            critical_count: counts.critical,
            severe_count: counts.severe,
            warning_count: counts.warning,
            affected_orders_count: counts.affected_order_ids.len() as i64,
            items,
        }
    }

    /// V15 P0-B15：持久化缺料预警快照
    ///
    /// 幂等策略：同物料且 status != 'resolved' 的 alert 视为"未解决"，
    /// 已存在则更新快照字段（required/available/shortage/deficit_rate/level/affected_orders_count/updated_at），
    /// 不存在则插入新记录（生成 alert_no = MS-YYYYMMDD-NNN）。
    ///
    /// 设计考量：
    /// - 不在循环内做 N 次 DB 查询，先批量查询未解决 alerts 按 material_id 索引
    /// - 整个持久化过程在一个事务内完成，失败则回滚
    /// - 持久化失败不阻断 detect_shortages（降级为 warn，与事件发布策略一致）
    async fn persist_alerts(&self, items: &[MaterialShortageItem]) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        let material_ids: Vec<i32> = items.iter().map(|i| i.material_id).collect();
        let now = Utc::now();

        // 批量查询未解决 alerts（status != 'resolved'）按 material_id 索引
        let existing_alerts = alert_model::Entity::find()
            .filter(alert_model::Column::MaterialId.is_in(material_ids))
            .filter(alert_model::Column::Status.ne("resolved"))
            .all(&*self.db)
            .await?;

        let mut existing_map: HashMap<i32, alert_model::Model> = HashMap::new();
        for a in existing_alerts {
            existing_map.insert(a.material_id, a);
        }

        let txn = self.db.begin().await?;

        for item in items {
            let level_str = format!("{:?}", item.level);
            let affected_orders_count = item.affected_orders.len() as i32;

            if let Some(existing) = existing_map.get(&item.material_id) {
                // 更新快照字段（保留原 status / purchase_request_id / purchase_order_id / identified_at）
                let mut active: alert_model::ActiveModel = existing.clone().into();
                active.required_quantity = Set(item.required_quantity);
                active.available_quantity = Set(item.available_quantity);
                active.shortage_quantity = Set(item.shortage_quantity);
                active.deficit_rate = Set(item.deficit_rate);
                active.level = Set(level_str);
                active.affected_orders_count = Set(affected_orders_count);
                active.unit = Set(item.unit.clone());
                active.updated_at = Set(now);
                active.update(&txn).await?;
            } else {
                // 插入新 alert（生成 alert_no = MS-YYYYMMDD-NNN）
                let alert_no = self.generate_alert_no(&txn).await?;
                let active = alert_model::ActiveModel {
                    alert_no: Set(alert_no),
                    material_id: Set(item.material_id),
                    material_name: Set(item.material_name.clone()),
                    material_code: Set(Some(item.material_code.clone())),
                    required_quantity: Set(item.required_quantity),
                    available_quantity: Set(item.available_quantity),
                    shortage_quantity: Set(item.shortage_quantity),
                    deficit_rate: Set(item.deficit_rate),
                    level: Set(level_str),
                    status: Set("identified".to_string()),
                    affected_orders_count: Set(affected_orders_count),
                    purchase_request_id: Set(None),
                    purchase_order_id: Set(None),
                    unit: Set(item.unit.clone()),
                    identified_at: Set(now),
                    resolved_at: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                };
                active.insert(&txn).await?;
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// 生成缺料单号：MS-YYYYMMDD-NNN（NNN 为当天序号，从 001 开始）
    ///
    /// 通过查询当天已有的最大序号 + 1 保证唯一性。
    /// 并发场景下可能冲突（UNIQUE 约束会拒绝），调用方需重试。
    async fn generate_alert_no<C: ConnectionTrait>(&self, db: &C) -> Result<String, AppError> {
        let today = Utc::now();
        let date_str = today.format("%Y%m%d").to_string();
        let prefix = format!("MS-{}-", date_str);

        // 查询当天已有的最大序号
        let today_alerts = alert_model::Entity::find()
            .filter(alert_model::Column::AlertNo.starts_with(&prefix))
            .order_by_desc(alert_model::Column::AlertNo)
            .all(db)
            .await?;

        let next_seq = if let Some(latest) = today_alerts.first() {
            // 从 "MS-YYYYMMDD-NNN" 提取 NNN
            latest
                .alert_no
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .map(|n| n + 1)
                .unwrap_or(1)
        } else {
            1
        };

        Ok(format!("{}{:03}", prefix, next_seq))
    }

    /// 获取缺料预警列表（可按级别过滤）
    ///
    /// BE-P 优化（2026-06-26）：
    /// detect_shortages 是实时计算（非 DB 全量加载），内存分页是合理的。
    /// 优化点：先过滤再计算 total，避免构建完整 filtered Vec 再 skip/take。
    pub async fn list_alerts(
        &self,
        level_filter: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<MaterialShortageItem>, u64), AppError> {
        let summary = self
            .detect_shortages(ShortageCheckRequest {
                product_ids: None,
                date_from: None,
                date_to: None,
                threshold: None,
            })
            .await?;

        // 先过滤（惰性迭代器，不构建中间 Vec）
        let filtered: Vec<MaterialShortageItem> = if let Some(level) = level_filter {
            summary
                .items
                .into_iter()
                .filter(|i| format!("{:?}", i.level).to_uppercase() == level.to_uppercase())
                .collect()
        } else {
            summary.items
        };

        let total = filtered.len() as u64;
        let start = (page.saturating_sub(1) * page_size) as usize;
        let paged = filtered
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect();

        Ok((paged, total))
    }

    /// 查询物料库存映射：material_id -> 可用库存总量
    async fn get_material_stock_map(
        &self,
        material_ids: &[i32],
    ) -> Result<HashMap<i32, Decimal>, AppError> {
        if material_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let stocks = InventoryStockEntity::find()
            .filter(StockColumn::ProductId.is_in(material_ids.to_vec()))
            .filter(StockColumn::StockStatus.eq("正常"))
            .filter(StockColumn::QualityStatus.eq("合格"))
            .all(&*self.db)
            .await?;

        let mut map: HashMap<i32, Decimal> = HashMap::new();
        for stock in stocks {
            *map.entry(stock.product_id).or_insert(Decimal::ZERO) += stock.quantity_available;
        }

        Ok(map)
    }

    /// 查询产品名称和编号映射：product_id -> (name, code)
    async fn get_product_names(
        &self,
        product_ids: &[i32],
    ) -> Result<HashMap<i32, (String, String)>, AppError> {
        if product_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let products = ProductEntity::find()
            .filter(ProductColumn::Id.is_in(product_ids.to_vec()))
            .all(&*self.db)
            .await?;

        let mut map = HashMap::new();
        for p in products {
            map.insert(p.id, (p.name, p.code));
        }

        Ok(map)
    }

    /// 保存预警阈值配置（V15 P0-B15：upsert 到 material_shortage_threshold_configs 单行表）
    ///
    /// 单行配置表（id=1 固定）：先查询是否存在，存在则 update，不存在则 insert。
    /// 与 migration m0068 默认行（id=1 + 默认阈值）协同，保证首次启动即可读默认值。
    pub async fn save_threshold_config(
        &self,
        config: &ShortageThresholdConfig,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let existing = threshold_model::Entity::find_by_id(threshold_model::SINGLE_ROW_ID)
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            // update 已有单行配置
            let mut active: threshold_model::ActiveModel = threshold_model::ActiveModel {
                id: Set(threshold_model::SINGLE_ROW_ID),
                ..Default::default()
            };
            active.safety_factor = Set(config.safety_factor);
            active.critical_threshold = Set(config.critical_threshold);
            active.severe_threshold = Set(config.severe_threshold);
            active.updated_at = Set(now);
            active.update(&*self.db).await?;
        } else {
            // insert 单行配置（兜底：migration 默认行若被人工删除则重新插入）
            let active = threshold_model::ActiveModel {
                id: Set(threshold_model::SINGLE_ROW_ID),
                safety_factor: Set(config.safety_factor),
                critical_threshold: Set(config.critical_threshold),
                severe_threshold: Set(config.severe_threshold),
                updated_at: Set(now),
            };
            active.insert(&*self.db).await?;
        }

        tracing::info!(
            safety_factor = %config.safety_factor,
            critical_threshold = %config.critical_threshold,
            severe_threshold = %config.severe_threshold,
            "save_threshold_config: 阈值配置已持久化到 material_shortage_threshold_configs (id=1)"
        );
        Ok(())
    }

    /// 加载预警阈值配置（V15 P0-B15：从 material_shortage_threshold_configs 单行表读取）
    ///
    /// 若 DB 中无行（理论上 migration m0068 默认插入了一行），降级返回默认值。
    pub async fn load_threshold_config(
        &self,
    ) -> Result<ShortageThresholdConfig, AppError> {
        let row = threshold_model::Entity::find_by_id(threshold_model::SINGLE_ROW_ID)
            .one(&*self.db)
            .await?;

        match row {
            Some(r) => Ok(ShortageThresholdConfig {
                safety_factor: r.safety_factor,
                critical_threshold: r.critical_threshold,
                severe_threshold: r.severe_threshold,
            }),
            None => {
                // 降级：理论上 migration 已插入默认行，此处兜底防止人工删除后崩溃
                tracing::warn!(
                    "load_threshold_config: material_shortage_threshold_configs (id=1) 不存在，降级返回默认值"
                );
                Ok(ShortageThresholdConfig::default())
            }
        }
    }

    /// 生成补货建议
    pub async fn generate_replenishment_suggestions(
        &self,
        shortages: &[MaterialShortageItem],
    ) -> Result<Vec<ReplenishmentSuggestion>, AppError> {
        let mut suggestions = Vec::new();

        for shortage in shortages {
            if shortage.shortage_quantity > Decimal::ZERO {
                // 建议采购量 = 缺口数量 * 1.2 (20%余量)
                let suggested_quantity = shortage.shortage_quantity * Decimal::new(12, 1);

                suggestions.push(ReplenishmentSuggestion {
                    material_id: shortage.material_id,
                    material_name: shortage.material_name.clone(),
                    material_code: shortage.material_code.clone(),
                    shortage_quantity: shortage.shortage_quantity,
                    suggested_quantity,
                    unit: shortage.unit.clone(),
                    priority: match shortage.level {
                        ShortageLevel::Critical => "URGENT".to_string(),
                        ShortageLevel::Severe => "HIGH".to_string(),
                        ShortageLevel::Warning => "MEDIUM".to_string(),
                        ShortageLevel::Normal => "LOW".to_string(),
                    },
                    affected_orders_count: shortage.affected_orders.len() as i32,
                });
            }
        }

        // 按优先级排序
        suggestions.sort_by(|a, b| {
            let priority_order = |p: &str| match p {
                "URGENT" => 0,
                "HIGH" => 1,
                "MEDIUM" => 2,
                _ => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        Ok(suggestions)
    }

    /// 更新缺料预警状态（V15 P0-B15：持久化状态到 material_shortage_alerts 表）
    ///
    /// 状态机：identified → purchase_request → purchase_order → received → resolved
    /// - 查找该 material_id 最新未解决（status != 'resolved'）的 alert
    /// - 更新 status 字段；若新状态为 resolved，同步填入 resolved_at
    /// - 返回更新后的 alert 快照（含 level / status / 物料信息），供 handler 构建 DTO
    ///
    /// 设计：URL `/:id/status` 中的 id 语义为 material_id（与原桩实现一致），
    /// 因 persist_alerts 保证同 material_id 至多一条未解决 alert，故查找唯一。
    pub async fn update_status(
        &self,
        material_id: i32,
        new_status: &str,
    ) -> Result<alert_model::Model, AppError> {
        // 1. 查找该 material_id 最新未解决 alert
        let alert = alert_model::Entity::find()
            .filter(alert_model::Column::MaterialId.eq(material_id))
            .filter(alert_model::Column::Status.ne("resolved"))
            .order_by_desc(alert_model::Column::IdentifiedAt)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "未找到物料 {} 的未解决缺料预警（status != resolved），无法更新状态",
                    material_id
                ))
            })?;

        // 2. 更新 status + resolved_at（若为 resolved）+ updated_at
        let now = Utc::now();
        let mut active: alert_model::ActiveModel = alert.into();
        active.status = Set(new_status.to_string());
        if new_status == "resolved" {
            active.resolved_at = Set(Some(now));
        }
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;

        tracing::info!(
            alert_id = updated.id,
            alert_no = %updated.alert_no,
            material_id = material_id,
            new_status = new_status,
            "update_status: 缺料预警状态已持久化"
        );

        Ok(updated)
    }
}

/// 补货建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplenishmentSuggestion {
    pub material_id: i32,
    pub material_name: String,
    pub material_code: String,
    pub shortage_quantity: Decimal,
    pub suggested_quantity: Decimal,
    pub unit: Option<String>,
    pub priority: String,
    pub affected_orders_count: i32,
}
