//! 流转卡状态机流转 impl 子模块（flow_card_ops/card_state）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 FlowCardService 的 10 个状态流转方法
//!（schedule / start_preparing / complete_preparing / start_dyeing / complete_dyeing /
//! start_inspecting / complete / ship / terminate / reactivate）。
//! 状态流转校验纯函数 validate_status_transition 保留在 facade，本模块通过 Self:: 调用。

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Set};

use crate::models::production_flow_card::{ActiveModel as CardActiveModel, Model as CardModel};
use crate::models::status::flow_card as card_status;
use crate::services::flow_card_service::FlowCardService;
use crate::utils::error::AppError;

impl FlowCardService {
    // ===== 状态机流转 =====

    /// 排缸（pending → scheduled）
    pub async fn schedule(
        &self,
        id: i32,
        dye_batch_id: Option<i32>,
        dye_lot_no: Option<String>,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::SCHEDULED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::SCHEDULED.to_string());
        active.scheduled_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        if let Some(v) = dye_batch_id {
            active.dye_batch_id = Set(Some(v));
        }
        if let Some(v) = dye_lot_no {
            active.dye_lot_no = Set(Some(v));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开始备布（scheduled → preparing）
    pub async fn start_preparing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::PREPARING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::PREPARING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完成备布（回填实际配布数量，状态保持 preparing 等待进缸）
    pub async fn complete_preparing(
        &self,
        id: i32,
        actual_fabric_weight: Decimal,
    ) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != card_status::PREPARING {
            return Err(AppError::business(format!(
                "流转卡状态为 {}，仅 preparing 状态可完成备布",
                model.status
            )));
        }
        if actual_fabric_weight <= Decimal::ZERO {
            return Err(AppError::business("实际配布数量必须 > 0"));
        }

        let mut active: CardActiveModel = model.into();
        active.actual_fabric_weight = Set(Some(actual_fabric_weight));
        active.prepared_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 进缸染色（preparing → dyeing）
    pub async fn start_dyeing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::DYEING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::DYEING.to_string());
        active.dye_start_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 出缸（dyeing → dyed）
    pub async fn complete_dyeing(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::DYED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::DYED.to_string());
        active.dye_end_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 开始验布（dyed → inspecting）
    pub async fn start_inspecting(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::INSPECTING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::INSPECTING.to_string());
        active.inspected_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 完成验布入库（inspecting → completed）
    pub async fn complete(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::COMPLETED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::COMPLETED.to_string());
        active.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 发货（completed → shipped）
    pub async fn ship(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::SHIPPED)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::SHIPPED.to_string());
        active.shipped_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 终止（任意状态 → terminated，可重新激活回 pending）
    pub async fn terminate(&self, id: i32, reason: Option<String>) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == card_status::TERMINATED {
            return Err(AppError::business("流转卡已终止"));
        }
        if model.status == card_status::SHIPPED {
            return Err(AppError::business("已发货流转卡不可终止"));
        }

        let existing_remarks = model.remarks.clone().unwrap_or_default();
        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::TERMINATED.to_string());
        if let Some(r) = reason {
            let new_remarks = if existing_remarks.is_empty() {
                format!("[终止] {}", r)
            } else {
                format!("{}\n[终止] {}", existing_remarks, r)
            };
            active.remarks = Set(Some(new_remarks));
        }
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 重新激活（terminated → pending，回修订单重新进缸场景）
    pub async fn reactivate(&self, id: i32) -> Result<CardModel, AppError> {
        let model = self.get_by_id(id).await?;
        Self::validate_status_transition(&model.status, card_status::PENDING)?;

        let mut active: CardActiveModel = model.into();
        active.status = Set(card_status::PENDING.to_string());
        active.scheduled_at = Set(None);
        active.prepared_at = Set(None);
        active.dye_start_at = Set(None);
        active.dye_end_at = Set(None);
        active.inspected_at = Set(None);
        active.completed_at = Set(None);
        active.shipped_at = Set(None);
        active.current_step_seq = Set(1);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }
}
