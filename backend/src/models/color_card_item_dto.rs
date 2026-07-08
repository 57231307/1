//! 色卡仓储管理 - 色号 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建/更新色号请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct ColorItemDto {
    /// 色号编码
    #[validate(length(min = 1, max = 50))]
    pub color_code: String,

    /// 色号名称
    #[validate(length(min = 1, max = 200))]
    pub color_name: String,

    /// RGB 红（0-255）
    #[validate(range(min = 0, max = 255))]
    pub rgb_r: i32,

    /// RGB 绿（0-255）
    #[validate(range(min = 0, max = 255))]
    pub rgb_g: i32,

    /// RGB 蓝（0-255）
    #[validate(range(min = 0, max = 255))]
    pub rgb_b: i32,

    /// CMYK 青（0-100，nullable，自动计算时可省略）
    pub cmyk_c: Option<Decimal>,
    pub cmyk_m: Option<Decimal>,
    pub cmyk_y: Option<Decimal>,
    pub cmyk_k: Option<Decimal>,

    /// CIELab 色彩空间（nullable，自动计算时可省略）
    pub lab_l: Option<Decimal>,
    pub lab_a: Option<Decimal>,
    pub lab_b: Option<Decimal>,

    /// PANTONE 编码
    pub pantone_code: Option<String>,
    /// CNCS 编码
    pub cncs_code: Option<String>,
    /// 自定义编码
    pub custom_code: Option<String>,

    /// HEX 颜色值 #RRGGBB（必填，由 RGB 自动计算或前端传入）
    #[validate(length(min = 7, max = 7))]
    pub hex_value: String,

    /// 关联染色配方 ID
    pub dye_recipe_id: Option<i32>,

    /// 关联色号价格 ID
    pub product_color_price_id: Option<i64>,

    /// 色片图 URL
    pub swatch_image_url: Option<String>,

    /// 在色卡中的顺序
    pub sequence: Option<i32>,
}

/// 批量导入色号请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchImportItemsDto {
    /// 色号列表
    pub items: Vec<ColorItemDto>,
}

/// 批量导入响应 DTO
#[derive(Debug, Serialize, Clone)]
pub struct BatchImportResponse {
    /// 成功导入数量
    pub success_count: usize,
    /// 失败数量
    pub failed_count: usize,
    /// 失败明细
    pub errors: Vec<BatchImportError>,
    /// 导入后色卡总色号数
    pub total_colors: i32,
}

/// 批量导入错误明细
#[derive(Debug, Serialize, Clone)]
pub struct BatchImportError {
    /// 失败色号编码
    pub color_code: String,
    /// 失败原因
    pub reason: String,
}
