//! 色卡 Handler 内部辅助函数
//!
//! V15 P0-F03 重构：删除 record_to_info（borrow 模式已废弃，改用 issue.rs 的 From impl）

use crate::models::color_card_item;
use crate::models::color_card_response_dto::ColorItemInfo;

/// 色号列表查询参数
#[derive(Debug, serde::Deserialize)]
pub struct ListItemsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// color_card_item::Model → ColorItemInfo
pub fn item_to_info(m: color_card_item::Model) -> ColorItemInfo {
    ColorItemInfo {
        id: m.id,
        color_code: m.color_code,
        color_name: m.color_name,
        rgb_r: m.rgb_r,
        rgb_g: m.rgb_g,
        rgb_b: m.rgb_b,
        cmyk_c: m.cmyk_c,
        cmyk_m: m.cmyk_m,
        cmyk_y: m.cmyk_y,
        cmyk_k: m.cmyk_k,
        lab_l: m.lab_l,
        lab_a: m.lab_a,
        lab_b: m.lab_b,
        pantone_code: m.pantone_code,
        cncs_code: m.cncs_code,
        custom_code: m.custom_code,
        hex_value: m.hex_value,
        dye_recipe_id: m.dye_recipe_id,
        product_color_price_id: m.product_color_price_id,
        swatch_image_url: m.swatch_image_url,
        sequence: m.sequence,
    }
}
