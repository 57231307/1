//! BOM 递归展开
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的 BOM 展开方法。
//! 包含：get_default_bom/calculate_quantity_with_scrap/calculate_material_date
//! + process_bom_item/explode_bom_recursive/explode_bom

use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::bom::{Entity as BomEntity, Model as BomModel};
use crate::models::bom_item::{Entity as BomItemEntity, Model as BomItemModel};
use crate::models::status::common;
use crate::utils::error::AppError;

use super::types::{
    ExplodeBomArgs, MaterialRequirement, MrpExplodeQuery, RequirementCalcParams,
};
use crate::services::mrp_engine_service::MrpEngineService;

impl MrpEngineService {
    /// 获取产品的默认BOM
    async fn get_default_bom(&self, product_id: i32) -> Result<Option<BomModel>, AppError> {
        let bom = BomEntity::find()
            .filter(crate::models::bom::Column::ProductId.eq(product_id))
            .filter(crate::models::bom::Column::IsDefault.eq(true))
            .filter(crate::models::bom::Column::Status.eq(common::STATUS_ACTIVE))
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
        stock_cache: &mut std::collections::HashMap<i32, crate::services::mrp_engine_ops::types::StockInfo>,
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
        stock_cache: &mut std::collections::HashMap<i32, crate::services::mrp_engine_ops::types::StockInfo>,
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
        let mut stock_cache: std::collections::HashMap<i32, crate::services::mrp_engine_ops::types::StockInfo> =
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
}
