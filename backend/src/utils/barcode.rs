//! Code128 条码光栅渲染工具（成品布入库标签的条码图形产出）
//!
//! 条码的**编码语义**（符号表、A/B/C 子集自动切换、ISO/IEC 15417 mod-103 校验位）
//! 全部由 `code128` crate 产出，本模块不做任何自行编码；只做唯一无业务语义的
//! 像素操作：把库产出的标准条序（含左右各 10 模块静区的坐标）按整数倍 X 维度
//! 涂成白底黑条位图，再经 `image` 的 png 编码器产出可被
//! `docx_rs::Pic::new_with_dimensions` 直接消费的 PNG 字节。

use image::{ImageBuffer, ImageFormat, Luma};

use crate::utils::docx_export::DocxImage;
use crate::utils::error::AppError;

/// 单模块（最窄条，即「1X」）的像素宽度。
///
/// 取 2X 的原因：1X 在低dpi 热敏打印上边缘易糊导致误读，≥3X 会让 100mm 级
/// 卷标签整体条码宽度过大挤压文字区；PDA 识读样例常用 1.5X–2X，取整数上限。
/// 改动此值会同步改变画布宽（= 总模块数 × 本值），须连同标签纸排版复核。
/// （契约测试按本常量校验光栅几何，故公开而非私有魔数。）
pub const MODULE_WIDTH_PX: u32 = 2;

/// 条码图形高度（像素）。与 2X 窄条配合保证卷标纵向可扫识别带；
/// 调整本值需随标签模板版式一起复核（当前是全仓唯一的 docx 嵌入图）。
pub const BARCODE_HEIGHT_PX: u32 = 60;

/// 把文本码值渲染为 Code128 PNG 位图（供 docx 嵌入）。
///
/// 编码内容即传入值本身：标签口径下该值 = 匹行 `barcode` 列实测码值
/// （见 `services::print_service::get_inventory_piece_label_print_data` 的 fail-closed），
/// 不回落、不替换、不另造语义。
///
/// # Errors
///
/// 返回 `AppError::Internal` 仅当：
/// - 库声明的总宽（含静区）与实际条坐标越界不一致——编码器与渲染契约漂移的真实缺陷信号；
/// - PNG 编码失败——内存写入异常，保留原始错误供定位。
pub fn render_code128_png(content: &str) -> Result<DocxImage, AppError> {
    let code = code128::Code128::encode(content.as_bytes());

    // code128::Code128::len() 按 ISO/IEC 15417 已含左右各 10 模块静区（源码文档
    // 「with the quiet zone included」实测），bar_coordinates() 的 x 原点 10 即左静区宽。
    let width_px = code.len() as u32 * MODULE_WIDTH_PX;
    let height_px = BARCODE_HEIGHT_PX;
    let mut img: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width_px, height_px, Luma([255u8]));

    for bar in code.bar_coordinates() {
        let x_end = (bar.x + bar.width as u32) * MODULE_WIDTH_PX;
        if x_end > width_px {
            return Err(AppError::internal(format!(
                "Code128 渲染契约漂移：条坐标 x={} width={} 超出声明总宽 {}px",
                bar.x, bar.width, width_px
            )));
        }
        let y_end = height_px as usize;
        for y in 0..y_end {
            for x in (bar.x * MODULE_WIDTH_PX)..x_end {
                img.put_pixel(x, y as u32, Luma([0u8]));
            }
        }
    }

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| AppError::internal(format!("Code128 条码 PNG 编码失败: {}", e)))?;

    Ok(DocxImage {
        png: cursor.into_inner(),
        width_px,
        height_px,
    })
}
