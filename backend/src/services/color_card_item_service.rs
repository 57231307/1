//! 色号管理服务
//!
//! 提供色号 CRUD + 批量导入业务
//! 色彩空间自动转换（RGB → CMYK → CIELab）由 color_space_converter 工具支持
//! 创建时间: 2026-06-17

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_item::{self, ActiveModel as ItemActive, Entity as ItemEntity};
use crate::models::color_card_item_dto::{
    BatchImportError, BatchImportResponse, ColorItemDto,
};
use crate::utils::app_state::AppState;
use crate::utils::color_space_converter;

/// 业务错误
#[derive(Debug, Error)]
pub enum ItemError {
    #[error("色卡不存在")]
    ColorCardNotFound,
    #[error("色号不存在")]
    ItemNotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 色号管理服务
pub struct ColorCardItemService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardItemService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 列出色卡下所有色号
    pub async fn list(
        &self,
        color_card_id: i64,
        tenant_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<color_card_item::Model>, u64), ItemError> {
        // 验证色卡存在
        let _card = ColorCardEntity::find_by_id(color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ItemError::ColorCardNotFound)?;

        let paginator = ItemEntity::find()
            .filter(color_card_item::Column::ColorCardId.eq(color_card_id))
            .filter(color_card_item::Column::TenantId.eq(tenant_id))
            .order_by_asc(color_card_item::Column::Sequence)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 创建色号
    pub async fn create(
        &self,
        color_card_id: i64,
        tenant_id: i64,
        dto: ColorItemDto,
    ) -> Result<color_card_item::Model, ItemError> {
        // 1. 验证色卡
        let card = ColorCardEntity::find_by_id(color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ItemError::ColorCardNotFound)?;

        if card.status != "active" {
            return Err(ItemError::InvalidState);
        }

        // 2. 业务校验
        Self::validate_item(&dto)?;

        // 3. 自动计算缺失的色彩空间
        let (cmyk, lab) = Self::compute_color_spaces(&dto);

        // 4. 事务：插入 + 更新色卡 total_colors
        let txn = self.db.begin().await?;
        let now = Utc::now();
        let active = ItemActive {
            id: Default::default(),
            color_card_id: Set(color_card_id),
            color_code: Set(dto.color_code),
            color_name: Set(dto.color_name),
            rgb_r: Set(dto.rgb_r),
            rgb_g: Set(dto.rgb_g),
            rgb_b: Set(dto.rgb_b),
            cmyk_c: Set(cmyk.0),
            cmyk_m: Set(cmyk.1),
            cmyk_y: Set(cmyk.2),
            cmyk_k: Set(cmyk.3),
            lab_l: Set(lab.0),
            lab_a: Set(lab.1),
            lab_b: Set(lab.2),
            pantone_code: Set(dto.pantone_code),
            cncs_code: Set(dto.cncs_code),
            custom_code: Set(dto.custom_code),
            hex_value: Set(dto.hex_value),
            dye_recipe_id: Set(dto.dye_recipe_id),
            product_color_price_id: Set(dto.product_color_price_id),
            swatch_image_url: Set(dto.swatch_image_url),
            sequence: Set(dto.sequence.unwrap_or(0)),
            tenant_id: Set(tenant_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&txn).await?;

        // 更新色卡 total_colors
        // P9-1: 用 map_or 替代 unwrap，对 None 显式处理
        let mut card_active: color_card::ActiveModel = card.into();
        let new_total = card_active
            .total_colors
            .as_ref()
            .map(|v| v + 1)
            .unwrap_or_else(|| 1);
        card_active.total_colors = Set(new_total);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 更新色号
    pub async fn update(
        &self,
        color_card_id: i64,
        item_id: i64,
        tenant_id: i64,
        dto: ColorItemDto,
    ) -> Result<color_card_item::Model, ItemError> {
        let existing = ItemEntity::find_by_id(item_id)
            .filter(color_card_item::Column::ColorCardId.eq(color_card_id))
            .filter(color_card_item::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ItemError::ItemNotFound)?;

        Self::validate_item(&dto)?;
        let (cmyk, lab) = Self::compute_color_spaces(&dto);

        let mut active: ItemActive = existing.into();
        active.color_code = Set(dto.color_code);
        active.color_name = Set(dto.color_name);
        active.rgb_r = Set(dto.rgb_r);
        active.rgb_g = Set(dto.rgb_g);
        active.rgb_b = Set(dto.rgb_b);
        active.cmyk_c = Set(cmyk.0);
        active.cmyk_m = Set(cmyk.1);
        active.cmyk_y = Set(cmyk.2);
        active.cmyk_k = Set(cmyk.3);
        active.lab_l = Set(lab.0);
        active.lab_a = Set(lab.1);
        active.lab_b = Set(lab.2);
        if let Some(v) = dto.pantone_code {
            active.pantone_code = Set(Some(v));
        }
        if let Some(v) = dto.cncs_code {
            active.cncs_code = Set(Some(v));
        }
        if let Some(v) = dto.custom_code {
            active.custom_code = Set(Some(v));
        }
        active.hex_value = Set(dto.hex_value);
        if let Some(v) = dto.dye_recipe_id {
            active.dye_recipe_id = Set(Some(v));
        }
        if let Some(v) = dto.product_color_price_id {
            active.product_color_price_id = Set(Some(v));
        }
        if let Some(v) = dto.swatch_image_url {
            active.swatch_image_url = Set(Some(v));
        }
        if let Some(v) = dto.sequence {
            active.sequence = Set(v);
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除色号
    pub async fn delete(
        &self,
        color_card_id: i64,
        item_id: i64,
        tenant_id: i64,
    ) -> Result<(), ItemError> {
        let existing = ItemEntity::find_by_id(item_id)
            .filter(color_card_item::Column::ColorCardId.eq(color_card_id))
            .filter(color_card_item::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ItemError::ItemNotFound)?;

        // 事务：删除 + 更新色卡 total_colors
        let txn = self.db.begin().await?;
        let active: ItemActive = existing.into();
        active.delete(&txn).await?;

        let card = ColorCardEntity::find_by_id(color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or(ItemError::ColorCardNotFound)?;
        let mut card_active: color_card::ActiveModel = card.into();
        // P9-1: 用 map_or 替代 unwrap，对 None 显式回退
        let current = card_active
            .total_colors
            .take()
            .map(|v| v - 1)
            .unwrap_or(0)
            .max(0);
        card_active.total_colors = Set(current);
        card_active.updated_at = Set(Utc::now());
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(())
    }

    /// 批量导入色号
    pub async fn batch_import(
        &self,
        color_card_id: i64,
        tenant_id: i64,
        items: Vec<ColorItemDto>,
    ) -> Result<BatchImportResponse, ItemError> {
        // 验证色卡
        let card = ColorCardEntity::find_by_id(color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ItemError::ColorCardNotFound)?;

        if card.status != "active" {
            return Err(ItemError::InvalidState);
        }

        let mut success_count = 0;
        let mut errors = Vec::new();
        let mut imported_items = Vec::new();
        let now = Utc::now();

        let txn = self.db.begin().await?;

        for dto in items {
            // 校验
            if let Err(e) = Self::validate_item(&dto) {
                errors.push(BatchImportError {
                    color_code: dto.color_code,
                    reason: e.to_string(),
                });
                continue;
            }

            let (cmyk, lab) = Self::compute_color_spaces(&dto);

            let active = ItemActive {
                id: Default::default(),
                color_card_id: Set(color_card_id),
                color_code: Set(dto.color_code.clone()),
                color_name: Set(dto.color_name),
                rgb_r: Set(dto.rgb_r),
                rgb_g: Set(dto.rgb_g),
                rgb_b: Set(dto.rgb_b),
                cmyk_c: Set(cmyk.0),
                cmyk_m: Set(cmyk.1),
                cmyk_y: Set(cmyk.2),
                cmyk_k: Set(cmyk.3),
                lab_l: Set(lab.0),
                lab_a: Set(lab.1),
                lab_b: Set(lab.2),
                pantone_code: Set(dto.pantone_code),
                cncs_code: Set(dto.cncs_code),
                custom_code: Set(dto.custom_code),
                hex_value: Set(dto.hex_value),
                dye_recipe_id: Set(dto.dye_recipe_id),
                product_color_price_id: Set(dto.product_color_price_id),
                swatch_image_url: Set(dto.swatch_image_url),
                sequence: Set(dto.sequence.unwrap_or(0)),
                tenant_id: Set(tenant_id),
                created_at: Set(now),
                updated_at: Set(now),
            };

            match active.insert(&txn).await {
                Ok(model) => {
                    success_count += 1;
                    imported_items.push(model);
                }
                Err(e) => {
                    errors.push(BatchImportError {
                        color_code: dto.color_code,
                        reason: e.to_string(),
                    });
                }
            }
        }

        // 更新色卡 total_colors
        let mut card_active: color_card::ActiveModel = card.into();
        // P9-1: 用 map_or 替代 unwrap，对 None 显式回退
        // ActiveValue<Option<i32>>::take() 返回 Option<i32>
        let new_total = card_active
            .total_colors
            .take()
            .map(|v| v + success_count as i32)
            .unwrap_or(success_count as i32);
        card_active.total_colors = Set(new_total);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;

        Ok(BatchImportResponse {
            success_count,
            failed_count: errors.len(),
            errors,
            total_colors: new_total,
        })
    }

    /// 校验色号参数
    fn validate_item(dto: &ColorItemDto) -> Result<(), ItemError> {
        // HEX 格式校验
        if color_space_converter::hex_to_rgb(&dto.hex_value).is_err() {
            return Err(ItemError::Validation(format!(
                "HEX 值格式错误: {}（必须是 #RRGGBB 格式）",
                dto.hex_value
            )));
        }
        // RGB 范围（DTO 已有 validate 装饰器，此处冗余校验）
        if !(0..=255).contains(&dto.rgb_r)
            || !(0..=255).contains(&dto.rgb_g)
            || !(0..=255).contains(&dto.rgb_b)
        {
            return Err(ItemError::Validation("RGB 值必须在 0-255 之间".to_string()));
        }
        Ok(())
    }

    /// 自动计算色彩空间（CMYK + CIELab）
    /// 如果 DTO 中未提供，则从 RGB 计算
    fn compute_color_spaces(
        dto: &ColorItemDto,
    ) -> (
        (Option<Decimal>, Option<Decimal>, Option<Decimal>, Option<Decimal>),
        (Option<Decimal>, Option<Decimal>, Option<Decimal>),
    ) {
        let r = dto.rgb_r as u8;
        let g = dto.rgb_g as u8;
        let b = dto.rgb_b as u8;

        // CMYK
        let cmyk = color_space_converter::rgb_to_cmyk(r, g, b);
        let cmyk_c = dto
            .cmyk_c
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", cmyk.c)).ok());
        let cmyk_m = dto
            .cmyk_m
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", cmyk.m)).ok());
        let cmyk_y = dto
            .cmyk_y
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", cmyk.y)).ok());
        let cmyk_k = dto
            .cmyk_k
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", cmyk.k)).ok());

        // CIELab
        let lab = color_space_converter::rgb_to_lab(r, g, b);
        let lab_l = dto
            .lab_l
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", lab.l)).ok());
        let lab_a = dto
            .lab_a
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", lab.a)).ok());
        let lab_b = dto
            .lab_b
            .clone()
            .or_else(|| Decimal::from_str(&format!("{:.2}", lab.b)).ok());

        (
            (cmyk_c, cmyk_m, cmyk_y, cmyk_k),
            (lab_l, lab_a, lab_b),
        )
    }
}
