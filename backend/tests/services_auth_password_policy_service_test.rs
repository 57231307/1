#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mmcl_qmmtg() {
        let svc = PasswordPolicyService::new();
        let result = svc.validate("MyP@ssw0rd_2026!").await;
        assert!(result.is_valid, "强密码应通过：{:?}", result.errors);
    }

    #[tokio::test]
    async fn test_mmcl_rmmjj() {
        let svc = PasswordPolicyService::new();
        let result = svc.validate("123").await;
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_mmls_fzfy() {
        let mut history = PasswordHistory::new(5);
        history.push("hash1".to_string());
        history.push("hash2".to_string());
        assert!(history.contains("hash1"));
        assert!(!history.contains("hash3"));
    }

    #[test]
    fn test_mmls_rlsx() {
        let mut history = PasswordHistory::new(3);
        history.push("h1".to_string());
        history.push("h2".to_string());
        history.push("h3".to_string());
        history.push("h4".to_string());
        assert!(!history.contains("h1")); // 被淘汰
        assert!(history.contains("h4"));
    }

    #[test]
    fn test_mmgq() {
        let svc = PasswordPolicyService::new();
        let old = chrono::Utc::now() - chrono::Duration::days(100);
        assert!(svc.is_expired(old));
        let recent = chrono::Utc::now() - chrono::Duration::days(30);
        assert!(!svc.is_expired(recent));
    }

    #[test]
    fn test_cjmmsb() {
        assert!(is_common_password("Password"));
        assert!(is_common_password("123456"));
        assert!(!is_common_password("X7#mK9pQ@2vL"));
    }

    #[test]
    fn test_mmbhyhmpd() {
        assert!(contains_username_fragment("zhangsan@2026", "zhangsan"));
        assert!(!contains_username_fragment("X7#mK9pQ@2vL", "zhangsan"));
    }

    #[test]
    fn test_mmhmdgj() {
        let blacklist = build_password_blacklist();
        assert!(blacklist.contains("password"));
        assert!(blacklist.contains("123456"));
        assert!(!blacklist.contains("X7#mK9pQ@2vL"));
    }
}