use bingxi_backend::utils::color_space_converter::*;


#[test]
fn test_hex_to_rgb_basic() {
    // P0 6-2 修复：统一使用 `#` 前缀（与实现一致，避免无前缀格式触发 InvalidHexFormat）
    assert_eq!(hex_to_rgb("#FF0000").unwrap(), Rgb::new(255, 0, 0));
    assert_eq!(hex_to_rgb("#00FF00").unwrap(), Rgb::new(0, 255, 0));
    assert_eq!(hex_to_rgb("#0000FF").unwrap(), Rgb::new(0, 0, 255));
}

#[test]
fn test_hex_to_rgb_invalid() {
    assert!(hex_to_rgb("#FFF").is_err());
    assert!(hex_to_rgb("GGGGGG").is_err());
    assert!(hex_to_rgb("").is_err());
    assert!(hex_to_rgb("#1234567").is_err());
}

#[test]
fn test_rgb_to_cmyk_white() {
    let cmyk = rgb_to_cmyk(255, 255, 255);
    assert!(cmyk.c.abs() < 0.01);
    assert!(cmyk.m.abs() < 0.01);
    assert!(cmyk.y.abs() < 0.01);
    assert!(cmyk.k.abs() < 0.01);
}

#[test]
fn test_rgb_to_cmyk_black() {
    let cmyk = rgb_to_cmyk(0, 0, 0);
    assert_eq!(cmyk.c, 0.0);
    assert_eq!(cmyk.m, 0.0);
    assert_eq!(cmyk.y, 0.0);
    assert!((cmyk.k - 100.0).abs() < 0.01);
}

#[test]
fn test_rgb_to_cmyk_red() {
    let cmyk = rgb_to_cmyk(255, 0, 0);
    assert!(cmyk.c.abs() < 0.01);
    assert!((cmyk.m - 100.0).abs() < 0.01);
    assert!((cmyk.y - 100.0).abs() < 0.01);
    assert!(cmyk.k.abs() < 0.01);
}

#[test]
fn test_rgb_to_lab_white() {
    let lab = rgb_to_lab(255, 255, 255);
    // 白色 L 应接近 100，a 和 b 应接近 0
    assert!((lab.l - 100.0).abs() < 0.5);
    assert!(lab.a.abs() < 0.5);
    assert!(lab.b.abs() < 0.5);
}

#[test]
fn test_rgb_to_lab_black() {
    let lab = rgb_to_lab(0, 0, 0);
    // 黑色 L 应接近 0
    assert!(lab.l.abs() < 0.5);
}

#[test]
fn test_delta_e_is_acceptable() {
    // ΔE ≤ 3.0 视为色差可接受（GB/T 26377 行业标准）
    assert!(delta_e_is_acceptable(0.0));
    assert!(delta_e_is_acceptable(2.0));
    assert!(delta_e_is_acceptable(3.0));
    assert!(!delta_e_is_acceptable(3.1));
    assert!(!delta_e_is_acceptable(50.0));
}