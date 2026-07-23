//! 订单转换与产品列表
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的订单转换方法。
//! 包含：convert_to_orders（MRP 结果转订单）+ cancel_calculation（取消计算）
//! + list_products_for_mrp（产品列表查询）

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};

use crate::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Entity as MrpResultEntity, Model as MrpResultModel,
};
use crate::models::product::Entity as ProductEntity;
use crate::models::status::master_data;
use crate::models::status::mrp as mrp_status;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;

use crate::services::mrp_engine_service::MrpEngineService;

impl MrpEngineService {
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
}
