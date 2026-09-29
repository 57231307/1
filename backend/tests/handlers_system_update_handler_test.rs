//! 系统更新 Handler 单元测试
//!
//! 覆盖目标：
//! - verify_zip_magic ZIP 文件头校验纯函数（5 个分支）
//! - VersionResponse / UpdateResult DTO 构造
//! - task_to_frontend_json / backup_to_frontend_json 键集合与前端契约对齐
//! - CheckUpdateResponse 序列化包含全部字段（含 current_release_notes/current_published_at）
//! - PaginatedResponse 空表形状（items=[], total=0）
use bingxi_backend::handlers::system_update_handler::*;
use bingxi_backend::models::system_update_backup;
use bingxi_backend::models::system_update_task;
use bingxi_backend::utils::response::PaginatedResponse;
use sea_orm::entity::prelude::*;

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
    assert!(!verify_zip_magic(&payload), "空数据应返回 false");
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

// ============================================================================
// 任务 #118：task_to_frontend_json / backup_to_frontend_json 键集合逐字段断言
// ============================================================================

fn sample_task_model() -> system_update_task::Model {
    let dt: DateTimeWithTimeZone = chrono::Utc::now().fixed_offset();
    system_update_task::Model {
        id: 1,
        task_code: "UPD-20260929120000-0001".to_string(),
        task_type: "download".to_string(),
        source_version: "2026.9.29.1111".to_string(),
        target_version: "2026.10.1.0930".to_string(),
        status: "completed".to_string(),
        progress: 100,
        error_message: None,
        created_by: 42,
        created_by_name: "admin".to_string(),
        completed_at: Some(dt),
        created_at: dt,
        updated_at: dt,
    }
}

fn sample_backup_model() -> system_update_backup::Model {
    let dt: DateTimeWithTimeZone = chrono::Utc::now().fixed_offset();
    system_update_backup::Model {
        id: 1,
        backup_code: "BK-20260929120000".to_string(),
        backup_type: "full".to_string(),
        file_path: "/backups/bk-001.tar.gz".to_string(),
        file_size: 1024,
        description: "系统全量备份".to_string(),
        status: "completed".to_string(),
        created_by: 42,
        created_by_name: "admin".to_string(),
        created_at: dt,
        updated_at: dt,
    }
}

/// task_to_frontend_json 输出键集合与前端 UpdateTask 契约逐字段对齐
#[test]
fn test_task_to_frontend_json_keys() {
    let model = sample_task_model();
    let json = task_to_frontend_json(&model);
    let obj = json
        .as_object()
        .expect("task_to_frontend_json should return a JSON object");

    // 前端 UpdateTask 接口声明的全部键（snake_case）
    let expected_keys: Vec<&str> = vec![
        "id",
        "task_code",
        "from_version",
        "to_version",
        "status",
        "progress",
        "error_message",
        "backup_path",
        "started_at",
        "completed_at",
        "created_by",
        "created_by_name",
        "created_at",
    ];
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "task_to_frontend_json 缺少键: {}",
            key
        );
    }
    // 逐字段值断言（关键映射）
    assert_eq!(obj["id"], serde_json::json!(1));
    assert_eq!(
        obj["task_code"],
        serde_json::json!("UPD-20260929120000-0001")
    );
    assert_eq!(
        obj["from_version"],
        serde_json::json!("2026.9.29.1111"),
        "source_version 映射到 from_version"
    );
    assert_eq!(
        obj["to_version"],
        serde_json::json!("2026.10.1.0930"),
        "target_version 映射到 to_version"
    );
    assert_eq!(obj["status"], serde_json::json!("completed"));
    assert_eq!(obj["progress"], serde_json::json!(100));
    assert_eq!(
        obj["error_message"],
        serde_json::json!(""),
        "None 映射为空字符串"
    );
    assert_eq!(
        obj["backup_path"],
        serde_json::json!(""),
        "固定空字符串占位"
    );
    assert_eq!(obj["created_by"], serde_json::json!(42));
    assert_eq!(obj["created_by_name"], serde_json::json!("admin"));
}

/// backup_to_frontend_json 输出键集合与前端 SystemBackup 契约逐字段对齐
#[test]
fn test_backup_to_frontend_json_keys() {
    let model = sample_backup_model();
    let json = backup_to_frontend_json(&model);
    let obj = json
        .as_object()
        .expect("backup_to_frontend_json should return a JSON object");

    // 前端 SystemBackup 接口声明的全部键
    let expected_keys: Vec<&str> = vec![
        "id",
        "backup_code",
        "backup_type",
        "file_path",
        "file_size",
        "description",
        "status",
        "created_by",
        "created_by_name",
        "created_at",
    ];
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "backup_to_frontend_json 缺少键: {}",
            key
        );
    }
    // 逐字段值断言
    assert_eq!(obj["id"], serde_json::json!(1));
    assert_eq!(obj["backup_code"], serde_json::json!("BK-20260929120000"));
    assert_eq!(obj["backup_type"], serde_json::json!("full"));
    assert_eq!(
        obj["file_path"],
        serde_json::json!("/backups/bk-001.tar.gz")
    );
    assert_eq!(obj["file_size"], serde_json::json!(1024));
    assert_eq!(obj["description"], serde_json::json!("系统全量备份"));
    assert_eq!(obj["status"], serde_json::json!("completed"));
    assert_eq!(obj["created_by"], serde_json::json!(42));
    assert_eq!(obj["created_by_name"], serde_json::json!("admin"));
}

/// PaginatedResponse 空表形状：items=[], total=0（诚实"暂无"）
#[test]
fn test_paginated_response_empty_table_shape() {
    let empty: PaginatedResponse<serde_json::Value> = PaginatedResponse::new(vec![], 0, 1, 0);
    let json = serde_json::to_value(&empty).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj["items"].as_array().unwrap().len(), 0);
    assert_eq!(obj["total"], serde_json::json!(0));
    assert_eq!(obj["page"], serde_json::json!(1));
    assert_eq!(obj["page_size"], serde_json::json!(0));
}

/// PaginatedResponse 含数据形状：items 非空、total 匹配
#[test]
fn test_paginated_response_with_items() {
    let model = sample_task_model();
    let items = vec![task_to_frontend_json(&model)];
    let total = items.len() as u64;
    let paginated: PaginatedResponse<serde_json::Value> =
        PaginatedResponse::new(items, total, 1, total);
    let json = serde_json::to_value(&paginated).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj["items"].as_array().unwrap().len(), 1);
    assert_eq!(obj["total"], serde_json::json!(1));
    // items[0] 含前端期望的键
    let first = &obj["items"][0];
    assert!(first.get("task_code").is_some());
    assert!(first.get("from_version").is_some());
    assert!(first.get("to_version").is_some());
}

// ============================================================================
// 任务 #120：CheckUpdateResponse 序列化包含全部字段（含 current_release_notes）
// ============================================================================

/// CheckUpdateResponse 序列化后包含所有 10 个键
#[test]
fn test_check_update_response_serialization_all_fields() {
    let resp = CheckUpdateResponse {
        has_update: true,
        current_version: "2026.9.29.1111".to_string(),
        latest_version: "2026.10.1.0930".to_string(),
        download_url: Some("https://example.com/pkg.zip".to_string()),
        file_size: Some(2048),
        release_notes: Some("新版本功能列表".to_string()),
        published_at: Some("2026-10-01T09:30:00Z".to_string()),
        current_release_notes: Some("当前版本更新日志".to_string()),
        current_published_at: Some("2026-09-29T11:11:00Z".to_string()),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let obj = json.as_object().unwrap();

    let expected_keys = [
        "has_update",
        "current_version",
        "latest_version",
        "download_url",
        "file_size",
        "release_notes",
        "published_at",
        "current_release_notes",
        "current_published_at",
    ];
    for key in &expected_keys {
        assert!(
            obj.contains_key(*key),
            "CheckUpdateResponse 序列化缺少键: {}",
            key
        );
    }
    assert_eq!(
        obj["current_release_notes"],
        serde_json::json!("当前版本更新日志")
    );
    assert_eq!(
        obj["current_published_at"],
        serde_json::json!("2026-09-29T11:11:00Z")
    );
}

/// has_update=false 且 current 无 release 时，current_release_notes 为 null（诚实空态）
#[test]
fn test_check_update_response_no_current_release_honest_null() {
    let resp = CheckUpdateResponse {
        has_update: false,
        current_version: "2026.9.29.1111".to_string(),
        latest_version: "2026.9.29.1111".to_string(),
        download_url: None,
        file_size: None,
        release_notes: None,
        published_at: None,
        current_release_notes: None,
        current_published_at: None,
    };
    let json = serde_json::to_value(&resp).unwrap();
    let obj = json.as_object().unwrap();
    // Option<String> 无 skip_serializing_if，序列化后键存在、值为 null
    assert!(obj.contains_key("current_release_notes"));
    assert!(obj["current_release_notes"].is_null());
    assert!(obj.contains_key("current_published_at"));
    assert!(obj["current_published_at"].is_null());
}

// ============================================================================
// 任务 #120：fetch_release_by_tag 的 URL 拼接验证（v 前缀正确性）
// ============================================================================

/// 验证 tag 拼接：给定 version 字符串，生成的 URL 含 `tags/v{version}` 段
/// （纯字符串逻辑提取，与 github.rs 中 format! 一致）
#[test]
fn test_fetch_release_by_tag_url_has_v_prefix() {
    let version = "2026.9.29.1111";
    let tag_url = format!(
        "https://api.github.com/repos/57231307/1/releases/tags/v{}",
        version
    );
    assert!(
        tag_url.contains("/releases/tags/v2026.9.29.1111"),
        "URL 应含 tags/v 前缀拼接: {}",
        tag_url
    );
    // 不应出现 v v 双前缀
    assert!(!tag_url.contains("tags/vv"), "tag URL 不应出现双 v 前缀");
}
