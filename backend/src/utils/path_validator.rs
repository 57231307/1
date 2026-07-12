//! 路径校验工具（防 Tar Slip 路径穿越攻击）
//!
//! 批次 322 v9 复审低危修复：抽取 `backup.rs` 和 `upgrade.rs` 中重复的路径校验逻辑
//! 到共享模块，遵循 DRY 原则，统一维护安全校验策略。
//!
//! ## 拦截范围
//!
//! - **符号链接逃逸**：`canonicalize` 解析所有符号链接，校验最终路径在基准目录内
//! - **恶意嵌套目录**：递归深度上限 `MAX_RECURSION_DEPTH`，防止栈溢出 DoS
//! - **路径穿越**：`starts_with` 校验所有文件路径在基准目录范围内

use std::path::Path;

/// 递归深度上限（防止恶意嵌套目录导致栈溢出 DoS）
pub const MAX_RECURSION_DEPTH: usize = 100;

/// 校验解压后的所有文件路径都在指定目录范围内
///
/// 防止 Tar Slip 路径穿越攻击：攻击者在 tar 包中构造 `../../../etc/passwd` 等路径，
/// 解压后逃逸到基准目录外。本函数递归扫描基准目录下所有文件，使用 `canonicalize`
/// 解析符号链接后校验最终路径是否仍在基准目录内。
///
/// # 参数
/// - `base_dir`: 基准目录路径（字符串形式）
///
/// # 返回
/// - `Ok(())`: 所有文件路径都在基准目录范围内
/// - `Err(String)`: 校验失败，错误信息描述具体原因
pub fn validate_extracted_paths(base_dir: &str) -> Result<(), String> {
    let base_canonical = std::fs::canonicalize(base_dir)
        .map_err(|e| format!("无法解析基准目录 {}: {}", base_dir, e))?;
    validate_dir_recursive(&base_canonical, &base_canonical, 0)
}

/// 递归校验目录下所有文件路径都在基准目录范围内
///
/// # 参数
/// - `dir`: 当前正在校验的目录
/// - `base`: 基准目录（所有文件路径必须在此目录内）
/// - `depth`: 当前递归深度（防止恶意嵌套目录导致栈溢出）
///
/// # 返回
/// - `Ok(())`: 所有文件路径都在基准目录范围内
/// - `Err(String)`: 校验失败，错误信息描述具体原因
pub fn validate_dir_recursive(dir: &Path, base: &Path, depth: usize) -> Result<(), String> {
    if depth >= MAX_RECURSION_DEPTH {
        return Err(format!(
            "递归深度超过上限 {}，可能存在恶意嵌套目录",
            MAX_RECURSION_DEPTH
        ));
    }
    for entry in std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();
        // canonicalize 解析符号链接，防止通过符号链接逃逸
        let canonical = std::fs::canonicalize(&path)
            .map_err(|e| format!("解析路径失败 {:?}: {}", path, e))?;
        if !canonical.starts_with(base) {
            return Err(format!(
                "检测到路径穿越攻击：文件 {:?} 不在安全目录范围内",
                canonical
            ));
        }
        // 如果是目录，递归校验（深度 +1）
        if canonical.is_dir() {
            validate_dir_recursive(&canonical, base, depth + 1)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试正常目录校验通过
    #[test]
    fn test_validate_extracted_paths_normal() {
        let temp = std::env::temp_dir().join(format!(
            "bingxi_test_path_validator_normal_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("file1.txt"), "content").unwrap();
        std::fs::create_dir_all(temp.join("subdir")).unwrap();
        std::fs::write(temp.join("subdir/file2.txt"), "content").unwrap();

        let result = validate_extracted_paths(temp.to_str().unwrap());
        assert!(result.is_ok(), "正常目录应校验通过");

        std::fs::remove_dir_all(&temp).ok();
    }

    /// 测试不存在的目录返回错误
    #[test]
    fn test_validate_extracted_paths_nonexistent() {
        let result = validate_extracted_paths("/nonexistent/path/that/should/not/exist");
        assert!(result.is_err(), "不存在的目录应返回错误");
    }

    /// 测试递归深度上限
    #[test]
    fn test_validate_dir_recursive_depth_limit() {
        let temp = std::env::temp_dir().join(format!(
            "bingxi_test_path_validator_depth_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        let result = validate_dir_recursive(&temp, &temp, MAX_RECURSION_DEPTH);
        assert!(result.is_err(), "达到深度上限应返回错误");
        std::fs::remove_dir_all(&temp).ok();
    }

    /// 测试超过深度上限（depth + 1）
    #[test]
    fn test_validate_dir_recursive_exceed_depth() {
        let temp = std::env::temp_dir().join(format!(
            "bingxi_test_path_validator_exceed_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp).unwrap();
        let result = validate_dir_recursive(&temp, &temp, MAX_RECURSION_DEPTH + 1);
        assert!(result.is_err(), "超过深度上限应返回错误");
        std::fs::remove_dir_all(&temp).ok();
    }
}
