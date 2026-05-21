//! 缺料预警 Service
//!
//! 提供缺料检测、预警阈值配置、缺料清单生成等功能

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::bom::{Entity as BomEntity, Column as BomColumn};
use crate::models::bom_item::{Entity as BomItemEntity, Column as BomItemColumn};
use crate::models::inventory_stock::{Entity as InventoryStockEntity, Column as StockColumn};
use crate::models::production_order::{
    Entity as ProductionOrderEntity, Column as ProductionOrderColumn,
};
use crate::models::product::{Entity as ProductEntity, Column as ProductColumn};
use crate::models::tenant_config::{Entity as TenantConfigEntity, ActiveModel as TenantConfigActiveModel};
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
        let threshold = request.threshold.unwrap_or_default();

        // 1. 获取活跃的生产订单
        let mut order_query = ProductionOrderEntity::find()
            .filter(ProductionOrderColumn::Status.is_in(vec!["SCHEDULED", "IN_PROGRESS"]));

        if let Some(ref product_ids) = request.product_ids {
            order_query = order_query.filter(
                ProductionOrderColumn::ProductId.is_in(product_ids.clone()),
            );
        }
        if let Some(from) = request.date_from {
            order_query = order_query.filter(ProductionOrderColumn::PlannedEndDate.gte(from));
        }
        if let Some(to) = request.date_to {
            order_query = order_query.filter(ProductionOrderColumn::PlannedStartDate.lte(to));
        }

        let orders = order_query
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if orders.is_empty() {
            return Ok(ShortageSummary {
                total_materials_checked: 0,
                shortage_count: 0,
                critical_count: 0,
                severe_count: 0,
                warning_count: 0,
                affected_orders_count: 0,
                items: vec![],
            });
        }

        // 2. 按产品聚合需求数量
        let mut product_demands: HashMap<i32, Decimal> = HashMap::new();
        let mut product_orders: HashMap<i32, Vec<AffectedOrder>> = HashMap::new();

        for order in &orders {
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

        // 3. 查询这些产品的默认 BOM 及其物料
        let product_ids: Vec<i32> = product_demands.keys().cloned().collect();

        let boms = BomEntity::find()
            .filter(BomColumn::ProductId.is_in(product_ids.clone()))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq("ACTIVE"))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 4. 计算每种物料的总需求
        let bom_ids: Vec<i32> = boms.iter().map(|b| b.id).collect();
        let product_to_bom: HashMap<i32, i32> =
            boms.iter().map(|b| (b.product_id, b.id)).collect();

        let bom_items = if bom_ids.is_empty() {
            vec![]
        } else {
            BomItemEntity::find()
                .filter(BomItemColumn::BomId.is_in(bom_ids))
                .all(&*self.db)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?
        };

        // material_id -> (total_required, unit, [(product_id, qty_per_unit)])
        let mut material_requirements: HashMap<i32, (Decimal, Option<String>, Vec<(i32, Decimal)>)> =
            HashMap::new();

        for item in &bom_items {
            // 找到使用此 BOM 的产品
            for (product_id, bom_id) in &product_to_bom {
                if *bom_id == item.bom_id {
                    if let Some(&demand) = product_demands.get(product_id) {
                        let scrap_rate = item.scrap_rate.unwrap_or(Decimal::ZERO);
                        let qty_per_unit = item.quantity * (Decimal::ONE + scrap_rate);
                        let total_for_product = qty_per_unit * demand;

                        let entry = material_requirements
                            .entry(item.material_id)
                            .or_insert((Decimal::ZERO, item.unit.clone(), vec![]));
                        entry.0 += total_for_product;
                        entry.2.push((*product_id, qty_per_unit));
                    }
                }
            }
        }

        // 5. 查询物料库存
        let material_ids: Vec<i32> = material_requirements.keys().cloned().collect();
        let stock_map = self.get_material_stock_map(&material_ids).await?;

        // 6. 查询物料名称
        let material_names = self.get_product_names(&material_ids).await?;

        // 7. 汇总受影响的订单（按物料）
        let mut material_affected_orders: HashMap<i32, Vec<AffectedOrder>> = HashMap::new();
        for item in &bom_items {
            for (product_id, bom_id) in &product_to_bom {
                if *bom_id == item.bom_id {
                    if let Some(orders) = product_orders.get(product_id) {
                        material_affected_orders
                            .entry(item.material_id)
                            .or_default()
                            .extend(orders.clone());
                    }
                }
            }
        }

        // 8. 生成缺料清单
        let mut items = Vec::new();
        let mut critical_count = 0i64;
        let mut severe_count = 0i64;
        let mut warning_count = 0i64;
        let mut affected_order_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();

        for (material_id, (required, unit, _)) in &material_requirements {
            let available = stock_map.get(material_id).copied().unwrap_or(Decimal::ZERO);
            let shortage = if required > &available {
                *required - available
            } else {
                Decimal::ZERO
            };

            let deficit_rate = if *required > Decimal::ZERO {
                ((shortage / *required) * Decimal::from(100))
                    .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
            } else {
                Decimal::ZERO
            };

            let level = ShortageLevel::from_deficit_rate(deficit_rate);

            if level != ShortageLevel::Normal {
                match level {
                    ShortageLevel::Critical => critical_count += 1,
                    ShortageLevel::Severe => severe_count += 1,
                    ShortageLevel::Warning => warning_count += 1,
                    _ => {}
                }

                let affected = material_affected_orders
                    .get(material_id)
                    .cloned()
                    .unwrap_or_default();

                for ao in &affected {
                    affected_order_ids.insert(ao.order_id);
                }

                let (material_name, material_code) = material_names
                    .get(material_id)
                    .cloned()
                    .unwrap_or_else(|| (format!("物料#{}", material_id), String::new()));

                items.push(MaterialShortageItem {
                    material_id: *material_id,
                    material_name,
                    material_code,
                    required_quantity: *required,
                    available_quantity: available,
                    shortage_quantity: shortage,
                    deficit_rate,
                    level,
                    affected_orders: affected,
                    unit: unit.clone(),
                });
            }
        }

        // 按严重程度排序
        items.sort_by(|a, b| {
            let order = |l: &ShortageLevel| match l {
                ShortageLevel::Critical => 0,
                ShortageLevel::Severe => 1,
                ShortageLevel::Warning => 2,
                ShortageLevel::Normal => 3,
            };
            order(&a.level).cmp(&order(&b.level))
        });

        Ok(ShortageSummary {
            total_materials_checked: material_requirements.len() as i64,
            shortage_count: (critical_count + severe_count + warning_count),
            critical_count,
            severe_count,
            warning_count,
            affected_orders_count: affected_order_ids.len() as i64,
            items,
        })
    }

    /// 获取缺料预警列表（可按级别过滤）
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
        let start = (page * page_size) as usize;
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
            .filter(StockColumn::StockStatus.eq("ACTIVE"))
            .filter(StockColumn::QualityStatus.eq("QUALIFIED"))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut map = HashMap::new();
        for p in products {
            map.insert(p.id, (p.name, p.code));
        }

        Ok(map)
    }

    /// 保存预警阈值配置
    pub async fn save_threshold_config(
        &self,
        tenant_id: i32,
        config: &ShortageThresholdConfig,
    ) -> Result<(), AppError> {
        use sea_orm::Set;

        let config_json = serde_json::to_string(config)
            .map_err(|e| AppError::ValidationError(format!("配置序列化失败: {}", e)))?;

        // 查找现有配置
        let existing = TenantConfigEntity::find()
            .filter(crate::models::tenant_config::Column::TenantId.eq(tenant_id))
            .filter(crate::models::tenant_config::Column::ConfigKey.eq("shortage_threshold"))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(model) = existing {
            let mut active: TenantConfigActiveModel = model.into();
            active.config_value = Set(config_json);
            active.updated_at = Set(Utc::now());
            active.update(&*self.db).await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        } else {
            let active = TenantConfigActiveModel {
                id: Default::default(),
                tenant_id: Set(tenant_id),
                config_key: Set("shortage_threshold".to_string()),
                config_value: Set(config_json),
                config_type: Set("json".to_string()),
                description: Set(Some("缺料预警阈值配置".to_string())),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            active.insert(&*self.db).await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// 加载预警阈值配置
    pub async fn load_threshold_config(
        &self,
        tenant_id: i32,
    ) -> Result<ShortageThresholdConfig, AppError> {
        use crate::models::tenant_config::Entity as TenantConfigEntity;

        let config = TenantConfigEntity::find()
            .filter(crate::models::tenant_config::Column::TenantId.eq(tenant_id))
            .filter(crate::models::tenant_config::Column::ConfigKey.eq("shortage_threshold"))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        match config {
            Some(model) => {
                serde_json::from_str(&model.config_value)
                    .map_err(|e| AppError::ValidationError(format!("配置解析失败: {}", e)))
            }
            None => Ok(ShortageThresholdConfig::default()),
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
