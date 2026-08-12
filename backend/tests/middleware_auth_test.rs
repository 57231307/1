#[cfg(test)]
mod tests {
use bingxi_backend::middleware::auth::*;


    /// 低危 #4 修复：测试 Authorization 头脱敏（正常长度）
    #[test]
    fn test_mask_auth_header_normal_length() {
        let token = "Bearer abcdef1234567890.thisisafakesignaturevalue";
        let masked = mask_auth_header(token);
        // 保留前 12 字符
        assert!(masked.starts_with("Bearer abcd"), "应保留前 12 字符前缀");
        // 含长度信息
        assert!(masked.contains("(len="), "应包含原始长度信息");
        // 完整 token 不在脱敏结果中
        assert!(
            !masked.contains("thisisafakesignaturevalue"),
            "完整 token 不应出现在脱敏结果中"
        );
    }

    /// 低危 #4 修复：测试 Authorization 头脱敏（短 header）
    #[test]
    fn test_mask_auth_header_short() {
        let short = "abc";
        let masked = mask_auth_header(short);
        assert_eq!(masked, "***redacted***(len=3)");
    }

    /// 低危 #4 修复：测试 Authorization 头脱敏（边界 = 12 字符）
    #[test]
    fn test_mask_auth_header_boundary() {
        // "Bearer xxxxx" = 12 字符（B-e-a-r-e-r- -x-x-x-x-x），正好等于 PREFIX_KEEP
        // 走 if 分支，不暴露任何前缀字符
        let boundary = "Bearer xxxxx";
        let masked = mask_auth_header(boundary);
        assert_eq!(masked, "***redacted***(len=12)");
    }

    /// 低危 #4 修复：测试用户名脱敏（长用户名）
    #[test]
    fn test_mask_username_long() {
        let masked = mask_username("admin_user");
        assert_eq!(masked, "ad***", "长用户名应保留前 2 字符 + ***");
    }

    /// 低危 #4 修复：测试用户名脱敏（短用户名）
    #[test]
    fn test_mask_username_short() {
        assert_eq!(mask_username("ab"), "***");
        assert_eq!(mask_username("a"), "***");
        assert_eq!(mask_username(""), "***");
    }

    /// 低危 #4 修复：测试中文用户名脱敏（按字符而非字节截断）
    #[test]
    fn test_mask_username_chinese() {
        // 中文字符 1 个 = 3 字节，chars() 按 Unicode 字符截断
        // "管理员" = 3 字符 > 2 字符阈值，走 else 分支保留前 2 字符 → "管理***"
        // 关键：不能按字节截断（`&username[..6]` 在中文上会 panic at boundary）
        let masked = mask_username("管理员");
        assert_eq!(masked, "管理***", "中文用户名应按字符截断，保留前 2 字符");
    }
}