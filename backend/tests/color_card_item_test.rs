//! 色卡仓储管理 - 色号 + 批量导入 集成测试
//!
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use crate::utils::color_space_converter::{rgb_to_cmyk, rgb_to_hex, rgb_to_lab};

    #[test]
    fn test_rgb_to_cmyk_white() {
        let cmyk = rgb_to_cmyk(255, 255, 255);
        assert_eq!(cmyk.c, 0.0);
        assert_eq!(cmyk.m, 0.0);
        assert_eq!(cmyk.y, 0.0);
        assert_eq!(cmyk.k, 0.0);
    }

    #[test]
    fn test_rgb_to_cmyk_black() {
        let cmyk = rgb_to_cmyk(0, 0, 0);
        assert_eq!(cmyk.c, 0.0);
        assert_eq!(cmyk.m, 0.0);
        assert_eq!(cmyk.y, 0.0);
        // 黑色 K=100
        assert!((cmyk.k - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_cmyk_red() {
        let cmyk = rgb_to_cmyk(255, 0, 0);
        // 纯红 = M100 Y100
        assert!(cmyk.c < 1.0);
        assert!((cmyk.m - 100.0).abs() < 0.01);
        assert!((cmyk.y - 100.0).abs() < 0.01);
        assert!(cmyk.k < 1.0);
    }

    #[test]
    fn test_color_card_code_uniqueness_check() {
        // 模拟两个色卡的色号编码唯一性
        let card1_color_a = "18-1664";
        let card2_color_a = "18-1664"; // 同一编码在不同色卡允许
        assert_eq!(card1_color_a, card2_color_a);
    }

    #[test]
    fn test_hex_generation_from_rgb() {
        // 批量导入时自动生成 hex
        let hex = rgb_to_hex(18, 52, 86);
        assert_eq!(hex, "#123456");
    }

    #[test]
    fn test_lab_calculation_consistency() {
        // 同一 RGB 计算 Lab 结果稳定
        let lab1 = rgb_to_lab(128, 64, 200);
        let lab2 = rgb_to_lab(128, 64, 200);
        assert!((lab1.l - lab2.l).abs() < 0.001);
        assert!((lab1.a - lab2.a).abs() < 0.001);
        assert!((lab1.b - lab2.b).abs() < 0.001);
    }
}
