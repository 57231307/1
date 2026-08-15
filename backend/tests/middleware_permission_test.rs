use axum::http::Method;
use bingxi_backend::middleware::permission::*;
use bingxi_backend::models::financial_analysis_result::*;
use bingxi_backend::models::role_permission;
use bingxi_backend::services::auth::password_policy_service::*;
use bingxi_backend::services::report::*;
use chrono::Duration;
use chrono::Utc;
use std::sync::Arc;

/// 构造测试用权限模型
fn make_permission(
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> role_permission::Model {
    role_permission::Model {
        id: 1,
        role_id: 1,
        resource_type: resource_type.to_string(),
        resource_id,
        action: action.to_string(),
        allowed: true,
        permission_code: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ===== extract_resource_info 测试 =====

#[test]
fn test_extract_resource_info_bzljw_id() {
    let (rt, rid) = extract_resource_info("/api/v1/erp/users");
    assert_eq!(rt, "users");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_bzljd_id() {
    let (rt, rid) = extract_resource_info("/api/v1/erp/users/123");
    assert_eq!(rt, "users");
    assert_eq!(rid, Some(123));
}

#[test]
fn test_extract_resource_info_mkqzljw_id() {
    let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders");
    assert_eq!(rt, "orders");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_mkqzljd_id() {
    let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/456");
    assert_eq!(rt, "orders");
    assert_eq!(rid, Some(456));
}

#[test]
fn test_extract_resource_info_qtljd_id_hdz() {
    let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/123/approve");
    assert_eq!(rt, "orders");
    assert_eq!(rid, Some(123));
}

#[test]
fn test_extract_resource_info_f_api_lj() {
    let (rt, rid) = extract_resource_info("/health");
    assert_eq!(rt, "unknown");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_dlj() {
    let (rt, rid) = extract_resource_info("/api/v1");
    assert_eq!(rt, "unknown");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_klj() {
    let (rt, rid) = extract_resource_info("/");
    assert_eq!(rt, "unknown");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_dzdbwpw_id() {
    // V15 P0-S20 新增：动作关键字不应被误认为资源ID
    let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/approve");
    assert_eq!(rt, "orders");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_scymkqz() {
    // V15 P0-S21 新增：production 模块前缀应正确提取资源
    let (rt, rid) = extract_resource_info("/api/v1/erp/production/dye-batches/789");
    assert_eq!(rt, "dye-batches");
    assert_eq!(rid, Some(789));
}

#[test]
fn test_extract_resource_info_cgyxzpx() {
    // V15 P0-S21 修正：purchase（单数）应正确识别为模块前缀
    // V15 P1-14.4-C：purchase/orders 消歧为 purchase-orders（与权限定义对齐）
    let (rt, rid) = extract_resource_info("/api/v1/erp/purchase/orders");
    assert_eq!(rt, "purchase-orders");
    assert_eq!(rid, None);
}

#[test]
fn test_extract_resource_info_cgyxqql() {
    // V15 P1-14.4-C：采购域资源消歧映射
    let (rt, _) = extract_resource_info("/api/v1/erp/purchase/orders/123/approve");
    assert_eq!(rt, "purchase-orders");
    let (rt, _) = extract_resource_info("/api/v1/erp/purchase/returns/1");
    assert_eq!(rt, "purchase-returns");
    let (rt, _) = extract_resource_info("/api/v1/erp/purchase/receipts/1");
    assert_eq!(rt, "purchase-receipts");
    let (rt, _) = extract_resource_info("/api/v1/erp/purchase/contracts/1");
    assert_eq!(rt, "purchase-contracts");
    let (rt, _) = extract_resource_info("/api/v1/erp/purchase/prices/1");
    assert_eq!(rt, "purchase-prices");
}

#[test]
fn test_extract_resource_info_xsyxqql() {
    // V15 P1-14.4-C：销售域资源消歧映射（orders 保留原名）
    let (rt, _) = extract_resource_info("/api/v1/erp/sales/orders/123/approve");
    assert_eq!(rt, "orders");
    let (rt, _) = extract_resource_info("/api/v1/erp/sales/returns/1");
    assert_eq!(rt, "sales-returns");
    let (rt, _) = extract_resource_info("/api/v1/erp/sales/contracts/1");
    assert_eq!(rt, "sales-contracts");
    let (rt, _) = extract_resource_info("/api/v1/erp/sales/prices/1");
    assert_eq!(rt, "sales-prices");
}

// ===== extract_segment3 测试 =====

#[test]
fn test_extract_segment3_bzlj() {
    assert_eq!(extract_segment3("/api/v1/erp/users"), Some("users"));
    assert_eq!(extract_segment3("/api/v1/erp/sales/orders"), Some("sales"));
    assert_eq!(
        extract_segment3("/api/v1/erp/production/dye-batches"),
        Some("production")
    );
}

#[test]
fn test_extract_segment3_f_api_ljfh_none() {
    assert_eq!(extract_segment3("/health"), None);
    assert_eq!(extract_segment3("/api/v1"), None);
    assert_eq!(extract_segment3("/"), None);
}

// ===== extract_action_from_path 测试 =====

#[test]
fn test_extract_action_from_path_approve_dz() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/sales/orders/123/approve"),
        Some("approve".to_string())
    );
}

#[test]
fn test_extract_action_from_path_export_dz() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/users/export"),
        Some("export".to_string())
    );
}

#[test]
fn test_extract_action_from_path_print_dz() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/orders/456/print"),
        Some("print".to_string())
    );
}

#[test]
fn test_extract_action_from_path_reject_dz() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/orders/789/reject"),
        Some("reject".to_string())
    );
}

#[test]
fn test_extract_action_from_path_wdzfh_none() {
    assert_eq!(extract_action_from_path("/api/v1/erp/users"), None);
    assert_eq!(extract_action_from_path("/api/v1/erp/users/123"), None);
}

#[test]
fn test_extract_action_from_path_fdzgjzfh_none() {
    // 非动作关键字不应被识别为动作
    assert_eq!(extract_action_from_path("/api/v1/erp/users/profile"), None);
}

// ===== method_to_action 测试 =====

#[test]
fn test_method_to_action_get_ys_read() {
    assert_eq!(method_to_action(&Method::GET), "read");
}

#[test]
fn test_method_to_action_post_ys_create() {
    assert_eq!(method_to_action(&Method::POST), "create");
}

#[test]
fn test_method_to_action_put_ys_update() {
    assert_eq!(method_to_action(&Method::PUT), "update");
}

#[test]
fn test_method_to_action_patch_ys_update() {
    assert_eq!(method_to_action(&Method::PATCH), "update");
}

#[test]
fn test_method_to_action_delete_ys_delete() {
    assert_eq!(method_to_action(&Method::DELETE), "delete");
}

#[test]
fn test_method_to_action_wzffys_read() {
    // OPTIONS 等未明确映射的方法默认为 read
    assert_eq!(method_to_action(&Method::OPTIONS), "read");
}

// ===== CacheEntry 测试 =====

#[test]
fn test_cache_entry_xjwgq() {
    let entry = CacheEntry::new(true, Duration::minutes(5));
    assert!(!entry.is_expired());
    assert!(entry.payload);
}

#[test]
fn test_cache_entry_ygq() {
    // 构造一个已过期的缓存项（过期时间为当前时间减 1 分钟）
    let entry = CacheEntry {
        payload: false,
        expires_at: Utc::now() - Duration::minutes(1),
    };
    assert!(entry.is_expired());
}

// ===== invalidate_permission_cache 测试 =====

#[test]
fn test_invalidate_permission_cache_yczdjs() {
    // 插入缓存条目
    PERMISSION_CACHE.insert(
        9991,
        CacheEntry {
            payload: Arc::new(vec![]),
            expires_at: Utc::now() + Duration::minutes(5),
        },
    );
    assert!(PERMISSION_CACHE.contains_key(&9991));

    // 失效指定角色缓存
    invalidate_permission_cache(9991);
    assert!(!PERMISSION_CACHE.contains_key(&9991));
}

#[test]
fn test_invalidate_permission_cache_bczjsbbc() {
    // 失效不存在的角色缓存不应 panic
    invalidate_permission_cache(99999);

    // 验证函数执行成功（不抛异常即为通过）
    assert!(true, "invalidate_permission_cache 应正常执行");
}

#[test]
fn test_invalidate_all_permission_cache_qkqb() {
    // 插入多个缓存条目
    PERMISSION_CACHE.insert(
        9992,
        CacheEntry {
            payload: Arc::new(vec![]),
            expires_at: Utc::now() + Duration::minutes(5),
        },
    );
    PERMISSION_CACHE.insert(
        9993,
        CacheEntry {
            payload: Arc::new(vec![]),
            expires_at: Utc::now() + Duration::minutes(5),
        },
    );
    assert!(PERMISSION_CACHE.contains_key(&9992));
    assert!(PERMISSION_CACHE.contains_key(&9993));

    // 清空全部
    invalidate_all_permission_cache();
    assert!(PERMISSION_CACHE.is_empty());
}

// ===== V15 P1-14.11-C：缓存失效生命周期测试（insert→invalidate→reload→expiry 完整链路）=====

/// 构造带权限数据的缓存条目，用于生命周期测试
fn make_cache_entry(
    permissions: Vec<role_permission::Model>,
    ttl_minutes: i64,
) -> CacheEntry<Arc<Vec<role_permission::Model>>> {
    CacheEntry {
        payload: Arc::new(permissions),
        expires_at: Utc::now() + Duration::minutes(ttl_minutes),
    }
}

/// 构造已过期的缓存条目
fn make_expired_cache_entry(
    permissions: Vec<role_permission::Model>,
) -> CacheEntry<Arc<Vec<role_permission::Model>>> {
    CacheEntry {
        payload: Arc::new(permissions),
        expires_at: Utc::now() - Duration::minutes(1),
    }
}

/// 生命周期场景 1：insert → invalidate → reload 完整链路
#[test]
fn test_lifecycle_insert_invalidate_reload() {
    let role_id = 88001;
    // 清理可能的残留
    PERMISSION_CACHE.remove(&role_id);

    // 1. insert：插入权限缓存
    let perms_v1 = vec![make_permission("users", None, "read")];
    PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v1.clone(), 5));
    assert!(
        PERMISSION_CACHE.contains_key(&role_id),
        "insert 后缓存应存在"
    );
    assert_eq!(
        PERMISSION_CACHE.get(&role_id).unwrap().payload.len(),
        1,
        "缓存应含 1 条权限"
    );

    // 2. invalidate：失效缓存
    invalidate_permission_cache(role_id);
    assert!(
        !PERMISSION_CACHE.contains_key(&role_id),
        "invalidate 后缓存应被移除"
    );

    // 3. reload：重新加载（模拟 check_permission 重新查询 DB 后回填缓存）
    let perms_v2 = vec![
        make_permission("users", None, "read"),
        make_permission("users", None, "create"),
    ];
    PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));
    assert!(
        PERMISSION_CACHE.contains_key(&role_id),
        "reload 后缓存应重新存在"
    );
    assert_eq!(
        PERMISSION_CACHE.get(&role_id).unwrap().payload.len(),
        2,
        "reload 后应含 2 条权限（数据已更新）"
    );

    // 清理
    invalidate_permission_cache(role_id);
}

/// 生命周期场景 2：insert → expiry → reload 过期触发重新加载链路
#[test]
fn test_lifecycle_insert_expiry_reload() {
    let role_id = 88002;
    PERMISSION_CACHE.remove(&role_id);

    // 1. insert：插入已过期的缓存条目（模拟 TTL 到期）
    let perms_v1 = vec![make_permission("orders", None, "read")];
    PERMISSION_CACHE.insert(role_id, make_expired_cache_entry(perms_v1.clone()));
    assert!(PERMISSION_CACHE.contains_key(&role_id), "缓存条目存在");
    assert!(
        PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
        "缓存条目应已过期"
    );

    // 2. expiry：过期后应被 is_expired 识别（模拟 check_permission 中的过期清理逻辑）
    let cached = PERMISSION_CACHE.get(&role_id).unwrap();
    if cached.is_expired() {
        drop(cached);
        PERMISSION_CACHE.remove(&role_id);
    }
    assert!(!PERMISSION_CACHE.contains_key(&role_id), "过期条目应被移除");

    // 3. reload：重新加载新数据
    let perms_v2 = vec![
        make_permission("orders", None, "read"),
        make_permission("orders", None, "export"),
    ];
    PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));
    assert!(
        !PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
        "新条目不应过期"
    );
    assert_eq!(
        PERMISSION_CACHE.get(&role_id).unwrap().payload.len(),
        2,
        "reload 后应含 2 条权限"
    );

    // 清理
    invalidate_permission_cache(role_id);
}

/// 生命周期场景 3：完整链路 insert → read(hit) → invalidate → read(miss) → reload → read(hit) → expiry → read(miss)
#[test]
fn test_lifecycle_complete_chain() {
    let role_id = 88003;
    PERMISSION_CACHE.remove(&role_id);

    // 1. insert
    let perms = vec![make_permission("products", None, "read")];
    PERMISSION_CACHE.insert(role_id, make_cache_entry(perms.clone(), 5));

    // 2. read(hit)：缓存命中
    assert!(PERMISSION_CACHE.contains_key(&role_id), "缓存应命中");
    let hit_data = PERMISSION_CACHE.get(&role_id).unwrap().payload.clone();
    assert_eq!(hit_data.len(), 1, "命中数据应含 1 条权限");
    drop(hit_data);

    // 3. invalidate：失效缓存
    invalidate_permission_cache(role_id);

    // 4. read(miss)：缓存未命中
    assert!(!PERMISSION_CACHE.contains_key(&role_id), "缓存应未命中");

    // 5. reload：重新加载（含新权限）
    let perms_v2 = vec![
        make_permission("products", None, "read"),
        make_permission("products", None, "create"),
        make_permission("products", None, "update"),
    ];
    PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));

    // 6. read(hit)：重新命中，数据已更新
    assert!(PERMISSION_CACHE.contains_key(&role_id), "reload 后应命中");
    assert_eq!(
        PERMISSION_CACHE.get(&role_id).unwrap().payload.len(),
        3,
        "应含 3 条权限（数据已更新）"
    );

    // 7. expiry：模拟 TTL 到期（替换为已过期条目）
    PERMISSION_CACHE.insert(role_id, make_expired_cache_entry(perms_v2.clone()));
    assert!(
        PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
        "条目应已过期"
    );

    // 8. read(miss)：过期后视为未命中（模拟 check_permission 的过期检测逻辑）
    let is_miss = match PERMISSION_CACHE.get(&role_id) {
        Some(entry) => entry.is_expired(),
        None => true,
    };
    assert!(is_miss, "过期条目应视为未命中");

    // 清理
    invalidate_permission_cache(role_id);
}

/// 生命周期场景 4：多角色并发缓存生命周期隔离
#[test]
fn test_lifecycle_multi_role_isolation() {
    let role_a = 88004;
    let role_b = 88005;
    PERMISSION_CACHE.remove(&role_a);
    PERMISSION_CACHE.remove(&role_b);

    // 两个角色同时缓存
    PERMISSION_CACHE.insert(
        role_a,
        make_cache_entry(vec![make_permission("a", None, "read")], 5),
    );
    PERMISSION_CACHE.insert(
        role_b,
        make_cache_entry(vec![make_permission("b", None, "read")], 5),
    );

    // 失效 role_a，role_b 不受影响
    invalidate_permission_cache(role_a);
    assert!(!PERMISSION_CACHE.contains_key(&role_a), "role_a 应被失效");
    assert!(PERMISSION_CACHE.contains_key(&role_b), "role_b 不应受影响");

    // reload role_a
    PERMISSION_CACHE.insert(
        role_a,
        make_cache_entry(vec![make_permission("a", None, "read")], 5),
    );
    assert!(
        PERMISSION_CACHE.contains_key(&role_a),
        "role_a reload 后应存在"
    );

    // 清理
    invalidate_permission_cache(role_a);
    invalidate_permission_cache(role_b);
}

/// 生命周期场景 5：invalidate_all 后所有角色缓存全清空，可重新加载
#[test]
fn test_lifecycle_invalidate_all_then_reload() {
    let role_ids = [88006, 88007, 88008];
    for &rid in &role_ids {
        PERMISSION_CACHE.remove(&rid);
    }

    // 插入多个角色缓存
    for &rid in &role_ids {
        PERMISSION_CACHE.insert(
            rid,
            make_cache_entry(vec![make_permission("x", None, "read")], 5),
        );
    }

    // invalidate_all 清空全部
    invalidate_all_permission_cache();
    for &rid in &role_ids {
        assert!(
            !PERMISSION_CACHE.contains_key(&rid),
            "角色 {} 应被清空",
            rid
        );
    }

    // 重新加载单个角色
    PERMISSION_CACHE.insert(
        role_ids[0],
        make_cache_entry(vec![make_permission("y", None, "read")], 5),
    );
    assert!(
        PERMISSION_CACHE.contains_key(&role_ids[0]),
        "reload 后应存在"
    );
    assert!(
        !PERMISSION_CACHE.contains_key(&role_ids[1]),
        "其他角色仍应被清空"
    );

    // 清理
    invalidate_all_permission_cache();
}

// ===== extract_action_from_query 测试（V15 P0-S10）=====

#[test]
fn test_extract_action_from_query_print_dz() {
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=print".parse().unwrap();
    assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
}

#[test]
fn test_extract_action_from_query_export_dz() {
    let uri: axum::http::Uri = "/api/v1/erp/inventory/stocks?action=export"
        .parse()
        .unwrap();
    assert_eq!(extract_action_from_query(&uri), Some("export".to_string()));
}

#[test]
fn test_extract_action_from_query_download_dz() {
    let uri: axum::http::Uri = "/api/v1/erp/reports/finance?action=download"
        .parse()
        .unwrap();
    assert_eq!(
        extract_action_from_query(&uri),
        Some("download".to_string())
    );
}

#[test]
fn test_extract_action_from_query_w_action_csfh_none() {
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders?page=1".parse().unwrap();
    assert_eq!(extract_action_from_query(&uri), None);
}

#[test]
fn test_extract_action_from_query_bmdwdzfh_none() {
    // action=read 不在白名单中，防止客户端绕过权限
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=read".parse().unwrap();
    assert_eq!(extract_action_from_query(&uri), None);
}

#[test]
fn test_extract_action_from_query_wcxzfcfh_none() {
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders".parse().unwrap();
    assert_eq!(extract_action_from_query(&uri), None);
}

#[test]
fn test_extract_action_from_query_dcssb_action() {
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders?page=1&action=print&format=pdf"
        .parse()
        .unwrap();
    assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
}

#[test]
fn test_extract_action_from_query_url_bmjm() {
    // %70%72%69%6e%74 = "print"
    let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=%70%72%69%6e%74"
        .parse()
        .unwrap();
    assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
}

// ===== matches_permission 测试（安全核心）=====

#[test]
fn test_matches_permission_lxbppfh_false() {
    let p = make_permission("users", None, "read");
    assert!(!matches_permission(&p, "orders", None, "read"));
}

#[test]
fn test_matches_permission_qbppw_id() {
    let p = make_permission("users", None, "read");
    assert!(matches_permission(&p, "users", None, "read"));
}

#[test]
fn test_matches_permission_action_tpfpp() {
    let p = make_permission("users", None, "*");
    assert!(matches_permission(&p, "users", None, "read"));
    assert!(matches_permission(&p, "users", None, "create"));
    assert!(matches_permission(&p, "users", None, "delete"));
}

#[test]
fn test_matches_permission_id_jqppxd() {
    let p = make_permission("users", Some(100), "read");
    assert!(matches_permission(&p, "users", Some(100), "read"));
}

#[test]
fn test_matches_permission_id_jqppbdfh_false() {
    // 垂直越权防护：权限 ID=100 不能访问 ID=200
    let p = make_permission("users", Some(100), "read");
    assert!(!matches_permission(&p, "users", Some(200), "read"));
}

#[test]
fn test_matches_permission_qxw_id_qqy_id_fh_false() {
    // M-6 修复点：权限 resource_id=None 不能匹配请求 resource_id=Some
    // 防止拥有全局权限的用户操作特定资源（应通过 action="*" 明确授予）
    let p = make_permission("users", None, "read");
    assert!(!matches_permission(&p, "users", Some(100), "read"));
}

#[test]
fn test_matches_permission_qxy_id_qqw_id_fh_false() {
    let p = make_permission("users", Some(100), "read");
    assert!(!matches_permission(&p, "users", None, "read"));
}

#[test]
fn test_matches_permission_action_bppqftpffh_false() {
    let p = make_permission("users", None, "read");
    assert!(!matches_permission(&p, "users", None, "delete"));
}

#[test]
fn test_matches_permission_tpfj_id_jqpp() {
    // action="*" + resource_id 精确匹配的组合
    let p = make_permission("users", Some(100), "*");
    assert!(matches_permission(&p, "users", Some(100), "update"));
    assert!(!matches_permission(&p, "users", Some(200), "update"));
}

// ===== V15 P2 14.11-F：通配符匹配测试（*:* / resource:* / *:action）=====

#[test]
fn test_matches_permission_super_wildcard_resource() {
    // resource_type="*" + action="*"：超级通配，匹配任意资源任意动作
    let p = make_permission("*", None, "*");
    assert!(matches_permission(&p, "users", None, "read"));
    assert!(matches_permission(&p, "orders", None, "delete"));
    assert!(matches_permission(&p, "inventory", Some(5), "update"));
}

#[test]
fn test_matches_permission_wildcard_resource_fixed_action() {
    // resource_type="*" + action="read"：资源通配，匹配任意资源指定动作
    let p = make_permission("*", None, "read");
    assert!(matches_permission(&p, "users", None, "read"));
    assert!(matches_permission(&p, "products", None, "read"));
    assert!(!matches_permission(&p, "users", None, "delete"));
}

#[test]
fn test_matches_permission_fixed_resource_wildcard_action() {
    // resource_type="users" + action="*"：动作通配，匹配指定资源任意动作
    let p = make_permission("users", None, "*");
    assert!(matches_permission(&p, "users", None, "create"));
    assert!(matches_permission(&p, "users", None, "delete"));
    assert!(!matches_permission(&p, "orders", None, "create"));
}

#[test]
fn test_matches_permission_wildcard_resource_still_requires_id_match() {
    // resource_type="*" 不豁免 resource_id 垂直越权防护
    let p = make_permission("*", Some(100), "read");
    assert!(matches_permission(&p, "users", Some(100), "read"));
    assert!(!matches_permission(&p, "users", Some(200), "read"));
}
