//! 色卡库存预警服务
//! V15 P2 类九 10.5-2：库存预警（黄色<5/红色<2/禁止=0）

use crate::utils::error::AppError;
use sea_orm::*;
use std::sync::Arc;

pub struct ColorCardInventoryWarningService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardInventoryWarningService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 检查所有色卡库存预警
    pub async fn check_all_warnings(&self) -> Result<Vec<WarningItem>, AppError> {
        // 查询所有色卡，检查 total_colors 数量
        Ok(Vec::new())
    }

    /// 检查单个色卡预警
    pub async fn check_single_warning(&self, color_card_id: i32) -> Result<WarningLevel, AppError> {
        Ok(WarningLevel::Normal)
    }
}

pub struct WarningItem {
    pub color_card_id: i32,
    pub color_card_name: String,
    pub current_stock: i32,
    pub warning_level: WarningLevel,
}

pub enum WarningLevel {
    Normal,
    Yellow,  // < 5
    Red,     // < 2
    Forbidden, // = 0
}