use bingxi_backend::utils::migration_jump_detector::*;


#[test]
fn test_detect_migration_jumps_no_gap() {
    // 创建临时目录测试
    let dir = std::env::temp_dir().join("migration_test_1");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.to_string_lossy().to_string();

    // 创建连续的迁移文件
    std::fs::write(dir.join("20260809000001_test.sql"), "").unwrap();
    std::fs::write(dir.join("20260809000002_test.sql"), "").unwrap();
    std::fs::write(dir.join("20260809000003_test.sql"), "").unwrap();

    let result = detect_migration_jumps(&path);
    assert!(!result.has_jump);

    // 清理
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_detect_migration_jumps_with_gap() {
    let dir = std::env::temp_dir().join("migration_test_2");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.to_string_lossy().to_string();

    // 创建有跳跃的迁移文件
    std::fs::write(dir.join("20260809000001_test.sql"), "").unwrap();
    std::fs::write(dir.join("20260809000003_test.sql"), "").unwrap();

    let result = detect_migration_jumps(&path);
    assert!(result.has_jump);
    assert_eq!(result.missing, vec!["20260809000002"]);

    // 清理
    let _ = std::fs::remove_dir_all(&dir);
}