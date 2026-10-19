//! 链来源词表 ↔ 迁移 CHECK 一致性静态锁（CI 常规可跑，不依赖活库）
//!
//! ## 锁定的契约面
//! `sales_order_items.fulfillment_source` 的取值集在三处必须同为一套：
//! 1. 权威词表 `models/status/sales.rs::fulfillment_source::ALL`（写入方唯一事实来源）；
//! 2. 迁移 CHECK `chk_sales_order_item_fulfillment_source` 的取值清单（DB 侧兜底，
//!    拦住绕过服务层入参校验的旁路直写）；
//! 3. 服务层入参门控 `SalesOrderItemRequest::validate_fulfillment_source`（拒绝面文案）。
//! 本文件锁 1↔2 的取值集相等，并钉住「迁移的值域只能由 ALLOWED_SQL 单点注入」这一
//! 接线形态，防止有人另抄一套清单进 SQL 字符串（两处各维护 = 必然漂移）。
//!
//! 判据是**集合逐元素相等**，不是包含：
//! - 词表加值而 CHECK 未加 ⇒ 合法写入撞 23514，冒无据可依的裸 500；
//! - CHECK 比词表宽 ⇒ 旁路脚本能写出业务永不产生的脏 token，读取侧按词表分支时静默漏判。
//! 形态照 `tests/contract_wave11_export_inspection_result_check_test.rs` 的同族锁：
//! 从迁移原文按锚点解析，解析失守（锚点/引号/占位形态变化）直接 panic 点名，
//! 绝不「解析出空集就当通过」。
//!
//! ## NULL 语义（不进 ALL，也不受 CHECK 约束）
//! `ALL` 只列非 NULL 的三值。列可空，NULL = 未定档，是合法业务态；PG 三值逻辑下
//! `NULL IN (...)` 结果为 NULL 而非 FALSE，CHECK 天然放行 NULL。因此两侧都不必、
//! 也不许再各写一份 `OR ... IS NULL`——那会让取值清单在两处出现，破坏同源形态。

use bingxi_backend::models::status::sales::fulfillment_source;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// 承载本列 CHECK 的迁移域原文（域目录名与迁移名同源于业务语义，改名即红）。
const MIGRATION_REL: &str = "migration/src/domain/fulfillment_source_and_po_origin/mod.rs";
/// 词表清单的唯一注入锚点。
const ALLOWED_ANCHOR: &str = "const ALLOWED_SQL: &str = \"";
/// CHECK 约束名（迁移、模型注释、门控文案三处引用同一名字）。
const CHECK_NAME: &str = "chk_sales_order_item_fulfillment_source";

fn migration_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MIGRATION_REL);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取迁移域 {MIGRATION_REL} 失败: {e}（路径或改名需同步本锁）"))
}

/// 从迁移原文的 ALLOWED_SQL 字面量解析单引号 token 集合。
/// 锚点缺失、引号未闭合、解析出空集都点名 panic——空集比对会让任何词表都「相等」通过。
fn parse_migration_allowed(src: &str) -> BTreeSet<String> {
    let start = src.find(ALLOWED_ANCHOR).unwrap_or_else(|| {
        panic!("迁移域缺少 ALLOWED_SQL 锚点：CHECK 取值种子形态已改动，本锁的比对前提失效")
    });
    let rest = &src[start + ALLOWED_ANCHOR.len()..];
    let end = rest.find("\";").expect("ALLOWED_SQL 字面量未闭合");
    let literal = &rest[..end];
    let mut set = BTreeSet::new();
    let mut cursor = literal;
    while let Some(open) = cursor.find('\'') {
        let after = &cursor[open + 1..];
        let close = after.find('\'').expect("ALLOWED_SQL 内单引号未闭合");
        set.insert(after[..close].to_string());
        cursor = &after[close + 1..];
    }
    assert!(
        !set.is_empty(),
        "迁移 ALLOWED_SQL 解析出空集——解析失守，拒绝继续与词表比对"
    );
    set
}

/// 词表 ALL 与迁移 CHECK 取值集**逐元素集合相等**。
/// 改坏什么必红：任一侧单独增删值、把值改成中文或改成大写、ALLOWED_SQL 被拆成多处内联。
#[test]
fn fulfillment_source_vocab_equals_migration_check_set() {
    let src = migration_src();
    let check_set = parse_migration_allowed(&src);
    let vocab_set: BTreeSet<String> = fulfillment_source::ALL
        .iter()
        .map(|token| (*token).to_string())
        .collect();
    assert_eq!(
        vocab_set, check_set,
        "词表 fulfillment_source::ALL 与 CHECK {CHECK_NAME} 取值集必须集合相等：\n词表={vocab_set:?}\nCHECK={check_set:?}"
    );
    // 闭合三值：集合大小钉死，防止「加一个没人写的 token」被当成两侧一致通过。
    assert_eq!(
        vocab_set.len(),
        3,
        "链来源是闭合三值词表，实得 {vocab_set:?}（增删取值必须同时动词表、迁移与前端候选）"
    );
    // 语义锚点：三值各自的存在性（防止集合相等之外整体被换成另一套同长度 token）。
    assert_eq!(fulfillment_source::PRODUCTION, "production");
    assert_eq!(fulfillment_source::STOCK, "stock");
    assert_eq!(fulfillment_source::PURCHASE, "purchase");
    // 正向接线：CHECK 的取值由 ALLOWED_SQL 占位注入，迁移内不得内联第二套清单。
    assert!(
        src.contains(&format!(
            "ADD CONSTRAINT \"{CHECK_NAME}\" CHECK (\"fulfillment_source\" IN ({{allowed}}));"
        )),
        "迁移 CHECK 必须写成 `\"fulfillment_source\" IN ({{allowed}})` 占位形态（值域唯一由 ALLOWED_SQL 注入），内联第二套清单即漂移起点"
    );
}

/// 词表取值为全小写纯 ASCII：CHECK 逐字符敏感，大写或中文 token 会在写入侧造成
/// 「服务层放行、DB 拒收」或反向的分裂（本仓历史缺陷同型）。
#[test]
fn fulfillment_source_vocab_is_lowercase_ascii() {
    for token in fulfillment_source::ALL {
        assert!(
            token.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !token.is_empty(),
            "链来源取值 {token} 含非小写 ASCII 字符或为空（词表禁止中文/大写/空串）"
        );
    }
    // NULL 不在 ALL 内：ALL 只承载非 NULL 三值，NULL 由 Option 层单独放行。
    assert!(
        !fulfillment_source::ALL.contains(&""),
        "空串不得成为合法取值（空串既非词表值也非 NULL，读侧无法判定语义）"
    );
    assert!(
        !fulfillment_source::is_valid("production "),
        "is_valid 必须逐字符匹配，不得吞尾随空白（写入方不 trim，落库值与词表必须完全一致）"
    );
    assert!(
        !fulfillment_source::is_valid("PRODUCTION"),
        "is_valid 不得做大小写归一（归一会让 CHECK 逐字符敏感的列与入参校验两套判定分裂）"
    );
}

/// 门控与词表同源：允许清单文案必须由 `ALL` 拼装，不得在拒绝分支里另抄一套 token。
/// 改坏什么必红：门控换成硬编码清单且词表后续变更 ⇒ 用户看到的允许值与实际放行值不一致。
#[test]
fn write_gate_rejection_message_is_built_from_vocab() {
    let src = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/services/so/mod.rs"),
    )
    .expect("读取 services/so/mod.rs 失败");
    let code_only: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code_only.contains("fulfillment_source::ALL.join(\"/\")"),
        "拒绝文案的允许清单必须由 fulfillment_source::ALL 拼装（禁止另抄一套 token 进字符串）"
    );
    assert!(
        code_only.contains("fulfillment_source::is_valid(token)"),
        "写入门控必须用词表的 is_valid 判定，不得把三值展开成 match 字面量分支"
    );
    assert!(
        code_only.contains("AppError::validation_displayable"),
        "词表外拒绝必须走可外显的 validation_displayable（脱敏 business 会让用户看不到被拒原因）"
    );
}
