//! CRUD 宏出参类型化契约锁（本波契约：`serde_json::Value` 出参塌形 → 具体类型贯穿到出参）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/utils/crud_macro.rs` 的 `define_crud_handlers!` 与 `define_tuple_crud_handlers!`
//!   两个 CRUD 宏：生成 handler 的出参载荷类型一律来自类型形参 `$list_resp_ty` / `$item_resp_ty`，
//!   宏体中不得再出现 `ApiResponse<serde_json::Value>`（出参塌成 Value 即对 check-api-envelope
//!   判定器永远「无可比对形状」，是门禁盲区；list 载荷必须是统一分页信封
//!   `utils::response::PaginatedResponse{items,total,page,page_size}`，禁止手拼第二套信封）。
//! - 全部展开点的出参实参为具体类型，且 list 实参是 `PaginatedResponse<...>`：
//!   - handlers/department_handler.rs、handlers/product_category_handler.rs、handlers/warehouse_handler.rs
//!     （define_crud_handlers!）
//!   - handlers/email_handler.rs、handlers/oa_announcement_handler.rs、handlers/report_enhanced_handler.rs
//!     （define_tuple_crud_handlers!）
//!   - oa_announcement_handler.rs 的手写 `list`（覆盖宏 list 的同族站点）载荷同为
//!     `PaginatedResponse<oa_announcement::Model>`。
//! - 逐键出参形状（键名 + 值类型）：list 顶层信封键恰为 code/data（ApiResponse 成功出参，
//!   message/total 为 None 被 skip），data 键恰为 items/total/page/page_size；
//!   total/page/page_size 为数字，行对象键名与实体 Model 字段逐一对应（snake_case），
//!   整数列为数字、时间列与中文枚举 token 为字符串、可空列键恒存在且空值为 null。
//!   （本波三个域实体无 Decimal 列；Decimal→字符串口径由其它波次测试锁定，此处不重复。）
//!
//! 回潮判据：
//! 1) 任一宏展开点出参实参回退 `serde_json::Value` → 形态棘轮计数超过上限（0）判红；
//! 2) 展开点被改名/删除导致解析空集 → 判红（禁止「解析不到就跳过」的静默放行）；
//! 3) list 信封键偏离 items/total/page/page_size → 逐键形状断言判红；
//! 4) 行对象键名漂移 → 键集合「恰为」断言判红。
//!
//! 覆盖策略：纯 serde 形状断言（无任何 DB、不起服务），夹具为手写 Model 字面量；
//! 源码形态棘轮只读文本扫描，不执行任何编译。

use std::fs;
use std::path::{Path, PathBuf};

use bingxi_backend::models::{email_template, oa_announcement, report_subscription};
use bingxi_backend::utils::response::{ApiResponse, PaginatedResponse};
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use serde_json::Value;

/// 形态棘轮上限：宏展开点中出参仍为 `serde_json::Value` 的数量。本波清零，只准降不准升。
const VALUE_OUTPUT_RATCHET_MAX: usize = 0;
/// 展开点数量地板：低于该值说明宏调用被改名/删除或扫描根路径失效（解析空集必须判红）。
const KNOWN_INVOCATION_FLOOR: usize = 6;

fn sorted_keys(obj: &Value) -> Vec<String> {
    let mut ks: Vec<String> = obj
        .as_object()
        .unwrap_or_else(|| panic!("期望 JSON 对象，实际: {obj}"))
        .keys()
        .cloned()
        .collect();
    ks.sort();
    ks
}

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 8, 0, 0)
        .single()
        .expect("固定时间夹具必须可解析")
}

fn assert_envelope_top_level(env: &Value) {
    assert_eq!(
        sorted_keys(env),
        vec!["code", "data"],
        "ApiResponse 成功出参顶层键必须恰为 code/data（message/total 为 None 被 skip）"
    );
    assert_eq!(env["code"], Value::from(200));
}

/// 校验统一分页信封 data 的逐键形状：键恰为 items/total/page/page_size，total/page/page_size 为数字
fn assert_paginated_data_shape(data: &Value) {
    assert_eq!(
        sorted_keys(data),
        ["items", "page", "page_size", "total"],
        "list data 载荷键必须恰为 PaginatedResponse 的 items/total/page/page_size（第二套信封回潮判据）"
    );
    assert!(
        data["items"].is_array(),
        "items 必须是数组，实际: {}",
        data["items"]
    );
    assert!(
        data["total"].is_number(),
        "total 必须是数字，实际: {}",
        data["total"]
    );
    assert!(
        data["page"].is_number(),
        "page 必须是数字，实际: {}",
        data["page"]
    );
    assert!(
        data["page_size"].is_number(),
        "page_size 必须是数字，实际: {}",
        data["page_size"]
    );
}

fn sample_email_template(expiry_none: bool) -> email_template::Model {
    email_template::Model {
        id: 7,
        name: "对账单模板".to_string(),
        code: "STMT-001".to_string(),
        subject_template: "subject".to_string(),
        body_template: "body".to_string(),
        template_type: "NOTIFICATION".to_string(),
        variables: if expiry_none {
            None
        } else {
            Some(serde_json::json!([{"name": "order_no"}]))
        },
        description: if expiry_none {
            None
        } else {
            Some("d".to_string())
        },
        is_active: true,
        status: "ACTIVE".to_string(),
        created_by: 3,
        created_at: fixed_now(),
        updated_at: fixed_now() + Duration::seconds(1),
    }
}

fn sample_oa_announcement(nullable_none: bool) -> oa_announcement::Model {
    oa_announcement::Model {
        id: 11,
        title: "国庆放假安排".to_string(),
        content: "content".to_string(),
        announcement_type: "NOTICE".to_string(),
        publish_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("固定日期夹具必须可解析"),
        effective_date: NaiveDate::from_ymd_opt(2026, 1, 2).expect("固定日期夹具必须可解析"),
        expiry_date: if nullable_none {
            None
        } else {
            NaiveDate::from_ymd_opt(2026, 12, 31)
        },
        publisher_id: 3,
        status: "DRAFT".to_string(),
        is_top: false,
        attachments: if nullable_none {
            None
        } else {
            Some(serde_json::json!([]))
        },
        remarks: if nullable_none {
            None
        } else {
            Some("r".to_string())
        },
        visibility_scope: "ALL".to_string(),
        visible_scope_config: if nullable_none {
            None
        } else {
            Some(serde_json::json!({"department_ids": [2]}))
        },
        created_at: fixed_now(),
        updated_at: fixed_now() + Duration::seconds(1),
    }
}

fn sample_subscription(nullable_none: bool) -> report_subscription::Model {
    report_subscription::Model {
        id: 21,
        name: "月销售报表".to_string(),
        template_id: 5,
        frequency: "MONTHLY".to_string(),
        recipients: serde_json::json!(["a@example.com"]),
        parameters: if nullable_none {
            None
        } else {
            Some(serde_json::json!({"year": 2026}))
        },
        export_format: "pdf".to_string(),
        is_enabled: true,
        status: "ACTIVE".to_string(),
        next_run_at: if nullable_none {
            None
        } else {
            Some(fixed_now())
        },
        last_run_at: if nullable_none {
            None
        } else {
            Some(fixed_now())
        },
        last_run_status: if nullable_none {
            None
        } else {
            Some("success".to_string())
        },
        last_run_error: if nullable_none {
            None
        } else {
            Some("".to_string())
        },
        run_count: 2,
        retry_count: 0,
        max_retries: 3,
        next_retry_at: if nullable_none {
            None
        } else {
            Some(fixed_now())
        },
        created_by: 3,
        created_at: fixed_now(),
        updated_at: fixed_now() + Duration::seconds(1),
    }
}

#[test]
fn email_template_list_payload_is_typed_paginated_envelope() {
    let resp: ApiResponse<PaginatedResponse<email_template::Model>> =
        ApiResponse::success(PaginatedResponse::new(
            vec![sample_email_template(false), sample_email_template(true)],
            2,
            1,
            20,
        ));
    let env = serde_json::to_value(&resp).expect("ApiResponse 必然可序列化");
    assert_envelope_top_level(&env);
    assert_paginated_data_shape(&env["data"]);

    let rows = env["data"]["items"].as_array().expect("items 为数组");
    assert_eq!(rows.len(), 2, "夹具两行必须全在，实际 {}", rows.len());

    let expected = [
        "body_template",
        "code",
        "created_at",
        "created_by",
        "description",
        "id",
        "is_active",
        "name",
        "status",
        "subject_template",
        "template_type",
        "updated_at",
        "variables",
    ];
    for row in rows {
        assert_eq!(
            sorted_keys(row),
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "email_template::Model 出参键必须与实体字段逐一对应"
        );
        assert!(row["id"].is_number(), "整数列 id 必须是数字");
        assert!(
            row["created_by"].is_number(),
            "整数列 created_by 必须是数字"
        );
        assert!(row["is_active"].is_boolean(), "bool 列必须是布尔");
        assert!(
            row["created_at"].is_string(),
            "时间列必须序列化为字符串，实际: {}",
            row["created_at"]
        );
        assert!(row["template_type"].is_string(), "状态 token 必须是字符串");
    }
    // 可空列键恒存在、空值为 null（禁止 skip 掩盖缺键）
    assert!(rows[1].get("variables").is_some(), "variables 键必须恒存在");
    assert_eq!(rows[1]["variables"], Value::Null, "可空列空值必须是 null");
    assert_eq!(rows[1]["description"], Value::Null, "可空列空值必须是 null");
}

#[test]
fn email_template_single_records_are_typed_not_value() {
    // get：ApiResponse::success(item)；create/update：success_with_message(item, ...)
    let env = serde_json::to_value(ApiResponse::success(sample_email_template(false)))
        .expect("ApiResponse 必然可序列化");
    assert_envelope_top_level(&env);
    assert_eq!(
        env["data"]["id"],
        Value::from(7),
        "单条 data 必须是行实体本身"
    );

    let env_msg = serde_json::to_value(ApiResponse::success_with_message(
        sample_email_template(false),
        bingxi_backend::utils::messages::biz_msg::CREATE_OK,
    ))
    .expect("ApiResponse 必然可序列化");
    assert_eq!(
        sorted_keys(&env_msg),
        ["code", "data", "message"],
        "写操作信封顶层键恰为 code/data/message"
    );
}

#[test]
fn oa_announcement_list_payload_is_typed_paginated_envelope() {
    // 宏 generated::list 与同文件手写 list（覆盖站点）共用同一 service 信封
    let resp: ApiResponse<PaginatedResponse<oa_announcement::Model>> =
        ApiResponse::success(PaginatedResponse::new(
            vec![sample_oa_announcement(false), sample_oa_announcement(true)],
            2,
            1,
            20,
        ));
    let env = serde_json::to_value(&resp).expect("ApiResponse 必然可序列化");
    assert_envelope_top_level(&env);
    assert_paginated_data_shape(&env["data"]);

    let rows = env["data"]["items"].as_array().expect("items 为数组");
    let expected = [
        "announcement_type",
        "attachments",
        "content",
        "created_at",
        "effective_date",
        "expiry_date",
        "id",
        "is_top",
        "publisher_id",
        "publish_date",
        "remarks",
        "status",
        "title",
        "updated_at",
        "visible_scope_config",
        "visibility_scope",
    ];
    for row in rows {
        // 本断言锁定的是「出参键集合与实体字段集合一致」(既无缺键也无多余键)，与序列化顺序无关：
        // 真实行经 sorted_keys 归一顺序，即声明作者只比集合不比顺序；出参 JSON 对象的键序由
        // serde_json 的 Map 实现/结构体声明顺序决定，并非本契约要防的漂移面(见文件头回潮判据
        // 「键名漂移→键集合恰为」)。故两侧取集合相等比较，避免参考字面量书写顺序与字典序
        // 不一致(如 publish_date/publisher_id 一处)而误判红——缺键或多键仍判红，未放宽为「非空即可」。
        let actual_keys: std::collections::BTreeSet<String> =
            sorted_keys(row).into_iter().collect();
        let expected_keys: std::collections::BTreeSet<String> =
            expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            actual_keys, expected_keys,
            "oa_announcement::Model 出参键集合必须与实体字段集合逐一对应（缺键/多键判红，键序不参与比较）"
        );
        assert!(row["id"].is_number(), "整数列 id 必须是数字");
        assert!(
            row["publisher_id"].is_number(),
            "整数列 publisher_id 必须是数字"
        );
        assert!(
            row["publish_date"].is_string(),
            "日期列必须序列化为字符串，实际: {}",
            row["publish_date"]
        );
        assert!(row["status"].is_string(), "状态 token 必须是字符串");
    }
    assert_eq!(
        rows[1]["expiry_date"],
        Value::Null,
        "可空列空值必须是 null（键不得缺席）"
    );
    assert_eq!(rows[1]["attachments"], Value::Null, "可空列空值必须是 null");
    assert_eq!(
        rows[1]["visible_scope_config"],
        Value::Null,
        "可空列空值必须是 null"
    );
}

#[test]
fn report_subscription_list_payload_is_typed_paginated_envelope() {
    let resp: ApiResponse<PaginatedResponse<report_subscription::Model>> =
        ApiResponse::success(PaginatedResponse::new(
            vec![sample_subscription(false), sample_subscription(true)],
            2,
            1,
            20,
        ));
    let env = serde_json::to_value(&resp).expect("ApiResponse 必然可序列化");
    assert_envelope_top_level(&env);
    assert_paginated_data_shape(&env["data"]);

    let rows = env["data"]["items"].as_array().expect("items 为数组");
    let expected = [
        "created_at",
        "created_by",
        "export_format",
        "frequency",
        "id",
        "is_enabled",
        "last_run_at",
        "last_run_error",
        "last_run_status",
        "max_retries",
        "name",
        "next_retry_at",
        "next_run_at",
        "parameters",
        "recipients",
        "retry_count",
        "run_count",
        "status",
        "template_id",
        "updated_at",
    ];
    for row in rows {
        assert_eq!(
            sorted_keys(row),
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "report_subscription::Model 出参键必须与实体字段逐一对应"
        );
        assert!(row["id"].is_number(), "整数列 id 必须是数字");
        assert!(
            row["template_id"].is_number(),
            "整数列 template_id 必须是数字"
        );
        assert!(row["run_count"].is_number(), "整数列 run_count 必须是数字");
        assert!(
            row["max_retries"].is_number(),
            "整数列 max_retries 必须是数字"
        );
        assert!(row["recipients"].is_array(), "recipients 落库 JSON 数组");
        assert!(row["status"].is_string(), "状态 token 必须是字符串");
    }
    assert_eq!(
        rows[1]["next_retry_at"],
        Value::Null,
        "可空列空值必须是 null（键不得缺席）"
    );
    assert_eq!(rows[1]["parameters"], Value::Null, "可空列空值必须是 null");
}

/// 递归收集 root 下全部 .rs 文件（std 即可，不为测试引 walkdir）
fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", root.display()))
    {
        let entry = entry.expect("目录项必须可读");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 从 marker（含结尾 `!(`）起做括号配平，返回宏调用整体文本
fn extract_invocation(src: &str, marker_start: usize) -> String {
    let bytes = src.as_bytes();
    let mut idx = bytes[marker_start..]
        .iter()
        .position(|&b| b == b'(')
        .unwrap_or_else(|| panic!("marker 处未找到 '('（扫描逻辑需与源码同步升级，禁止静默跳过）"));
    idx += marker_start;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    let start = idx;
    while idx < bytes.len() {
        let c = bytes[idx];
        if in_str {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return src[start..=idx].to_string();
                    }
                }
                _ => {}
            }
        }
        idx += 1;
    }
    panic!("宏调用括号未闭合（扫描逻辑需与源码同步升级，禁止静默跳过）");
}

#[test]
fn crud_macro_output_typing_ratchet_has_no_value_escape() {
    let backend_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = backend_dir.join("src");

    // 1) 宏定义体内不得再出现塌形出参
    let macro_file = fs::read_to_string(src_dir.join("utils/crud_macro.rs"))
        .expect("crud_macro.rs 必须存在且可读");
    assert!(
        !macro_file.contains("ApiResponse<serde_json::Value>"),
        "crud_macro.rs 宏体中出现 ApiResponse<serde_json::Value>：出参塌回 Value 即门禁盲区回潮"
    );

    // 2) 全 src 扫描两个宏的展开点，出参实参不得为 serde_json::Value，list 实参必须是 PaginatedResponse
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    let markers = ["define_crud_handlers!", "define_tuple_crud_handlers!"];
    let mut invocation_count = 0usize;
    let mut value_output_count = 0usize;
    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        if path.ends_with("crud_macro.rs") {
            continue; // 宏定义文件本身不是展开点
        }
        let content =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {e}", path));
        for marker in &markers {
            let mut search_from = 0usize;
            while let Some(rel) = content[search_from..].find(marker) {
                let at = search_from + rel;
                search_from = at + marker.len();
                // 只认真实调用：marker 后紧跟 `(`（`macro_rules!` 定义与 use 导入不满足）
                let after = content[search_from..].trim_start();
                if !after.starts_with('(') {
                    continue;
                }
                let text = extract_invocation(&content, at);
                invocation_count += 1;
                if text.contains("serde_json::Value") {
                    value_output_count += 1;
                    offenders.push(format!("{:?} 展开点出参含 serde_json::Value", path));
                }
                if !text.contains("PaginatedResponse") {
                    offenders.push(format!(
                        "{:?} 展开点 list 实参不是统一分页信封 PaginatedResponse",
                        path
                    ));
                }
            }
        }
    }

    assert!(
        invocation_count >= KNOWN_INVOCATION_FLOOR,
        "宏展开点解析到 {invocation_count} 处（地板 {KNOWN_INVOCATION_FLOOR}）：宏调用被改名/删除或扫描根失效，解析空集必须判红"
    );
    assert_eq!(
        value_output_count,
        VALUE_OUTPUT_RATCHET_MAX,
        "出参仍为 Value 的宏展开点数量 {value_output_count} 未收敛到上限 {VALUE_OUTPUT_RATCHET_MAX}\
         （棘轮只准降不准升，上限即零容忍）:\n{}",
        offenders.join("\n")
    );
    assert!(
        offenders
            .iter()
            .all(|o| !o.contains("list 实参不是统一分页信封")),
        "存在 list 未用 PaginatedResponse 的展开点（第二套信封回潮）:\n{}",
        offenders.join("\n")
    );
}
