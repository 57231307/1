//! MRP 计算执行
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的计算执行方法。
//! 包含：run_mrp_calculation（单次计算）+ run_mrp_calculation_for_line（按指定行号前缀计算）
//! + batch_calculate（批量计算）
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
    /// 执行MRP计算并保存结果（批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `MrpCalculationQuery`，；消除 `clippy::too_many_arguments` 警告。）
    ///
    /// 单产品入口：行号前缀由本函数自生成（`MRP{毫秒时间戳}`），保持销售订单审批
    /// （`services/so/order_workflow.rs`）与生产订单创建（`services/production_order_ops/crud.rs`）
    /// 两处调用方的历史行为不变。批量场景请走 `run_mrp_calculation_for_line`，由批次调用方统一决定编号。
    pub async fn run_mrp_calculation(
        &self,
        query: MrpCalculationQuery,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let line_no = format!("MRP{}", Utc::now().timestamp_millis());
        self.run_mrp_calculation_for_line(query, &line_no).await
    }

    /// 以给定行号前缀执行一次（单产品）MRP 计算并落库。
    ///
    /// `line_no` 即本次计算主物料行的 `calculation_no`，其 BOM 子行为 `{line_no}-{子行序}`；
    /// 由 batch_calculate 统一编号为 `{批次号}-{行序}`，使同一批次内主行/子行互不相同、
    /// 不同批次间因批次号不同而互不相同（`mrp_results.calculation_no` 带 UNIQUE 约束），
    /// 同时让整批行都能以批次号作前缀被检索出来。
    pub(crate) async fn run_mrp_calculation_for_line(
        &self,
        query: MrpCalculationQuery,
        line_no: &str,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let mut results = Vec::new();

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
        let main_active_model = Self::build_main_result_active_model(line_no, &main_req);
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
            let sub_active_model = Self::build_sub_result_active_model(line_no, idx, req);
            let sub_result = sub_active_model.insert(&*self.db).await?;
            results.push(sub_result);
        }

        Ok(results)
    }

    /// 构建主物料 MRP 结果 ActiveModel
    fn build_main_result_active_model(
        line_no: &str,
        main_req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
            calculation_no: Set(line_no.to_string()),
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
        line_no: &str,
        idx: usize,
        req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
            calculation_no: Set(format!("{}-{}", line_no, idx + 1)),
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
                req.bom_level, req.on_hand_quantity, req.in_transit_quantity, req.shortage_quantity
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
        // 一次批量计算 = 一个批次：批次号既作为响应返回，也作为批内所有行号的前缀，
        // 使 /mrp/results?calculation_no=<批次号> 能按前缀检索到整批行。
        let calculation_no = format!("MRPB{}", Utc::now().timestamp_millis());
        let mut all_results = Vec::new();
        let mut all_requirements = Vec::new();

        // v16 批次 43 修复：循环外批量预加载所有顶层 product_id 的库存信息，
        // 避免循环内重复调用 calculate_requirement 查询同一产品库存（N+1 查询）
        let top_product_ids: Vec<i32> = request.items.iter().map(|i| i.product_id).collect();
        let top_stock_map = self.get_stock_info_batch(&top_product_ids).await?;

        for (line_idx, item) in request.items.iter().enumerate() {
            let results = self
                .run_mrp_calculation_for_line(
                    MrpCalculationQuery {
                        product_id: item.product_id,
                        required_quantity: item.required_quantity,
                        required_date: item.required_date,
                        source_type: request.source_type.clone(),
                        source_id: request.source_id,
                        consider_safety_stock: request.consider_safety_stock,
                        consider_in_transit: request.consider_in_transit,
                    },
                    &format!("{calculation_no}-{line_idx}"),
                )
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
                    lead_time_days: 7,
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
