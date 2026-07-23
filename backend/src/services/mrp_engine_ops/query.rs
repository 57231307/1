//! MRP 结果查询与导出
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的结果查询方法。
//! 包含：get_results/get_requirements/get_material_detail/export_calculation

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::models::mrp_result::{Entity as MrpResultEntity, Model as MrpResultModel};
use crate::models::status::mrp as mrp_status;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::xlsx_export::XlsxTable;

use super::types::{MaterialRequirement, StockInfo};
use crate::services::mrp_engine_service::MrpEngineService;
use chrono::NaiveDate;

impl MrpEngineService {
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
