//! 客户信用列表信封与 Decimal 出参形态的静态锁。
//!
//! 功能，钉住四层判据。
//! 一，list_credits 的出参信封必须是 PaginatedResponse，服务层算出的 total 必须随信封返回调用方，
//! 不允许以丢弃绑定的方式静默漏掉。
//! 二，Cargo.toml 中 rust_decimal 只启用 serde 特性，即 Decimal 的 JSON 出参形态是字符串。
//! 三，不连库直接用 serde_json 序列化 Decimal 复核第二层的实际行为，特性一旦被改成浮点形态本用例立刻变红。
//! 四，前端 customer_credit 出参接口的金额键声明必须与第二层与第三层判定的真实出参形态一致，
//! 两种特性状态下都给出明确结论，特性缺失时不允许静默通过。
//! 调用方，cargo test 静态用例与门禁，不连库，不启动服务。
//! 入参，读取 backend/Cargo.toml，backend/src/handlers/customer_credit_handler.rs，
//! frontend/src/api/customer-credit.ts 三个文件的文本。
//! 传给谁，只在测试内做文本比对，不把任何数据传给服务层，数据库或网络。
//! 存什么，无写入。
//! 存哪里，无写入。

use rust_decimal::Decimal;
use std::fs;
use std::path::Path;

/// 会让 Decimal 出参从 JSON 字符串变成 JSON 数字的 rust_decimal 特性名集合。
/// 命中任一名称即视为出参形态改变，前端金额声明必须同步改成数字。
const FLOAT_FORMING_FEATURES: [&str; 4] = [
    "serde-float",
    "serde-with-float",
    "serde-arbitrary-precision",
    "serde-with-arbitrary-precision",
];

/// 前端 customer_credit 出参接口里由后端 Decimal 承载的金额键。
const MONEY_KEYS: [&str; 3] = ["credit_limit", "used_credit", "available_credit"];

/// 读取仓库内文件文本。
/// 入参为相对 backend 目录的路径，由 CARGO_MANIFEST_DIR 定位，避免依赖测试运行时的当前工作目录。
fn read_repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("无法读取 {}: {e}", path.display()))
}

/// 取 Cargo.toml 中 rust_decimal 的依赖声明行。
/// 只认依赖段里的赋值行，排除 sea-orm 的 with-rust_decimal 特性名与注释干扰。
fn rust_decimal_dep_line(cargo_toml: &str) -> &str {
    cargo_toml
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("rust_decimal =") || l.starts_with("rust_decimal="))
        .unwrap_or_else(|| {
            panic!("Cargo.toml 应声明 rust_decimal 依赖，实际内容如下：{cargo_toml}")
        })
}

/// 依据 Cargo.toml 声明判定 Decimal 出参是否为字符串形态。
/// 返回 true 表示未启用任何浮点化特性，即出参为字符串。
fn decimal_wire_form_is_string(cargo_toml: &str) -> bool {
    let line = rust_decimal_dep_line(cargo_toml);
    !FLOAT_FORMING_FEATURES.iter().any(|f| line.contains(f))
}

/// 提取 list_credits 函数体文本，用于把出参信封与 total 的去向钉在单个函数范围内。
/// 入参为 handler 文件全文，按大括号配平扫描，避免把相邻 handler 的断言误算进来。
fn list_credits_fn_text(handler_src: &str) -> &str {
    let start = handler_src
        .find("pub async fn list_credits")
        .expect("customer_credit_handler.rs 应存在 list_credits 入口函数");
    let body_start = handler_src[start..]
        .find('{')
        .map(|i| start + i)
        .expect("list_credits 应有函数体起始大括号");
    let mut depth = 0usize;
    let bytes = handler_src.as_bytes();
    let mut idx = body_start;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &handler_src[start..=idx];
                }
            }
            _ => {}
        }
        idx += 1;
    }
    panic!("list_credits 函数体大括号未配平，无法定位函数结束位置");
}

/// 提取前端 CustomerCredit 出参接口体文本，只看该接口，不看请求体 DTO。
/// 入参为 frontend/src/api/customer-credit.ts 全文。
fn customer_credit_interface_block(api_src: &str) -> &str {
    let marker = "export interface CustomerCredit {";
    let start = api_src
        .find(marker)
        .expect("frontend/src/api/customer-credit.ts 应声明 CustomerCredit 出参接口");
    let rest = &api_src[start..];
    let end = rest.find('}').expect("CustomerCredit 接口缺少结束大括号");
    &rest[..end]
}

/// 取接口体内某个键的声明类型文本，键不存在或类型无法解析时直接判红。
/// 入参为接口体文本与键名，返回形如 string 或 number 的类型词，可选标记与分号被剥掉。
fn declared_type_of(block: &str, key: &str) -> String {
    let line = block
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&format!("{key}?")) || l.starts_with(&format!("{key}:")))
        .unwrap_or_else(|| panic!("CustomerCredit 接口应声明金额键 {key}，实际接口体：{block}"));
    let after_colon = line
        .split_once(':')
        .map(|(_, t)| t)
        .expect("CustomerCredit 接口的金额键声明缺少类型冒号");
    after_colon.trim().trim_end_matches(';').trim().to_string()
}

#[test]
fn list_credits_returns_paginated_envelope_and_carries_total() {
    let handler_src = read_repo_file("src/handlers/customer_credit_handler.rs");
    let fn_text = list_credits_fn_text(&handler_src);

    assert!(
        fn_text.contains("ApiResponse<PaginatedResponse<customer_credit::Model>>"),
        "list_credits 出参信封应为 ApiResponse 包 PaginatedResponse，供前端分页器读取总数，实际函数体：{fn_text}"
    );
    assert!(
        fn_text.contains("PaginatedResponse::new"),
        "list_credits 应通过 PaginatedResponse::new 把 items 与 total 与 page 与 page_size 一并返回，实际函数体：{fn_text}"
    );
    let nows: String = fn_text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        nows.contains("PaginatedResponse::new(credits,total,pageasu64,page_sizeasu64,)"),
        "list_credits 构造分页信封的实参次序应为 credits 与 total 与 page 与 page_size，\
         服务层算出的总数必须进信封，实际函数体：{fn_text}"
    );
    assert!(
        fn_text.contains("let (credits, total)"),
        "list_credits 应绑定服务层 get_list 返回的 total 并进入响应信封，实际函数体：{fn_text}"
    );
    assert!(
        !handler_src.contains("_total"),
        "customer_credit_handler.rs 不应出现下划线前缀的丢弃绑定，服务层已算出的总数被静默丢掉即为缺陷"
    );
    assert!(
        handler_src.contains("PaginatedResponse"),
        "customer_credit_handler.rs 应引入 PaginatedResponse 作为分页信封类型"
    );
    assert!(
        fn_text.contains("CustomerCreditService::clamp_page"),
        "list_credits 的页码应复用服务层 clamp_page 既有口径，不新造缺省值，实际函数体：{fn_text}"
    );
}

#[test]
fn rust_decimal_dependency_pins_string_serialization_feature_set() {
    let cargo_toml = read_repo_file("Cargo.toml");
    let line = rust_decimal_dep_line(&cargo_toml);

    assert!(
        line.contains("features = [\"serde\"]"),
        "rust_decimal 依赖应只启用 serde 特性以保证金额出参为字符串，实际声明行：{line}"
    );
    for feature in FLOAT_FORMING_FEATURES {
        assert!(
            !line.contains(feature),
            "rust_decimal 不应启用 {feature}，启用后 Decimal 出参从字符串变为数字，前端金额声明必须同步改成 number，实际声明行：{line}"
        );
    }
    assert!(
        decimal_wire_form_is_string(&cargo_toml),
        "按当前 Cargo.toml 判定，Decimal 出参形态应为字符串；判定不成立时说明依赖声明出现未登记的浮点化写法，实际声明行：{line}"
    );
}

/// 行为复核，不连库直接序列化 Decimal，证明特性面声明与真实出参形态一致。
/// 特性一旦被改成浮点形态，本用例断言的实际字符串会变为数字文本而立刻变红，并给出同步前端的线索。
#[test]
fn decimal_value_is_serialized_as_json_string_at_runtime() {
    let value = Decimal::new(123456, 2);
    let json = serde_json::to_string(&value).expect("Decimal 序列化应成功");

    assert_eq!(
        json, "\"1234.56\"",
        "Decimal 出参形态应为带引号的 JSON 字符串，实际为 {json}；若 Cargo.toml 启用了浮点化特性，\
         则后端出参已变成数字，frontend/src/api/customer-credit.ts 的 CustomerCredit 金额键必须同步改成 number"
    );
}

/// 前端金额声明与后端出参形态的一致性双向锁。
/// 期望形态由本进程真实序列化一次 Decimal 得出，不靠文本推测，特性未启用时要求前端声明 string，
/// 启用浮点化特性后要求前端声明 number，两种状态都必须给出明确结论，不做跳过。
#[test]
fn frontend_credit_amount_declarations_match_backend_decimal_wire_form() {
    let api_src = read_repo_file("../frontend/src/api/customer-credit.ts");
    let block = customer_credit_interface_block(&api_src);
    let sample = serde_json::to_value(Decimal::new(100, 2)).expect("Decimal 序列化应成功");
    let expected_type = if sample.is_string() {
        "string"
    } else {
        "number"
    };

    for key in MONEY_KEYS {
        let declared = declared_type_of(block, key);
        assert_eq!(
            declared, expected_type,
            "后端 Decimal 的真实金额出参形态是 {expected_type}，\
             但 frontend/src/api/customer-credit.ts 的 CustomerCredit 接口把 {key} 声明成 {declared}，\
             声明与真实形态不符会让按声明取数的消费方拿到错误类型且运行期不报错；\
             请把该键改成 {expected_type}，或同步调整 Cargo.toml 的 rust_decimal 特性"
        );
    }
}
