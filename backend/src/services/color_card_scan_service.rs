//! 色卡扫码查询服务
//!
//! 提供按色号编码 / 色号 ID 查询完整色号信息（含 RGB/CMYK/Lab + 配方 + 价格）
//! 创建时间: 2026-06-17

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_item::{self, Entity as ItemEntity};
use crate::models::color_card_response_dto::{PriceSummary, RecipeSummary, ScanResult};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;

/// 业务错误
#[allow(dead_code)] // TODO(tech-debt): 业务接入完整错误链后移除
#[derive(Debug, Error)]
pub enum ScanError {
    #[error("色号不存在")]
    ItemNotFound,
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 扫码查询服务
pub struct ColorCardScanService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)] // TODO(tech-debt): 色卡扫码路由接入后移除
impl ColorCardScanService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 按色号编码扫码查询（全局搜索）
    pub async fn scan_by_code(
        &self,
        color_code: &str,
        tenant_id: i64,
    ) -> Result<ScanResult, AppError> {
        // 1. 查找色号
        let item = ItemEntity::find()
            .filter(color_card_item::Column::ColorCode.eq(color_code))
            .filter(color_card_item::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色号不存在"))?;

        // 2. 加载色卡
        let card = ColorCardEntity::find_by_id(item.color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色卡不存在"))?;

        // 3. 加载配方（如有）
        let recipe_summary = if let Some(recipe_id) = item.dye_recipe_id {
            // dye_recipe.id 是 i32，color_card_item.dye_recipe_id 是 Option<i64>，用 try_from 安全转换
            let recipe = crate::models::dye_recipe::Entity::find_by_id(i32::try_from(recipe_id).map_err(|e| {
                AppError::validation(format!("配方 ID 超出 i32 范围: {}", e))
            })?)
                .one(&*self.db)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            recipe.map(|r| RecipeSummary {
                id: i64::from(r.id),
                recipe_name: r.recipe_name.unwrap_or_default(),
                fabric_type: r.fabric_type,
                color_no: r.color_no,
                temperature: r.temperature,
                time_minutes: r.time_minutes,
            })
        } else {
            None
        };

        // 4. 加载价格（如有）
        let price_summary = if let Some(price_id) = item.product_color_price_id {
            let price = crate::models::product_color_price::Entity::find_by_id(price_id)
                .one(&*self.db)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            price.map(|p| PriceSummary {
                id: p.id,
                base_price: p.base_price,
                currency: p.currency,
                effective_from: p.effective_from,
                customer_level: p.customer_level,
            })
        } else {
            None
        };

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

    /// 按色号 ID 扫码查询
    pub async fn scan_by_id(
        &self,
        item_id: i64,
        tenant_id: i64,
    ) -> Result<ScanResult, AppError> {
        let item = ItemEntity::find_by_id(item_id)
            .filter(color_card_item::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
            .ok_or_else(|| AppError::not_found("色号不存在"))?;

        // 复用 scan_by_code
        self.scan_by_code(&item.color_code, tenant_id).await
    }
}

// 死代码清理（2026-06-26）：_ensure_dye_recipe_used / _ensure_price_used 为抑制未使用导入的 hack，
// 现已删除多余的 use 语句，这两个函数一并删除。
