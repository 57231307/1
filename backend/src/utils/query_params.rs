//! 查询参数边界归一化：把「空串查询参数」在 HTTP 边界一次性视为「未提供」。
//!
//! 业务背景：列表页未填写的筛选项（状态/关键字/供应商/单据号等）会以
//! `?status=`、`?keyword=` 形态提交。axum 的 `Query` 提取器把空串反序列化成
//! `Some("")`，service 层 `if let Some(v) = ...` 于是生成 `WHERE col = ''`，
//! 对状态/编号这类取值恒 0 行 → 页面「有数据但列表空」。
//!
//! 治本点：缺失键与空串应在「反序列化边界」收敛为同一语义（None）。本模块提供
//! 两个入口，语义一致：
//! - [`empty_str_as_none`] / [`empty_str_vec_as_none`]：字段级 serde 反序列化器，
//!   供查询 DTO 显式标注（首选写法）；
//! - [`normalize_empty_query_params`]：全量边界中间件，在 handler 之前剔除空值 query
//!   键，一次性覆盖全部查询 DTO（含未来新增），避免逐 service 手写 `!s.is_empty()`
//!   这种可漂移的重复实现。
//!
//! 「空」的判定与项目既有正确实现（`inventory_stock_service.rs` 用 `!s.is_empty()`）对齐，
//! 并进一步把纯空白视为未提供（`col = '   '` 同样恒 0 行）；非空值原样保留、不裁剪，
//! 以免改变合法取值。

use std::borrow::Cow;

use axum::{extract::Request, http::Uri, middleware::Next, response::Response};
use serde::Deserialize;

/// 判断某个 query 值是否应被视为「未提供」（空串或纯空白）。
pub fn is_empty_query_value(value: &str) -> bool {
    value.trim().is_empty()
}

/// 从原始 query 串中剔除空值键。
///
/// 返回 `(归一化后的 query, 是否发生改动)`。无空值时原样返回、`changed=false`，
/// 保证不含空值参数的常规请求零开销透传、不被重新编码。
pub fn strip_empty_query_values(raw: &str) -> (String, bool) {
    let mut had_empty = false;
    let mut pairs = Vec::new();
    for (key, value) in form_urlencoded::parse(raw.as_bytes()) {
        if is_empty_query_value(&value) {
            had_empty = true;
        } else {
            pairs.push((key.into_owned(), value.into_owned()));
        }
    }
    if !had_empty {
        return (raw.to_string(), false);
    }
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    // form_urlencoded 1.2.2 的 Serializer 只有 append_pair / extend_pairs（没有 pair/push_pair）。
    serializer.extend_pairs(pairs);
    (serializer.finish(), true)
}

/// 中间件：在请求进入路由/handler 之前，把只含空值筛选项的 query 键剔除并重写请求 URI。
pub async fn normalize_empty_query_params(mut req: Request, next: Next) -> Response {
    let Some(raw) = req.uri().query().map(str::to_string) else {
        // 无 query 的请求（含 POST/PUT/DELETE 与不带筛选的 GET）零改动透传。
        return next.run(req).await;
    };
    let (normalized, changed) = strip_empty_query_values(&raw);
    if changed {
        let path = req.uri().path();
        let rewritten = if normalized.is_empty() {
            path.to_string()
        } else {
            format!("{path}?{normalized}")
        };
        // rewritten 由「合法 path（原 URI 已编码）」+「form_urlencoded 输出（纯合法 percent 编码）」
        // 拼成，必然是合法 URI；解析失败仅可能是编码逻辑本身回归，故显式 panic 暴露而非静默回退掩盖。
        let uri: Uri = rewritten
            .parse()
            .expect("归一化后的 query 应为合法 URI（form_urlencoded 输出保证）");
        *req.uri_mut() = uri;
    }
    next.run(req).await
}

/// 字段级 serde 反序列化器：`Option<String>` 空串/纯空白 → `None`，非空 → `Some(原值)`。
///
/// 用法（查询 DTO 字段，`Option` 缺省天然为 None，故 `default` 可省，保留以自文档化）：
/// ```ignore
/// #[serde(default, deserialize_with = "crate::utils::query_params::empty_str_as_none")]
/// pub status: Option<String>,
/// ```
pub fn empty_str_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<Cow<'de, str>>::deserialize(deserializer)?;
    Ok(match raw {
        Some(v) if is_empty_query_value(&v) => None,
        Some(v) => Some(v.into_owned()),
        None => None,
    })
}

/// 字段级 serde 反序列化器：`Option<Vec<String>>`（如逗号分隔多值）逐项剔除空值，
/// 过滤后为空集时归一为 `None`，使「未填多选筛选项」与缺省同义。
pub fn empty_str_vec_as_none<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<Vec<Cow<'de, str>>>::deserialize(deserializer)?;
    let Some(items) = raw else {
        return Ok(None);
    };
    let kept: Vec<String> = items
        .into_iter()
        .filter(|v| !is_empty_query_value(v))
        .map(Cow::into_owned)
        .collect();
    // 过滤后为空集与「未提供多选筛选项」同义 → None，绝不退化成 Some([])（Some([]) 会被
    // service 层的 `is_some()` 判成有效过滤）。
    Ok(Some(kept).filter(|v| !v.is_empty()))
}
