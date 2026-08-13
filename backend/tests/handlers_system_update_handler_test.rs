//! 系统更新 Handler 单元测试（批次 394 补测）
//!
//! 覆盖目标：
//! - verify_zip_magic ZIP 文件头校验纯函数（5 个分支）
//! - VersionResponse / UpdateResult DTO 构造（1 个测试）
use bingxi_backend::handlers::system_update_handler::*;

/// test_verify_zip_magichfzip：PK\x03\x04 开头应返回 true。
#[test]
fn test_verify_zip_magichfzip() {
    let data = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00];
    assert!(verify_zip_magic(&data), "合法 ZIP 头应返回 true");
}

/// test_verify_zip_magicksj：空切片应返回 false（无法匹配 4 字节前缀）。
#[test]
fn test_verify_zip_magicksj() {
    let payload: [u8; 0] = [];
    assert!(!verify_zip_magic(&data), "空数据应返回 false");
}

/// test_verify_zip_magicfzipwj：JPEG/PNG 文件头应返回 false。
#[test]
fn test_verify_zip_magicfzipwj() {
    let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00];
    assert!(!verify_zip_magic(&jpeg_header), "JPEG 头应返回 false");

    // PNG 文件头
    let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
    assert!(!verify_zip_magic(&png_header), "PNG 头应返回 false");
}

/// test_verify_zip_magicbfpp：仅前 3 字节匹配或不足 4 字节应返回 false。
#[test]
fn test_verify_zip_magicbfpp() {
    let partial = [0x50, 0x4B, 0x03, 0x00]; // 第 4 字节不匹配
    assert!(!verify_zip_magic(&partial), "部分匹配应返回 false");

    let partial2 = [0x50, 0x4B, 0x03]; // 仅 3 字节
    assert!(!verify_zip_magic(&partial2), "仅 3 字节应返回 false");
}

/// test_verify_zip_magicqh4zj：[0x50, 0x4B, 0x03, 0x04] 应返回 true。
#[test]
fn test_verify_zip_magicqh4zj() {
    let exact = [0x50, 0x4B, 0x03, 0x04];
    assert!(
        verify_zip_magic(&exact),
        "恰好 4 字节合法 ZIP 头应返回 true"
    );
}

/// test_versionresponsehupdateresultgz：验证结构体能正确构造并设置字段。
#[test]
fn test_versionresponsehupdateresultgz() {
    // VersionResponse 构造
    let version_resp = VersionResponse {
        version: "1.0.0".to_string(),
        release_date: "2026-07-14".to_string(),
        changelog: Some("修复若干问题".to_string()),
    };
    assert_eq!(version_resp.version, "1.0.0");
    assert_eq!(version_resp.release_date, "2026-07-14");
    assert!(version_resp.changelog.is_some());

    // UpdateResult 成功构造
    let update_result = UpdateResult {
        success: true,
        message: "更新成功".to_string(),
        new_version: Some("2.0.0".to_string()),
    };
    assert!(update_result.success);
    assert_eq!(update_result.message, "更新成功");
    assert_eq!(update_result.new_version, Some("2.0.0".to_string()));

    // UpdateResult 失败构造
    let failed_result = UpdateResult {
        success: false,
        message: "更新失败".to_string(),
        new_version: None,
    };
    assert!(!failed_result.success);
    assert!(failed_result.new_version.is_none());
}
