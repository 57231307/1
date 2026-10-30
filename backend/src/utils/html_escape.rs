//! HTML 文本上下文转义与邮件正文危险标记检测
//!
//! 本模块承担三件事，且都不改写输入（第三件为净化，丢弃不合法片段）：
//!
//! 1. [`escape_html`]：把人工录入的业务字段值（供应商名、客户名、仓库名、单据号、
//!    订阅名等）转义成 HTML 文本，供调用方拼进邮件正文。转义集为 `& < > " ' /`，
//!    与邮件模板族既有口径逐字一致，避免同一仓内出现两套转义宽度。
//!    调用方：`services/event_notification_service.rs` 的正文构造、
//!    `services/report_subscription_scheduler.rs` 的订阅邮件构造、
//!    `handlers/email_handler.rs` 的 HTML 模板变量替换、`services/email_service.rs`
//!    的邮件模板族与发信出口。入参：待插入的字段值原文。传给谁：返回值由调用方拼进
//!    HTML 字符串，最终经 `EmailService` 的发送链路交给邮件服务商，落库形态是
//!    `email_logs.html_content`（JSONB 文本列）中的已转义片段。
//!
//! 2. [`scan_dangerous_markup`]：对已经成形的 HTML 正文做标记级检测（只读，不净化）。
//!    判定只认「真实标记」——必须以字面 `<` 起始进入标签态，因此被 `escape_html`
//!    转义成 `&lt;script&gt;` 的文本不会被误判，那正是转义已生效的证据。
//!    覆盖四类：脚本标签、标签上的内联事件属性（`on` 前缀）、URL 型属性上的危险协议
//!    （先做字符引用解码与控制符与空白归一，挡数字字符引用拼出的 javascript: 一类混淆）、
//!    `style` 属性与 `<style>` 元素体内的表达式调用。
//!    边界：这是发信出口的纵深检测，不是完整 sanitizer；合法排版标记
//!    （div/table/strong/a 与 style 中的普通声明）一律放行，命中危险构造则由
//!    [`ensure_markup_safe`] 拒绝发送并点名来源出口，绝不静默放行。
//!    已知未覆盖项：`data:image/svg+xml`（SVG 内嵌脚本）与 CSS `@import` 远程样式，
//!    二者在主流邮件客户端均不执行脚本，需要时再单独扩类。
//!
//! 3. [`sanitize_css_block`]：对 `<style>` 元素体内的原始 CSS 做上下文级净化。
//!    采用显式白名单语义：移除非法尖括号、限制 @规则集合、校验 url() 协议、丢弃表达式调用。
//!    调用方：`handlers/print_handler.rs` 的 `preview_print_template`，把数据库中存储的
//!    用户自定义样式安全嵌入预览文档的 `<style>` 上下文。

use crate::utils::error::AppError;

/// 脚本标签名（小写形态，标签解析时已统一转小写）
const SCRIPT_TAG: &str = "script";

/// 样式标签名，其元素体内的表达式调用需单独检测
const STYLE_TAG: &str = "style";

/// 取值会被邮件客户端按 URL 解析的属性名，危险协议只能从这些位置生效
const URL_ATTRIBUTES: [&str; 12] = [
    "href",
    "src",
    "action",
    "formaction",
    "xlink:href",
    "background",
    "poster",
    "cite",
    "longdesc",
    "dynsrc",
    "lowsrc",
    "data",
];

/// 会被邮件客户端执行的伪协议前缀（按归一后的小写形态比对）
const DANGEROUS_URL_SCHEMES: [&str; 4] =
    ["javascript:", "vbscript:", "livescript:", "data:text/html"];

/// 样式里可触发脚本求值的调用标记（IE `expression()` 与现代 `eval()` 形态）
const STYLE_EXPRESSION_MARKERS: [&str; 2] = ["expression(", "eval("];

/// 字符引用点名表中会被小写比对的名（键一律存小写，比对时把读到的名也转小写）
const NAMED_CHARACTER_REFERENCES: [(&str, char); 17] = [
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("colon", ':'),
    ("sol", '/'),
    ("num", '#'),
    ("lpar", '('),
    ("rpar", ')'),
    ("excl", '!'),
    ("quest", '?'),
    ("comma", ','),
    ("period", '.'),
    ("semi", ';'),
    ("tab", '\t'),
    ("newline", '\n'),
];

/// 拒绝文案里单条命中的最大回显长度，防止超长正文把日志撑爆
const DETAIL_MAX_CHARS: usize = 80;

/// 字符引用体的最大长度，实体名与数字位都远小于此值；同时把单次解码的查找代价限成常数
const MAX_REFERENCE_BODY_LEN: usize = 12;

/// 一条危险标记命中
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupViolation {
    /// 危险类别名（脚本标签 / 内联事件属性 / 危险 URL 协议 / 样式表达式）
    pub rule: &'static str,
    /// 命中的标记片段，供点名定位是哪段正文、哪个属性
    pub detail: String,
}

/// 转义 HTML 文本与属性值上下文中的特殊字符。
/// 入参：待插入 HTML 的字段值原文；返回：可安全落入文本节点或带引号属性值的字符串。
/// 传给谁：所有服务端拼接邮件 HTML 正文的构造点，以及 `EmailTemplate` 模板族。
pub fn escape_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            '/' => result.push_str("&#x2F;"),
            _ => result.push(ch),
        }
    }
    result
}

/// 扫描 HTML 正文中的危险标记，返回命中列表（空列表表示未检出）。
/// 入参：已成形的完整正文；同一类别只保留首个命中，避免同类刷屏。
/// 传给谁：[`ensure_markup_safe`]，由发信出口调用。
pub fn scan_dangerous_markup(html: &str) -> Vec<MarkupViolation> {
    let chars: Vec<char> = html.chars().collect();
    let mut violations: Vec<MarkupViolation> = Vec::new();
    let mut pos = 0usize;

    while pos < chars.len() {
        if chars[pos] != '<' {
            pos += 1;
            continue;
        }
        // 注释片段内的文本不会执行，整段跳过
        if matches_text(&chars, pos, "<!--") {
            pos = match find_text(&chars, pos + 4, "-->") {
                Some(end) => end + 3,
                None => break,
            };
            continue;
        }
        // 文档类型声明与处理指令：整段跳到下一个 '>'
        if matches_text(&chars, pos, "<!") || matches_text(&chars, pos, "<?") {
            pos = match chars[pos..].iter().position(|c| *c == '>') {
                Some(offset) => pos + offset + 1,
                None => break,
            };
            continue;
        }
        if !chars
            .get(pos + 1)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == '/')
        {
            // 字面 `<` 后不接字母或斜杠，属于不会成标签的裸文本
            pos += 1;
            continue;
        }
        let Some(tag) = read_tag(&chars, pos) else {
            pos += 1;
            continue;
        };
        record_tag_violations(&tag, &mut violations);
        // `<style>` 元素体内的表达式调用不在属性上，需单独扫正文
        if tag.name == STYLE_TAG && !tag.closing {
            match find_close_tag(&chars, tag.value_end, STYLE_TAG) {
                Some(close_at) => {
                    record_style_body_violations(&chars[tag.value_end..close_at], &mut violations);
                    pos = close_at;
                }
                None => {
                    record_style_body_violations(&chars[tag.value_end..], &mut violations);
                    break;
                }
            }
            continue;
        }
        pos = tag.value_end;
    }

    violations
}

/// 检测不通过则构造拒绝错误。
/// 入参：`html` 待发送正文、`source_label` 发信出口名（用于点名哪条腿）、
/// `subject` 邮件主题（用于定位是哪个模板或哪条订阅）。
/// 传给谁：`EmailService` 的通用发信出口与 HTML 直发出口；错误由调用方向上传递，
/// 队列腿会落进 `email_logs.error_message`，事件通知腿会进 `tracing::warn` 日志。
pub fn ensure_markup_safe(html: &str, source_label: &str, subject: &str) -> Result<(), AppError> {
    let violations = scan_dangerous_markup(html);
    if violations.is_empty() {
        return Ok(());
    }
    let named: Vec<String> = violations
        .iter()
        .map(|v| format!("{}[{}]", v.rule, v.detail))
        .collect();
    Err(AppError::business(format!(
        "邮件正文含危险 HTML 标记，已拒绝发送：发信出口={}，主题={}，命中={}",
        source_label,
        subject,
        named.join("；")
    )))
}

/// 一个已解析的起始或结束标签
struct ParsedTag {
    /// 标签名（小写）
    name: String,
    /// 是否闭合标签
    closing: bool,
    /// 属性列表（名已转小写，值为原文）
    attributes: Vec<(String, String)>,
    /// 该标签 `>` 之后的下一个待解析下标
    value_end: usize,
    /// 用于点名的原文片段（已截断）
    snippet: String,
}

/// 从 `start`（指向 `<`）解析一个标签，解析不出标签名时返回 None。
/// 入参：正文字符数组与起始下标；传给谁：[`scan_dangerous_markup`]。
fn read_tag(chars: &[char], start: usize) -> Option<ParsedTag> {
    let mut cursor = start + 1;
    let closing = chars.get(cursor) == Some(&'/');
    if closing {
        cursor += 1;
    }
    let name_begin = cursor;
    while cursor < chars.len() && is_tag_name_char(chars[cursor]) {
        cursor += 1;
    }
    if cursor == name_begin {
        return None;
    }
    let name: String = chars[name_begin..cursor]
        .iter()
        .collect::<String>()
        .to_lowercase();
    let mut attributes: Vec<(String, String)> = Vec::new();

    let end = loop {
        cursor = skip_whitespace(chars, cursor);
        if cursor >= chars.len() {
            break chars.len();
        }
        match chars[cursor] {
            '>' => break cursor + 1,
            '/' => {
                cursor += 1;
                if chars.get(cursor) == Some(&'>') {
                    break cursor + 1;
                }
            }
            '=' => {
                cursor += 1;
            }
            _ => {
                let (attr_name, attr_value, next) = read_attribute(chars, cursor);
                if !attr_name.is_empty() {
                    attributes.push((attr_name, attr_value));
                }
                cursor = next;
            }
        }
    };

    let snippet = truncate_snippet(&chars[start..end].iter().collect::<String>());
    Some(ParsedTag {
        name,
        closing,
        attributes,
        value_end: end,
        snippet,
    })
}

/// 解析一个属性，返回属性名（小写）、属性值原文与下一个待解析下标。
/// 入参：正文字符数组与属性起始下标；传给谁：[`read_tag`]。
fn read_attribute(chars: &[char], start: usize) -> (String, String, usize) {
    let name_begin = start;
    let mut cursor = start;
    while cursor < chars.len() && !is_attribute_delimiter(chars[cursor]) {
        cursor += 1;
    }
    let name: String = chars[name_begin..cursor]
        .iter()
        .collect::<String>()
        .to_lowercase();
    if cursor == name_begin {
        // 属性名位置是意料之外的字符，跳一个字符保证扫描前进
        return (name, String::new(), cursor + 1);
    }
    let mut value = String::new();
    cursor = skip_whitespace(chars, cursor);
    if chars.get(cursor) != Some(&'=') {
        return (name, value, cursor);
    }
    cursor += 1;
    cursor = skip_whitespace(chars, cursor);
    let quoted = chars
        .get(cursor)
        .copied()
        .filter(|c| *c == '"' || *c == '\'');
    if let Some(quote_ch) = quoted {
        cursor += 1;
        let value_begin = cursor;
        while cursor < chars.len() && chars[cursor] != quote_ch {
            cursor += 1;
        }
        value = chars[value_begin..cursor].iter().collect();
        if cursor < chars.len() {
            cursor += 1;
        }
    } else {
        let value_begin = cursor;
        while cursor < chars.len() && !chars[cursor].is_whitespace() && chars[cursor] != '>' {
            cursor += 1;
        }
        value = chars[value_begin..cursor].iter().collect();
    }
    (name, value, cursor)
}

/// 把单个标签的危险构造登记进命中列表
fn record_tag_violations(tag: &ParsedTag, violations: &mut Vec<MarkupViolation>) {
    if !tag.closing && tag.name == SCRIPT_TAG {
        push_violation(violations, "脚本标签", tag.snippet.clone());
    }
    for (name, value) in &tag.attributes {
        if name.len() > 2 && name.starts_with("on") {
            push_violation(
                violations,
                "内联事件属性",
                truncate_snippet(&format!("<{}> 上的 {}=", tag.name, name)),
            );
        }
        if URL_ATTRIBUTES.contains(&name.as_str()) {
            let normalized = normalize_url_value(value);
            for scheme in DANGEROUS_URL_SCHEMES {
                if normalized.starts_with(scheme) {
                    push_violation(
                        violations,
                        "危险 URL 协议",
                        truncate_snippet(&format!("<{}> 的 {}={}…", tag.name, name, scheme)),
                    );
                    break;
                }
            }
        }
        if name == STYLE_TAG && has_style_expression_marker(value) {
            push_violation(
                violations,
                "样式表达式",
                truncate_snippet(&format!("<{}> 的 style 属性", tag.name)),
            );
        }
    }
}

/// 把 `<style>` 元素体内的表达式调用登记进命中列表
fn record_style_body_violations(body: &[char], violations: &mut Vec<MarkupViolation>) {
    let text: String = body.iter().collect();
    if has_style_expression_marker(&text) {
        push_violation(violations, "样式表达式", truncate_snippet("<style> 元素体"));
    }
}

/// 同一类别只登记首个命中
fn push_violation(violations: &mut Vec<MarkupViolation>, rule: &'static str, detail: String) {
    if violations.iter().any(|v| v.rule == rule) {
        return;
    }
    violations.push(MarkupViolation { rule, detail });
}

/// 样式文本里是否含表达式调用（先解字符引用再归一，挡混淆写法）
fn has_style_expression_marker(raw: &str) -> bool {
    let normalized: String = decode_character_references(raw)
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    STYLE_EXPRESSION_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

/// URL 取值归一：解字符引用、去控制符与空白、转小写
fn normalize_url_value(raw: &str) -> String {
    decode_character_references(raw)
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// 解码数字与少量具名字符引用，用于还原混淆后的协议与表达式形态。
/// 入参：可能含数字字符引用（十进制或十六进制码点）与具名字符引用（冒号、左括号一类）
/// 写法的原文；返回：单趟解码结果。
/// 传给谁：[`normalize_url_value`] 与 [`has_style_expression_marker`]。
fn decode_character_references(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut pos = 0usize;
    while pos < chars.len() {
        if chars[pos] != '&' {
            out.push(chars[pos]);
            pos += 1;
            continue;
        }
        let window_end = (pos + 1 + MAX_REFERENCE_BODY_LEN).min(chars.len());
        let Some(offset) = chars[pos + 1..window_end].iter().position(|c| *c == ';') else {
            out.push('&');
            pos += 1;
            continue;
        };
        let semi = pos + 1 + offset;
        let body: String = chars[pos + 1..semi].iter().collect();
        match decode_reference_body(&body) {
            Some(decoded) => {
                out.push(decoded);
                pos = semi + 1;
            }
            None => {
                out.push('&');
                pos += 1;
            }
        }
    }
    out
}

/// 解析单个字符引用体（不含 `&` 与 `;`），无法解析时返回 None 表示按原文保留
fn decode_reference_body(body: &str) -> Option<char> {
    if body.is_empty() || body.len() > MAX_REFERENCE_BODY_LEN {
        return None;
    }
    if let Some(digits) = body.strip_prefix('#') {
        if let Some(hex) = digits
            .strip_prefix('x')
            .or_else(|| digits.strip_prefix('X'))
        {
            let code = u32::from_str_radix(hex, 16).ok()?;
            return char::from_u32(code);
        }
        let code = digits.parse::<u32>().ok()?;
        return char::from_u32(code);
    }
    let lowered = body.to_lowercase();
    NAMED_CHARACTER_REFERENCES
        .iter()
        .find(|(name, _)| *name == lowered)
        .map(|(_, ch)| *ch)
}

/// `at` 起是否逐字匹配 `needle`
fn matches_text(haystack: &[char], at: usize, needle: &str) -> bool {
    let mut iter = haystack[at..].iter();
    needle.chars().all(|c| iter.next() == Some(&c))
}

/// 从 `from` 起查找 `-->` 这类精确文本，返回命中下标
fn find_text(haystack: &[char], from: usize, needle: &str) -> Option<usize> {
    let span = needle.chars().count();
    let mut pos = from;
    while pos + span <= haystack.len() {
        if matches_text(haystack, pos, needle) {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

/// 从 `from` 起查找 `</name` 形态的闭合标签起始下标（容忍 `</` 与名之间的空白）
fn find_close_tag(haystack: &[char], from: usize, name: &str) -> Option<usize> {
    let mut pos = from;
    while pos + 2 < haystack.len() {
        if haystack[pos] == '<' && haystack[pos + 1] == '/' {
            let mut probe = pos + 2;
            while probe < haystack.len() && haystack[probe].is_whitespace() {
                probe += 1;
            }
            if matches_text_ignore_case(haystack, probe, name) {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

/// `at` 起是否匹配 `needle`（大小写不敏感）
fn matches_text_ignore_case(haystack: &[char], at: usize, needle: &str) -> bool {
    let mut iter = haystack[at..].iter();
    needle
        .chars()
        .all(|c| iter.next().is_some_and(|h| h.eq_ignore_ascii_case(&c)))
}

fn skip_whitespace(chars: &[char], from: usize) -> usize {
    let mut pos = from;
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    pos
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == ':'
}

fn is_attribute_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '=' | '>' | '/' | '<')
}

/// 回显片段限长
fn truncate_snippet(text: &str) -> String {
    if text.chars().count() <= DETAIL_MAX_CHARS {
        return text.to_string();
    }
    let kept: String = text.chars().take(DETAIL_MAX_CHARS).collect();
    format!("{}…", kept)
}

// ─────────────────────────────────────────────────────────────────────────────
// CSS 原始文本上下文净化（<style> 元素体）
// ─────────────────────────────────────────────────────────────────────────────

/// 允许出现在打印模板样式块中的 @规则名（小写）。
/// 设计意图：打印面样式只需排版与分页能力，不需要远程引入或行为指令。
const ALLOWED_AT_RULES: [&str; 4] = ["@media", "@page", "@font-face", "@supports"];

/// `url()` 内允许出现的前缀协议白名单（小写）。
/// 设计意图：打印面只允许内联 data URI（图标/字体），不允许任何远程或脚本协议。
const ALLOWED_URL_SCHEMES: [&str; 1] = ["data:"];

/// 对 `<style>` 元素体内的原始 CSS 做上下文级净化，返回安全文本。
///
/// 白名单语义：
/// 1. 尖括号（`<`、`>`）在合法 CSS 文本流中永远不会出现，一旦出现即代表越界标记，直接移除。
/// 2. 以 `@` 开头的规则，只允许 [`ALLOWED_AT_RULES`] 中列出的名称通过；`@import` 不在名单，
///    因为它会加载远程样式，可被用来注入任意内容。
/// 3. `url(` 后的协议前缀必须在 [`ALLOWED_URL_SCHEMES`] 白名单中，否则整条声明丢弃；
///    这挡的是 `url(javascript:...)` 和 `url(http://evil)` 一类。
/// 4. `expression(` 与 `javascript:` 在合法 CSS 中永远不会出现，出现即执行意图，移除所在 token。
///
/// 入参：`css` 原始样式文本（来自数据库存储的用户录入值）。
/// 返回：净化后的文本，可安全嵌入 `<style>...</style>` 上下文。
/// 调用方：`handlers/print_handler.rs` 的 `preview_print_template`。
pub fn sanitize_css_block(css: &str) -> String {
    // 步骤 1：移除尖括号，阻止 `</style>` 或 `<script>` 越界
    let without_angle = css.replace(['<', '>'], "");

    // 步骤 2-4：逐声明扫描，按白名单丢弃不合法的 @规则、url 协议与表达式调用
    let mut result = String::with_capacity(without_angle.len());
    let mut pending_segment = String::new();

    for line in without_angle.lines() {
        let trimmed = line.trim();
        // 检测 @规则：不在白名单则整行跳过
        if trimmed.starts_with('@') {
            let at_name = extract_at_rule_name(trimmed);
            if !ALLOWED_AT_RULES.contains(&at_name.as_str()) {
                continue;
            }
        }
        // 检测 url( 中的协议
        if let Some(cleaned) = sanitize_url_in_line(trimmed) {
            pending_segment.clear();
            pending_segment.push_str(&cleaned);
        } else {
            // url 协议不合法，丢弃整行
            continue;
        }

        // 检测 expression( 与 javascript:（即使大小写或空白混淆，归一后比对）
        let normalized_for_check: String = pending_segment
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        if normalized_for_check.contains("expression(")
            || normalized_for_check.contains("javascript:")
            || normalized_for_check.contains("vbscript:")
        {
            continue;
        }

        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&pending_segment);
    }

    result
}

/// 从以 `@` 开头的行提取规则名（小写，含 `@`），用于与白名单比对。
/// 入参：trim 后以 `@` 起始的文本；传给谁：[`sanitize_css_block`]。
fn extract_at_rule_name(line: &str) -> String {
    let name_end = line
        .find(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ';')
        .unwrap_or(line.len());
    line[..name_end].to_lowercase()
}

/// 检查一行中 `url(` 后的协议是否安全。返回 `Some(净化后文本)` 或 `None`（整行丢弃）。
/// 入参：单行文本；传给谁：[`sanitize_css_block`]。
fn sanitize_url_in_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let url_fn_start = match lower.find("url(") {
        Some(pos) => pos,
        None => return Some(line.to_string()),
    };
    // 提取 url( 后的内容直到匹配的 ')'
    let after_paren = &line[url_fn_start + 4..];
    let close_paren = after_paren.find(')').unwrap_or(after_paren.len());
    let url_content = after_paren[..close_paren]
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    let url_lower = url_content.to_lowercase();
    // 无协议前缀（相对路径或 data:）则按白名单检查
    let scheme_end = url_lower.find(':');
    if let Some(idx) = scheme_end {
        let scheme = &url_lower[..=idx];
        if !ALLOWED_URL_SCHEMES.contains(&scheme) {
            return None;
        }
    }
    // 不含冒号 = 相对路径，打印预览场景中无远程加载需求，也安全丢弃
    else if !url_lower.is_empty() {
        return None;
    }
    Some(line.to_string())
}
