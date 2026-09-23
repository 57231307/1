//! 查询参数空串归一化 - 回归测试
//!
//! 锁定「空串查询参数在反序列化边界视为未提供」这一语义：未填筛选项以 `?status=` /
//! `?keyword=` 空串提交时，若被反序列化成 Some("") 会生成 `WHERE col = ''` 恒 0 行。
//! 覆盖三条路径（空串 / 非空串 / 缺省），并锁定边界中间件对 query 空值键的剔除。

use bingxi_backend::utils::query_params::{
    empty_str_as_none, is_empty_query_value, strip_empty_query_values,
};
use serde::Deserialize;

/// 采购列表查询 DTO 的最小同型体：status/keyword 用字段级反序列化器保护
/// （生产代码里由 OrderQueryParams + 边界中间件双重覆盖）。
#[derive(Debug, Deserialize)]
struct FilterQuery {
    #[allow(dead_code)]
    page: Option<u64>,
    #[serde(default, deserialize_with = "empty_str_as_none")]
    status: Option<String>,
    #[serde(default, deserialize_with = "empty_str_as_none")]
    keyword: Option<String>,
}

/// 路径一：空串参数不产生过滤（反序列化为 None）。
#[test]
fn empty_string_param_does_not_filter() {
    let q: FilterQuery = serde_json::from_str(r#"{"page":1,"status":"","keyword":""}"#).unwrap();
    assert_eq!(q.status, None, "空串 status 必须归一为 None");
    assert_eq!(q.keyword, None, "空串 keyword 必须归一为 None");
}

/// 路径二：非空串参数产生过滤（原值透传，不裁剪空格以外的内容）。
#[test]
fn non_empty_string_param_filters() {
    let q: FilterQuery =
        serde_json::from_str(r#"{"status":"approved","keyword":"PO-2026"}"#).unwrap();
    assert_eq!(q.status.as_deref(), Some("approved"));
    assert_eq!(q.keyword.as_deref(), Some("PO-2026"));
}

/// 路径三：缺省参数不产生过滤（键缺失 → None）。
#[test]
fn absent_param_does_not_filter() {
    let q: FilterQuery = serde_json::from_str(r#"{"page":1}"#).unwrap();
    assert_eq!(q.status, None);
    assert_eq!(q.keyword, None);
}

/// 纯空白等同于未提供；含内部空格的合法值保持原样。
#[test]
fn whitespace_only_is_empty_but_internal_space_preserved() {
    let q: FilterQuery = serde_json::from_str(r#"{"status":"   ","keyword":"a b"}"#).unwrap();
    assert_eq!(q.status, None);
    assert_eq!(q.keyword.as_deref(), Some("a b"));
    assert!(is_empty_query_value("  \t "));
    assert!(!is_empty_query_value("x"));
}

/// 边界中间件：真实前端采购页 URL `page=1&page_size=20&keyword=&status=` 被归一化为
/// 不含 status/keyword 两键，从而让下游反序列化得到 None。
#[test]
fn middleware_strips_empty_query_values() {
    let (normalized, changed) = strip_empty_query_values("page=1&page_size=20&keyword=&status=");
    assert!(changed);
    assert_eq!(normalized, "page=1&page_size=20");

    let (normalized_all_empty, changed) = strip_empty_query_values("status=&keyword=");
    assert!(changed);
    assert_eq!(normalized_all_empty, "", "全空 query 归一为空串");
}

/// 边界中间件：不含空值的常规请求零改动透传（不触发重新编码）。
#[test]
fn middleware_passes_through_non_empty_query() {
    let raw = "status=approved&page=2";
    let (normalized, changed) = strip_empty_query_values(raw);
    assert!(!changed, "无空值参数不应标记为已改动");
    assert_eq!(normalized, raw);
}

/// 边界中间件：非空值原样保留，仅剔除空值键（含保留同名多值中的非空项）。
#[test]
fn middleware_preserves_non_empty_pairs() {
    let (normalized, changed) = strip_empty_query_values("keyword=&status=draft&tag=a&tag=");
    assert!(changed);
    // 顺序与编码由 form_urlencoded 决定，逐对断言更稳健
    let pairs: Vec<(String, String)> = form_urlencoded::parse(normalized.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let has = |k: &str, v: &str| pairs.iter().any(|(a, b)| a == k && b == v);
    assert!(has("status", "draft"));
    assert!(!has("keyword", ""));
    assert!(has("tag", "a"));
    assert!(!pairs.iter().any(|(_, v)| v.is_empty()));
}
