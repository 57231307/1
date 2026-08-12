    use bingxi_backend::services::auth::password_policy_service::*;
    use bingxi_backend::utils::admin_checker::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_cache_entry_expiration() {
        let entry = AdminCacheEntry::new(true, 5);
        assert!(!entry.is_expired());

        let expired_entry = AdminCacheEntry {
            is_admin: true,
            expires_at: Utc::now() - Duration::minutes(1),
        };
        assert!(expired_entry.is_expired());
    }

    #[test]
    fn test_clear_admin_role_cache() {
        // 插入测试数据
        ADMIN_ROLE_CACHE.insert(1, AdminCacheEntry::new(true, 5));
        ADMIN_ROLE_CACHE.insert(2, AdminCacheEntry::new(false, 5));

        // 清除特定角色
        clear_admin_role_cache(Some(1));
        assert!(!ADMIN_ROLE_CACHE.contains_key(&1));
        assert!(ADMIN_ROLE_CACHE.contains_key(&2));

        // 清除所有
        clear_admin_role_cache(None);
        assert!(ADMIN_ROLE_CACHE.is_empty());
    }

    // ===== V15 P1-14.11-B：is_system 不注入 *:* 的单元测试 =====

    /// 验证 ADMIN_ROLE_CODE 常量值为 "admin"
    #[test]
    fn test_admin_role_code_constant_value() {
        assert_eq!(ADMIN_ROLE_CODE, "admin");
    }

    /// 验证 AUDITOR_ROLE_CODE 常量值为 "auditor"
    #[test]
    fn test_auditor_role_code_constant_value() {
        assert_eq!(AUDITOR_ROLE_CODE, "auditor");
    }

    /// 验证 MANAGER_ROLE_CODE 常量值为 "manager"
    #[test]
    fn test_manager_role_code_constant_value() {
        assert_eq!(MANAGER_ROLE_CODE, "manager");
    }

    /// V15 P1-14.11-B：admin 角色（code="admin"）应被识别为管理员
    #[test]
    fn test_admin_role_code_matches_admin() {
        assert_eq!("admin", ADMIN_ROLE_CODE);
        assert!("admin" == ADMIN_ROLE_CODE);
    }

    /// V15 P1-14.11-B：manager 角色不应被识别为管理员（即使 is_system=true）
    #[test]
    fn test_manager_role_not_admin_even_if_system() {
        let manager_code = "manager";
        assert!(manager_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(manager_code));
    }

    /// V15 P1-14.11-B：operator 角色不应被识别为管理员（即使 is_system=true）
    #[test]
    fn test_operator_role_not_admin_even_if_system() {
        let operator_code = "operator";
        assert!(operator_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(operator_code));
    }

    /// V15 P1-14.11-B：customer 角色不应被识别为管理员
    #[test]
    fn test_customer_role_not_admin() {
        let customer_code = "customer";
        assert!(customer_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(customer_code));
    }

    /// V15 P1-14.11-B：auditor 角色不应被识别为管理员（职责分离）
    #[test]
    fn test_auditor_role_not_admin() {
        let auditor_code = "auditor";
        assert!(auditor_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(auditor_code));
    }

    /// V15 P1-14.11-B：空字符串和未知角色不应被识别为管理员
    #[test]
    fn test_unknown_role_not_admin() {
        assert!(!should_be_admin_by_code(""));
        assert!(!should_be_admin_by_code("unknown"));
        assert!(!should_be_admin_by_code("ADMIN"));
        assert!(!should_be_admin_by_code("Admin"));
    }
}