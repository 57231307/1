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
//! `frontend/scripts/route-snapshot.txt`；缺陷形态夹具与检测力探针全部内联本文件
//! 的夹具自证节，不落 backend/tests 之外的目录。
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
fn banned_hits<'a>(body: &str, banned: &'a [&'a str]) -> Vec<&'a str> {
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

/// 活体控制判据（锚点枚数与表头枚数对齐）：行构造锚点的物理枚数、采集到的键枚数与
/// 表头枚数三方同数且大于零，两侧任何一枚对不上即红
/// 功能 给两侧相互比对的静态锁补一条采集器读不到的独立事实。键写成常量标识符引用或变量
/// 引用时，采集侧与被比侧同缺同序，逐枚比对照样判绿，这叫判据空转；锚点物理枚数把被静默
/// 跳过的那一枚显出来。本判据不替代也不放宽表头逐枚有序序列那条判据的 21 枚断言。
/// 调用方 CI backend 测试 target 非 db 组。
/// 入参 PO_QUERY 与 SO_EXPORT 两侧的行构造函数体与表头函数体，经 include_str 读入后按
/// 函数名切出。
/// 传给谁 只回本测试断言。
/// 存什么 不写库，断言活体控制线索为空，两侧各 21 枚。
/// 存哪里 不落地。
#[test]
fn row_insert_anchor_count_matches_header_enum_count() {
    for (src, headers_fn, row_fn, domain) in [
        (PO_QUERY, "csv_headers", "build_csv_rows", "采购"),
        (
            SO_EXPORT,
            "build_order_csv_headers",
            "order_to_csv_row",
            "销售",
        ),
    ] {
        let headers = vec_tokens_in(&fn_body(src, headers_fn));
        let report = live_control_report(&fn_body(src, row_fn), &headers);
        assert!(
            report.is_none(),
            "{domain} 导出腿活体控制判红，实际线索 {clue}",
            clue = report.unwrap_or_default()
        );
    }
}

/// 提取函数体内全部成对双引号 token（按出现顺序），供表头逐字比对
fn vec_tokens_in(body: &str) -> Vec<String> {
    quoted_tokens(body)
}

/// 行构造键序列
/// 功能 只取每一枚行构造锚点之后的第一个字符串字面量当键，按出现顺序成列，多行锚点也要认，
/// 值侧字面量不进序列。取窗从锚点一直到串尾，不按固定字节长度截断，因为被测函数体含中文，
/// 固定窗口既会越过串尾也可能落在非字符边界上，而最后一枚键常常距串尾不足一枚窗口长度，
/// 截断即丢尾键。锚点定位与枚数统计共用 insert_anchor_positions，两把采集器读同一批锚点。
/// 调用方 header_and_row_keys_are_same_21_tokens_in_order 与
/// row_insert_anchor_count_matches_header_enum_count 与夹具自证各件。
/// 入参 body，由 fn_body 按函数名切出的行构造函数体。
/// 传给谁 只回本测试断言，不外传不落库。
/// 存什么 不写库，返回键名序列。键写成常量或变量引用时该枚被静默跳过，序列短于锚点枚数。
/// 存哪里 结果只在断言内比对，夹具片段就地拼装在本文件。
fn insert_keys(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for p in insert_anchor_positions(body) {
        if body[p..].starts_with('"') {
            if let Some(first) = quoted_tokens(&body[p..]).first() {
                out.push(first.clone());
            }
        }
    }
    out
}

/// 行构造锚点位置
/// 功能 按出现顺序找出每一枚行构造锚点之后第一段非空白且非左括号处的位置，序列枚数就是
/// 函数体内行构造锚点的物理枚数。采集键与活体控制共用这一把扫描，保证两侧数到同一批锚点，
/// 不至于两把独立扫描各丢各的锚点而让判据自相矛盾。跳空白与左括号按字符推进，保证位置
/// 始终落在字符边界上；锚点之后全是空白与左括号时位置落到串尾。
/// 调用方 insert_keys 与 insert_anchor_key_forms 与 live_control_report。
/// 入参 body，由 fn_body 按函数名切出的行构造函数体。
/// 传给谁 只回本测试内部的采集器与活体控制。
/// 存什么 不写库，返回锚点位置序列。
/// 存哪里 结果只在断言内使用，不落文件。
fn insert_anchor_positions(body: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut idx = 0;
    while let Some(rel) = body[idx..].find(".insert(") {
        let start = idx + rel + ".insert(".len();
        let p = body[start..]
            .char_indices()
            .find(|(_, c)| !c.is_whitespace() && *c != '(')
            .map_or(body.len(), |(off, _)| start + off);
        out.push(p);
        idx = p;
    }
    out
}

/// 行构造锚点的键写法摘要
/// 功能 取每一枚锚点之后的第一段键写法文本，遇逗号、右括号、分号或换行即止。字面量键的
/// 摘要以双引号开头，常量标识符引用、变量引用与函数调用形态的摘要不以双引号开头，由此把
/// 采集侧静默跳过的那枚锚点原形暴露出来，供活体控制在失败消息里点名。
/// 调用方 live_control_report。
/// 入参 body，由 fn_body 按函数名切出的行构造函数体。
/// 传给谁 只回活体控制的失败线索。
/// 存什么 不写库，返回与锚点同序的键写法摘要序列。
/// 存哪里 结果只进断言消息，不落文件。
fn insert_anchor_key_forms(body: &str) -> Vec<String> {
    insert_anchor_positions(body)
        .into_iter()
        .map(|p| {
            let end = body[p..]
                .find(|c| matches!(c, ',' | ')' | ';' | '\n' | '\r'))
                .map_or(body.len(), |off| p + off);
            body[p..end].trim_end().to_string()
        })
        .collect()
}

/// 失败线索里的集合写法
/// 功能 把差集与摘要序列拼成一串可读文本，空集合写成汉字 无，避免失败消息里出现读不懂的空白。
/// 调用方 live_control_report。
/// 入参 items，待拼装的字符串序列。
/// 传给谁 只回活体控制的失败线索。
/// 存什么 不写库，返回拼装后的线索文本。
/// 存哪里 结果只进断言消息，不落文件。
fn clue_list(items: &[String]) -> String {
    if items.is_empty() {
        "无".to_string()
    } else {
        items.join("、")
    }
}

/// 活体控制判据
/// 功能 把行构造体内锚点的物理枚数、采集到的键枚数与表头枚数三方对齐，任两方不等或出现
/// 零枚即返回失败线索。两侧相互比对的静态锁在键写成常量引用时会同缺同序照样判绿，而锚点
/// 物理枚数是采集器读不到的独立事实，缺口藏不住。表头自身的枚数与逐枚序列继续由
/// header_and_row_keys_are_same_21_tokens_in_order 钉住，本判据不替代也不放宽它。
/// 调用方 row_insert_anchor_count_matches_header_enum_count 与变异自检夹具。
/// 入参 row_body，由 fn_body 切出的行构造函数体；headers，该侧表头的 token 序列。
/// 传给谁 只回调用它的断言。
/// 存什么 不写库，返回失败线索，内含三方实际枚数、被静默跳过的锚点写法、表头有而键缺的
/// 差集与键有而表头缺的差集；三方对齐时返回空。
/// 存哪里 不落地，线索只进断言消息。
fn live_control_report(row_body: &str, headers: &[String]) -> Option<String> {
    let anchor_count = insert_anchor_positions(row_body).len();
    let keys = insert_keys(row_body);
    let key_count = keys.len();
    let header_count = headers.len();
    if anchor_count == header_count && key_count == anchor_count && anchor_count > 0 {
        return None;
    }
    let skipped: Vec<String> = insert_anchor_key_forms(row_body)
        .into_iter()
        .filter(|form| !form.starts_with('"'))
        .collect();
    let mut pool = keys;
    let mut header_only = Vec::new();
    for header in headers {
        match pool.iter().position(|k| k == header) {
            Some(pos) => {
                pool.remove(pos);
            }
            None => header_only.push(header.clone()),
        }
    }
    let key_only = pool;
    Some(format!(
        "锚点 {anchor_count} 枚、键 {key_count} 枚、表头 {header_count} 枚，被静默跳过的锚点写法为 {x}，表头有而键缺为 {h}，键有而表头缺为 {k}",
        x = clue_list(&skipped),
        h = clue_list(&header_only),
        k = clue_list(&key_only),
    ))
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

// ---- 夹具自证：注入旧形态必须被同一判据抓到（七件都在，缺一即假锁）----

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

/// 夹具五：中文密集且尾键紧贴串尾的片段必须被完整捕获
/// 功能：证明 insert_keys 的取窗对尾部键有效，密集中文既不越界也不切字符边界，
/// 并与下方活体控制一起自证该夹具仍能钉住截断炸法。
/// 调用方：CI backend 测试 target（非 db 组），与判据四共用同一枚 insert_keys。
/// 入参：本测试就地拼装的中文密集函数体片段，最后一个键距串尾不足 200 字节。
/// 传给谁：只回本测试断言。
/// 存什么：不写库，拼出的键序列与期望序列逐枚同序比对。
/// 存哪里：夹具片段只在内存拼装，不落文件。
#[test]
fn insert_keys_captures_tail_key_in_chinese_dense_body() {
    let mut body = String::from(
        "fn order_to_csv_row() -> HashMap<String, String> {\n    let mut row = HashMap::new();\n",
    );
    let mut want: Vec<String> = Vec::new();
    for i in 1..=40 {
        body.push_str(&format!(
            "    row.insert(\"交付条款{i:02}\", o.term_{i}.unwrap_or_default()); // 说明文本\n"
        ));
        want.push(format!("交付条款{i:02}"));
    }
    body.push_str("    row.insert(\"备注\", o.notes.clone());\n    row\n}\n");
    want.push("备注".to_string());

    let got = insert_keys(&body);
    assert_eq!(
        got, want,
        "中文密集片段的尾部键必须逐枚捕获，掉尾或错序即取窗又回到截断形态"
    );

    let tail_anchor = body
        .rfind(".insert(")
        .unwrap_or_else(|| panic!("夹具必须自带最后一个 .insert( 锚点"));
    let tail_room = body.len() - (tail_anchor + ".insert(".len());
    assert!(
        tail_room < 200,
        "夹具失效控制：最后一个键距串尾 {tail_room} 字节，已不小于 200，钉不住截断炸法"
    );
}

/// 夹具六：无 .insert( 锚点的片段必须返回空集合而不是 panic
/// 功能：反向对照，证明 insert_keys 只从锚点后取键，不会把值侧字面量当键空转采集。
/// 调用方：CI backend 测试 target（非 db 组），与夹具五配对自证判据有检测力。
/// 入参：本测试就地写出的无锚点函数体片段，片段内含成对双引号的列名字面量。
/// 传给谁：只回本测试断言。
/// 存什么：不写库，断言返回集合为空。
/// 存哪里：夹具片段内联在本文件。
#[test]
fn insert_keys_returns_empty_without_any_insert_anchor() {
    let body = "fn order_to_csv_row() -> Vec<String> {\n\
                let cells = vec![\"订单编号\", \"供应商名称\", \"备注\"];\n\
                cells.into_iter().collect()\n\
            }\n";
    let got = insert_keys(body);
    assert!(
        got.is_empty(),
        "无 .insert( 锚点的片段不该产出键，实际采到 {joined}，说明取键已脱离锚点",
        joined = got.join("、")
    );
}

/// 夹具七，变异自检，键写成常量标识符引用
/// 功能 让判定器自证有检测力，报 0 之前先证明它能判负。先在变异夹具上复现判据空转的实形，
/// 行构造侧与表头侧都把同一枚备注键写成同一枚常量标识符引用，两侧同缺同序，按现有采集
/// 逻辑该键被静默丢掉，逐枚比对照样判等；再证明锚点枚数对齐这条活体控制把同一枚缺口判红，
/// 且失败线索点名被跳过的锚点写法与三方实际枚数与差集。
/// 调用方 CI backend 测试 target 非 db 组，与活体控制共用同一把 live_control_report。
/// 入参 本测试就地写出的五枚锚点行构造片段与两份表头片段，一份表头同样写成常量引用，
/// 另一份表头保留字面量。
/// 传给谁 只回本测试断言。
/// 存什么 不写库，断言锚点枚数、被静默丢掉的键、两侧同缺同序的判等事实、失败线索内容。
/// 存哪里 夹具片段内联在本文件，不落文件。
#[test]
fn mutation_constant_key_reference_is_dropped_and_anchor_control_catches_it() {
    let row_body = "    fn order_to_csv_row() -> HashMap<String, String> {\n        let mut row = HashMap::new();\n        row.insert(\"订单编号\".to_string(), o.order_no);\n        row.insert(\"客户ID\".to_string(), o.customer_id.to_string());\n        row.insert(HEADER_REMARK, o.notes.unwrap_or_default());\n        row.insert(\"状态\".to_string(), o.status);\n        row.insert(\"余额\".to_string(), o.balance_amount.to_string());\n        row\n    }\n";
    let headers_both_constant = "    fn build_order_csv_headers() -> Vec<String> {\n        vec![\n            \"订单编号\".to_string(),\n            \"客户ID\".to_string(),\n            HEADER_REMARK.to_string(),\n            \"状态\".to_string(),\n            \"余额\".to_string(),\n        ]\n    }\n";
    let headers_literal_kept = "    fn build_order_csv_headers() -> Vec<String> {\n        vec![\n            \"订单编号\".to_string(),\n            \"客户ID\".to_string(),\n            \"备注\".to_string(),\n            \"状态\".to_string(),\n            \"余额\".to_string(),\n        ]\n    }\n";

    let anchor_count = insert_anchor_positions(row_body).len();
    assert_eq!(anchor_count, 5, "夹具失效控制，行构造侧必须留下五枚锚点");
    let keys = insert_keys(row_body);
    assert!(
        !keys.iter().any(|k| k == "备注"),
        "键写成常量标识符引用时该枚本就该被现有采集逻辑静默跳过，采到了说明扫描已不认字面量这一形"
    );
    assert_eq!(
        keys.len(),
        anchor_count - 1,
        "被静默丢掉的必须恰好是一枚，实际采到 {got} 枚",
        got = keys.len()
    );

    let same_missing = vec_tokens_in(headers_both_constant);
    assert_eq!(
        keys, same_missing,
        "表头侧写成同一枚常量引用后两侧同缺同序，逐枚比对判据在此判等，这条断言钉住判据空转的实形"
    );

    let blind_clue = live_control_report(row_body, &same_missing).unwrap_or_else(|| {
        panic!("活体控制对两侧同缺同序的变异夹具放绿，判定器对常量引用形态没有检测力")
    });
    assert!(
        blind_clue.contains("HEADER_REMARK"),
        "线索必须点名被静默跳过的那枚锚点写法，实际线索 {clue}",
        clue = blind_clue
    );
    assert!(
        blind_clue.contains("锚点 5 枚")
            && blind_clue.contains("键 4 枚")
            && blind_clue.contains("表头 4 枚"),
        "线索必须带三方实际枚数，实际线索 {clue}",
        clue = blind_clue
    );

    let kept_headers = vec_tokens_in(headers_literal_kept);
    let diff_clue = live_control_report(row_body, &kept_headers)
        .unwrap_or_else(|| panic!("活体控制对表头侧保留字面量的变异夹具放绿，枚数对齐判据失效"));
    assert!(
        diff_clue.contains("表头有而键缺为 备注"),
        "线索必须给出表头有而键缺的差集，实际线索 {clue}",
        clue = diff_clue
    );
    assert!(
        diff_clue.contains("键 4 枚") && diff_clue.contains("表头 5 枚"),
        "两侧枚数对不上时线索要同时报出键侧与表头侧枚数，实际线索 {clue}",
        clue = diff_clue
    );
}
