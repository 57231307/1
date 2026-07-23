//! MRP 计算执行
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的计算执行方法。
//! 包含：run_mrp_calculation（单次计算）+ batch_calculate（批量计算）
//! + build_main_result_active_model/build_sub_result_active_model（ActiveModel 构建）

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Set};

use crate::models::mrp_result::{ActiveModel as MrpResultActiveModel, Model as MrpResultModel};
// 批次 235 v13 P1-1：MRP 结果状态常量接入（规则 0）
use crate::models::status::mrp as mrp_status;
use crate::utils::error::AppError;

use super::types::{
    MaterialRequirement, MrpCalculationQuery, MrpCalculationRequest, MrpCalculationSummary,
    MrpExplodeQuery, RequirementCalcParams, StockInfo,
};
use crate::services::mrp_engine_service::MrpEngineService;

impl MrpEngineService {
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

        // 计算主物料需求
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

        // 构建并保存主物料结果
        let main_active_model =
            Self::build_main_result_active_model(&calculation_no, &main_req);
        let main_result = main_active_model.insert(&*self.db).await?;
        results.push(main_result);

        // 展开 BOM 获取子物料需求
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

        // 遍历构建并保存子物料结果
        for (idx, req) in sub_requirements.iter().enumerate() {
            let sub_active_model =
                Self::build_sub_result_active_model(&calculation_no, idx, req);
            let sub_result = sub_active_model.insert(&*self.db).await?;
            results.push(sub_result);
        }

        Ok(results)
    }

    /// 构建主物料 MRP 结果 ActiveModel
    fn build_main_result_active_model(
        calculation_no: &str,
        main_req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
            calculation_no: Set(calculation_no.to_string()),
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
        }
    }

    /// 构建子物料 MRP 结果 ActiveModel
    fn build_sub_result_active_model(
        calculation_no: &str,
        idx: usize,
        req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
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
        }
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
}
