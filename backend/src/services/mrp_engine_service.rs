//! MRP物料需求计算引擎
//!
//! 基于BOM和库存数据计算物料需求，支持多层BOM展开和批量计算

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::bom::{Entity as BomEntity, Model as BomModel};
use crate::models::bom_item::Entity as BomItemEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Entity as MrpResultEntity, Model as MrpResultModel,
};
use crate::models::product::Entity as ProductEntity;
use crate::utils::error::AppError;

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

/// BOM展开节点
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BomNode {
    product_id: i32,
    quantity: Decimal,
    level: i32,
    scrap_rate: Option<Decimal>,
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

    /// 计算单个物料需求
    #[allow(clippy::too_many_arguments)]
    pub async fn calculate_requirement(
        &self,
        product_id: i32,
        required_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
        consider_safety_stock: bool,
        consider_in_transit: bool,
        bom_level: i32,
    ) -> Result<MaterialRequirement, AppError> {
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
            .filter(crate::models::bom::Column::Status.eq("ACTIVE"))
            .one(&*self.db)
            .await?;

        Ok(bom)
    }

    /// 递归展开BOM
    #[allow(clippy::too_many_arguments)]
    async fn explode_bom_recursive(
        &self,
        product_id: i32,
        parent_quantity: Decimal,
        required_date: NaiveDate,
        source_type: &str,
        source_id: Option<i32>,
        current_level: i32,
        max_level: i32,
        consider_safety_stock: bool,
        consider_in_transit: bool,
        results: &mut Vec<MaterialRequirement>,
    ) -> Result<(), AppError> {
        if current_level > max_level {
            return Ok(());
        }

        let bom = match self.get_default_bom(product_id).await? {
            Some(bom) => bom,
            None => return Ok(()),
        };

        let bom_items = BomItemEntity::find()
            .filter(crate::models::bom_item::Column::BomId.eq(bom.id))
            .all(&*self.db)
            .await?;

        for item in bom_items {
            let base_quantity = parent_quantity * item.quantity;
            let quantity_with_scrap = if let Some(scrap_rate) = item.scrap_rate {
                if scrap_rate > Decimal::ZERO {
                    base_quantity * (Decimal::ONE + (scrap_rate / Decimal::from(100)))
                } else {
                    base_quantity
                }
            } else {
                base_quantity
            };

            // 提前期计算：根据BOM层级递减，每层7天提前期
            let lead_time_days = 7 * current_level as i64;
            let lead_time = Duration::days(lead_time_days);
            let material_date = required_date - lead_time;

            let requirement = self
                .calculate_requirement(
                    item.material_id,
                    quantity_with_scrap,
                    material_date,
                    source_type.to_string(),
                    source_id,
                    consider_safety_stock,
                    consider_in_transit,
                    current_level,
                )
                .await?;

            results.push(requirement);

            Box::pin(self.explode_bom_recursive(
                item.material_id,
                quantity_with_scrap,
                material_date,
                source_type,
                source_id,
                current_level + 1,
                max_level,
                consider_safety_stock,
                consider_in_transit,
                results,
            ))
            .await?;
        }

        Ok(())
    }

    /// 展开BOM计算子物料需求
    #[allow(clippy::too_many_arguments)]
    pub async fn explode_bom(
        &self,
        product_id: i32,
        parent_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
        consider_safety_stock: bool,
        consider_in_transit: bool,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let mut requirements = Vec::new();

        self.explode_bom_recursive(
            product_id,
            parent_quantity,
            required_date,
            &source_type,
            source_id,
            1,
            10,
            consider_safety_stock,
            consider_in_transit,
            &mut requirements,
        )
        .await?;

        Ok(requirements)
    }

    /// 执行MRP计算并保存结果
    #[allow(clippy::too_many_arguments)]
    pub async fn run_mrp_calculation(
        &self,
        product_id: i32,
        required_quantity: Decimal,
        required_date: NaiveDate,
        source_type: String,
        source_id: Option<i32>,
        consider_safety_stock: bool,
        consider_in_transit: bool,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let mut results = Vec::new();
        let calculation_no = format!("MRP{}", Utc::now().timestamp_millis());

        let main_req = self
            .calculate_requirement(
                product_id,
                required_quantity,
                required_date,
                source_type.clone(),
                source_id,
                consider_safety_stock,
                consider_in_transit,
                0,
            )
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
            status: Set("PLANNED".to_string()),
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
            .explode_bom(
                product_id,
                required_quantity,
                required_date,
                source_type,
                source_id,
                consider_safety_stock,
                consider_in_transit,
            )
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
                status: Set("PLANNED".to_string()),
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

        for item in request.items {
            let results = self
                .run_mrp_calculation(
                    item.product_id,
                    item.required_quantity,
                    item.required_date,
                    request.source_type.clone(),
                    request.source_id,
                    request.consider_safety_stock,
                    request.consider_in_transit,
                )
                .await?;

            all_results.extend(results);

            let requirement = self
                .calculate_requirement(
                    item.product_id,
                    item.required_quantity,
                    item.required_date,
                    request.source_type.clone(),
                    request.source_id,
                    request.consider_safety_stock,
                    request.consider_in_transit,
                    0,
                )
                .await?;

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

        let total = select.clone().count(&*self.db).await?;

        let results = select
            .order_by_desc(crate::models::mrp_result::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

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
            MrpResultEntity::find().filter(crate::models::mrp_result::Column::Status.eq("PLANNED"));

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

        let mut requirements = Vec::new();

        for result in mrp_results {
            let stock_info = self.get_stock_info(result.product_id).await?;

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
        let mut updated_results = Vec::new();

        for id in result_ids {
            let result = MrpResultEntity::find_by_id(id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found("MRP结果不存在"))?;

            if result.status != "PLANNED" {
                return Err(AppError::validation(format!(
                    "MRP结果 {} 状态不是PLANNED，无法转换",
                    id
                )));
            }

            let mut active_model: crate::models::mrp_result::ActiveModel = result.into();
            let new_status = match order_type.as_str() {
                "PURCHASE" => "CONFIRMED",
                "PRODUCTION" => "RELEASED",
                _ => return Err(AppError::validation("无效的订单类型")),
            };

            active_model.status = Set(new_status.to_string());
            active_model.updated_at = Set(Utc::now());

            let updated = active_model.update(&*self.db).await?;

            updated_results.push(updated);
        }

        Ok(updated_results)
    }

    /// 获取缺料预警列表
    #[allow(dead_code)]
    pub async fn get_shortage_alerts(
        &self,
        days_ahead: i64,
    ) -> Result<Vec<MaterialRequirement>, AppError> {
        let alert_date = Utc::now().date_naive() + Duration::days(days_ahead);

        let mrp_results = MrpResultEntity::find()
            .filter(crate::models::mrp_result::Column::RequiredDate.lte(alert_date))
            .filter(crate::models::mrp_result::Column::Status.eq("PLANNED"))
            .all(&*self.db)
            .await?;

        let mut alerts = Vec::new();

        for result in mrp_results {
            let stock_info = self.get_stock_info(result.product_id).await?;

            let shortage = if result.required_quantity > stock_info.available {
                result.required_quantity - stock_info.available
            } else {
                Decimal::ZERO
            };

            if shortage > Decimal::ZERO {
                alerts.push(MaterialRequirement {
                    product_id: result.product_id,
                    required_quantity: result.required_quantity,
                    required_date: result.required_date.unwrap_or(alert_date),
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

        Ok(alerts)
    }

    /// 删除MRP计算结果
    #[allow(dead_code)]
    pub async fn delete_results(&self, calculation_no: &str) -> Result<u64, AppError> {
        let result = MrpResultEntity::delete_many()
            .filter(crate::models::mrp_result::Column::CalculationNo.eq(calculation_no))
            .exec(&*self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// 列出可用于 MRP 计算的产品
    pub async fn list_products_for_mrp(
        &self,
        keyword: Option<String>,
    ) -> Result<Vec<crate::models::product::Model>, AppError> {
        let mut query = ProductEntity::find()
            .filter(crate::models::product::Column::IsDeleted.eq(false))
            .filter(crate::models::product::Column::Status.eq("active"));

        if let Some(kw) = keyword {
            let trimmed = kw.trim();
            if !trimmed.is_empty() {
                let pattern = format!("%{}%", trimmed);
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
        let result = MrpResultEntity::find_by_id(calculation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("MRP结果不存在"))?;

        if result.status == "CANCELLED" {
            return Ok(result);
        }

        let mut active_model: MrpResultActiveModel = result.into();
        active_model.status = Set("CANCELLED".to_string());
        active_model.updated_at = Set(Utc::now());
        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }

    /// 导出指定 MRP 计算编号下的所有结果为 CSV
    pub async fn export_calculation(&self, calculation_id: i32) -> Result<Vec<u8>, AppError> {
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

        let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
        wtr.write_record([
            "ID",
            "计算编号",
            "产品ID",
            "需求数量",
            "需求日期",
            "来源类型",
            "来源ID",
            "计划订单数量",
            "计划订单日期",
            "状态",
            "备注",
            "创建时间",
        ])
        .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;

        for r in &results {
            wtr.write_record([
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
            ])
            .map_err(|e| AppError::validation(format!("CSV写入错误: {}", e)))?;
        }

        let bytes = wtr
            .into_inner()
            .map_err(|e| AppError::validation(format!("CSV序列化错误: {}", e)))?;
        Ok(bytes)
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
