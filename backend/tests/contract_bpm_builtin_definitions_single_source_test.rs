//! 契约锁：出厂内置 BPM 流程定义与两入口（单据快捷审批 / 流程审批中心）办理腿同源
//!
//! 钉住的机理（三条各自对应一个历史缺陷形态）：
//! 1. 编码同源：`bpm_definition_seed` 出厂清单、`bpm_ops::process_key` 常量、
//!    `event_bus_ops/listener.rs` 的 business_type 分发分支、前端
//!    `useBpmApProc.ts` 回跳路由表四处取值逐字符一致。任何一侧漂移
//!    （种子种了 A 发起侧查 B、监听器分支写不进业务表、前端跳进不存在的路由落 404）
//!    都会让对应族别退化为「假可供流程」。
//! 2. 单一办理腿：快捷审批的 approve/reject 在 commit 后必须调
//!    `close_pending_tasks_for_business`（引擎写口唯一）；事件回写分支必须调用
//!    域服务自身方法（状态词表/状态门/审计唯一），禁止裸 SQL 直写业务表——
//!    历史 handle_bpm_generic_status_update 按猜想的 pending_approval/approved
//!    字面量更新多张表，与真实词表（budget draft、payment APPROVING 大写等）
//!    不同源，属静默空转的第二写口，本锁使其无法回潮。
//! 3. 活体真值：八枚出厂定义逐枚「能建实例、能办理、能驳回」，全部经真实
//!    `start_process`/`approve_task` 路径创建并推进，断言回读落库值
//!    （任务态、处理人、时间戳、实例态、单号非空），不接受手写库行冒充。
//!
//! 静态段不依赖活库（CI 常规可跑）；活体段需 `TEST_DATABASE_URL` 指向已迁移
//! PostgreSQL（夹具语义见 tests/test_common）。

mod test_common;

use std::fs;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::bpm_instance as instance_status;
use bingxi_backend::models::status::bpm_task as task_status;
use bingxi_backend::models::{
    ap_invoice, ap_payment_request, ap_payment_request_item, bpm_process_definition,
    bpm_process_instance, bpm_task, role, supplier, user,
};
use bingxi_backend::services::ap_payment_request_service::ApPaymentRequestService;
use bingxi_backend::services::bpm_service::BpmService;
use bingxi_backend::utils::admin_checker::ADMIN_ROLE_CODE;

fn read_rel(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
        .replace('\r', "")
}

/// 剔除以 `//`（含 `///`）起始的行：守卫对象是代码；文档注释里点名被禁形态
/// （说明缺陷机理）是正当用途，不应误报。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取函数体：从签名处起，到下一个同级 fn 标记前为止（与
/// contract_wave11_bpm_task_writeback_case_test.rs 同口径，按函数体收窄判据）。
fn fn_body(src: &str, signature: &str) -> String {
    let anchor = src
        .find(signature)
        .unwrap_or_else(|| panic!("待锁函数不存在: {signature}"));
    let tail = &src[anchor..];
    let next = [
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

/// 族别 ↔（流程编码，business_type）映射：本锁四张表（种子清单 / process_key /
/// 监听分支 / 前端回跳）共同的权威对照，取值与运行代码逐字符同源。
const FAMILIES: &[(&str, &str)] = &[
    ("quotation_approval", "quotation"),
    ("sales_order_approval", "sales_order"),
    ("purchase_order_approval", "purchase_order"),
    ("production_order_approval", "production_order"),
    ("ap_payment_request_approval", "ap_payment_request"),
    ("purchase_return_approval", "purchase_return"),
    ("sales_return_approval", "sales_return"),
    ("export_approval_approval", "export_approval"),
];

/// 已被裁定撤种的「假可供」编码（业务发起侧无 start_process 调用点，普通角色
/// 根本建不出实例）；任何一侧回潮引用即判红，待对应车道接入发起腿后再由该车道
/// 走流程重新入表。
const WITHDRAWN_FAKE_CODES: &[&str] = &[
    "inventory_transfer_approval",
    "inventory_adjustment_approval",
    "inventory_count_approval",
    "budget_approval",
    "ap_invoice_approval",
    "color_price_approval",
    "bom_approval",
];

// ---------------------------------------------------------------------------
// 1) 静态形态锁
// ---------------------------------------------------------------------------

/// 种子出厂清单的 code 集合 == process_key 常量值集合（逐字符同源，数量一致）
#[test]
fn seed_codes_equal_process_key_constants() {
    let seed = read_rel("src/services/bpm_definition_seed.rs");
    let keys_mod = read_rel("src/services/bpm_ops/mod.rs");
    let seed_body = {
        let anchor = seed
            .find("const BUILTIN_APPROVAL_DEFINITIONS")
            .expect("出厂清单不存在");
        let end = seed[anchor..].find("];").expect("出厂清单未闭合");
        seed[anchor..anchor + end].to_string()
    };
    let mut seed_codes: Vec<&str> = Vec::new();
    let mut rest = seed_body.as_str();
    while let Some(pos) = rest.find("code: \"") {
        let after = &rest[pos + "code: \"".len()..];
        let close = after.find('"').expect("code 字面量未闭合");
        seed_codes.push(&after[..close]);
        rest = &after[close..];
    }
    let keys_src = code_only(&keys_mod);
    let key_values: Vec<&str> = {
        let anchor = keys_src
            .find("pub mod process_key {")
            .expect("process_key 模块不存在");
        let body = &keys_src[anchor..];
        let end = body.find("\n}").expect("process_key 模块未闭合");
        body[..end]
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.starts_with("pub const") && l.contains(": &str = \"") {
                    let after = l.split(": &str = \"").nth(1)?;
                    after.split('"').next()
                } else {
                    None
                }
            })
            .collect()
    };
    assert_eq!(
        seed_codes.len(),
        FAMILIES.len(),
        "出厂内置定义应为 {} 枚（真实可发起族别），实得 {seed_codes:?}",
        FAMILIES.len()
    );
    for (code, _bt) in FAMILIES {
        assert!(
            seed_codes.contains(code),
            "种子清单缺少流程编码 {code}（与发起点 process_key 漂移即发起恒 404）"
        );
        assert!(
            key_values.contains(code),
            "process_key 常量缺少 {code}（发起点与种子不同源）"
        );
    }
    for code in seed_codes.clone() {
        assert!(
            FAMILIES.iter().any(|(c, _)| *c == code),
            "种子清单出现对照表外的编码 {code}：新增内置定义必须同步本锁族别对照与监听分支"
        );
    }
}

/// 监听器 business_type 分支与族别对照一致；办理回写禁止裸 SQL 第二写口
#[test]
fn listener_arms_match_families_and_forbid_raw_sql_writeback() {
    let listener = code_only(&read_rel("src/services/event_bus_ops/listener.rs"));
    for (_code, bt) in FAMILIES {
        assert!(
            listener.contains(&format!("\"{bt}\" =>")),
            "事件总线缺少 business_type \"{bt}\" 的 BPM 回写分支（中心办理结果写不回业务表）"
        );
    }
    for forbidden in [
        "handle_bpm_generic_status_update",
        "UPDATE {table}",
        "from_sql_and_values",
    ] {
        assert!(
            !listener.contains(forbidden),
            "监听器禁止再引入通用裸 SQL 状态回写（第二写口绕过域服务状态门与副作用），命中: {forbidden}"
        );
    }
    // 新四族的回写必须调用域服务自身的审批方法（与快捷审批同一办理腿函数）
    let arms = [
        (
            "async fn handle_bpm_ap_payment_request(",
            "ap_payment_request_service::ApPaymentRequestService::new",
            ".approve(",
            ".reject(",
        ),
        (
            "async fn handle_bpm_purchase_return(",
            "purchase_return_service::PurchaseReturnService::new",
            ".approve_return(",
            ".reject_return(",
        ),
        (
            "async fn handle_bpm_sales_return(",
            "sales_return_service::SalesReturnService::new",
            ".approve_return(",
            ".reject_return(",
        ),
        (
            "async fn handle_bpm_export_approval(",
            "export_approval_service::ExportApprovalService::new",
            ".approve(",
            ".reject(",
        ),
    ];
    for (sig, ctor, approve_call, reject_call) in arms {
        let body = fn_body(&listener, sig);
        assert!(
            body.contains(ctor) && body.contains(approve_call) && body.contains(reject_call),
            "回写分支 {sig} 必须经 {ctor} 调用域服务 {approve_call}/{reject_call}（单源办理腿），函数体:\n{body}"
        );
    }
}

/// 撤种的假可供编码不得在任何一侧回潮
#[test]
fn withdrawn_fake_definitions_stay_withdrawn() {
    let seed = read_rel("src/services/bpm_definition_seed.rs");
    let keys_mod = read_rel("src/services/bpm_ops/mod.rs");
    let listener = code_only(&read_rel("src/services/event_bus_ops/listener.rs"));
    for code in WITHDRAWN_FAKE_CODES {
        for (label, src) in [
            ("种子清单", &seed),
            ("process_key", &keys_mod),
            ("监听器", &listener),
        ] {
            assert!(
                !src.contains(&format!("\"{code}\"")) && !src.contains(code),
                "{label} 中不得再引用已撤种假可供编码 {code}（业务发起腿未接入前，种子=只有超管能用的死流程）"
            );
        }
    }
    // 库存调拨等族别的 business_type 分支也不得在监听器回潮（无发起点即假可供）
    for bt in [
        "inventory_transfer",
        "inventory_adjustment",
        "inventory_count",
        "budget",
        "ap_invoice",
        "color_price",
        "bom",
    ] {
        assert!(
            !listener.contains(&format!("\"{bt}\" =>")),
            "监听器不得为无发起点的族别 {bt} 预留回写分支（承载未接入的族属假可供通路）"
        );
    }
}

/// 三条新接入快捷腿：提交启动 BPM、审批/拒绝关闭 BPM 待办，全部单源引用常量
#[test]
fn new_quick_legs_start_and_close_bpm_from_single_source() {
    let cases = [
        (
            "src/services/ap_payment_request_service.rs",
            "pub async fn submit(",
            "async fn start_ap_payment_request_bpm(",
            "process_key::AP_PAYMENT_REQUEST_APPROVAL",
            "\"ap_payment_request\"",
            "pub async fn approve(",
            "pub async fn reject(",
        ),
        (
            "src/services/purchase_return_service.rs",
            "pub async fn submit_return(",
            "async fn start_purchase_return_bpm(",
            "process_key::PURCHASE_RETURN_APPROVAL",
            "\"purchase_return\"",
            "pub async fn approve_return(",
            "pub async fn reject_return(",
        ),
        (
            "src/services/sales_return_service.rs",
            "pub async fn submit_return(",
            "async fn start_sales_return_bpm(",
            "process_key::SALES_RETURN_APPROVAL",
            "\"sales_return\"",
            "pub async fn approve_return(",
            "pub async fn reject_return(",
        ),
    ];
    for (file, submit_sig, start_sig, key_const, bt_literal, approve_sig, reject_sig) in cases {
        let src = code_only(&read_rel(file));
        let submit_body = fn_body(&src, submit_sig);
        assert!(
            submit_body.contains(start_sig.split('(').next().unwrap()),
            "{file}：提交审批腿必须启动 BPM（调用 {start_sig}），否则出厂定义无发起点=假可供"
        );
        assert!(
            submit_body.contains("rollback_") && submit_body.contains("bpm_start_failure_error"),
            "{file}：BPM 启动失败必须补偿回滚单据状态并把真实文案返回提交人"
        );
        let start_body = fn_body(&src, start_sig);
        assert!(
            start_body.contains(key_const) && start_body.contains(bt_literal),
            "{file}：发起的 process_key 必须引 {key_const}、business_type 必须为 {bt_literal}（与监听分支逐字符同源）"
        );
        for fn_sig in [approve_sig, reject_sig] {
            let body = fn_body(&src, fn_sig);
            assert!(
                body.contains("close_pending_tasks_for_business(") && body.contains(bt_literal),
                "{file}::{fn_sig}：快捷审批必须经引擎唯一写口关闭 BPM 待办（两入口状态一致的机理本体）"
            );
        }
    }
}

/// 前端回跳路由表与族别对照同源，且不得再指向不存在的路由（404 空白页根因）
#[test]
fn frontend_return_route_map_matches_families() {
    let src = code_only(&read_rel(
        "../frontend/src/views/bpm/approval/composables/useBpmApProc.ts",
    ));
    let anchor = src
        .find("const BUSINESS_ROUTE_MAP")
        .expect("回跳路由表不存在");
    let end = src[anchor..].find("};").expect("回跳路由表未闭合");
    let table = &src[anchor..anchor + end];
    for (_code, bt) in FAMILIES {
        assert!(
            table.contains(&format!("{bt}:")),
            "前端回跳表缺少族别 {bt}：中心办理后用户停不回来源单据（回跳链路断点）"
        );
    }
    for stale in [
        "inventory_transfer",
        "inventory_adjustment",
        "inventory_count",
        "budget",
        "ap_invoice",
        "color_price",
        "bom",
    ] {
        assert!(
            !table.contains(&format!("{stale}:")),
            "前端回跳表不得保留已撤种族别 {stale} 的映射（指向无从产生的实例数据）"
        );
    }
    assert!(
        !table.contains("/color-price"),
        "回跳路径必须真实存在于 router/index.ts：'/color-price' 是历史断链（正确为 /color-prices/list），跳入即 404"
    );
}

// ---------------------------------------------------------------------------
// 2) 活体真值锁（TEST_DATABASE_URL 指向已迁移 PostgreSQL；CI 串行组执行）
// ---------------------------------------------------------------------------

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间，None 不可达")
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

async fn ensure_admin_role(db: &DatabaseConnection) -> i32 {
    if let Some(r) = role::Entity::find()
        .filter(role::Column::Code.eq(ADMIN_ROLE_CODE))
        .one(db)
        .await
        .unwrap()
    {
        return r.id;
    }
    let now = Utc::now();
    role::ActiveModel {
        name: Set("系统管理员".to_string()),
        code: Set(ADMIN_ROLE_CODE.to_string()),
        is_system: Set(true),
        data_scope: Set("all".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子 admin 角色失败")
    .id
}

async fn seed_user_with_role(db: &DatabaseConnection, name: &str, role_id: i32) -> i32 {
    let now = Utc::now();
    user::ActiveModel {
        username: Set(name.to_string()),
        password_hash: Set("x".repeat(60)),
        real_name: Set(Some(name.to_string())),
        role_id: Set(Some(role_id)),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子用户失败")
    .id
}

/// 激活的单节点审批流程定义（审批人=指定用户），形态与出厂种子构造同源。
async fn seed_active_definition(db: &DatabaseConnection, code: &str, assignee_id: i32) {
    let now = Utc::now();
    let config = serde_json::json!({
        "nodes": [
            { "id": "start", "type": "start_event", "name": "提交审批" },
            { "id": "approve_task", "type": "user_task", "name": "人工审批",
              "assignee_type": "user", "assignee_value": assignee_id },
            { "id": "end", "type": "end_event", "name": "完成" }
        ],
        "edges": [
            { "source": "start", "target": "approve_task" },
            { "source": "approve_task", "target": "end" }
        ]
    });
    bpm_process_definition::ActiveModel {
        name: Set(format!("{code}-内置定义活体锁流程")),
        code: Set(code.to_string()),
        config: Set(Some(config)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子流程定义失败");
}

async fn seed_supplier(db: &DatabaseConnection) -> i32 {
    let tag = unique_tag();
    supplier::ActiveModel {
        supplier_code: Set(format!("BPMSS-{tag}")),
        supplier_name: Set("内置定义契约锁供应商".to_string()),
        supplier_short_name: Set("内锁供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000BPMSS0001X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期非法")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000009".to_string()),
        contact_phone: Set("13800000000".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子供应商失败")
    .id
}

async fn seed_invoice(db: &DatabaseConnection, supplier_id: i32, created_by: i32) -> i32 {
    let tag = unique_tag();
    let now = Utc::now();
    let amt = dec("1000.00");
    let inv = ap_invoice::ActiveModel {
        invoice_no: Set(format!("BPMINV-{tag}")),
        supplier_id: Set(supplier_id),
        invoice_type: Set("PURCHASE".to_string()),
        invoice_date: Set(today()),
        due_date: Set(today()),
        amount: Set(amt),
        paid_amount: Set(Decimal::ZERO),
        unpaid_amount: Set(amt),
        invoice_status: Set(
            bingxi_backend::models::status::finance::ap_invoice::INVOICE_AUDITED.to_string(),
        ),
        created_by: Set(created_by),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子应付发票失败");
    inv.id
}

async fn seed_request_row(db: &DatabaseConnection, supplier_id: i32, created_by: i32) -> i32 {
    let tag = unique_tag();
    let now = Utc::now();
    ap_payment_request::ActiveModel {
        request_no: Set(format!("BPMPRQ-{tag}")),
        request_date: Set(today()),
        supplier_id: Set(supplier_id),
        payment_type: Set("FINAL".to_string()),
        payment_method: Set("TT".to_string()),
        request_amount: Set(dec("50000.00")),
        approval_status: Set("DRAFT".to_string()),
        created_by: Set(created_by),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子付款申请失败")
    .id
}

async fn seed_request_item(db: &DatabaseConnection, request_id: i32, invoice_id: i32) {
    ap_payment_request_item::ActiveModel {
        request_id: Set(request_id),
        invoice_id: Set(invoice_id),
        apply_amount: Set(dec("1000.00")),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子付款申请明细失败");
}

/// 八枚出厂内置定义逐枚「能建实例、能办理、能驳回」的活体真值。
///
/// 建实例走真实 `BpmService::start_process`（定义查找、首任务解析、取号全链路）；
/// 办理/驳回走真实 `approve_task`（行锁、状态门、事件收集全链路）；断言全部回读
/// 落库值。这是「种子不是假可供」的运行时证据：定义解析不出审批人、节点形态不可
/// 执行、推进旁路任何一项都会在此判红。
#[tokio::test]
async fn builtin_seed_creates_and_disposes_every_definition_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let role_id = ensure_admin_role(&db).await;
    let tag = unique_tag();
    let approver_id = seed_user_with_role(&db, &format!("bpm_seed_approver_{tag}"), role_id).await;

    // 出厂幂等种子：真实落库八枚 ACTIVE 定义
    bingxi_backend::services::bpm_definition_seed::seed_builtin_bpm_definitions(&db)
        .await
        .expect("出厂流程定义种子执行失败");
    let seeded = bpm_process_definition::Entity::find()
        .all(&*db)
        .await
        .unwrap();
    let mut seeded_codes: Vec<String> = seeded.iter().map(|d| d.code.clone()).collect();
    seeded_codes.sort();
    let mut expected: Vec<String> = FAMILIES.iter().map(|(c, _)| c.to_string()).collect();
    expected.sort();
    assert_eq!(
        seeded_codes, expected,
        "出厂种子落库编码必须与族别对照完全一致（多一枚=假可供回潮，少一枚=发起 404）"
    );
    // 幂等复跑不重复不报错
    bingxi_backend::services::bpm_definition_seed::seed_builtin_bpm_definitions(&db)
        .await
        .expect("出厂种子幂等复跑失败");
    let count_after = bpm_process_definition::Entity::find()
        .all(&*db)
        .await
        .unwrap()
        .len();
    assert_eq!(
        count_after,
        expected.len(),
        "幂等复跑不得新增流程定义行（ON CONFLICT DO NOTHING 口径）"
    );

    let service = BpmService::new(db.clone());
    for (idx, (code, business_type)) in FAMILIES.iter().enumerate() {
        let business_id: i64 = 900_000 + idx as i64;
        let resp = service
            .start_process(bingxi_backend::models::dto::bpm_dto::StartProcessRequest {
                process_key: code.to_string(),
                business_type: business_type.to_string(),
                business_id,
                title: format!("内置定义活体锁 {code}"),
                initiator_id: approver_id,
                initiator_name: "bpm_seed_approver".to_string(),
                initiator_department_id: None,
                priority: None,
                form_data: None,
                variables: None,
            })
            .await
            .unwrap_or_else(|e| panic!("出厂定义 {code} 应能建实例，start_process 失败: {e}"));
        assert!(
            resp.instance_no.starts_with("BPM"),
            "实例必须经真实取号器落号，实得 {}",
            resp.instance_no
        );
        let tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(resp.instance_id))
            .all(&*db)
            .await
            .unwrap();
        assert_eq!(tasks.len(), 1, "定义 {code} 首任务应恰好一条");
        assert_eq!(
            tasks[0].status.as_deref(),
            Some(task_status::PENDING),
            "定义 {code} 首任务必须以写入方词表值待处理落库"
        );
        assert_eq!(
            tasks[0].actual_handler_id,
            Some(approver_id),
            "定义 {code} 的出厂审批人解析必须命中种子绑定的用户"
        );

        // 前一半族别走「办理」，后一半走「驳回」；两形态都必须真实推进
        let dispose = if idx % 2 == 0 {
            bingxi_backend::services::bpm_ops::task::APPROVE_ACTION
        } else {
            bingxi_backend::services::bpm_ops::task::REJECT_ACTION
        };
        service
            .approve_task(
                bingxi_backend::models::dto::bpm_dto::ApproveTaskRequest {
                    task_id: tasks[0].id,
                    handler_id: approver_id,
                    handler_name: String::new(),
                    action: dispose.to_string(),
                    approval_opinion: Some(format!("内置定义活体锁 {dispose}")),
                    attachment_urls: None,
                },
                Some(approver_id),
            )
            .await
            .unwrap_or_else(|e| panic!("定义 {code} 经引擎办理 {dispose} 失败: {e}"));

        let task = bpm_task::Entity::find_by_id(tasks[0].id)
            .one(&*db)
            .await
            .unwrap()
            .expect("任务行必须仍在");
        let want_task_status = if dispose == bingxi_backend::services::bpm_ops::task::APPROVE_ACTION
        {
            task_status::COMPLETED
        } else {
            task_status::REJECTED
        };
        assert_eq!(
            task.status.as_deref(),
            Some(want_task_status),
            "定义 {code} 办理后任务状态应回读为 {want_task_status}"
        );
        assert_eq!(
            task.actual_handler_id,
            Some(approver_id),
            "定义 {code} 办理后审批人记录必须落已认证操作人"
        );
        assert!(
            task.handled_at.is_some(),
            "定义 {code} 办理后必须落处理时间戳"
        );
        let inst = bpm_process_instance::Entity::find_by_id(resp.instance_id)
            .one(&*db)
            .await
            .unwrap()
            .expect("实例行必须仍在");
        let want_inst_status = if dispose == bingxi_backend::services::bpm_ops::task::APPROVE_ACTION
        {
            instance_status::COMPLETED
        } else {
            instance_status::TERMINATED
        };
        assert_eq!(
            inst.status.as_deref(),
            Some(want_inst_status),
            "定义 {code} 单节点链路办理完实例应回读为 {want_inst_status}"
        );
    }
}

/// 付款申请族：提交即建实例（发起腿活体真值）；快捷通过/驳回把裁决与理由推进进
/// 同一引擎写口（两入口同源收敛的运行时证据）；缺定义时提交显式失败并补偿回滚。
#[tokio::test]
async fn payment_request_bpm_integration_live() {
    let db = Arc::new(test_common::setup_test_db().await);
    let role_id = ensure_admin_role(&db).await;
    let tag = unique_tag();
    let creator = seed_user_with_role(&db, &format!("pr_creator_{tag}"), role_id).await;
    let approver = seed_user_with_role(&db, &format!("pr_approver_{tag}"), role_id).await;
    let supplier_id = seed_supplier(&db).await;
    let invoice_id = seed_invoice(&db, supplier_id, creator).await;

    // 前置：无 ACTIVE 定义时提交必须失败且单据回 DRAFT（不允许「审批中无工单」死单）
    let orphan_id = seed_request_row(&db, supplier_id, creator).await;
    seed_request_item(&db, orphan_id, invoice_id).await;
    let svc = ApPaymentRequestService::new(db.clone());
    let err = svc
        .submit(orphan_id, creator)
        .await
        .expect_err("缺流程定义时提交必须失败（假可供形态）");
    let msg = err.to_string();
    assert!(
        msg.contains("审批流程启动失败") && msg.contains("ap_payment_request_approval"),
        "失败文案必须点名流程标识，实得: {msg}"
    );
    let back = ap_payment_request::Entity::find_by_id(orphan_id)
        .one(&*db)
        .await
        .unwrap()
        .expect("付款申请行必须仍在");
    assert_eq!(
        back.approval_status, "DRAFT",
        "BPM 启动失败后必须补偿回滚到草稿（回读真值），实得 {}",
        back.approval_status
    );

    // 正常链路：定义在册 → 提交建实例 → 快捷通过 → 引擎任务真实完成
    seed_active_definition(&db, "ap_payment_request_approval", approver).await;
    let doc1 = seed_request_row(&db, supplier_id, creator).await;
    seed_request_item(&db, doc1, invoice_id).await;
    let submitted = svc
        .submit(doc1, creator)
        .await
        .expect("提交应成功并启动 BPM");
    assert_eq!(submitted.approval_status, "APPROVING");
    let inst = bpm_process_instance::Entity::find()
        .filter(bpm_process_instance::Column::BusinessType.eq("ap_payment_request"))
        .filter(bpm_process_instance::Column::BusinessId.eq(i64::from(doc1)))
        .one(&*db)
        .await
        .unwrap()
        .expect("提交后必须存在流程实例（发起腿活体真值）");
    let task = bpm_task::Entity::find()
        .filter(bpm_task::Column::InstanceId.eq(inst.id))
        .filter(bpm_task::Column::Status.eq(task_status::PENDING))
        .one(&*db)
        .await
        .unwrap()
        .expect("实例下必须有待办任务");

    svc.approve(doc1, approver).await.expect("快捷审批通过失败");
    let task = bpm_task::Entity::find_by_id(task.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("任务行必须仍在");
    assert_eq!(
        task.status.as_deref(),
        Some(task_status::COMPLETED),
        "快捷审批通过后 BPM 任务必须经引擎唯一写口推进为 completed"
    );
    assert_eq!(
        task.actual_handler_id,
        Some(approver),
        "任务处理人必须记已认证操作人（与中心办理同一写入点）"
    );
    assert!(task.handled_at.is_some(), "任务必须落处理时间戳");
    let inst = bpm_process_instance::Entity::find_by_id(inst.id)
        .one(&*db)
        .await
        .unwrap()
        .expect("实例行必须仍在");
    assert_eq!(inst.status.as_deref(), Some(instance_status::COMPLETED));

    // 快捷驳回：单据 REJECTED、理由落业务列，引擎任务 rejected、理由进 approval_opinion
    let doc2 = seed_request_row(&db, supplier_id, creator).await;
    seed_request_item(&db, doc2, invoice_id).await;
    svc.submit(doc2, creator).await.expect("提交应成功");
    let reason = "金额超预算，付款暂缓";
    svc.reject(doc2, reason.to_string(), approver)
        .await
        .expect("快捷驳回失败");
    let doc2row = ap_payment_request::Entity::find_by_id(doc2)
        .one(&*db)
        .await
        .unwrap()
        .expect("付款申请行必须仍在");
    assert_eq!(doc2row.approval_status, "REJECTED");
    assert_eq!(doc2row.rejected_reason.as_deref(), Some(reason));
    let task2 = bpm_task::Entity::find()
        .filter(
            bpm_task::Column::InstanceId.eq(bpm_process_instance::Entity::find()
                .filter(bpm_process_instance::Column::BusinessType.eq("ap_payment_request"))
                .filter(bpm_process_instance::Column::BusinessId.eq(i64::from(doc2)))
                .one(&*db)
                .await
                .unwrap()
                .expect("实例必须存在")
                .id),
        )
        .all(&*db)
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("驳回链路任务必须存在");
    assert_eq!(
        task2.status.as_deref(),
        Some(task_status::REJECTED),
        "快捷驳回后 BPM 任务必须真实推进为 rejected"
    );
    assert_eq!(
        task2.approval_opinion.as_deref(),
        Some(reason),
        "驳回理由必须进 bpm_task.approval_opinion（引擎侧理由唯一载体）"
    );
}
