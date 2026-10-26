//! 契约锁：销售订单转销售合同链路 字段映射逐点静态遍历锁
//!
//! 锁三件事，全部为静态源文件判定，不连库不发网络请求：
//! 1. 正向带出：转换链五个文件（handler DTO、转换服务入参与预填、合同建头装配、
//!    前端 API 类型、对话框提交体）的字段映射逐点遍历，DTO 每个字段都必须在
//!    服务入参构造里被搬运且来源是 self 对应字段；预填每一列必须等于来源订单行
//!    或产品主档的对应列；落库 ActiveModel 每一列的来源表达式逐点对上。
//! 2. 反向禁道：客户端可见的四个结构（ConvertItemDto、ConvertFromOrderDto、
//!    ConvertItemPayload、ConvertFromOrderPayload）里不得存在折扣率、税率、任何
//!    金额列、客户改派、来源订单回指、编号、状态、建单人等产品身份与审计字段的
//!    写入通道；对话框源文件里这些键名连字面量都不得出现。
//! 3. 单源与门：印花税只引用 SalesContractService::calculate_stamp_tax，行金额只
//!    引用 so::amounts，转单侧不得有第二套算式；可转状态门取值只引用 so_status
//!    词表常量；重复转换判据按 source_order_id 加非终态状态集合过滤且拒绝文案点名
//!    既有合同号；前端两个入口的可转状态集合都引用 utils/sales-status 单源常量，
//!    视图内不得再抄第二套字面量数组。
//!
//! 变异自证：判定器以"返回问题清单"的纯函数实现，锁用例断清单为空；另按缺陷
//! 形态各造一份内存变异源（抽掉一条搬运、注入一个禁用字段、删一列落库映射、
//! 带出值改成常量），断言同一判定器点名出该缺陷，证明判定器有检测能力，
//! 而不是恒绿的空转。

use std::collections::{BTreeMap, BTreeSet};

const HANDLER: &str = include_str!("../src/handlers/sales_contract_handler.rs");
const CONVERT: &str = include_str!("../src/services/sales_contract_convert_service.rs");
const CONTRACT_SVC: &str = include_str!("../src/services/sales_contract_service.rs");
const FE_API: &str = include_str!("../../frontend/src/api/sales-contract.ts");
const FE_DIALOG: &str = include_str!("../../frontend/src/views/sales/ConvertToContractDialog.vue");
const FE_TABLE: &str =
    include_str!("../../frontend/src/views/sales/components/SalesOrderTable.vue");
const FE_DETAIL: &str = include_str!("../../frontend/src/views/sales/OrderDetail.vue");
const FE_SALES_STATUS: &str = include_str!("../../frontend/src/utils/sales-status.ts");

fn norm(s: &str) -> String {
    s.replace('\r', "")
}

/// 从锚点取第一层花括号内的内容（跳过双引号字符串内的花括号）
fn block_inner(hay: &str, opener: &str) -> String {
    let at = hay
        .find(opener)
        .unwrap_or_else(|| panic!("静态锁失配，找不到锚点: {opener}"));
    let chars: Vec<char> = hay[at..].chars().collect();
    let open = chars
        .iter()
        .position(|c| *c == '{')
        .unwrap_or_else(|| panic!("静态锁失配，锚点后无左花括号: {opener}"));
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = None;
    let mut end = None;
    let mut i = open;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '{' => {
                    depth += 1;
                    if depth == 1 {
                        start = Some(i + 1);
                    }
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    let s = start.expect("静态锁失配，块未闭合");
    let e = end.expect("静态锁失配，块未闭合");
    chars[s..e].iter().collect()
}

/// 去掉整行 // 注释与行尾 // 注释（源文件块内无合法 URL 字面量，简化成立）
fn strip_comments(block: &str) -> String {
    block
        .lines()
        .map(|line| {
            let t = line.trim_start();
            if t.starts_with("//") {
                String::new()
            } else {
                match line.find("//") {
                    Some(pos) => line[..pos].to_string(),
                    None => line.to_string(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 段首取键名：第一个两侧都不是冒号的 ASCII 冒号之前、去掉尾随 '?' 后的末尾标识符
fn key_of_segment(seg: &str) -> Option<(String, String)> {
    let s = seg.trim();
    if s.is_empty() || s.starts_with("..") {
        return None;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut colon = None;
    for (i, c) in chars.iter().enumerate() {
        if *c == ':' {
            let prev_colon = i > 0 && chars[i - 1] == ':';
            let next_colon = i + 1 < chars.len() && chars[i + 1] == ':';
            if !prev_colon && !next_colon {
                colon = Some(i);
                break;
            }
        }
    }
    let p = colon?;
    let head: String = chars[..p].iter().collect();
    let head = head.trim_end().trim_end_matches('?').trim_end().to_string();
    let key: String = head
        .chars()
        .rev()
        .take_while(|c| is_ident_char(*c))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if key.is_empty() || key.chars().next().unwrap().is_ascii_digit() {
        return None;
    }
    let value: String = chars[p + 1..].iter().collect();
    Some((key, value))
}

/// 花括号块内容按第一层逗号（Rust）或分号（TS 接口）切段，取每段的键与值原文
fn top_level_entries(inner: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut seg = String::new();
    for c in inner.chars() {
        if in_string {
            seg.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                seg.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                seg.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                seg.push(c);
            }
            ',' | ';' if depth == 0 => {
                if let Some((k, v)) = key_of_segment(&seg) {
                    out.insert(k, v);
                }
                seg.clear();
            }
            _ => seg.push(c),
        }
    }
    if let Some((k, v)) = key_of_segment(&seg) {
        out.insert(k, v);
    }
    out
}

fn struct_field_names(src: &str, opener: &str) -> BTreeSet<String> {
    top_level_entries(&strip_comments(&block_inner(src, opener)))
        .into_keys()
        .collect()
}

fn squeeze(v: &str) -> String {
    v.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 禁用写入通道清单：折扣率、税率、金额列、身份与审计列，客户端任何一侧都不得出现
const FORBIDDEN_CLIENT_FIELDS: &[&str] = &[
    "discount_percent",
    "discount_amount",
    "tax_percent",
    "tax_amount",
    "subtotal",
    "amount",
    "total_amount",
    "customer_id",
    "source_order_id",
    "source_order_item_id",
    "contract_no",
    "status",
    "created_by",
    "product_id",
    "stamp_tax_amount",
];

/// 客户端四结构 + 对话框提交体的禁用通道判定器，返回问题清单
fn client_channel_problems(handler: &str, convert: &str, api: &str, dialog: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let _ = convert;
    let dto_sets = [
        (
            "ConvertItemDto",
            struct_field_names(handler, "pub struct ConvertItemDto {"),
        ),
        (
            "ConvertFromOrderDto",
            struct_field_names(handler, "pub struct ConvertFromOrderDto {"),
        ),
        (
            "ConvertItemPayload",
            struct_field_names(api, "export interface ConvertItemPayload {"),
        ),
        (
            "ConvertFromOrderPayload",
            struct_field_names(api, "export interface ConvertFromOrderPayload {"),
        ),
    ];
    for (name, fields) in dto_sets {
        for f in FORBIDDEN_CLIENT_FIELDS {
            if fields.contains(*f) {
                problems.push(format!("{name} 出现禁用写入字段 {f}"));
            }
        }
    }
    for f in FORBIDDEN_CLIENT_FIELDS {
        let needle = format!("{f}:");
        if has_key_position(dialog, &needle) {
            problems.push(format!("对话框提交体出现禁用字段字面量 {needle}"));
        }
    }
    problems
}

/// 键位判定：needle 出现且其前一字符不是标识符字符或点号（排除 source_amount: 这类
/// 前缀粘连与非键位取值位置的误报）
fn has_key_position(hay: &str, needle: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = hay[from..].find(needle) {
        let idx = from + rel;
        let prev_ok = match hay[..idx].chars().next_back() {
            None => true,
            Some(c) => !(c.is_ascii_alphanumeric() || c == '_' || c == '.'),
        };
        if prev_ok {
            return true;
        }
        from = idx + 1;
    }
    false
}

/// DTO 字段到服务入参搬运的正向判定器（表头 with self 前缀、行 with 闭包参数名）
fn header_carry_problems(handler: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let fn_inner = block_inner(handler, "fn into_request");
    let carried = top_level_entries(&strip_comments(&block_inner(
        &fn_inner,
        "ConvertFromOrderRequest {",
    )));
    let dto = struct_field_names(handler, "pub struct ConvertFromOrderDto {");
    for f in &dto {
        match carried.get(f) {
            Some(v) => {
                let sq = squeeze(v);
                if !sq.contains(&format!("self.{f}")) {
                    problems.push(format!("表头字段 {f} 的搬运来源不是 self.{f}: {v}"));
                }
            }
            None => problems.push(format!("表头字段 {f} 未搬运进服务入参")),
        }
    }
    for k in carried.keys() {
        if !dto.contains(k) {
            problems.push(format!("搬运了 DTO 中不存在的表头字段 {k}"));
        }
    }
    problems
}

fn item_carry_problems(handler: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let fn_inner = block_inner(handler, "fn into_request");
    let carried = top_level_entries(&strip_comments(&block_inner(
        &fn_inner,
        "ConvertOrderItemInput {",
    )));
    let dto = struct_field_names(handler, "pub struct ConvertItemDto {");
    let input = struct_field_names(&norm(CONVERT), "pub struct ConvertOrderItemInput {");
    for f in &dto {
        match carried.get(f) {
            Some(v) => {
                let sq = squeeze(v);
                if !sq.contains(&format!("it.{f}")) {
                    problems.push(format!("行字段 {f} 的搬运来源不是 it.{f}: {v}"));
                }
            }
            None => problems.push(format!("行字段 {f} 未搬运进服务入参")),
        }
    }
    for k in carried.keys() {
        if !dto.contains(k) {
            problems.push(format!("搬运了 DTO 中不存在的行字段 {k}"));
        }
    }
    if input != dto {
        problems.push(format!(
            "服务入参行结构与 DTO 行结构不一致 DTO={:?} 入参={:?}",
            dto, input
        ));
    }
    problems
}

/// 预填带出逐点判定器：每列的来源表达式（squeeze 后须整体包含对应来源片段）
fn preview_carry_problems(convert: &str) -> Vec<String> {
    const POINTS: &[(&str, &[&str])] = &[
        ("order_item_id", &["item.id"]),
        ("product_id", &["item.product_id"]),
        ("product_code", &["product.code"]),
        ("product_name", &["product.name"]),
        ("product_spec", &["product.specification"]),
        ("unit", &["product.unit"]),
        ("quantity", &["item.quantity"]),
        ("unit_price", &["item.unit_price"]),
        ("discount_percent", &["item.discount_percent"]),
        ("discount_amount", &["item.discount_amount"]),
        ("tax_percent", &["item.tax_percent"]),
        ("tax_amount", &["item.tax_amount"]),
        ("subtotal", &["item.subtotal"]),
        ("total_amount", &["item.total_amount"]),
        ("color_no", &["item.color_no"]),
        ("width", &["item.width"]),
        ("gram_weight", &["item.gram_weight"]),
        ("fabric_composition", &["product.fabric_composition"]),
        ("quantity_tolerance_pct", &["item.quantity_tolerance_pct"]),
        ("delivery_date", &["header_delivery_date"]),
        ("remarks", &["item.notes"]),
    ];
    let mut problems = Vec::new();
    let built = top_level_entries(&strip_comments(&block_inner(
        convert,
        "push(ContractPreviewItem {",
    )));
    let declared = struct_field_names(convert, "pub struct ContractPreviewItem {");
    if built.keys().collect::<BTreeSet<_>>() != declared.iter().collect::<BTreeSet<_>>() {
        problems.push(format!(
            "预填行构造列集合与声明不一致 构造={:?} 声明={:?}",
            built.keys().collect::<Vec<_>>(),
            declared
        ));
    }
    for (field, needles) in POINTS {
        match built.get(*field) {
            Some(v) => {
                let sq = squeeze(v);
                for n in *needles {
                    if !sq.contains(&squeeze(n)) {
                        problems.push(format!("预填列 {field} 的来源不含 {n}: {v}"));
                    }
                }
            }
            None => problems.push(format!("预填列 {field} 缺失")),
        }
    }
    let header = top_level_entries(&strip_comments(&block_inner(
        convert,
        "Ok(ContractPreview {",
    )));
    for (field, needle) in [
        ("customer_id", "order.customer_id"),
        ("order_no", "order.order_no"),
        ("existing_contract", "existing.map"),
    ] {
        match header.get(field) {
            Some(v) if !squeeze(v).contains(needle) => {
                problems.push(format!("预填表头 {field} 的来源不含 {needle}: {v}"));
            }
            None => problems.push(format!("预填表头 {field} 缺失")),
            _ => {}
        }
    }
    problems
}

/// 落库行 ActiveModel 的逐点判定器：列名与来源表达式一一对上
fn item_active_model_problems(convert: &str) -> Vec<String> {
    const POINTS: &[(&str, &[&str])] = &[
        ("contract_id", &["contract.id"]),
        ("product_id", &["src.product_id"]),
        ("product_name", &["product.name"]),
        ("product_spec", &["product.specification"]),
        ("unit", &["product.unit"]),
        ("quantity", &["li.quantity"]),
        (
            "quantity_tolerance_pct",
            &["li.quantity_tolerance_pct", "src.quantity_tolerance_pct"],
        ),
        ("unit_price", &["li.unit_price"]),
        ("amount", &["line.subtotal"]),
        (
            "delivery_date",
            &["li.delivery_date", "header_delivery_date"],
        ),
        ("color_no", &["li.color_no", "src.color_no"]),
        ("discount_percent", &["line.discount_percent"]),
        ("discount_amount", &["line.discount_amount"]),
        ("tax_percent", &["line.tax_percent"]),
        ("tax_amount", &["line.tax_amount"]),
        ("source_order_item_id", &["src.id"]),
        ("remarks", &["li.remarks", "src.notes"]),
    ];
    const AMOUNT_COLUMNS: &[&str] = &[
        "amount",
        "discount_percent",
        "discount_amount",
        "tax_percent",
        "tax_amount",
    ];
    let mut problems = Vec::new();
    let set = top_level_entries(&strip_comments(&block_inner(
        convert,
        "sales_contract_item::ActiveModel {",
    )));
    for (field, needles) in POINTS {
        match set.get(*field) {
            Some(v) => {
                let sq = squeeze(v);
                for n in *needles {
                    if !sq.contains(&squeeze(n)) {
                        problems.push(format!("合同行列 {field} 的来源不含 {n}: {v}"));
                    }
                }
            }
            None => problems.push(format!("合同行列 {field} 未落值")),
        }
    }
    for col in AMOUNT_COLUMNS {
        if let Some(v) = set.get(*col)
            && squeeze(v).contains("li.")
        {
            problems.push(format!("合同行金额列 {col} 竟引用了客户端入参 li.: {v}"));
        }
    }
    problems
}

/// 建头请求逐点判定器：来源订单回指、合计与编号的来源
fn header_active_model_problems(convert: &str, contract_svc: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let convert_fn = &convert[convert
        .find("pub async fn convert")
        .expect("找不到 convert 函数")..];
    let req = top_level_entries(&strip_comments(&block_inner(
        convert_fn,
        "CreateSalesContractRequest {",
    )));
    for (field, needle) in [
        ("customer_id", "order.customer_id"),
        ("total_amount", "totals.total_amount"),
        ("source_order_id", "Some(order_id)"),
        ("contract_no", "None"),
    ] {
        match req.get(field) {
            Some(v) if !squeeze(v).contains(needle) => {
                problems.push(format!("建头字段 {field} 的来源不含 {needle}: {v}"));
            }
            None => problems.push(format!("建头字段 {field} 缺失")),
            _ => {}
        }
    }
    let line_amounts = block_inner(convert_fn, "calculate_line_amounts(");
    if !squeeze(&convert_fn[convert_fn.find(".map(|li|").expect("找不到行金额映射闭包")..])
        .contains("src.discount_percent")
        || !squeeze(&convert_fn[convert_fn.find(".map(|li|").expect("找不到行金额映射闭包")..])
            .contains("src.tax_percent")
    {
        problems.push(format!(
            "行金额重算未回读订单行折扣税率原值: {line_amounts}"
        ));
    }
    let builder = block_inner(contract_svc, "fn build_sales_contract_active_model");
    if !squeeze(&builder).contains("source_order_id:Set(req.source_order_id)") {
        problems.push("建头 ActiveModel 未显式落 source_order_id，转单溯源会静默丢失".to_string());
    }
    problems
}

/// 前后端类型逐字段对齐判定器（序列化名两侧必须同集合）
fn fe_be_shape_problems(handler: &str, convert: &str, api: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let pairs = [
        (
            "ConvertItemDto",
            struct_field_names(handler, "pub struct ConvertItemDto {"),
            struct_field_names(api, "export interface ConvertItemPayload {"),
        ),
        (
            "ConvertFromOrderDto",
            struct_field_names(handler, "pub struct ConvertFromOrderDto {"),
            struct_field_names(api, "export interface ConvertFromOrderPayload {"),
        ),
        (
            "ContractPreviewItem",
            struct_field_names(convert, "pub struct ContractPreviewItem {"),
            struct_field_names(api, "export interface ContractPreviewItem {"),
        ),
        (
            "ContractPreview",
            struct_field_names(convert, "pub struct ContractPreview {"),
            struct_field_names(api, "export interface ContractPreview {"),
        ),
        (
            "ExistingContract",
            struct_field_names(convert, "pub struct ExistingContract {"),
            struct_field_names(api, "export interface ExistingContract {"),
        ),
    ];
    for (name, rust, ts) in pairs {
        if rust != ts {
            problems.push(format!(
                "{name} 两侧字段集合不一致 Rust={:?} TS={:?}",
                rust, ts
            ));
        }
    }
    problems
}

/// 对话框提交体与 TS 载荷、以及 items 闭包与行载荷的逐点对齐判定器
fn dialog_payload_problems(api: &str, dialog: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let fn_inner = block_inner(dialog, "function buildPayload(): ConvertFromOrderPayload {");
    let sent = top_level_entries(&strip_comments(&block_inner(&fn_inner, "return {")));
    let payload = struct_field_names(api, "export interface ConvertFromOrderPayload {");
    if sent.keys().collect::<BTreeSet<_>>() != payload.iter().collect::<BTreeSet<_>>() {
        problems.push(format!(
            "buildPayload 顶层键集合与 ConvertFromOrderPayload 不一致 提交={:?} 载荷={:?}",
            sent.keys().collect::<Vec<_>>(),
            payload
        ));
    }
    let map_slice = &fn_inner[fn_inner.find("form.items.map(").expect("找不到 items 映射")..];
    let closure = block_inner(map_slice, "row => {");
    let item_sent = top_level_entries(&strip_comments(&block_inner(&closure, "return {")));
    let item_payload = struct_field_names(api, "export interface ConvertItemPayload {");
    if item_sent.keys().collect::<BTreeSet<_>>() != item_payload.iter().collect::<BTreeSet<_>>() {
        problems.push(format!(
            "buildPayload 行提交键集合与 ConvertItemPayload 不一致 提交={:?} 载荷={:?}",
            item_sent.keys().collect::<Vec<_>>(),
            item_payload
        ));
    }
    problems
}

/// 可转状态门判定器：后端只引用词表常量，前端两入口只引用单源常量
fn status_gate_problems(convert: &str, table: &str, detail: &str, utils: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let gate = block_inner(convert, "fn ensure_convertible_status");
    for c in [
        "APPROVED",
        "PARTIAL_SHIPPED",
        "SHIPPED",
        "COMPLETED",
        "DRAFT",
        "PENDING",
    ] {
        let needle = format!("so_status::{c}");
        if !gate.contains(&needle) {
            problems.push(format!("可转状态门未引用词表常量 {needle}"));
        }
    }
    // 未审核组与兜底组消息分流：cancelled/rejected 与词表外取值一律走兜底拒绝（fail-closed）
    if !gate.contains("尚未审核通过") || !gate.contains("已了结或失效") {
        problems.push("可转状态门的两组拒绝文案分流丢失".to_string());
    }
    if utils
        .find("SALES_ORDER_CONVERTIBLE_TO_CONTRACT_STATUSES")
        .is_none()
    {
        problems.push("utils/sales-status.ts 缺少前端可转状态单源常量".to_string());
    }
    for (file, src) in [("SalesOrderTable.vue", table), ("OrderDetail.vue", detail)] {
        if !src.contains("SALES_ORDER_CONVERTIBLE_TO_CONTRACT_STATUSES") {
            problems.push(format!("{file} 未引用前端可转状态单源常量"));
        }
        if src.contains("const CONVERTIBLE_STATUSES") {
            problems.push(format!("{file} 抄了第二套可转状态字面量数组"));
        }
    }
    problems
}

/// 重复转换互斥判定器：判据按来源订单加非终态状态过滤，拒绝文案点名既有合同号
fn duplicate_gate_problems(convert: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let finder = block_inner(convert, "fn find_effective_converted_contract");
    for needle in [
        "Column::SourceOrderId.eq(order_id)",
        "contract_status::DRAFT",
        "contract_status::ACTIVE",
    ] {
        if !finder.contains(needle) {
            problems.push(format!("重复转换判据缺少 {needle}"));
        }
    }
    let convert_fn = &convert[convert
        .find("pub async fn convert")
        .expect("找不到 convert")..];
    if !convert_fn.contains("不能重复转换") || !convert_fn.contains("exist.contract_no") {
        problems.push("重复转换拒绝文案未点名既有合同号，用户不可识别".to_string());
    }
    problems
}

/// 印花税与金额单源判定器：转单侧不得有第二套算式
fn money_single_source_problems(convert: &str, contract_svc: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !convert.contains("SalesContractService::calculate_stamp_tax(") {
        problems.push("转单链路未复用 SalesContractService::calculate_stamp_tax".to_string());
    }
    if convert.contains("Decimal::new(") {
        problems.push("转单链路内出现第二套税率数字面量".to_string());
    }
    if !convert.contains("use crate::services::so::amounts") {
        problems.push("转单链路未引用 so::amounts 金额单源".to_string());
    }
    if !contract_svc.contains("\"sale\" | \"purchase_sale\" =>") {
        problems.push("印花税单源实现本身的类型取值形状变化，需人工复核".to_string());
    }
    problems
}

// ---------------------------------------------------------------------------
// 正向与反向锁（真实源码上判定器问题清单必须为空）
// ---------------------------------------------------------------------------

#[test]
fn convert_chain_has_no_client_side_amount_or_identity_channel() {
    let problems = client_channel_problems(
        &norm(HANDLER),
        &norm(CONVERT),
        &norm(FE_API),
        &norm(FE_DIALOG),
    );
    assert!(
        problems.is_empty(),
        "客户端写入通道检查失败，折扣税率金额或身份审计字段不得可送: {problems:?}"
    );
}

#[test]
fn convert_dto_header_fields_are_all_carried_into_service_request() {
    let problems = header_carry_problems(&norm(HANDLER));
    assert!(
        problems.is_empty(),
        "表头字段搬运逐点检查失败: {problems:?}"
    );
}

#[test]
fn convert_dto_item_fields_are_all_carried_into_service_input() {
    let problems = item_carry_problems(&norm(HANDLER));
    assert!(problems.is_empty(), "行字段搬运逐点检查失败: {problems:?}");
}

#[test]
fn preview_carries_order_line_and_product_values_point_by_point() {
    let problems = preview_carry_problems(&norm(CONVERT));
    assert!(problems.is_empty(), "预填带出逐点检查失败: {problems:?}");
}

#[test]
fn contract_item_active_model_maps_every_column_from_the_right_side() {
    let problems = item_active_model_problems(&norm(CONVERT));
    assert!(
        problems.is_empty(),
        "合同行落库映射逐点检查失败: {problems:?}"
    );
}

#[test]
fn contract_header_is_built_from_order_totals_and_source_order_id() {
    let problems = header_active_model_problems(&norm(CONVERT), &norm(CONTRACT_SVC));
    assert!(
        problems.is_empty(),
        "合同建头来源逐点检查失败: {problems:?}"
    );
}

#[test]
fn frontend_types_match_backend_shapes_field_by_field() {
    let problems = fe_be_shape_problems(&norm(HANDLER), &norm(CONVERT), &norm(FE_API));
    assert!(
        problems.is_empty(),
        "前后端形状逐字段对齐检查失败: {problems:?}"
    );
}

#[test]
fn dialog_submit_payload_matches_whitelist_exactly() {
    let problems = dialog_payload_problems(&norm(FE_API), &norm(FE_DIALOG));
    assert!(
        problems.is_empty(),
        "对话框提交体与白名单载荷对齐检查失败: {problems:?}"
    );
}

#[test]
fn convertible_status_gate_stays_single_sourced() {
    let problems = status_gate_problems(
        &norm(CONVERT),
        &norm(FE_TABLE),
        &norm(FE_DETAIL),
        &norm(FE_SALES_STATUS),
    );
    assert!(problems.is_empty(), "可转状态门单源检查失败: {problems:?}");
}

#[test]
fn duplicate_conversion_is_detected_and_named_to_the_user() {
    let problems = duplicate_gate_problems(&norm(CONVERT));
    assert!(problems.is_empty(), "重复转换互斥检查失败: {problems:?}");
}

#[test]
fn money_math_has_only_one_source_on_the_convert_leg() {
    let problems = money_single_source_problems(&norm(CONVERT), &norm(CONTRACT_SVC));
    assert!(
        problems.is_empty(),
        "金额与印花税单源检查失败: {problems:?}"
    );
}

// ---------------------------------------------------------------------------
// 变异自证（缺陷形态必须让上面的判定器点名出该缺陷）
// ---------------------------------------------------------------------------

#[test]
fn mutation_dropping_a_header_carry_is_detected() {
    let mutated = norm(HANDLER).replace("payment_terms: self.payment_terms,", "");
    let problems = header_carry_problems(&mutated);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("payment_terms") && p.contains("未搬运")),
        "变异自证失效：抽掉表头搬运行后判定器未点名 payment_terms: {problems:?}"
    );
}

#[test]
fn mutation_adding_discount_channel_to_item_dto_is_detected() {
    let mutated = norm(HANDLER).replace(
        "pub struct ConvertItemDto {",
        "pub struct ConvertItemDto { pub discount_percent: Option<String>,",
    );
    let problems =
        client_channel_problems(&mutated, &norm(CONVERT), &norm(FE_API), &norm(FE_DIALOG));
    assert!(
        problems
            .iter()
            .any(|p| p.contains("ConvertItemDto") && p.contains("discount_percent")),
        "变异自证失效：给行 DTO 注入折扣通道后判定器未点名: {problems:?}"
    );
}

#[test]
fn mutation_dropping_source_order_item_backref_column_is_detected() {
    let mutated = norm(CONVERT).replace("source_order_item_id: Set(Some(src.id)),", "");
    let problems = item_active_model_problems(&mutated);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("source_order_item_id") && p.contains("未落值")),
        "变异自证失效：删掉溯源列映射后判定器未点名: {problems:?}"
    );
}

#[test]
fn mutation_replacing_preview_carry_with_constant_is_detected() {
    let mutated = norm(CONVERT).replace("subtotal: item.subtotal,", "subtotal: Decimal::ZERO,");
    let problems = preview_carry_problems(&mutated);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("subtotal") && p.contains("item.subtotal")),
        "变异自证失效：预填带出被改成常量后判定器未点名: {problems:?}"
    );
}

#[test]
fn mutation_reintroducing_second_status_array_in_view_is_detected() {
    // 变异形态：列表视图回退成自抄一份字面量数组（判定器要点名第二套集合）
    let mutated = format!(
        "{}\nconst CONVERTIBLE_STATUSES = ['approved', 'partial_shipped', 'shipped', 'completed'];\n",
        norm(FE_TABLE)
    );
    let problems = status_gate_problems(
        &norm(CONVERT),
        &mutated,
        &norm(FE_DETAIL),
        &norm(FE_SALES_STATUS),
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("SalesOrderTable.vue") && p.contains("第二套")),
        "变异自证失效：视图重新抄第二套状态数组后判定器未点名: {problems:?}"
    );
}

#[test]
fn mutation_reintroducing_stamp_tax_rate_literal_is_detected() {
    let mutated = norm(CONVERT).replace(
        "pub struct ConvertResult {",
        "pub struct ConvertResult { const FAKE_STAMP: rust_decimal::Decimal = Decimal::new(3, 4);",
    );
    let problems = money_single_source_problems(&mutated, &norm(CONTRACT_SVC));
    assert!(
        problems.iter().any(|p| p.contains("第二套税率数字面量")),
        "变异自证失效：转单侧注入第二套印花税率字面量后判定器未点名: {problems:?}"
    );
}
