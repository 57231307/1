//! 公司档案 logo 写入口判据契约测试（纯函数层，不触库）
//!
//! 钉住的对象与事实：
//! - 正向：真实 PNG 裸 base64 通过；带 `data:image/png;base64,` 前缀、MIME 变体前缀
//!   （image/x-png）、含换行折行的同一 PNG 值都经归一后通过（用户"上传的就是 png
//!   却被拒"的三类真实形态在写入口不再被误拒）。
//! - 反向：伪 png（内容 JPEG/GIF/WebP/BMP/ICO）被点名"实际检测为 <格式>"；非法
//!   base64 点名"不是合法的 base64"；超 512KB 点名"超过 512KB 上限"。三类失败
//!   消息互不相同，错误原因可追踪，不用统一兜底文案掩盖。
//!
//! 断言只取后端真实 AppError 出参 message（to_response），不 mock、不放宽。

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use bingxi_backend::services::company_profile_service::{LOGO_MAX_BYTES, decode_logo_png};

/// 构造最小合法 PNG 头部字节（签名 8 + IHDR 块），魔数判据只需前 4 字节命中。
fn png_bytes() -> Vec<u8> {
    let mut v: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    v.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&[0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08]);
    v.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]);
    v
}

fn b64(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// 拒绝路径小工具：Ok 视为判据失效直接 panic，Err 时取真实出参 message。
fn reject_message(
    result: Result<Vec<u8>, bingxi_backend::utils::error::AppError>,
    ctx: &str,
) -> String {
    match result {
        Ok(_) => panic!("{ctx}：本应被拒却通过了"),
        Err(err) => err.to_response().message,
    }
}

#[test]
fn valid_png_bare_base64_passes() {
    let payload = b64(&png_bytes());
    let decoded = decode_logo_png(&payload).unwrap_or_else(|e| {
        panic!(
            "合法 PNG 裸 base64 不应被拒，实际：{}",
            e.to_response().message
        )
    });
    assert_eq!(decoded, png_bytes());
}

#[test]
fn data_url_prefixed_and_mime_variant_png_pass_after_normalization() {
    let payload = b64(&png_bytes());
    for prefixed in [
        format!("data:image/png;base64,{payload}"),
        format!("data:image/x-png;base64,{payload}"),
        format!("  data:image/png;base64,{payload}  "),
    ] {
        let decoded = decode_logo_png(&prefixed).unwrap_or_else(|e| {
            panic!(
                "带前缀的合法 PNG 不应被拒（{prefixed}）：{}",
                e.to_response().message
            )
        });
        assert_eq!(decoded, png_bytes(), "前缀归一后应还原出同一 PNG 字节");
    }
}

#[test]
fn wrapped_base64_png_passes_after_normalization() {
    let payload = b64(&png_bytes());
    let wrapped = payload
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let decoded = decode_logo_png(&wrapped)
        .unwrap_or_else(|e| panic!("折行的合法 PNG 不应被拒：{}", e.to_response().message));
    assert_eq!(decoded, png_bytes());
}

#[test]
fn fake_png_is_rejected_with_actual_format_named() {
    let cases: Vec<(&str, Vec<u8>, &str)> = vec![
        (
            "JPEG 伪装 png",
            vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3, 4],
            "JPEG",
        ),
        ("GIF 伪装 png", b"GIF89a\x01\x00".to_vec(), "GIF"),
        (
            "WebP 伪装 png",
            [b"RIFF".to_vec(), vec![0, 0, 0, 0], b"WEBP".to_vec()].concat(),
            "WebP",
        ),
        ("BMP 伪装 png", b"BMxxxx".to_vec(), "BMP"),
        ("ICO 伪装 png", vec![0x00, 0x00, 0x01, 0x00, 1, 1], "ICO"),
        (
            "仅前 4 字节对但签名后 4 字节畸形的伪 png",
            vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x00, 0x00, 0x00],
            "未知格式",
        ),
    ];
    for (label, bytes, expected_name) in cases {
        let payload = b64(&bytes);
        let msg = reject_message(decode_logo_png(&payload), label);
        assert!(
            msg.contains(expected_name),
            "{label} 的拒绝消息应点名实际格式 {expected_name}，实际：{msg}"
        );
    }
}

#[test]
fn invalid_base64_and_oversize_each_get_distinct_messages() {
    let msg = reject_message(decode_logo_png("这不是base64!!!"), "非 base64 串必须被拒");
    assert!(
        msg.contains("不是合法的 base64"),
        "应点名 base64 非法，实际：{msg}"
    );

    let mut big = png_bytes();
    big.resize(LOGO_MAX_BYTES + 1, 0);
    let msg = reject_message(decode_logo_png(&b64(&big)), "超限 PNG 必须被拒");
    assert!(msg.contains("512KB"), "应点名大小超限，实际：{msg}");
    assert!(
        !msg.contains("仅支持 PNG"),
        "超限失败不得复用格式文案，实际：{msg}"
    );
}
