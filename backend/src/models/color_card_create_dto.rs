//! 色卡仓储管理 - 创建/更新 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateColorCardDto {
    /// 色卡编号（如 PANTONE-TPX-2024-SS）
    #[validate(length(min = 1, max = 50))]
    pub card_no: String,

    /// 色卡名称
    #[validate(length(min = 1, max = 200))]
    pub card_name: String,

    /// 色卡类型：PANTONE / CNCS / CUSTOM
    #[validate(length(min = 1, max = 50))]
    pub card_type: String,

    /// 季节标签
    pub season: Option<String>,

    /// 品牌
    pub brand: Option<String>,

    /// 描述
    pub description: Option<String>,

    /// 封面图 URL
    pub cover_image_url: Option<String>,
}

/// 更新色卡请求 DTO（仅未归档状态可更新）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateColorCardDto {
    pub card_name: Option<String>,
    pub card_type: Option<String>,
    pub season: Option<String>,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
}

/// 归档色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArchiveColorCardDto {
    /// 归档原因
    pub reason: Option<String>,
}

/// 色卡列表查询参数（V15 P0-F03：从 color_card_borrow_dto.rs 迁移，borrow 模式已废弃）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListColorCardsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub card_type: Option<String>,
    pub season: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}
