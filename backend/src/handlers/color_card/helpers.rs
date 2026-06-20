//! 色卡 Handler 内部辅助函数
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 集中 ListItemsQuery 查询参数 + Model → DTO 转换 + CSV 转义
//! 行为完全保持一致（仅结构重构）

use crate::models::color_card_borrow_record;
use crate::models::color_card_item;
use crate::models::color_card_response_dto::{BorrowRecordInfo, ColorItemInfo};

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

/// color_card_borrow_record::Model → BorrowRecordInfo
pub fn record_to_info(m: color_card_borrow_record::Model) -> BorrowRecordInfo {
    BorrowRecordInfo {
        id: m.id,
        color_card_id: m.color_card_id,
        color_card_no: None,
        color_card_name: None,
        customer_id: m.customer_id,
        customer_name: None,
        borrowed_by: m.borrowed_by,
        borrowed_by_name: None,
        borrowed_at: m.borrowed_at,
        expected_return_at: m.expected_return_at,
        actual_return_at: m.actual_return_at,
        status: m.status,
        purpose: m.purpose,
        notes: m.notes,
        compensation_amount: m.compensation_amount,
    }
}

/// CSV 单元格转义（含逗号/引号/换行则加双引号并转义内部引号）
pub fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
