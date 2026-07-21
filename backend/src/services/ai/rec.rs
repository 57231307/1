//! AI 智能推荐服务（ai/rec）
//!
//! 提供四类推荐与配套的库存分析能力：
//! - `optimize_inventory` 库存优化（安全库存 + 再订货点 + 建议订货量）
//! - `get_abc_classification` ABC 分类（A 类累计 0-80%，B 80-95%，C 95-100%）
//! - `get_inventory_turnover` 库存周转率（按产品聚合出库与库存）
//! - `generate_recommendations` 智能推荐入口（REORDER / BUNDLE / TREND / PRICE_ADJUST）
//!
//! 拆分自原 `ai_analysis_service.rs` 的 `// 库存优化` + `// 智能推荐` 两段方法集合。

use chrono::{Duration, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

use crate::models::inventory_stock::{
    Entity as InventoryStockEntity, Model as InventoryStockModel,
};
use crate::models::inventory_transaction::Entity as InventoryTransactionEntity;
use crate::models::sales_order_item::Entity as SalesOrderItemEntity;
use crate::utils::error::AppError;

use super::{
    mean, std_deviation, AbcClassification, AiAnalysisService, InventorySuggestion,
    InventoryTurnover, SmartRecommendation,
};

impl AiAnalysisService {
    /// 按产品聚合出库数据
    fn collect_outbound_by_product(
        transactions: &[crate::models::inventory_transaction::Model],
    ) -> HashMap<i32, Vec<f64>> {
        let mut outbound_by_product: HashMap<i32, Vec<f64>> = HashMap::new();
        for tx in transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                outbound_by_product
                    .entry(tx.product_id)
                    .or_default()
                    .push(qty);
            }
        }
        outbound_by_product
    }

    /// 计算单产品的需求统计（日均值、标准差）和库存优化建议
    fn compute_inventory_suggestion(
        &self,
        stock: &InventoryStockModel,
        outbound_qtys: Option<&Vec<f64>>,
        transactions: &[crate::models::inventory_transaction::Model],
        abc: &str,
    ) -> InventorySuggestion {
        let pid = stock.product_id;
        let current = stock.quantity_available.to_f64().unwrap_or(0.0);

        // 计算出库统计
        let (_avg_daily_demand, _demand_std, safety_stock, reorder_point, reorder_quantity, suggested) =
            if let Some(qtys) = outbound_qtys {
                let daily_map = self.aggregate_daily_from_transactions(pid, transactions);
                let daily_values: Vec<f64> = daily_map.values().copied().collect();
                let avg = if daily_values.is_empty() {
                    0.0
                } else {
                    mean(&daily_values)
                };
                let std = if daily_values.len() > 1 {
                    std_deviation(&daily_values)
                } else {
                    avg * 0.3
                };
                let _total: f64 = qtys.iter().sum();

                // ABC 分类影响服务水平
                let service_level_z = match abc {
                    "A" => 2.33, // 99% 服务水平
                    "B" => 1.65, // 95% 服务水平
                    _ => 1.28,   // 90% 服务水平
                };

                // 安全库存 = Z * σ * √(LT)  假设提前期 LT = 7 天
                let lead_time = 7.0_f64;
                let ss = service_level_z * std * lead_time.sqrt();
                let rp = avg * lead_time + ss;
                let rq = avg * 30.0;
                let sg = ss + avg * 30.0;

                (avg, std, ss, rp, rq, sg)
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            };

        let reason = if current <= 0.0 {
            format!(
                "库存为零! ABC分类={}, 安全库存={:.0}, 建议立即补货 {:.0}",
                abc, safety_stock, reorder_quantity
            )
        } else if current < reorder_point {
            format!(
                "库存({:.0})低于再订货点({:.0}), ABC分类={}, 安全库存={:.0}, 建议补货 {:.0}",
                current, reorder_point, abc, safety_stock, reorder_quantity
            )
        } else if current > suggested * 2.0 {
            format!(
                "库存({:.0})过高, 超过建议水平({:.0})的2倍, ABC分类={}, 建议减少采购或促销",
                current, suggested, abc
            )
        } else {
            format!(
                "库存水平正常, ABC分类={}, 安全库存={:.0}",
                abc, safety_stock
            )
        };

        InventorySuggestion {
            product_id: pid,
            current_stock: stock.quantity_available,
            suggested_stock: Decimal::try_from(suggested.max(0.0)).unwrap_or(Decimal::ZERO),
            reorder_point: Decimal::try_from(reorder_point.max(0.0)).unwrap_or(Decimal::ZERO),
            reorder_quantity: Decimal::try_from(reorder_quantity.max(0.0))
                .unwrap_or(Decimal::ZERO),
            reason,
        }
    }

    /// 库存优化建议
    /// 基于历史出库数据计算安全库存，结合 ABC 分类给出建议
    pub async fn optimize_inventory(
        &self,
        product_id: Option<i32>,
    ) -> Result<Vec<InventorySuggestion>, AppError> {
        let mut select = InventoryStockEntity::find();

        if let Some(pid) = product_id {
            select = select.filter(crate::models::inventory_stock::Column::ProductId.eq(pid));
        }

        let stocks = select.all(&*self.db).await?;

        if stocks.is_empty() {
            return Ok(Vec::new());
        }

        // 获取所有产品的出库历史数据
        let start_date = Utc::now().date_naive() - Duration::days(90);
        let transactions = InventoryTransactionEntity::find()
            .filter(
                crate::models::inventory_transaction::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

        // 按产品聚合出库数据
        let outbound_by_product = Self::collect_outbound_by_product(&transactions);

        // ABC 分类
        let abc_classifications = self.compute_abc_classification(&stocks, &outbound_by_product);
        let abc_map: HashMap<i32, &str> = abc_classifications
            .iter()
            .map(|c| (c.product_id, c.category.as_str()))
            .collect();

        let mut suggestions = Vec::new();

        for stock in stocks {
            let pid = stock.product_id;
            let abc = abc_map.get(&pid).copied().unwrap_or("C");

            let suggestion = self.compute_inventory_suggestion(
                &stock,
                outbound_by_product.get(&pid),
                &transactions,
                abc,
            );
            suggestions.push(suggestion);
        }

        Ok(suggestions)
    }

    /// ABC 分类分析
    /// A 类: 累计销售额占比 0-80%
    /// B 类: 累计销售额占比 80-95%
    /// C 类: 累计销售额占比 95-100%
    ///
    /// 批次 86 v2 复审 P2-12 修复：加 LIMIT 兜底防止全表加载内存爆炸
    /// 原 `.find().all()` 无限制加载所有库存记录，大表会 OOM
    pub async fn get_abc_classification(&self) -> Result<Vec<AbcClassification>, AppError> {
        let stocks = InventoryStockEntity::find()
            .limit(10_000)
            .all(&*self.db)
            .await?;

        let start_date = Utc::now().date_naive() - Duration::days(90);
        let transactions = InventoryTransactionEntity::find()
            .filter(
                crate::models::inventory_transaction::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

        let mut outbound_by_product: HashMap<i32, Vec<f64>> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                outbound_by_product
                    .entry(tx.product_id)
                    .or_default()
                    .push(qty);
            }
        }

        Ok(self.compute_abc_classification(&stocks, &outbound_by_product))
    }

    fn compute_abc_classification(
        &self,
        stocks: &[InventoryStockModel],
        outbound_by_product: &HashMap<i32, Vec<f64>>,
    ) -> Vec<AbcClassification> {
        // 计算每个产品的总出库量
        let mut product_sales: Vec<(i32, f64)> = stocks
            .iter()
            .map(|s| {
                let total = outbound_by_product
                    .get(&s.product_id)
                    .map(|v| v.iter().sum::<f64>())
                    .unwrap_or(0.0);
                (s.product_id, total)
            })
            .collect();

        // 按销售额降序排列
        product_sales.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let grand_total: f64 = product_sales.iter().map(|(_, v)| v).sum();
        if grand_total == 0.0 {
            return product_sales
                .into_iter()
                .map(|(pid, _)| AbcClassification {
                    product_id: pid,
                    category: "C".to_string(),
                    total_sales: Decimal::ZERO,
                    cumulative_ratio: 1.0,
                })
                .collect();
        }

        let mut cumulative = 0.0;
        product_sales
            .into_iter()
            .map(|(pid, sales)| {
                cumulative += sales;
                let ratio = cumulative / grand_total;
                let category = if ratio <= 0.80 {
                    "A"
                } else if ratio <= 0.95 {
                    "B"
                } else {
                    "C"
                };
                AbcClassification {
                    product_id: pid,
                    category: category.to_string(),
                    total_sales: Decimal::try_from(sales).unwrap_or(Decimal::ZERO),
                    cumulative_ratio: (ratio * 1000.0).round() / 1000.0,
                }
            })
            .collect()
    }

    /// 库存周转率计算
    pub async fn get_inventory_turnover(
        &self,
        product_id: Option<i32>,
        days: i64,
    ) -> Result<Vec<InventoryTurnover>, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(days);

        let mut tx_select = InventoryTransactionEntity::find().filter(
            crate::models::inventory_transaction::Column::CreatedAt.gte(
                start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_default()
                    .and_utc(),
            ),
        );
        if let Some(pid) = product_id {
            tx_select =
                tx_select.filter(crate::models::inventory_transaction::Column::ProductId.eq(pid));
        }

        let transactions = tx_select.all(&*self.db).await?;

        // 按产品聚合出库量
        let mut outbound_map: HashMap<i32, f64> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                *outbound_map.entry(tx.product_id).or_insert(0.0) += qty;
            }
        }

        // 获取当前库存
        let mut stock_select = InventoryStockEntity::find();
        if let Some(pid) = product_id {
            stock_select =
                stock_select.filter(crate::models::inventory_stock::Column::ProductId.eq(pid));
        }
        let stocks = stock_select.all(&*self.db).await?;

        // 按产品聚合当前库存
        let mut stock_map: HashMap<i32, f64> = HashMap::new();
        for s in &stocks {
            let qty = s.quantity_on_hand.to_f64().unwrap_or(0.0);
            *stock_map.entry(s.product_id).or_insert(0.0) += qty;
        }

        let mut results = Vec::new();
        let all_pids: std::collections::HashSet<i32> = outbound_map
            .keys()
            .chain(stock_map.keys())
            .cloned()
            .collect();

        for pid in all_pids {
            let total_outbound = outbound_map.get(&pid).copied().unwrap_or(0.0);
            let current_stock = stock_map.get(&pid).copied().unwrap_or(0.0);

            // 平均库存近似为当前库存（简化处理）
            let avg_stock = current_stock;
            let turnover_rate = if avg_stock > 0.0 {
                (total_outbound / avg_stock * 365.0 / days as f64 * 100.0).round() / 100.0
            } else {
                0.0
            };

            results.push(InventoryTurnover {
                product_id: pid,
                turnover_rate,
                avg_stock: Decimal::try_from(avg_stock).unwrap_or(Decimal::ZERO),
                total_outbound: Decimal::try_from(total_outbound).unwrap_or(Decimal::ZERO),
                period_days: days,
            });
        }

        // 按周转率降序排列
        results.sort_by(|a, b| {
            b.turnover_rate
                .partial_cmp(&a.turnover_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// 智能推荐 - 基于历史数据生成推荐
    pub async fn generate_recommendations(
        &self,
        recommendation_type: String,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let mut recommendations = Vec::new();

        match recommendation_type.as_str() {
            "REORDER" => {
                // 基于库存优化的补货推荐
                let suggestions = self.optimize_inventory(None).await?;
                let mut reorder_items: Vec<SmartRecommendation> = suggestions
                    .into_iter()
                    .filter(|s| s.current_stock < s.reorder_point)
                    .map(|s| {
                        let urgency = (s.reorder_point - s.current_stock).to_f64().unwrap_or(0.0);
                        SmartRecommendation {
                            recommendation_type: "REORDER".to_string(),
                            target_id: s.product_id,
                            target_type: "PRODUCT".to_string(),
                            score: urgency,
                            reason: s.reason,
                        }
                    })
                    .collect();

                reorder_items.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                recommendations = reorder_items.into_iter().take(limit).collect();
            }
            "BUNDLE" => {
                // 基于关联规则的捆绑采购推荐
                // 分析经常一起被销售的产品组合
                recommendations = self.generate_association_recommendations(limit).await?;
            }
            "TREND" => {
                // 基于销售趋势的产品推荐
                recommendations = self.generate_trend_recommendations(limit).await?;
            }
            "PRICE_ADJUST" => {
                // 基于库存水平的价格调整推荐
                recommendations = self.generate_price_recommendations(limit).await?;
            }
            _ => {
                // 默认: 综合推荐
                let mut reorder =
                    Box::pin(self.generate_recommendations("REORDER".to_string(), limit / 2))
                        .await?;
                let mut trend =
                    Box::pin(self.generate_recommendations("TREND".to_string(), limit / 2)).await?;
                recommendations.append(&mut reorder);
                recommendations.append(&mut trend);
            }
        }

        Ok(recommendations)
    }

    /// 基于关联规则的采购推荐
    /// 分析哪些产品经常在同一时期被销售，推荐关联产品
    async fn generate_association_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let items = self.query_recent_sales_items(60).await?;
        let (co_occurrence, product_count, total_orders) =
            Self::compute_co_occurrence_stats(&items);
        let assoc_scores = Self::find_strong_associations(
            &co_occurrence,
            &product_count,
            total_orders,
        );
        let recommendations = Self::build_assoc_recommendations(assoc_scores, limit);
        Ok(recommendations)
    }

    /// 查询近 N 天销售订单明细
    async fn query_recent_sales_items(
        &self,
        days: i64,
    ) -> Result<Vec<crate::models::sales_order_item::Model>, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(days);
        SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 按订单分组产品并计算共现频率与产品频次
    fn compute_co_occurrence_stats(
        items: &[crate::models::sales_order_item::Model],
    ) -> (
        HashMap<(i32, i32), usize>,
        HashMap<i32, usize>,
        usize,
    ) {
        let mut order_products: HashMap<i32, Vec<i32>> = HashMap::new();
        for item in items {
            order_products
                .entry(item.order_id)
                .or_default()
                .push(item.product_id);
        }
        let mut co_occurrence: HashMap<(i32, i32), usize> = HashMap::new();
        let mut product_count: HashMap<i32, usize> = HashMap::new();
        for products in order_products.values() {
            let unique: std::collections::HashSet<i32> = products.iter().cloned().collect();
            for &pid in &unique {
                *product_count.entry(pid).or_insert(0) += 1;
            }
            let sorted: Vec<i32> = {
                let mut v: Vec<i32> = unique.into_iter().collect();
                v.sort();
                v
            };
            for i in 0..sorted.len() {
                for j in (i + 1)..sorted.len() {
                    *co_occurrence.entry((sorted[i], sorted[j])).or_insert(0) += 1;
                }
            }
        }
        let total_orders = order_products.len().max(1);
        (co_occurrence, product_count, total_orders)
    }

    /// 找出强关联规则（support>5%, confidence>30%），返回 (p1,p2,lift,reason)
    fn find_strong_associations(
        co_occurrence: &HashMap<(i32, i32), usize>,
        product_count: &HashMap<i32, usize>,
        total_orders: usize,
    ) -> Vec<(i32, i32, f64, String)> {
        let total = total_orders as f64;
        let mut assoc_scores: Vec<(i32, i32, f64, String)> = Vec::new();
        for ((p1, p2), &count) in co_occurrence {
            let support = count as f64 / total;
            let conf1 = count as f64
                / product_count.get(p1).copied().unwrap_or(1).max(1) as f64;
            let _conf2 = count as f64
                / product_count.get(p2).copied().unwrap_or(1).max(1) as f64;
            if support > 0.05 && conf1 > 0.3 {
                let lift = support
                    / ((product_count.get(p1).unwrap_or(&0).to_f64().unwrap_or(0.0) / total)
                        * (product_count.get(p2).unwrap_or(&0).to_f64().unwrap_or(0.0) / total));
                assoc_scores.push((
                    *p1,
                    *p2,
                    lift,
                    format!(
                        "产品 {} 与产品 {} 经常一起被购买 (共现率={:.0}%, 提升度={:.2})",
                        p1, p2, conf1 * 100.0, lift
                    ),
                ));
            }
        }
        assoc_scores
    }

    /// 按提升度降序构建关联推荐列表
    fn build_assoc_recommendations(
        mut assoc_scores: Vec<(i32, i32, f64, String)>,
        limit: usize,
    ) -> Vec<SmartRecommendation> {
        assoc_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        assoc_scores
            .into_iter()
            .take(limit)
            .map(|(source, target, score, reason)| SmartRecommendation {
                recommendation_type: "BUNDLE".to_string(),
                target_id: target,
                target_type: "PRODUCT".to_string(),
                score,
                reason: format!("与产品 {} 关联推荐: {}", source, reason),
            })
            .collect()
    }

    /// 基于销售趋势的产品推荐
    async fn generate_trend_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let now = Utc::now().date_naive();
        let recent_start = now - Duration::days(30);
        let earlier_start = now - Duration::days(60);

        // 最近 30 天的销售
        let recent_items = SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    recent_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

        // 前 30 天的销售
        let earlier_items = SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    earlier_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.lt(recent_start
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_default()
                    .and_utc()),
            )
            .all(&*self.db)
            .await?;

        // 按产品聚合
        let mut recent_sales: HashMap<i32, f64> = HashMap::new();
        for item in &recent_items {
            *recent_sales.entry(item.product_id).or_insert(0.0) +=
                item.quantity_meters.to_f64().unwrap_or(0.0);
        }

        let mut earlier_sales: HashMap<i32, f64> = HashMap::new();
        for item in &earlier_items {
            *earlier_sales.entry(item.product_id).or_insert(0.0) +=
                item.quantity_meters.to_f64().unwrap_or(0.0);
        }

        // 计算增长率
        let mut growth_items: Vec<(i32, f64, f64, f64)> = Vec::new();
        for (pid, &recent) in &recent_sales {
            let earlier = earlier_sales.get(pid).copied().unwrap_or(0.0);
            let growth = if earlier > 0.0 {
                (recent - earlier) / earlier
            } else if recent > 0.0 {
                1.0 // 新品
            } else {
                0.0
            };
            growth_items.push((*pid, recent, earlier, growth));
        }

        // 按增长率降序排列
        growth_items.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let recommendations: Vec<SmartRecommendation> = growth_items
            .into_iter()
            .take(limit)
            .map(|(pid, recent, earlier, growth)| {
                let reason = if earlier <= 0.0 && recent > 0.0 {
                    format!("新品热销: 最近30天销量={:.0}", recent)
                } else if growth > 0.5 {
                    format!(
                        "销量快速增长: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent,
                        earlier,
                        growth * 100.0
                    )
                } else if growth > 0.0 {
                    format!(
                        "销量稳步增长: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent,
                        earlier,
                        growth * 100.0
                    )
                } else {
                    format!(
                        "销量下降: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent,
                        earlier,
                        growth * 100.0
                    )
                };
                SmartRecommendation {
                    recommendation_type: "TREND".to_string(),
                    target_id: pid,
                    target_type: "PRODUCT".to_string(),
                    score: (growth * 100.0).round() / 100.0,
                    reason,
                }
            })
            .collect();

        Ok(recommendations)
    }

    /// 基于库存水平的价格调整推荐
    ///
    /// 批次 86 v2 复审 P2-13 修复：加 LIMIT 兜底防止全表加载内存爆炸
    async fn generate_price_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let stocks = InventoryStockEntity::find()
            .limit(10_000)
            .all(&*self.db)
            .await?;

        let start_date = Utc::now().date_naive() - Duration::days(30);
        let transactions = InventoryTransactionEntity::find()
            .filter(
                crate::models::inventory_transaction::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

        let mut outbound_map: HashMap<i32, f64> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                *outbound_map.entry(tx.product_id).or_insert(0.0) +=
                    tx.quantity_meters.to_f64().unwrap_or(0.0);
            }
        }

        let mut recommendations = Vec::new();

        for stock in &stocks {
            let qty = stock.quantity_available.to_f64().unwrap_or(0.0);
            let outbound = outbound_map.get(&stock.product_id).copied().unwrap_or(0.0);

            if qty > 500.0 && outbound < qty * 0.1 {
                recommendations.push(SmartRecommendation {
                    recommendation_type: "PRICE_ADJUST".to_string(),
                    target_id: stock.product_id,
                    target_type: "PRODUCT".to_string(),
                    score: qty / outbound.max(1.0),
                    reason: format!(
                        "库存积压({:.0})且近30天出库量({:.0})低，建议降价促销或捆绑销售",
                        qty, outbound
                    ),
                });
            }
        }

        recommendations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        recommendations.truncate(limit);

        Ok(recommendations)
    }
}
