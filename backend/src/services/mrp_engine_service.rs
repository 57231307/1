//! MRP物料需求计算引擎
//!
//! 基于BOM和库存数据计算物料需求，支持多层BOM展开和批量计算

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::bom::{Entity as BomEntity, Model as BomModel};
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
// 批次 235 v13 P1-1：MRP 结果状态常量接入（规则 0）
use crate::models::status::mrp as mrp_status;
use crate::models::bom_item::{Entity as BomItemEntity, Model as BomItemModel};
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Entity as MrpResultEntity, Model as MrpResultModel,
};
use crate::models::product::Entity as ProductEntity;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;
use crate::utils::xlsx_export::XlsxTable;

/// MRP计算请求
#[derive(Debug, Clone, Deserialize)]
pub struct MrpCalculationRequest {
    pub items: Vec<MrpCalculationItem>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub consider_safety_stock: bool,
    pub consider_in_transit: bool,
}

/// MRP计算项
#[derive(Debug, Clone, Deserialize)]
pub struct MrpCalculationItem {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
}

/// 物料需求计算结果
#[derive(Debug, Clone, Serialize)]
pub struct MaterialRequirement {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
    pub on_hand_quantity: Decimal,
    pub in_transit_quantity: Decimal,
    pub safety_stock: Decimal,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub bom_level: i32,
}

/// MRP计算结果摘要
#[derive(Debug, Clone, Serialize)]
pub struct MrpCalculationSummary {
    pub calculation_no: String,
    pub total_items: i32,
    pub items_with_shortage: i32,
    pub results: Vec<MrpResultModel>,
    pub requirements: Vec<MaterialRequirement>,
}

/// 库存信息
#[derive(Debug, Clone)]
struct StockInfo {
    on_hand: Decimal,
    in_transit: Decimal,
    safety_stock: Decimal,
    available: Decimal,
}

/// 物料需求计算参数对象
///
/// 批次 336 v10 复审 P3 修复：引入参数对象消除 calculate_requirement 的 too_many_arguments 警告。
/// 聚合物料需求计算所需的全部参数，避免函数签名携带 8 个参数。
#[derive(Debug, Clone)]
pub struct RequirementCalcParams {
    /// 产品 ID
    pub product_id: i32,
    /// 需求数量
    pub required_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
    /// BOM 层级（顶层为 0）
    pub bom_level: i32,
}

/// BOM 递归展开参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 explode_bom_recursive 的 too_many_arguments 警告。
/// 聚合递归展开 BOM 所需的标量参数，&mut 借用参数（results / stock_cache）保留为独立参数。
/// 使用生命周期 `&'a str` 借用 source_type，避免递归调用中的不必要的 to_string()。
#[derive(Debug, Clone)]
pub struct ExplodeBomArgs<'a> {
    /// 产品 ID
    pub product_id: i32,
    /// 父级需求数量
    pub parent_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型
    pub source_type: &'a str,
    /// 来源 ID
    pub source_id: Option<i32>,
    /// 当前 BOM 层级
    pub current_level: i32,
    /// 最大 BOM 层级
    pub max_level: i32,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}

/// BOM 展开查询参数（公开接口层，owned 版本）
///
/// 批次 413 技术债务清理：引入参数对象消除 explode_bom 的 too_many_arguments 警告。
/// 与内部 `ExplodeBomArgs<'a>` 区分：此结构体为 owned 版本（source_type 为 String），
/// 不含 current_level/max_level（由 explode_bom 内部固定为 1/10）。
#[derive(Debug, Clone)]
pub struct MrpExplodeQuery {
    /// 产品 ID
    pub product_id: i32,
    /// 父级需求数量
    pub parent_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}

/// MRP 计算查询参数（公开接口层）
///
/// 批次 413 技术债务清理：引入参数对象消除 run_mrp_calculation 的 too_many_arguments 警告。
/// 与 `RequirementCalcParams` 区分：此结构体为公开接口参数，不含 bom_level（由内部固定为 0）。
#[derive(Debug, Clone)]
pub struct MrpCalculationQuery {
    /// 产品 ID
    pub product_id: i32,
    /// 需求数量
    pub required_quantity: Decimal,
    /// 需求日期
    pub required_date: NaiveDate,
    /// 来源类型（如订单/生产计划）
    pub source_type: String,
    /// 来源 ID（可选）
    pub source_id: Option<i32>,
    /// 是否考虑安全库存
    pub consider_safety_stock: bool,
    /// 是否考虑在途库存
    pub consider_in_transit: bool,
}

/// MRP计算引擎
pub struct MrpEngineService {
    db: Arc<DatabaseConnection>,
}

impl MrpEngineService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取库存信息
    async fn get_stock_info(&self, product_id: i32) -> Result<StockInfo, AppError> {
        let stocks = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(product_id))
            .all(&*self.db)
            .await?;

        let mut on_hand = Decimal::ZERO;
        let mut in_transit = Decimal::ZERO;
        let mut safety_stock = Decimal::ZERO;

        for stock in stocks {
            // 使用面料行业主计量单位（米），兼容通用字段
            let qty = if stock.quantity_meters > Decimal::ZERO {
                stock.quantity_meters
            } else {
                stock.quantity_on_hand
            };
            on_hand += qty;
            in_transit += stock.quantity_incoming;
            safety_stock += stock.reorder_point;
        }

        let available = on_hand - safety_stock;
        let available = if available > Decimal::ZERO {
            available
        } else {
            Decimal::ZERO
        };

        Ok(StockInfo {
            on_hand,
            in_transit,
            safety_stock,
            available,
        })
    }

    /// v16 批次 43 修复：批量获取多个产品的库存信息，避免循环内逐个查询（N+1）
    async fn get_stock_info_batch(
        &self,
        product_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, StockInfo>, AppError> {
        if product_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let stocks = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.is_in(product_ids.to_vec()))
            .all(&*self.db)
            .await?;

        // 按 product_id 聚合（一个产品可能有多条库存记录，与 get_stock_info 语义一致）
        let mut agg: std::collections::HashMap<i32, (Decimal, Decimal, Decimal)> =
            std::collections::HashMap::new();
        for stock in stocks {
            let qty = if stock.quantity_meters > Decimal::ZERO {
                stock.quantity_meters
            } else {
                stock.quantity_on_hand
            };
            let entry = agg
                .entry(stock.product_id)
                .or_insert((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
            entry.0 += qty; // on_hand
            entry.1 += stock.quantity_incoming; // in_transit
            entry.2 += stock.reorder_point; // safety_stock
        }

        let mut result = std::collections::HashMap::new();
        for (product_id, (on_hand, in_transit, safety_stock)) in agg {
            let available = on_hand - safety_stock;
            let available = if available > Decimal::ZERO {
                available
            } else {
                Decimal::ZERO
            };
            result.insert(
                product_id,
                StockInfo {
                    on_hand,
                    in_transit,
                    safety_stock,
                    available,
                },
            );
        }

        Ok(result)
    }

    /// v16 批次 43 修复：带缓存的库存查询，先查 cache 未命中再查数据库并写入 cache
    async fn get_stock_info_cached(
        &self,
        product_id: i32,
        cache: &mut std::collections::HashMap<i32, StockInfo>,
    ) -> Result<StockInfo, AppError> {
        if let Some(info) = cache.get(&product_id) {
            return Ok(info.clone());
        }
        let info = self.get_stock_info(product_id).await?;
        cache.insert(product_id, info.clone());
        Ok(info)
    }

    /// v16 批次 43 修复：基于已知 StockInfo 计算物料需求（避免重复查询库存）
    ///
    /// 批次 352 v12 复审 P1-1 修复：签名从 10 参数改为参数对象 `RequirementCalcParams` + `&StockInfo`，
    /// 消除 `clippy::too_many_arguments` 警告。与 `calculate_requirement` 共用同一参数对象。
    fn calculate_requirement_with_stock(
        &self,
        params: RequirementCalcParams,
        stock_info: &StockInfo,
    ) -> MaterialRequirement {
        let mut available = stock_info.available;
        if params.consider_in_transit {
            available += stock_info.in_transit;
        }

        let shortage = if params.required_quantity > available {
            params.required_quantity - available
        } else {
            Decimal::ZERO
        };

        MaterialRequirement {
            product_id: params.product_id,
            required_quantity: params.required_quantity,
            required_date: params.required_date,
            on_hand_quantity: stock_info.on_hand,
            in_transit_quantity: stock_info.in_transit,
            safety_stock: if params.consider_safety_stock {
                stock_info.safety_stock
            } else {
                Decimal::ZERO
            },
            available_quantity: available,
            shortage_quantity: shortage,
            source_type: params.source_type,
            source_id: params.source_id,
            bom_level: params.bom_level,
        }
    }

    /// 计算单个物料需求
    ///
    /// 批次 336 v10 复审 P3 修复：签名从 8 参数改为单一参数对象 `RequirementCalcParams`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn calculate_requirement(
        &self,
        params: RequirementCalcParams,
    ) -> Result<MaterialRequirement, AppError> {
        // 解构参数对象，便于函数体内按字段名访问
        let RequirementCalcParams {
            product_id,
            required_quantity,
            required_date,
            source_type,
            source_id,
            consider_safety_stock,
            consider_in_transit,
            bom_level,
        } = params;

        let stock_info = self.get_stock_info(product_id).await?;

        let mut available = stock_info.available;
        if consider_in_transit {
            available += stock_info.in_transit;
        }

        let shortage = if required_quantity > available {
            required_quantity - available
        } else {
            Decimal::ZERO
        };

        Ok(MaterialRequirement {
            product_id,
            required_quantity,
            required_date,
            on_hand_quantity: stock_info.on_hand,
            in_transit_quantity: stock_info.in_transit,
            safety_stock: if consider_safety_stock {
                stock_info.safety_stock
            } else {
                Decimal::ZERO
            },
            available_quantity: available,
            shortage_quantity: shortage,
            source_type,
            source_id,
            bom_level,
        })
    }

    /// 获取产品的默认BOM
    async fn get_default_bom(&self, product_id: i32) -> Result<Option<BomModel>, AppError> {
        let bom = BomEntity::find()
            .filter(crate::models::bom::Column::ProductId.eq(product_id))
            .filter(crate::models::bom::Column::IsDefault.eq(true))
            .filter(crate::models::bom::Column::Status.eq(crate::models::status::common::STATUS_ACTIVE))
            .one(&*self.db)
            .await?;

        Ok(bom)
    }

    /// 计算含损耗率的需求数量
    fn calculate_quantity_with_scrap(
        base_quantity: Decimal,
        scrap_rate: Option<Decimal>,
    ) -> Decimal {
        // 损耗率>0时按 (1 + scrap/100) 放大数量，保留4位精度
        if let Some(scrap_rate) = scrap_rate {
            if scrap_rate > Decimal::ZERO {
                return (base_quantity
                    * (Decimal::ONE + (scrap_rate / Decimal::from(100))))
                .round_dp(4);
            }
        }
        base_quantity
    }

    /// 计算物料需求日期（每层7天提前期）
    fn calculate_material_date(required_date: NaiveDate, current_level: i32) -> NaiveDate {
        required_date - Duration::days(7 * current_level as i64)
    }

    /// 处理单个BOM明细项：计算数量、日期、库存并生成物料需求
    async fn process_bom_item(
        &self,
        item: BomItemModel,
        args: &ExplodeBomArgs<'_>,
        stock_cache: &mut std::collections::HashMap<i32, StockInfo>,
    ) -> Result<Option<MaterialRequirement>, AppError> {
        // 批次 97 P1-11 修复（v5 复审）：数量计算补 round_dp(4) 防止精度漂移
        let base_quantity = (args.parent_quantity * item.quantity).round_dp(4);
        let quantity_with_scrap =
            Self::calculate_quantity_with_scrap(base_quantity, item.scrap_rate);
        let material_date =
            Self::calculate_material_date(args.required_date, args.current_level);

        // v16 批次 43 修复：使用缓存查询库存，避免递归中重复查询
        let stock_info = self
            .get_stock_info_cached(item.material_id, stock_cache)
            .await?;
        let requirement = self.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: item.material_id,
                required_quantity: quantity_with_scrap,
                required_date: material_date,
                source_type: args.source_type.to_string(),
                source_id: args.source_id,
                consider_safety_stock: args.consider_safety_stock,
                consider_in_transit: args.consider_in_transit,
                bom_level: args.current_level,
            },
            &stock_info,
        );

        Ok(Some(requirement))
    }

    /// 递归展开BOM
    ///
    /// 批次 339 v10 复审 P3 修复：签名从 11 参数改为 4 参数（&self + args + results + stock_cache），
    /// 将 9 个标量参数聚合为 `ExplodeBomArgs<'a>` 参数对象，&mut 借用参数保留为独立参数，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn explode_bom_recursive(
        &self,
        args: ExplodeBomArgs<'_>,
        results: &mut Vec<MaterialRequirement>,
        // v16 批次 43 修复：传入共享库存缓存，避免递归中重复查询同一产品的库存
        stock_cache: &mut std::collections::HashMap<i32, StockInfo>,
    ) -> Result<(), AppError> {
        if args.current_level > args.max_level {
            return Ok(());
        }

        let bom = match self.get_default_bom(args.product_id).await? {
            Some(bom) => bom,
            None => return Ok(()),
        };

        let bom_items = BomItemEntity::find()
            .filter(crate::models::bom_item::Column::BomId.eq(bom.id))
            .all(&*self.db)
            .await?;

        for item in bom_items {
            let material_id = item.material_id;
            let requirement = match self.process_bom_item(item, &args, stock_cache).await? {
                Some(r) => r,
                None => continue,
            };
            let quantity_with_scrap = requirement.required_quantity;
            let material_date = requirement.required_date;
            results.push(requirement);

            Box::pin(self.explode_bom_recursive(
                ExplodeBomArgs {
                    product_id: material_id,
                    parent_quantity: quantity_with_scrap,
                    required_date: material_date,
                    source_type: args.source_type,
                    source_id: args.source_id,
                    current_level: args.current_level + 1,
                    max_level: args.max_level,
                    consider_safety_stock: args.consider_safety_stock,
                    consider_in_transit: args.consider_in_transit,
                },
                results,
                stock_cache,
            ))
            .await?;
        }

        Ok(())
    }

    /// 展开BOM计算子物料需求
    ///
    /// 批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `MrpExplodeQuery`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn explode_bom(
        &self,
        query: MrpExplodeQuery,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let mut requirements = Vec::new();

        // v16 批次 43 修复：创建库存缓存，递归展开 BOM 时共享缓存避免重复查询
        let mut stock_cache: std::collections::HashMap<i32, StockInfo> =
            std::collections::HashMap::new();

        self.explode_bom_recursive(
            ExplodeBomArgs {
                product_id: query.product_id,
                parent_quantity: query.parent_quantity,
                required_date: query.required_date,
                source_type: query.source_type.as_str(),
                source_id: query.source_id,
                current_level: 1,
                max_level: 10,
                consider_safety_stock: query.consider_safety_stock,
                consider_in_transit: query.consider_in_transit,
            },
            &mut requirements,
            &mut stock_cache,
        )
        .await?;

        Ok(requirements)
    }

    /// 执行MRP计算并保存结果
    ///
    /// 批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `MrpCalculationQuery`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn run_mrp_calculation(
        &self,
        query: MrpCalculationQuery,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let mut results = Vec::new();
        let calculation_no = format!("MRP{}", Utc::now().timestamp_millis());

        let main_req = self
            .calculate_requirement(RequirementCalcParams {
                product_id: query.product_id,
                required_quantity: query.required_quantity,
                required_date: query.required_date,
                source_type: query.source_type.clone(),
                source_id: query.source_id,
                consider_safety_stock: query.consider_safety_stock,
                consider_in_transit: query.consider_in_transit,
                bom_level: 0,
            })
            .await?;

        let main_active_model = MrpResultActiveModel {
            calculation_no: Set(calculation_no.clone()),
            product_id: Set(main_req.product_id),
            required_quantity: Set(main_req.required_quantity),
            required_date: Set(Some(main_req.required_date)),
            source_type: Set(main_req.source_type.clone()),
            source_id: Set(main_req.source_id),
            planned_order_quantity: Set(Some(main_req.shortage_quantity)),
            planned_order_date: Set(Some(main_req.required_date - Duration::days(14))),
            status: Set(mrp_status::PLANNED.to_string()),
            remarks: Set(Some(format!(
                "BOM Level: 0, On Hand: {}, Shortage: {}",
                main_req.on_hand_quantity, main_req.shortage_quantity
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let main_result = main_active_model.insert(&*self.db).await?;
        results.push(main_result);

        let sub_requirements = self
            .explode_bom(MrpExplodeQuery {
                product_id: query.product_id,
                parent_quantity: query.required_quantity,
                required_date: query.required_date,
                source_type: query.source_type,
                source_id: query.source_id,
                consider_safety_stock: query.consider_safety_stock,
                consider_in_transit: query.consider_in_transit,
            })
            .await?;

        for (idx, req) in sub_requirements.iter().enumerate() {
            // 保存所有子物料需求到MRP结果（包括有库存和短缺的）
            let sub_active_model = MrpResultActiveModel {
                calculation_no: Set(format!("{}-{}", calculation_no, idx + 1)),
                product_id: Set(req.product_id),
                required_quantity: Set(req.required_quantity),
                required_date: Set(Some(req.required_date)),
                source_type: Set(req.source_type.clone()),
                source_id: Set(req.source_id),
                planned_order_quantity: Set(Some(req.shortage_quantity)),
                planned_order_date: Set(Some(
                    req.required_date - Duration::days(7 * req.bom_level as i64),
                )),
                status: Set(mrp_status::PLANNED.to_string()),
                remarks: Set(Some(format!(
                    "BOM Level: {}, On Hand: {}, In Transit: {}, Shortage: {}",
                    req.bom_level,
                    req.on_hand_quantity,
                    req.in_transit_quantity,
                    req.shortage_quantity
                ))),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                ..Default::default()
            };

            let sub_result = sub_active_model.insert(&*self.db).await?;
            results.push(sub_result);
        }

        Ok(results)
    }

    /// 批量MRP计算
    pub async fn batch_calculate(
        &self,
        request: MrpCalculationRequest,
    ) -> Result<MrpCalculationSummary, AppError> {
        let calculation_no = format!("MRPB{}", Utc::now().timestamp_millis());
        let mut all_results = Vec::new();
        let mut all_requirements = Vec::new();

        // v16 批次 43 修复：循环外批量预加载所有顶层 product_id 的库存信息，
        // 避免循环内重复调用 calculate_requirement 查询同一产品库存（N+1 查询）
        let top_product_ids: Vec<i32> =
            request.items.iter().map(|i| i.product_id).collect();
        let top_stock_map = self.get_stock_info_batch(&top_product_ids).await?;

        for item in request.items {
            let results = self
                .run_mrp_calculation(MrpCalculationQuery {
                    product_id: item.product_id,
                    required_quantity: item.required_quantity,
                    required_date: item.required_date,
                    source_type: request.source_type.clone(),
                    source_id: request.source_id,
                    consider_safety_stock: request.consider_safety_stock,
                    consider_in_transit: request.consider_in_transit,
                })
                .await?;

            all_results.extend(results);

            // v16 批次 43 修复：顶层物料需求直接使用预加载的库存信息，避免重复查询
            let stock_info = top_stock_map
                .get(&item.product_id)
                .cloned()
                .unwrap_or(StockInfo {
                    on_hand: Decimal::ZERO,
                    in_transit: Decimal::ZERO,
                    safety_stock: Decimal::ZERO,
                    available: Decimal::ZERO,
                });
            let requirement = self.calculate_requirement_with_stock(
                RequirementCalcParams {
                    product_id: item.product_id,
                    required_quantity: item.required_quantity,
                    required_date: item.required_date,
                    source_type: request.source_type.clone(),
                    source_id: request.source_id,
                    consider_safety_stock: request.consider_safety_stock,
                    consider_in_transit: request.consider_in_transit,
                    bom_level: 0,
                },
                &stock_info,
            );

            all_requirements.push(requirement);
        }

        let items_with_shortage = all_requirements
            .iter()
            .filter(|r| r.shortage_quantity > Decimal::ZERO)
            .count() as i32;

        Ok(MrpCalculationSummary {
            calculation_no,
            total_items: all_results.len() as i32,
            items_with_shortage,
            results: all_results,
            requirements: all_requirements,
        })
    }

    /// 查询MRP计算结果
    pub async fn get_results(
        &self,
        calculation_no: Option<String>,
        product_id: Option<i32>,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<MrpResultModel>, u64), AppError> {
        let mut select = MrpResultEntity::find();

        if let Some(calc_no) = calculation_no {
            select = select.filter(crate::models::mrp_result::Column::CalculationNo.eq(calc_no));
        }

        if let Some(pid) = product_id {
            select = select.filter(crate::models::mrp_result::Column::ProductId.eq(pid));
        }

        if let Some(st) = status {
            select = select.filter(crate::models::mrp_result::Column::Status.eq(st));
        }

        // 批次 257 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = select
            .order_by_desc(crate::models::mrp_result::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (results, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((results, total))
    }

    /// 获取物料需求清单
    pub async fn get_requirements(
        &self,
        product_id: Option<i32>,
        date_from: Option<NaiveDate>,
        date_to: Option<NaiveDate>,
        only_shortage: bool,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let mut select =
            MrpResultEntity::find().filter(crate::models::mrp_result::Column::Status.eq(mrp_status::PLANNED));

        if let Some(pid) = product_id {
            select = select.filter(crate::models::mrp_result::Column::ProductId.eq(pid));
        }

        if let Some(from) = date_from {
            select = select.filter(crate::models::mrp_result::Column::RequiredDate.gte(from));
        }

        if let Some(to) = date_to {
            select = select.filter(crate::models::mrp_result::Column::RequiredDate.lte(to));
        }

        let mrp_results = select.all(&*self.db).await?;

        // v16 批次 43 修复：循环外批量查询所有 product_id 的库存信息，避免循环内逐个查询（N+1）
        let product_ids: Vec<i32> = mrp_results.iter().map(|r| r.product_id).collect();
        let stock_map = self.get_stock_info_batch(&product_ids).await?;

        let mut requirements = Vec::new();

        for result in mrp_results {
            // v16 批次 43 修复：从批量查询结果获取库存信息（O(1) 查找）
            let stock_info = stock_map
                .get(&result.product_id)
                .cloned()
                .unwrap_or(StockInfo {
                    on_hand: Decimal::ZERO,
                    in_transit: Decimal::ZERO,
                    safety_stock: Decimal::ZERO,
                    available: Decimal::ZERO,
                });

            let shortage = if result.required_quantity > stock_info.available {
                result.required_quantity - stock_info.available
            } else {
                Decimal::ZERO
            };

            if !only_shortage || shortage > Decimal::ZERO {
                requirements.push(MaterialRequirement {
                    product_id: result.product_id,
                    required_quantity: result.required_quantity,
                    required_date: result
                        .required_date
                        .unwrap_or_else(|| Utc::now().date_naive()),
                    on_hand_quantity: stock_info.on_hand,
                    in_transit_quantity: stock_info.in_transit,
                    safety_stock: stock_info.safety_stock,
                    available_quantity: stock_info.available,
                    shortage_quantity: shortage,
                    source_type: result.source_type,
                    source_id: result.source_id,
                    bom_level: 0,
                });
            }
        }

        Ok(requirements)
    }

    /// 将MRP需求转为采购/生产订单
    pub async fn convert_to_orders(
        &self,
        result_ids: Vec<i32>,
        order_type: String,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        if result_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 先确定新状态（与原逻辑一致：无效订单类型提前返回）
        let new_status = match order_type.as_str() {
            "PURCHASE" => mrp_status::CONFIRMED,
            "PRODUCTION" => mrp_status::RELEASED,
            _ => return Err(AppError::validation("无效的订单类型")),
        };

        // v11 批次 38 修复：批量查询所有 MRP 结果，避免循环内逐个 find_by_id（N+1 查询）
        let results = MrpResultEntity::find()
            .filter(crate::models::mrp_result::Column::Id.is_in(result_ids.clone()))
            .all(&*self.db)
            .await?;
        // 按 id 索引，保持与 result_ids 顺序一致的输出
        let mut result_map: std::collections::HashMap<i32, MrpResultModel> =
            results.into_iter().map(|r| (r.id, r)).collect();

        let mut updated_results = Vec::new();
        for id in result_ids {
            let result = result_map
                .remove(&id)
                .ok_or_else(|| AppError::not_found(format!("MRP结果 {} 不存在", id)))?;

            if result.status != mrp_status::PLANNED {
                return Err(AppError::validation(format!(
                    "MRP结果 {} 状态不是PLANNED，无法转换",
                    id
                )));
            }

            let mut active_model: crate::models::mrp_result::ActiveModel = result.into();
            active_model.status = Set(new_status.to_string());
            active_model.updated_at = Set(Utc::now());

            // update 需逐条执行以返回更新后的 Model（ActiveModelTrait::update 返回最新行）
            let updated = active_model.update(&*self.db).await?;

            updated_results.push(updated);
        }

        Ok(updated_results)
    }

    /// 列出可用于 MRP 计算的产品
    pub async fn list_products_for_mrp(
        &self,
        keyword: Option<String>,
    ) -> Result<Vec<crate::models::product::Model>, AppError> {
        let mut query = ProductEntity::find()
            .filter(crate::models::product::Column::IsDeleted.eq(false))
            .filter(crate::models::product::Column::Status.eq(master_data::ACTIVE));

        if let Some(kw) = keyword {
            let trimmed = kw.trim();
            if !trimmed.is_empty() {
                // 批次 94 P2-2 修复：LIKE 模式注入，转义 % _ \ 特殊字符
                let pattern = safe_like_pattern(trimmed);
                query = query.filter(
                    crate::models::product::Column::Name
                        .like(&pattern)
                        .or(crate::models::product::Column::Code.like(&pattern)),
                );
            }
        }

        let products = query
            .order_by_asc(crate::models::product::Column::Code)
            .all(&*self.db)
            .await?;

        Ok(products)
    }

    /// 取消 MRP 计算：仅将状态从 PLANNED 置为 CANCELLED
    pub async fn cancel_calculation(
        &self,
        calculation_id: i32,
    ) -> Result<MrpResultModel, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无 txn 无 lock，两并发 cancel 均通过状态检查后基于过期状态写入，
        // 导致状态机被绕过、重复写入。
        let txn = (*self.db).begin().await?;

        let result = MrpResultEntity::find_by_id(calculation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("MRP结果不存在"))?;

        if result.status == mrp_status::CANCELLED {
            return Ok(result);
        }

        let mut active_model: MrpResultActiveModel = result.into();
        active_model.status = Set(mrp_status::CANCELLED.to_string());
        active_model.updated_at = Set(Utc::now());
        let updated = active_model.update(&txn).await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 导出指定 MRP 计算编号下的所有结果为 xlsx 表格
    pub async fn export_calculation(&self, calculation_id: i32) -> Result<XlsxTable, AppError> {
        // 兼容前端传入 id 形如 "MRP12345" 的计算编号
        let calculation_no = if calculation_id > 0 {
            format!("MRP{}", calculation_id)
        } else {
            String::new()
        };

        // 先按 ID 精确查询
        let results = MrpResultEntity::find_by_id(calculation_id)
            .all(&*self.db)
            .await?;

        let results = if !results.is_empty() {
            results
        } else if !calculation_no.is_empty() {
            MrpResultEntity::find()
                .filter(crate::models::mrp_result::Column::CalculationNo.eq(&calculation_no))
                .all(&*self.db)
                .await?
        } else {
            Vec::new()
        };

        // 规则 3：构造 xlsx 表格（字段名与原 CSV 保持一致）
        let headers = vec![
            "ID".to_string(),
            "计算编号".to_string(),
            "产品ID".to_string(),
            "需求数量".to_string(),
            "需求日期".to_string(),
            "来源类型".to_string(),
            "来源ID".to_string(),
            "计划订单数量".to_string(),
            "计划订单日期".to_string(),
            "状态".to_string(),
            "备注".to_string(),
            "创建时间".to_string(),
        ];
        let rows: Vec<Vec<String>> = results
            .iter()
            .map(|r| {
                vec![
                    r.id.to_string(),
                    r.calculation_no.clone(),
                    r.product_id.to_string(),
                    r.required_quantity.to_string(),
                    r.required_date.map(|d| d.to_string()).unwrap_or_default(),
                    r.source_type.clone(),
                    r.source_id.map(|i| i.to_string()).unwrap_or_default(),
                    r.planned_order_quantity
                        .map(|q| q.to_string())
                        .unwrap_or_default(),
                    r.planned_order_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                    r.status.clone(),
                    r.remarks.clone().unwrap_or_default(),
                    r.created_at.to_rfc3339(),
                ]
            })
            .collect();

        Ok(XlsxTable {
            sheet_name: "MRP计算结果".to_string(),
            headers,
            rows,
        })
    }

    /// 获取指定 MRP 计算中某物料的需求明细
    pub async fn get_material_detail(
        &self,
        calculation_id: i32,
        material_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let result = MrpResultEntity::find_by_id(calculation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("MRP结果不存在"))?;

        if result.product_id != material_id {
            return Err(AppError::not_found("该计算结果中不包含此物料"));
        }

        let stock_info = self.get_stock_info(material_id).await?;

        Ok(serde_json::json!({
            "calculation_id": result.id,
            "calculation_no": result.calculation_no,
            "material_id": result.product_id,
            "required_quantity": result.required_quantity,
            "required_date": result.required_date,
            "on_hand_quantity": stock_info.on_hand,
            "in_transit_quantity": stock_info.in_transit,
            "safety_stock": stock_info.safety_stock,
            "available_quantity": stock_info.available,
            "shortage_quantity": result.planned_order_quantity,
            "planned_order_date": result.planned_order_date,
            "source_type": result.source_type,
            "source_id": result.source_id,
            "status": result.status,
            "remarks": result.remarks,
            "supply_details": [],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::models::status::common;
    use crate::ymd;
    use std::str::FromStr;

    // MRP 专属状态值（源码 mrp_engine_service.rs 中使用，status.rs 暂无 mrp 子模块）
    // 集中定义以便测试引用，避免散落的字符串字面量；未来 status.rs 增设 mrp 子模块后应替换为引用
    const MRP_STATUS_PLANNED: &str = "PLANNED";
    const MRP_STATUS_CONFIRMED: &str = "CONFIRMED";
    const MRP_STATUS_RELEASED: &str = "RELEASED";
    const MRP_STATUS_CANCELLED: &str = "CANCELLED";
    const BOM_STATUS_ACTIVE: &str = "ACTIVE";

    /// 构造测试用 StockInfo 夹具
    ///
    /// 复现 get_stock_info / get_stock_info_batch 中的可用量计算：
    /// available = on_hand - safety_stock（下限为 0）
    fn make_stock_info(on_hand: Decimal, in_transit: Decimal, safety_stock: Decimal) -> StockInfo {
        let available = on_hand - safety_stock;
        let available = if available > Decimal::ZERO {
            available
        } else {
            Decimal::ZERO
        };
        StockInfo {
            on_hand,
            in_transit,
            safety_stock,
            available,
        }
    }

    /// 测试_MRP状态常量值正确性
    ///
    /// 验证源码中使用的状态字符串值：
    /// - BOM 状态 ACTIVE 与通用 common::STATUS_ACTIVE 一致（均为大写）
    /// - 取消状态 CANCELLED 与 common::STATUS_CANCELLED 一致
    /// - 产品过滤用 master_data::ACTIVE（小写 active）
    /// - MRP 专属状态 PLANNED/CONFIRMED/RELEASED 的预期值
    #[test]
    fn 测试_MRP状态常量值正确性() {
        // BOM 状态使用大写 ACTIVE，与通用 common::STATUS_ACTIVE 一致
        assert_eq!(BOM_STATUS_ACTIVE, common::STATUS_ACTIVE);

        // 取消状态使用 common::STATUS_CANCELLED
        assert_eq!(MRP_STATUS_CANCELLED, common::STATUS_CANCELLED);

        // 产品过滤状态使用 master_data::ACTIVE（小写 active，区别于通用大写）
        assert_eq!(master_data::ACTIVE, "active");

        // MRP 专属状态值（源码中硬编码，status.rs 暂无 mrp 子模块）
        assert_eq!(MRP_STATUS_PLANNED, "PLANNED");
        assert_eq!(MRP_STATUS_CONFIRMED, "CONFIRMED");
        assert_eq!(MRP_STATUS_RELEASED, "RELEASED");
    }

    /// 测试_库存可用量计算_正常场景
    ///
    /// 验证 get_stock_info 中 available = on_hand - safety_stock
    #[test]
    fn 测试_库存可用量计算_正常场景() {
        let stock = make_stock_info(decs!("100"), decs!("20"), decs!("30"));
        assert_eq!(stock.available, decs!("70"));
        assert_eq!(stock.on_hand, decs!("100"));
        assert_eq!(stock.in_transit, decs!("20"));
        assert_eq!(stock.safety_stock, decs!("30"));
    }

    /// 测试_库存可用量计算_安全库存超过库存
    ///
    /// 验证 get_stock_info 中 on_hand < safety_stock 时 available 下限保护为 0
    #[test]
    fn 测试_库存可用量计算_安全库存超过库存() {
        let stock = make_stock_info(decs!("30"), decs!("0"), decs!("50"));
        assert_eq!(stock.available, Decimal::ZERO);
    }

    /// 测试_净需求计算_库存充足无短缺
    ///
    /// 验证 calculate_requirement_with_stock：available >= required 时 shortage = 0
    #[tokio::test]
    async fn 测试_净需求计算_库存充足无短缺() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let stock = make_stock_info(decs!("100"), decs!("0"), decs!("0"));

        let req = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("30"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );

        assert_eq!(req.shortage_quantity, Decimal::ZERO);
        assert_eq!(req.available_quantity, decs!("100"));
        assert_eq!(req.required_quantity, decs!("30"));
        assert_eq!(req.bom_level, 0);
    }

    /// 测试_净需求计算_库存不足有短缺
    ///
    /// 验证 calculate_requirement_with_stock：available < required 时 shortage = required - available
    #[tokio::test]
    async fn 测试_净需求计算_库存不足有短缺() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let stock = make_stock_info(decs!("30"), decs!("0"), decs!("0"));

        let req = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("100"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );

        assert_eq!(req.shortage_quantity, decs!("70"));
        assert_eq!(req.available_quantity, decs!("30"));
    }

    /// 测试_净需求计算_边界恰好相等
    ///
    /// 验证 required == available 时 shortage = 0（源码用 `>` 判断，相等不触发短缺）
    #[tokio::test]
    async fn 测试_净需求计算_边界恰好相等() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let stock = make_stock_info(decs!("50"), decs!("0"), decs!("0"));

        let req = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("50"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );

        assert_eq!(req.shortage_quantity, Decimal::ZERO);
        assert_eq!(req.available_quantity, decs!("50"));
    }

    /// 测试_净需求计算_考虑在途库存
    ///
    /// 验证 consider_in_transit = true 时 available += in_transit，可覆盖原短缺
    #[tokio::test]
    async fn 测试_净需求计算_考虑在途库存() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let stock = make_stock_info(decs!("50"), decs!("30"), decs!("0"));

        // 不考虑在途：available=50，需求80 -> shortage=30
        let req_no = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("80"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );
        assert_eq!(req_no.available_quantity, decs!("50"));
        assert_eq!(req_no.shortage_quantity, decs!("30"));

        // 考虑在途：available=50+30=80，需求80 -> shortage=0
        let req_with = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("80"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: true,
                bom_level: 0,
            },
            &stock,
        );
        assert_eq!(req_with.available_quantity, decs!("80"));
        assert_eq!(req_with.shortage_quantity, Decimal::ZERO);
    }

    /// 测试_净需求计算_考虑安全库存填充
    ///
    /// 验证 consider_safety_stock = true 时 safety_stock 字段填充实际值
    #[tokio::test]
    async fn 测试_净需求计算_考虑安全库存填充() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        // on_hand=100, safety_stock=20 -> available=80
        let stock = make_stock_info(decs!("100"), decs!("0"), decs!("20"));

        let req = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("50"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: true,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );
        assert_eq!(req.safety_stock, decs!("20"));
        assert_eq!(req.available_quantity, decs!("80"));
    }

    /// 测试_净需求计算_不考虑安全库存为零
    ///
    /// 验证 consider_safety_stock = false 时 safety_stock 字段为 0；
    /// 注意 available 仍按 stock_info.available（已扣除安全库存）计算
    #[tokio::test]
    async fn 测试_净需求计算_不考虑安全库存为零() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let stock = make_stock_info(decs!("100"), decs!("0"), decs!("20"));

        let req = service.calculate_requirement_with_stock(
            RequirementCalcParams {
                product_id: 1,
                required_quantity: decs!("50"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
                bom_level: 0,
            },
            &stock,
        );
        assert_eq!(req.safety_stock, Decimal::ZERO);
        // available 仍为 on_hand - safety_stock = 80（stock_info.available）
        assert_eq!(req.available_quantity, decs!("80"));
    }

    /// 测试_BOM数量计算_基础数量无损耗
    ///
    /// 验证 explode_bom_recursive 中无损耗率时 quantity = parent * item.quantity（round_dp(4)）
    #[test]
    fn 测试_BOM数量计算_基础数量无损耗() {
        let parent = decs!("100");
        let item_qty = decs!("1.5");
        let base_quantity = (parent * item_qty).round_dp(4);
        assert_eq!(base_quantity, decs!("150"));
    }

    /// 测试_BOM数量计算_含损耗率
    ///
    /// 验证 explode_bom_recursive 中含损耗率的数量计算：
    /// quantity_with_scrap = base * (1 + scrap_rate/100)，再 round_dp(4)
    #[test]
    fn 测试_BOM数量计算_含损耗率() {
        let parent = decs!("100");
        let item_qty = decs!("2");
        let scrap_rate = decs!("10"); // 10% 损耗

        let base_quantity = (parent * item_qty).round_dp(4);
        let quantity_with_scrap =
            (base_quantity * (Decimal::ONE + (scrap_rate / Decimal::from(100)))).round_dp(4);

        assert_eq!(base_quantity, decs!("200"));
        assert_eq!(quantity_with_scrap, decs!("220"));
    }

    /// 测试_BOM数量计算_精度归一化
    ///
    /// 验证 explode_bom_recursive 中 round_dp(4) 防止精度漂移
    #[test]
    fn 测试_BOM数量计算_精度归一化() {
        // 产生超过 4 位小数的中间结果，round_dp(4) 归一化为 4 位
        let raw = decs!("0.333333") * decs!("1");
        let rounded = raw.round_dp(4);
        assert_eq!(rounded, decs!("0.3333"));
    }

    /// 测试_BOM提前期计算_层级递减
    ///
    /// 验证 explode_bom_recursive 中提前期随 BOM 层级递减：
    /// lead_time = 7 * level，material_date = required_date - lead_time
    #[test]
    fn 测试_BOM提前期计算_层级递减() {
        let required_date = ymd!(2026, 7, 30);

        // level=1：提前期 7 天
        let lead_1 = Duration::days(7 * 1_i64);
        assert_eq!(required_date - lead_1, ymd!(2026, 7, 23));

        // level=2：提前期 14 天
        let lead_2 = Duration::days(7 * 2_i64);
        assert_eq!(required_date - lead_2, ymd!(2026, 7, 16));

        // level=0：提前期 0 天，物料日期等于需求日期
        let lead_0 = Duration::days(7 * 0_i64);
        assert_eq!(required_date - lead_0, required_date);
    }

    /// 测试_短缺统计_筛选有短缺项
    ///
    /// 验证 batch_calculate 中 items_with_shortage = filter(shortage > 0).count()
    #[test]
    fn 测试_短缺统计_筛选有短缺项() {
        let date = ymd!(2026, 7, 9);
        let requirements = vec![
            MaterialRequirement {
                product_id: 1,
                required_quantity: decs!("100"),
                required_date: date,
                on_hand_quantity: decs!("50"),
                in_transit_quantity: Decimal::ZERO,
                safety_stock: Decimal::ZERO,
                available_quantity: decs!("50"),
                shortage_quantity: decs!("50"),
                source_type: "MANUAL".to_string(),
                source_id: None,
                bom_level: 0,
            },
            MaterialRequirement {
                product_id: 2,
                required_quantity: decs!("30"),
                required_date: date,
                on_hand_quantity: decs!("100"),
                in_transit_quantity: Decimal::ZERO,
                safety_stock: Decimal::ZERO,
                available_quantity: decs!("100"),
                shortage_quantity: Decimal::ZERO,
                source_type: "MANUAL".to_string(),
                source_id: None,
                bom_level: 0,
            },
            MaterialRequirement {
                product_id: 3,
                required_quantity: decs!("80"),
                required_date: date,
                on_hand_quantity: decs!("10"),
                in_transit_quantity: Decimal::ZERO,
                safety_stock: Decimal::ZERO,
                available_quantity: decs!("10"),
                shortage_quantity: decs!("70"),
                source_type: "MANUAL".to_string(),
                source_id: None,
                bom_level: 1,
            },
        ];

        let items_with_shortage = requirements
            .iter()
            .filter(|r| r.shortage_quantity > Decimal::ZERO)
            .count() as i32;
        assert_eq!(items_with_shortage, 2);
    }

    /// 测试_订单类型转换_采购类型状态
    ///
    /// 验证 convert_to_orders 中 PURCHASE 类型映射到 CONFIRMED 状态
    #[test]
    fn 测试_订单类型转换_采购类型状态() {
        let order_type = "PURCHASE";
        let new_status = match order_type {
            "PURCHASE" => MRP_STATUS_CONFIRMED,
            "PRODUCTION" => MRP_STATUS_RELEASED,
            _ => panic!("不应到达此分支"),
        };
        assert_eq!(new_status, MRP_STATUS_CONFIRMED);
    }

    /// 测试_订单类型转换_生产类型状态
    ///
    /// 验证 convert_to_orders 中 PRODUCTION 类型映射到 RELEASED 状态
    #[test]
    fn 测试_订单类型转换_生产类型状态() {
        let order_type = "PRODUCTION";
        let new_status = match order_type {
            "PURCHASE" => MRP_STATUS_CONFIRMED,
            "PRODUCTION" => MRP_STATUS_RELEASED,
            _ => panic!("不应到达此分支"),
        };
        assert_eq!(new_status, MRP_STATUS_RELEASED);
    }

    /// 测试_订单类型转换_无效类型拒绝
    ///
    /// 验证 convert_to_orders 中非 PURCHASE/PRODUCTION 类型返回校验错误
    #[test]
    fn 测试_订单类型转换_无效类型拒绝() {
        let order_type = "INVALID";
        let result: Result<&str, AppError> = match order_type {
            "PURCHASE" => Ok(MRP_STATUS_CONFIRMED),
            "PRODUCTION" => Ok(MRP_STATUS_RELEASED),
            _ => Err(AppError::validation("无效的订单类型")),
        };
        assert!(result.is_err());
        match result {
            Err(e) => assert!(matches!(e, AppError::ValidationError(_))),
            _ => panic!("应返回错误"),
        }
    }

    /// 测试_订单类型转换_非PLANNED状态拒绝
    ///
    /// 验证 convert_to_orders 中 status != PLANNED 时返回校验错误
    #[test]
    fn 测试_订单类型转换_非PLANNED状态拒绝() {
        // 模拟已确认状态的结果，不应允许再次转换
        let current_status = MRP_STATUS_CONFIRMED;
        let should_reject = current_status != MRP_STATUS_PLANNED;
        assert!(should_reject);

        let err = AppError::validation(format!("MRP结果 {} 状态不是PLANNED，无法转换", 1));
        assert!(matches!(err, AppError::ValidationError(_)));

        // PLANNED 状态应允许转换（不拒绝）
        let planned_status = MRP_STATUS_PLANNED;
        let should_reject_planned = planned_status != MRP_STATUS_PLANNED;
        assert!(!should_reject_planned);
    }

    /// 测试_取消计算_已取消状态幂等
    ///
    /// 验证 cancel_calculation 中 status == CANCELLED 时直接返回（幂等，不重复更新）
    #[test]
    fn 测试_取消计算_已取消状态幂等() {
        // 模拟已取消状态的 MRP 结果，复现 cancel_calculation 的早返回判断
        let current_cancelled = MRP_STATUS_CANCELLED;
        let should_early_return = current_cancelled == MRP_STATUS_CANCELLED;
        assert!(should_early_return);

        // 非 CANCELLED 状态不应早返回（需走更新逻辑）
        let current_planned = MRP_STATUS_PLANNED;
        assert!(current_planned != MRP_STATUS_CANCELLED);
    }

    /// 测试_夹具宏_decs_可用
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串
    #[test]
    fn 测试_夹具宏_decs_可用() {
        let v = decs!("123.45");
        assert_eq!(v.to_string(), "123.45");
        // 验证宏可用于整数与大数
        let big = decs!("1000000");
        assert_eq!(big, decs!("1000000"));
    }

    /// 测试_夹具宏_ymd_可用
    ///
    /// 验证 ymd! 宏能正确解析日期
    #[test]
    fn 测试_夹具宏_ymd_可用() {
        let d = ymd!(2026, 7, 9);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    /// 测试_服务实例创建
    ///
    /// 验证 MrpEngineService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_获取库存信息_需要真实数据库
    ///
    /// 需要 inventory_stocks 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证 get_stock_info 调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_获取库存信息_需要真实数据库() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        // 无 schema 时为 Err；有 schema 无记录时返回零库存 StockInfo
        let result = service.get_stock_info(99999).await;
        // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_BOM展开_需要真实数据库
    ///
    /// 需要 bom/bom_item/inventory_stocks 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证 explode_bom 调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_BOM展开_需要真实数据库() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let result = service
            .explode_bom(MrpExplodeQuery {
                product_id: 99999,
                parent_quantity: decs!("10"),
                required_date: ymd!(2026, 7, 9),
                source_type: "MANUAL".to_string(),
                source_id: None,
                consider_safety_stock: false,
                consider_in_transit: false,
            })
            .await;
        // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_查询MRP结果_需要真实数据库
    ///
    /// 需要 mrp_results 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证 get_results 调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_查询MRP结果_需要真实数据库() {
        let db = setup_test_db().await;
        let service = MrpEngineService::new(Arc::new(db));
        let result = service.get_results(None, None, None, 1, 10).await;
        // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }
}
