//! 色卡扫码查询服务
//!
//! 提供按色号编码 / 色号 ID 查询完整色号信息（含 RGB/CMYK/Lab + 配方 + 价格）
//! 创建时间: 2026-06-17

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_item::{self, Entity as ItemEntity};
use crate::models::color_card_response_dto::{PriceSummary, RecipeSummary, ScanResult};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;

/// 扫码查询服务
pub struct ColorCardScanService {
    db: Arc<DatabaseConnection>,
}

// v11 批次 147 P2-B：移除失效的 dead_code 标注（被 handlers/color_card/scan_export.rs 真实调用）
impl ColorCardScanService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 按色号编码扫码查询（全局搜索）
    pub async fn scan_by_code(
        &self,
        color_code: &str,
    ) -> Result<ScanResult, AppError> {
        // 查找色号 + 关联色卡
        let (item, card) = self.fetch_color_item_and_card(color_code).await?;

        // 加载配方 / 价格（Copy 字段直接取用，无需 clone）
        let recipe_summary = self.load_recipe_summary(item.dye_recipe_id).await?;
        let price_summary = self.load_price_summary(item.product_color_price_id).await?;

        // 组装返回结果
        Ok(ScanResult {
            color_item: crate::models::color_card_response_dto::ColorItemInfo {
                id: item.id,
                color_code: item.color_code,
                color_name: item.color_name,
                rgb_r: item.rgb_r,
                rgb_g: item.rgb_g,
                rgb_b: item.rgb_b,
                cmyk_c: item.cmyk_c,
                cmyk_m: item.cmyk_m,
                cmyk_y: item.cmyk_y,
                cmyk_k: item.cmyk_k,
                lab_l: item.lab_l,
                lab_a: item.lab_a,
                lab_b: item.lab_b,
                pantone_code: item.pantone_code,
                cncs_code: item.cncs_code,
                custom_code: item.custom_code,
                hex_value: item.hex_value,
                dye_recipe_id: item.dye_recipe_id,
                product_color_price_id: item.product_color_price_id,
                swatch_image_url: item.swatch_image_url,
                sequence: item.sequence,
            },
            color_card_no: card.card_no,
            color_card_name: card.card_name,
            recipe_summary,
            price_summary,
        })
    }

    /// 查找色号并加载关联色卡
    async fn fetch_color_item_and_card(
        &self,
        color_code: &str,
    ) -> Result<(color_card_item::Model, color_card::Model), AppError> {
        // 按色号编码查找
        let item = ItemEntity::find()
            .filter(color_card_item::Column::ColorCode.eq(color_code))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色号不存在"))?;

        // 加载所属色卡
        let card = ColorCardEntity::find_by_id(item.color_card_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色卡不存在"))?;

        Ok((item, card))
    }

    /// 加载配方摘要（如有）
    async fn load_recipe_summary(
        &self,
        recipe_id: Option<i64>,
    ) -> Result<Option<RecipeSummary>, AppError> {
        if let Some(recipe_id) = recipe_id {
            let recipe = crate::models::dye_recipe::Entity::find_by_id(recipe_id)
                .one(&*self.db)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            Ok(recipe.map(|r| RecipeSummary {
                id: i64::from(r.id),
                recipe_name: r.recipe_name.unwrap_or_default(),
                fabric_type: r.fabric_type,
                color_no: r.color_no,
                temperature: r.temperature,
                time_minutes: r.time_minutes,
            }))
        } else {
            Ok(None)
        }
    }

    /// 加载价格摘要（如有）
    async fn load_price_summary(
        &self,
        price_id: Option<i32>,
    ) -> Result<Option<PriceSummary>, AppError> {
        if let Some(price_id) = price_id {
            let price = crate::models::product_color_price::Entity::find_by_id(price_id)
                .one(&*self.db)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            Ok(price.map(|p| PriceSummary {
                id: p.id,
                base_price: p.base_price,
                currency: p.currency,
                effective_from: p.effective_from,
                customer_level: p.customer_level,
            }))
        } else {
            Ok(None)
        }
    }

    /// 按色号 ID 扫码查询
    pub async fn scan_by_id(
        &self,
        item_id: i64,
    ) -> Result<ScanResult, AppError> {
        let item = ItemEntity::find_by_id(item_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色号不存在"))?;

        // 复用 scan_by_code
        self.scan_by_code(&item.color_code).await
    }
}

// 死代码清理（2026-06-26）：_ensure_dye_recipe_used / _ensure_price_used 为抑制未使用导入的 hack，
// 现已删除多余的 use 语句，这两个函数一并删除。
