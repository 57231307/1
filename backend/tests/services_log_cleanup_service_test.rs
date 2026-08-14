use axum::extract::Path;
use bingxi_backend::handlers::production_recipe_handler::*;
use bingxi_backend::services::log_cleanup_service::*;
use chrono::Duration;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;

/// 构造唯一临时目录：/tmp/bingxi_log_cleanup_test_<uuid>
fn make_temp_dir() -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("bingxi_log_cleanup_test_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).expect("创建临时目录失败");
    dir
}

#[test]
fn cleanup_deletes_files_older_than_cutoff() {
    let dir = make_temp_dir();
    // 创建若干文件（mtime = 当前时间）
    fs::File::create(dir.join("recent.log")).expect("创建文件失败");
    fs::create_dir_all(dir.join("audit")).expect("创建子目录失败");
    fs::File::create(dir.join("audit").join("recent_audit.log")).expect("创建文件失败");

    // cutoff 设为未来 1 小时：当前 mtime < 未来 cutoff，所有文件应被删除
    let future_cutoff = SystemTime::now() + Duration::from_secs(3600);
    let deleted = LogCleanupService::cleanup_dir_recursive(&dir, future_cutoff).expect("清理失败");

    assert_eq!(deleted, 2);
    assert!(!dir.join("recent.log").exists());
    assert!(!dir.join("audit").join("recent_audit.log").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cleanup_preserves_files_newer_than_cutoff() {
    let dir = make_temp_dir();
    fs::File::create(dir.join("keep.log")).expect("创建文件失败");

    // cutoff 设为过去 1 小时：当前 mtime > 过去 cutoff，文件应保留
    let past_cutoff = SystemTime::now() - Duration::from_secs(3600);
    let deleted = LogCleanupService::cleanup_dir_recursive(&dir, past_cutoff).expect("清理失败");

    assert_eq!(deleted, 0);
    assert!(dir.join("keep.log").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cleanup_missing_dir_returns_zero() {
    let missing = Path::new("/nonexistent/bingxi_log_cleanup_test_missing_dir");
    let cutoff = SystemTime::now();
    let deleted = LogCleanupService::cleanup_dir_recursive(missing, cutoff).expect("应返回 0");
    assert_eq!(deleted, 0);
}
