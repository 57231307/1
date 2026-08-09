use std::path::Path;
use tracing::warn;

/// 迁移跳跃检测结果
#[derive(Debug)]
pub struct MigrationJumpResult {
    pub has_jump: bool,
    pub expected: Vec<String>,
    pub actual: Vec<String>,
    pub missing: Vec<String>,
}

/// batch-17 P3: 迁移跳跃检测
/// 检查迁移文件序列是否有跳跃（缺失的迁移号）
pub fn detect_migration_jumps(migrations_dir: &str) -> MigrationJumpResult {
    let path = Path::new(migrations_dir);
    if !path.exists() {
        return MigrationJumpResult {
            has_jump: false,
            expected: vec![],
            actual: vec![],
            missing: vec![],
        };
    }

    let mut migration_ids: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // 提取迁移 ID（时间戳部分）
            if let Some(id) = name.split('_').next() {
                if id.len() >= 14 && id.chars().all(|c| c.is_ascii_digit()) {
                    migration_ids.push(id.to_string());
                }
            }
        }
    }

    migration_ids.sort();
    migration_ids.dedup();

    let mut missing = Vec::new();
    for i in 0..migration_ids.len().saturating_sub(1) {
        let current = &migration_ids[i];
        let next = &migration_ids[i + 1];

        // 检查是否有跳跃（假设序列号递增）
        if let (Ok(curr_num), Ok(next_num)) = (current.parse::<u64>(), next.parse::<u64>()) {
            if next_num - curr_num > 1 {
                for gap in (curr_num + 1)..next_num {
                    missing.push(format!("{:014}", gap));
                }
            }
        }
    }

    let has_jump = !missing.is_empty();

    if has_jump {
        warn!(
            "检测到迁移序列跳跃: 缺失 {:?}",
            missing
        );
    }

    MigrationJumpResult {
        has_jump,
        expected: vec![],
        actual: migration_ids,
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
