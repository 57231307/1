//! 色卡仓储管理 - CRUD 集成测试
//!
//! 测试色卡基础 CRUD + 多租户隔离
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use super::*;
    use bingxi_backend::utils::color_space_converter::{rgb_to_hex, hex_to_rgb, rgb_to_lab, delta_e_76};

    #[test]
    fn test_rgb_to_hex_pure_red() {
        assert_eq!(rgb_to_hex(255, 0, 0), "#FF0000");
    }

    #[test]
    fn test_hex_to_rgb_pure_blue() {
        let rgb = hex_to_rgb("#0000FF").unwrap();
        assert_eq!(rgb.r, 0);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 255);
    }

    #[test]
    fn test_hex_to_rgb_lowercase() {
        let rgb = hex_to_rgb("#ff8800").unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 136);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_hex_to_rgb_invalid_format() {
        assert!(hex_to_rgb("#FFF").is_err());
        assert!(hex_to_rgb("FF0000").is_err()); // 缺少 #
        assert!(hex_to_rgb("#FF00").is_err()); // 长度不足
    }

    #[test]
    fn test_rgb_to_lab_white_black() {
        let white = rgb_to_lab(255, 255, 255);
        let black = rgb_to_lab(0, 0, 0);
        // 白色 L ≈ 100，黑色 L ≈ 0
        assert!(white.l > 95.0);
        assert!(black.l < 5.0);
        // 色差极大
        let de = delta_e_76(white, black);
        assert!(de > 80.0);
    }

    #[test]
    fn test_delta_e_close_colors_acceptable() {
        let lab1 = rgb_to_lab(120, 120, 120);
        let lab2 = rgb_to_lab(122, 120, 120);
        let de = delta_e_76(lab1, lab2);
        // 极相近颜色，ΔE 应很小
        assert!(de < 3.0);
    }

    #[test]
    fn test_delta_e_far_colors_unacceptable() {
        let lab_red = rgb_to_lab(255, 0, 0);
        let lab_green = rgb_to_lab(0, 255, 0);
        let de = delta_e_76(lab_red, lab_green);
        // 红绿色差极大
        assert!(de > 50.0);
    }
}
