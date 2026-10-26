#![allow(dead_code)]
//! 色卡仓储管理 - 创建/更新 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建色卡请求 DTO
///
/// 不含初始库存入参：建卡登记的是色卡设计档案（卡号、卡名、类型、季节与封面等描述性信息），
/// 印制与入库是建卡之后的独立登记动作，两者语义分开，新建卡的库存由
/// services/color_card_crud_service.rs::create 落 0。库存数量一律经
/// UpdateColorCardDto 的 stock_quantity 显式提交写入，调用方为
/// handlers/color_card/crud.rs::update_color_card。
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
///
/// 缺省即不改：字段为 None 表示本次提交不触碰该列，保留库中原值；显式提交才写入。
/// stock_quantity 是色卡实物库存的登记入口，由 handlers/color_card/crud.rs::update_color_card
/// 接收、services/color_card_crud_service.rs::update 校验并写 color_cards.stock_quantity
/// （ INTEGER NOT NULL DEFAULT 0，语义为色卡总库存张数）；取值下界依赖库中已发放量，
/// 故校验放在 update 内（与 card_type 校验同族，经 CrudError::Validation 出 400）。
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateColorCardDto {
    pub card_name: Option<String>,
    pub card_type: Option<String>,
    pub season: Option<String>,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    /// 色卡总库存张数，登记后需满足不小于该卡已发放量
    pub stock_quantity: Option<i32>,
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
