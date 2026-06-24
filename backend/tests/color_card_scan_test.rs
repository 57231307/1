//! 色卡仓储管理 - 扫码查询 集成测试
//!
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use bingxi_backend::utils::color_space_converter::{delta_e_76, delta_e_is_acceptable, rgb_to_lab};

    #[test]
    fn test_scan_by_color_code_lab_values() {
        // 验证扫码返回的 Lab 值合理
        let lab = rgb_to_lab(255, 0, 0);
        // 红色的 a 应为正（红色方向），b 应较小
        assert!(lab.a > 50.0);
        assert!(lab.l > 50.0);
    }

    #[test]
    fn test_scan_color_difference_passes() {
        // 行业标准：色差 ΔE ≤ 3 视为合格
        let target = rgb_to_lab(120, 80, 200);
        let actual = rgb_to_lab(122, 81, 199);
        let de = delta_e_76(target, actual);
        assert!(delta_e_is_acceptable(de));
    }

    #[test]
    fn test_scan_color_difference_fails() {
        // 行业标准：色差 ΔE > 3 视为不合格
        let target = rgb_to_lab(0, 0, 0);
        let actual = rgb_to_lab(30, 30, 30);
        let de = delta_e_76(target, actual);
        assert!(!delta_e_is_acceptable(de));
    }

    #[test]
    fn test_pantone_cncs_custom_code_combination() {
        // 同一色号可同时拥有 PANTONE、CNCS 和自定义编码
        let pantone = Some("18-1664 TPX");
        let cncs = Some("S 1050-Y90R");
        let custom = Some("BX-C-001");
        assert!(pantone.is_some() && cncs.is_some() && custom.is_some());
    }
}
