//! 色卡库存预警服务
//! V15 P2 类九 10.5-2：库存预警（每日 08:00 执行，黄色<5/红色<2/禁止=0）
use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::utils::error::AppError;
use sea_orm::*;
use serde::Serialize;
use std::sync::Arc;

/// 库存预警级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningLevel {
    /// 库存充足（>= 5）
    Normal,
    /// 黄色预警：库存 < 5
    Yellow,
    /// 红色预警：库存 < 2
    Red,
    /// 禁止发放：库存 = 0
    Forbidden,
}

impl WarningLevel {
    /// 按库存数量判定预警级别
    pub fn from_stock(stock: i32) -> Self {
        match stock {
            0 => Self::Forbidden,
            1 => Self::Red,
            2..=4 => Self::Yellow,
            _ => Self::Normal,
        }
    }
}

/// 色卡库存预警条目
#[derive(Debug, Clone, Serialize)]
pub struct WarningItem {
    pub color_card_id: i32,
    pub color_card_name: String,
    pub current_stock: i32,
    pub warning_level: WarningLevel,
}

/// 色卡库存预警服务
pub struct ColorCardInventoryWarningService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardInventoryWarningService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 检查所有色卡库存预警（仅返回存在风险的色卡；非 archived 状态）
    pub async fn check_all_warnings(&self) -> Result<Vec<WarningItem>, AppError> {
        let cards = ColorCardEntity::find()
            .filter(color_card::Column::Status.ne("archived"))
            .all(&*self.db)
            .await?;

        let mut warnings = Vec::new();
        for card in cards {
            let level = WarningLevel::from_stock(card.stock_quantity);
            if level == WarningLevel::Normal {
                continue;
            }
            warnings.push(WarningItem {
                color_card_id: card.id as i32,
                color_card_name: card.card_name.clone(),
                current_stock: card.stock_quantity,
                warning_level: level,
            });
        }
        Ok(warnings)
    }

    /// 检查单个色卡预警（色卡不存在时返回 NotFound）
    pub async fn check_single_warning(&self, color_card_id: i32) -> Result<WarningLevel, AppError> {
        let card = ColorCardEntity::find_by_id(color_card_id as i64)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("色卡 {} 不存在", color_card_id)))?;
        Ok(WarningLevel::from_stock(card.stock_quantity))
    }
}
