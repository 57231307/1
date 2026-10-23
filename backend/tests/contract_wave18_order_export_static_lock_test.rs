//! 订单导出取数腿与留痕面静态锁（不连库，纯源码切片判定）
//!
//! 功能：把「导出腿不得回到分页/封顶旧形态」钉到函数体粒度——列表腿 list_orders 的
//! paginate 是合法形态，全文匹配会误报，故所有禁词判定只作用于按函数名切出的函数体。
//! 调用方：CI backend 测试 target（非 db 组）；判据主体为 query.rs / delivery_ops/
//! export.rs / order_query.rs 的两条导出腿与两个 handler 的 export_orders。
//! 入参：无运行时入参；源码经 include_str! 读入，route-snapshot.txt 为 URL 权威侧。
//! 传给谁：只回本测试断言；审计路径锁把 handler 字面量与快照行逐字比对。
//! 存什么：不写库；禁词命中集合、21 列表头字面量序列与审计键名逐字节相等即全部判据。
//! 存哪里：读 `backend/src/services/po/order_ops/query.rs`、
//! `backend/src/services/so/order_query.rs`、
//! `backend/src/services/so/delivery_ops/export.rs`、两个 handler 与
//! `frontend/scripts/route-snapshot.txt`；旧形态夹具三件放仓库外交接目录（见文末），
//! 不落 backend/tests。
#![allow(
    clippy::too_many_lines,
    reason = "静态锁按判据分段，拆分会打散禁词表与探针的同处可读性"
)]

/// 导出腿函数体禁词：分页形态、固定一万页大小、旧调用点与丢弃服务端总数的旧变量名
const LEG_BANNED: &[&str] = &[
    ".paginate(",
    "paginate_with_total(",
    ".list_orders(",
    "10_000",
    "10000",
    " _total",
];

/// handler 函数体禁词：一万行阈值分支的常量、字面量与报错文案
const HANDLER_BANNED: &[&str] = &["10_000", "10000", "MAX_", "超过上限"];

const PO_QUERY: &str = include_str!("../src/services/po/order_ops/query.rs");
const SO_QUERY: &str = include_str!("../src/services/so/order_query.rs");
const SO_EXPORT: &str = include_str!("../src/services/so/delivery_ops/export.rs");
const PO_HANDLER: &str = include_str!("../src/handlers/purchase_order_handler.rs");
const SO_HANDLER: &str = include_str!("../src/handlers/sales_order_handler.rs");
const SNAPSHOT: &str = include_str!("../../frontend/scripts/route-snapshot.txt");

/// 从 `fn <name>` 起点切到下一个同层函数签名前，返回函数体文本；
/// 顶层 handler 无缩进，impl 内方法缩进 4，两种边界都要认。
fn fn_body(src: &str, name: &str) -> String {
    let start = src
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("活体控制：{name} 必须还在源码里"));
    let rest = &src[start..];
    let body_start = rest.find('\n').map(|i| i + 1).unwrap_or(0);
    let mut end = rest.len();
    for marker in [
        "\n    fn ",
        "\n    pub fn ",
        "\n    pub async fn ",
        "\n    async fn ",
        "\nfn ",
        "\npub fn ",
        "\npub async fn ",
        "\nasync fn ",
    ] {
        if let Some(i) = rest[body_start..].find(marker) {
            let pos = i + body_start;
            if pos < end {
                end = pos;
            }
        }
    }
    rest[..end].to_string()
}

/// 切片内命中的禁词清单（逐字子串匹配，不做大小写归一与模糊比对）
fn banned_hits(body: &str, banned: &[&str]) -> Vec<&'static str> {
    banned
        .iter()
        .copied()
        .filter(|b| body.contains(b))
        .collect()
}

/// 判据①（两腿同形）：四条导出取数函数体内禁词零命中
#[test]
fn export_leg_bodies_have_no_pagination_or_cap() {
    for (src, name) in [
        (PO_QUERY, "export_orders_to_xlsx"),
        (PO_QUERY, "list_all_orders_for_export"),
        (SO_EXPORT, "export_orders_to_xlsx"),
        (SO_QUERY, "list_all_orders_for_export"),
    ] {
        let body = fn_body(src, name);
        assert!(
            !body.is_empty(),
            "活体非空控制：{name} 的函数体必须能切出且非空"
        );
        let hits = banned_hits(&body, LEG_BANNED);
        assert!(
            hits.is_empty(),
            "导出腿 {name} 命中禁词 {hits:?}：分页或封顶形态回潮即判红"
        );
    }
}

/// 判据②（两腿同形·handler 面）：两个 export_orders 体内阈值分支禁词零命中
#[test]
fn export_handlers_have_no_row_cap_branch() {
    for (src, domain) in [(PO_HANDLER, "采购"), (SO_HANDLER, "销售")] {
        let body = fn_body(src, "export_orders");
        assert!(
            body.contains("WatermarkConfig") && body.contains("record_async"),
            "活体控制：{domain} export_orders 必须仍装配水印与异步审计留痕"
        );
        let hits = banned_hits(&body, HANDLER_BANNED);
        assert!(hits.is_empty(), "{domain} export_orders 命中禁词 {hits:?}");
    }
}

/// 判据③（审计留痕三列逐字）：request_path 字面量与 route-snapshot 真实 URL 逐字比对，
/// resource_type 为小写下划线形态，after_snapshot 键名 exported_rows 逐字在
#[test]
fn audit_trace_columns_match_route_snapshot_verbatim() {
    for (src, handler_key, want_path, want_restype) in [
        (
            PO_HANDLER,
            "purchase_order_handler::export_orders",
            "/api/v1/erp/purchase/orders/export",
            "purchase_order",
        ),
        (
            SO_HANDLER,
            "sales_order_handler::export_orders",
            "/api/v1/erp/sales/orders/export",
            "sales_order",
        ),
    ] {
        let body = fn_body(src, "export_orders");

        let anchor = "request_path: Some(\"";
        let at = body
            .find(anchor)
            .unwrap_or_else(|| panic!("{handler_key} 的审计事件必须带 request_path 字面量"));
        let rest = &body[at + anchor.len()..];
        let got_path = rest.split('"').next().unwrap_or_default();
        assert_eq!(
            got_path, want_path,
            "{handler_key} 留痕路径与期望字面量不逐字相等（挂载前缀单复数即判据）"
        );

        let snap_line = SNAPSHOT
            .lines()
            .find(|l| l.split('\t').nth(2) == Some(handler_key))
            .unwrap_or_else(|| panic!("route-snapshot 必须登记 {handler_key}"));
        let snap_url = snap_line.split('\t').nth(1).unwrap_or_default();
        assert_eq!(
            got_path, snap_url,
            "{handler_key} 留痕路径与 route-snapshot 真实 URL 不逐字相等"
        );

        assert!(
            body.contains(&format!(
                "resource_type: Some(\"{want_restype}\".to_string())"
            )),
            "{handler_key} 的 resource_type 必须逐字为 {want_restype}（写侧快照形态，禁大写变体与兜底）"
        );
        assert!(
            body.contains("\"exported_rows\": row_count")
                && body.contains("\"total\": server_total")
                && body.contains("\"export_complete\": export_complete"),
            "{handler_key} 的 after_snapshot 必须逐字含 exported_rows/total/export_complete 三键"
        );
    }
}

/// 判据④（21 列逐字 + 装配无关）：两侧表头函数与行构造键序列逐字同序，导出腿零 `.items`
#[test]
fn header_and_row_keys_are_same_21_tokens_in_order() {
    let po_headers = vec_tokens_in(&fn_body(PO_QUERY, "csv_headers"));
    let so_headers = vec_tokens_in(&fn_body(SO_EXPORT, "build_order_csv_headers"));
    assert_eq!(po_headers.len(), 21, "采购表头必须恰 21 枚");
    assert_eq!(so_headers.len(), 21, "销售表头必须恰 21 枚");
    assert_eq!(&po_headers[0], "订单编号", "两侧首列均为订单编号");

    let po_row = insert_keys(&fn_body(PO_QUERY, "build_csv_rows"));
    let so_row = insert_keys(&fn_body(SO_EXPORT, "order_to_csv_row"));
    assert_eq!(po_row, po_headers, "采购行构造键序列必须与表头逐字同序");
    assert_eq!(so_row, so_headers, "销售行构造键序列必须与表头逐字同序");

    let so_export_body = fn_body(SO_EXPORT, "export_orders_to_xlsx");
    assert!(
        !so_export_body.contains(".items"),
        "销售导出腿不得再读分页结果的 .items（旧取数腿形态）"
    );
}

/// 提取函数体内全部成对双引号 token（按出现顺序），供表头逐字比对
fn vec_tokens_in(body: &str) -> Vec<String> {
    quoted_tokens(body)
}

/// 行构造键序列：只取 `.insert(` 之后第一个字符串字面量（多行 insert 也要认），
/// 避免把 "%Y-%m-%d %H:%M:%S" 这类值侧字面量误采进键序列
fn insert_keys(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0;
    while let Some(rel) = body[idx..].find(".insert(") {
        let mut p = idx + rel + ".insert(".len();
        let bytes = body.as_bytes();
        while p < bytes.len() && (bytes[p] as char).is_whitespace()
            || p < bytes.len() && bytes[p] == b'('
        {
            p += 1;
        }
        if p < bytes.len() && bytes[p] == b'"' {
            let tokens = quoted_tokens(&body[p..p + 200]);
            if let Some(first) = tokens.first() {
                out.push(first.clone());
            }
        }
        idx = p;
    }
    out
}

/// 按出现顺序取出全部成对双引号 token
fn quoted_tokens(seg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = seg.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut tok = String::new();
            for n in chars.by_ref() {
                if n == '"' {
                    break;
                }
                tok.push(n);
            }
            if !tok.is_empty() {
                out.push(tok);
            }
        }
    }
    out
}

// ---- 夹具自证：注入旧形态必须被同一判据抓到（四件都在，缺一即假锁）----

/// 夹具一：旧取数腿形态 `self.list_orders(1, 10000, ...)` 与被丢的 `_total` 必须点名
#[test]
fn fixture_old_leg_shape_is_caught() {
    let old = "let (orders, _total) = self.list_orders(1, 10000, status, supplier_id, None, None).await?;";
    let hits = banned_hits(old, LEG_BANNED);
    for want in [".list_orders(", "10000", " _total"] {
        assert!(hits.contains(&want), "旧取数腿夹具的禁词 {want} 未被抓到");
    }
}

/// 夹具二：旧阈值分支 `const MAX_... = 10_000;` 加「超过上限」必须点名
#[test]
fn fixture_old_cap_branch_is_caught() {
    let old = "const MAX_PURCHASE_ORDER_EXPORT_ROWS: usize = 10_000;\n\
               if row_count > MAX_PURCHASE_ORDER_EXPORT_ROWS {\n\
                   return Err(AppError::bad_request(format!(\"采购订单导出条数 {} 超过上限 {}，请缩小筛选范围后重试\", row_count, MAX_PURCHASE_ORDER_EXPORT_ROWS)));\n\
               }";
    let hits = banned_hits(old, HANDLER_BANNED);
    for want in ["MAX_", "10_000", "超过上限"] {
        assert!(hits.contains(&want), "旧阈值分支夹具的禁词 {want} 未被抓到");
    }
}

/// 夹具三：复数 `/purchases/` 必须与真实 URL 判为不等（快照侧逐字为单数）
#[test]
fn fixture_plural_path_is_rejected_against_snapshot() {
    let plural = "/api/v1/erp/purchases/orders/export";
    let snap_line = SNAPSHOT
        .lines()
        .find(|l| l.split('\t').nth(2) == Some("purchase_order_handler::export_orders"))
        .expect("route-snapshot 必须登记采购导出");
    let snap_url = snap_line.split('\t').nth(1).unwrap_or_default();
    assert_ne!(
        plural, snap_url,
        "复数路径与快照真实 URL 必须判不等（判等即假锁）"
    );
    assert_eq!(snap_url, "/api/v1/erp/purchase/orders/export");
}

/// 夹具四：合规形状必须零命中（否则禁词表本身是假锁）
#[test]
fn fixture_compliant_shape_has_zero_hits() {
    let compliant = "let (orders, server_total) = self.list_all_orders_for_export(status, supplier_id, None, None).await?;\n\
                     let export_complete = PurchaseOrderService::export_rows_complete(orders.len(), server_total);";
    assert!(
        banned_hits(compliant, LEG_BANNED).is_empty(),
        "合规夹具被判红：禁词表过宽"
    );
    assert!(
        banned_hits(compliant, HANDLER_BANNED).is_empty(),
        "合规夹具被判红：handler 禁词表过宽"
    );
}
