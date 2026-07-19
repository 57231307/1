//! 色彩空间转换工具
//!
//! 提供纺织行业色卡管理所需的各种色彩空间转换功能：
//! - HEX ↔ RGB
//! - RGB ↔ CMYK
//! - RGB → CIELab (D65 参考白点, sRGB 色彩空间)
//! - CIELab ΔE 色差计算（CIE76 公式）
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §1.2 + §7
//! 创建时间: 2026-06-17

use thiserror::Error;

/// 色彩空间转换错误
#[derive(Debug, Error, PartialEq)]
pub enum ColorSpaceError {
    #[error("HEX 格式错误: 必须是 #RRGGBB 格式")]
    InvalidHexFormat,
    #[error("RGB 值越界: {0}")]
    RgbOutOfRange(String),
}

/// RGB 颜色（0-255）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// CMYK 颜色（0-100 百分比）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cmyk {
    pub c: f64,
    pub m: f64,
    pub y: f64,
    pub k: f64,
}

/// CIELab 颜色
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// HEX 转 RGB
pub fn hex_to_rgb(hex: &str) -> Result<Rgb, ColorSpaceError> {
    let trimmed = hex.trim();
    let hex_no_prefix = trimmed
        .strip_prefix('#')
        .ok_or(ColorSpaceError::InvalidHexFormat)?;

    if hex_no_prefix.len() != 6 {
        return Err(ColorSpaceError::InvalidHexFormat);
    }

    let r = u8::from_str_radix(&hex_no_prefix[0..2], 16)
        .map_err(|_| ColorSpaceError::InvalidHexFormat)?;
    let g = u8::from_str_radix(&hex_no_prefix[2..4], 16)
        .map_err(|_| ColorSpaceError::InvalidHexFormat)?;
    let b = u8::from_str_radix(&hex_no_prefix[4..6], 16)
        .map_err(|_| ColorSpaceError::InvalidHexFormat)?;

    Ok(Rgb::new(r, g, b))
}

/// RGB 转 HEX（#RRGGBB 格式，大写）
pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// RGB 转 CMYK（0-100 百分比）
pub fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> Cmyk {
    let r_norm = r as f64 / 255.0;
    let g_norm = g as f64 / 255.0;
    let b_norm = b as f64 / 255.0;

    let k = 1.0 - r_norm.max(g_norm).max(b_norm);

    if (k - 1.0).abs() < f64::EPSILON {
        // 纯黑色
        return Cmyk {
            c: 0.0,
            m: 0.0,
            y: 0.0,
            k: 100.0,
        };
    }

    let c = (1.0 - r_norm - k) / (1.0 - k);
    let m = (1.0 - g_norm - k) / (1.0 - k);
    let y = (1.0 - b_norm - k) / (1.0 - k);

    Cmyk {
        c: c * 100.0,
        m: m * 100.0,
        y: y * 100.0,
        k: k * 100.0,
    }
}

/// RGB 转 CIELab（D65 参考白点，sRGB 色彩空间）
pub fn rgb_to_lab(r: u8, g: u8, b: u8) -> Lab {
    // 1. sRGB → Linear RGB（去 Gamma）
    let r_lin = srgb_to_linear(r as f64 / 255.0);
    let g_lin = srgb_to_linear(g as f64 / 255.0);
    let b_lin = srgb_to_linear(b as f64 / 255.0);

    // 2. Linear RGB → XYZ（sRGB to XYZ 矩阵 D65）
    let x = r_lin * 0.4124564 + g_lin * 0.3575761 + b_lin * 0.1804375;
    let y = r_lin * 0.2126729 + g_lin * 0.7151522 + b_lin * 0.0721750;
    let z = r_lin * 0.0193339 + g_lin * 0.1191920 + b_lin * 0.9503041;

    // 3. XYZ → Lab（D65 参考白点）
    // 参考白点: Xn=0.95047, Yn=1.0, Zn=1.08883
    let xn = 0.95047;
    let yn = 1.0;
    let zn = 1.08883;

    let fx = lab_f(x / xn);
    let fy = lab_f(y / yn);
    let fz = lab_f(z / zn);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    Lab { l, a, b }
}

/// sRGB 去 Gamma
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Lab 转换辅助函数
fn lab_f(t: f64) -> f64 {
    if t > 0.008856 {
        t.cbrt()
    } else {
        (7.787 * t) + (16.0 / 116.0)
    }
}

/// 判断色差是否在可接受范围内（ΔE ≤ 3.0，GB/T 26377 行业标准）
pub fn delta_e_is_acceptable(delta_e: f64) -> bool {
    delta_e <= 3.0
}

/// CIE76 色差公式（ΔE76）
///
/// 公式：ΔE76 = sqrt((L1-L2)² + (a1-a2)² + (b1-b2)²)
/// 用于计算两个 CIELab 颜色之间的欧氏距离，是纺织行业色差判定的基础公式。
pub fn delta_e_76(lab1: Lab, lab2: Lab) -> f64 {
    let dl = lab1.l - lab2.l;
    let da = lab1.a - lab2.a;
    let db = lab1.b - lab2.b;
    (dl * dl + da * da + db * db).sqrt()
}

// ----------------------------------------------------------------------
// 单元测试
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!((cmyk.c - 0.0).abs() < 0.01);
        assert!((cmyk.m - 0.0).abs() < 0.01);
        assert!((cmyk.y - 0.0).abs() < 0.01);
        assert!((cmyk.k - 0.0).abs() < 0.01);
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
        assert!((cmyk.c - 0.0).abs() < 0.01);
        assert!((cmyk.m - 100.0).abs() < 0.01);
        assert!((cmyk.y - 100.0).abs() < 0.01);
        assert!((cmyk.k - 0.0).abs() < 0.01);
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
}
