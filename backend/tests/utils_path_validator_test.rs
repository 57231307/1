    use super::*;
#[cfg(test)]
mod tests {

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