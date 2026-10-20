//! 销售订单与采购入库「状态真实性写口门」静态契约锁（不连库，纯 #[test]）。
//!
//! 锁定契约（三处写口，词表唯一事实来源均为 models/status 写入方词表）：
//! - 销售订单建单口 POST，状态键只允许缺省或与服务端定初值 sales_order::CREATE_INITIAL
//!   逐字符相同，直传其余任意状态（含 approved）一律 400，防止绕过提交审批起点与信用门。
//! - 销售订单编辑保存口 PUT，只放行与当前态逐字符相同的幂等写入。"当前态到目标态"
//!   的合法出边集合定义在词表 sales_order::EDIT_OUT_EDGES（当前为空，真实流转全绑
//!   提交、审批、拒绝、取消、发货、完成等带副作用臂的权威操作），任何跳级跃迁 400。
//! - 采购入库建单口，来源采购订单当前态必须属于词表 purchase_order::RECEIPT_ELIGIBLE
//!   （已审批、部分收货）才放行建单，DRAFT、REJECTED 等未批准或已终止态 400 并点名
//!   当前态与允许清单，防止未获批订单直接收货入账并被回写终态。
//!
//! 检测力自证（两条正交通道，均有注入用例，无"写不出检测力锁"的占位断言）：
//! - 函数通道，门函数本体被真实调用，把词表内全部两两相异状态组合逐对注入编辑门，
//!   每一对都必须被拒且错误族为 BUSINESS_ERROR。取值域外（大写借值、空串、他域 token）
//!   必须落 VALIDATION_ERROR 族。入库门逐条注入采购订单九个状态，仅两个收货态放行。
//!   谁把出边集合放宽、把流转门退化成只判取值域、或把词表换成全集白名单，对应用例当场红。
//! - 接线通道，写落点按"门调用先于状态写入或主表插入"的字面序锁定（文本去空白化，
//!   对换行不敏感）。并以变异注入自证检测力，删掉门调用、把门挪到写入之后、
//!   去掉行锁、初值改回读请求状态，同一判据必须逐项报缺。
//!
//! 执行环境：随 backend/tests 静态锁族运行（同 bad_debt_status_vocabulary_test 形态），
//! 不需要 TEST_DATABASE_URL。文案口径按信封机器码断言为主，不断死措辞细节。

use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::status::sales_order as so_status;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::services::so::order::SalesService;
use bingxi_backend::utils::error::AppError;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// 通用辅助
// ---------------------------------------------------------------------------

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败，锁前提失效: {e}", path.display()))
}

/// 去空白化，接线判据不依赖换行与缩进（rustfmt 重排不影响对拍）。
fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 取可外显变体的文案（状态门必须走 business_displayable 与 validation_displayable 家族，
/// 走脱敏通道即红，这是"拒绝可外显、不错报 500"的机器码级证明）。
fn displayable_message(err: &AppError) -> String {
    match err {
        AppError::ValidationErrorDisplayable(msg) | AppError::BusinessErrorDisplayable(msg) => {
            msg.clone()
        }
        other => panic!(
            "状态门拒绝必须落在可外显变体（既有 business_displayable 与 validation_displayable 家族），实得 {other:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// 一、销售订单编辑保存口，流转门函数级检测力（逐对注入全部非法跃迁）
// ---------------------------------------------------------------------------

/// 词表内任意两个不同状态组合都注入 PUT 门，必须逐对被拒（含 draft 改 approved 这条
/// 绕过 BPM 与 MRP 的原始非法跃迁）。同值幂等写必须放行（键在场但未改状态是合法语义）。
#[test]
fn so_edit_gate_rejects_every_cross_state_pair_and_allows_idempotent_write() {
    let mut injected_pairs = 0usize;
    for current in so_status::ALL.iter().copied() {
        for target in so_status::ALL.iter().copied() {
            let result = SalesService::validate_status_edit_write(current, target);
            if current == target {
                assert!(
                    result.is_ok(),
                    "与当前态 {current} 相同的幂等写必须放行（语义是没改状态），实被拒: {result:?}"
                );
            } else {
                injected_pairs += 1;
                let err = match result {
                    Err(err) => err,
                    Ok(()) => panic!("非法跃迁注入 {current} 改 {target} 必须被门拒绝，实被放行"),
                };
                assert_eq!(
                    err.error_code(),
                    "BUSINESS_ERROR",
                    "流转门拒绝必须归 BUSINESS_ERROR（400），不得错报 5xx，实得 {err:?}"
                );
                assert!(
                    matches!(err, AppError::BusinessErrorDisplayable(_)),
                    "流转门文案是公开业务规则，必须可外显，实得 {err:?}"
                );
                let msg = displayable_message(&err);
                assert!(
                    msg.contains(current) && msg.contains(target),
                    "拒绝必须点名当前态 {current} 与目标态 {target}，实际文案: {msg}"
                );
                assert!(
                    msg.contains(&so_status::ALL.join("/")),
                    "拒绝必须点名取值域允许清单（来自词表 ALL，禁止第二套手写清单），实际文案: {msg}"
                );
            }
        }
    }
    // 前提防退化：8 个取值应注入 8×7 对相异组合，若词表被掏空本锁将失去检测力
    assert_eq!(
        injected_pairs,
        so_status::ALL.len() * (so_status::ALL.len() - 1),
        "非法跃迁注入对数必须覆盖词表全部相异组合"
    );
}

/// 取值域门与流转门分属两个错误族，任何一侧把另一侧的越界吞进来都判红：
/// 大写借值（采购词表形态）、带尾空格、他域小写 token、中文 token、空串，
/// 都必须落 VALIDATION_ERROR 而非 BUSINESS_ERROR，且不得放行。
#[test]
fn so_edit_gate_rejects_out_of_domain_as_validation_error() {
    let allow_list = so_status::ALL.join("/");
    for bad in [
        "DRAFT",
        "APPROVED",
        "Pending",
        "approved ",
        "in_review",
        "合格",
        "",
    ] {
        for current in [so_status::DRAFT, so_status::PENDING] {
            let err = SalesService::validate_status_edit_write(current, bad)
                .expect_err("词表外目标取值必须被取值域门拒绝");
            assert_eq!(
                err.error_code(),
                "VALIDATION_ERROR",
                "词表外 {bad:?} 必须归 VALIDATION_ERROR（取值域门），不得混入流转门，实得 {err:?}"
            );
            assert!(
                displayable_message(&err).contains(&allow_list),
                "取值域拒绝必须点名词表 ALL 允许清单"
            );
        }
    }
    // 键缺席是"不写状态列"的合法语义，不得被当成越界值吞掉或判错
    assert!(
        SalesService::validate_create_initial_status(None).is_ok(),
        "建单状态键缺席必须放行（服务端落定初值）"
    );
}

// ---------------------------------------------------------------------------
// 二、销售订单建单口，初值门函数级检测力
// ---------------------------------------------------------------------------

/// 初值定义就是词表 DRAFT 本身（不新造 token）。词表内其余七个取值逐一注入建单门
/// 必须被拒。显式传初值必须放行（与缺省同义，幂等表达不误伤）。
#[test]
fn so_create_gate_forces_server_initial_value() {
    assert_eq!(
        so_status::CREATE_INITIAL,
        so_status::DRAFT,
        "建单定初值必须逐字符等于词表 DRAFT（唯一事实来源，门内不手写取值）"
    );
    assert!(
        SalesService::validate_create_initial_status(Some(so_status::CREATE_INITIAL)).is_ok(),
        "显式传初值 {create} 是合法建单形态，不得被误拦",
        create = so_status::CREATE_INITIAL
    );
    for token in so_status::ALL.iter().copied() {
        if token == so_status::CREATE_INITIAL {
            continue;
        }
        let err = SalesService::validate_create_initial_status(Some(token))
            .expect_err("词表内非初值状态不得经建单直写（POST 直传 approved 的原始缺陷形态）");
        assert_eq!(
            err.error_code(),
            "BUSINESS_ERROR",
            "建单初值门拒绝必须归 BUSINESS_ERROR，实得 {err:?}"
        );
        assert!(
            matches!(err, AppError::BusinessErrorDisplayable(_)),
            "建单门文案必须可外显，实得 {err:?}"
        );
        assert!(
            displayable_message(&err).contains(so_status::CREATE_INITIAL),
            "拒绝必须点名唯一合法初值"
        );
    }
}

// ---------------------------------------------------------------------------
// 三、采购入库建单口，来源订单状态门函数级检测力
// ---------------------------------------------------------------------------

/// 允许清单逐条就是词表常量本身（不多不少、不新造 token）。采购订单九个状态逐一
/// 注入门，仅 APPROVED 与 PARTIAL_RECEIVED 放行，其余七个必须 400 且点名
/// 订单号、当前态与允许清单（含 DRAFT 与 REJECTED 这两个原始缺陷放行的状态）。
#[test]
fn receipt_gate_admits_only_continuable_order_states() {
    assert_eq!(
        po_status::RECEIPT_ELIGIBLE,
        &[po_status::APPROVED, po_status::PARTIAL_RECEIVED],
        "允许清单必须逐条等于词表两个可继续收货常量（放宽或收窄都是口径变更，须走裁定）"
    );
    let all_po_states: &[&str] = &[
        po_status::DRAFT,
        po_status::PENDING_APPROVAL,
        po_status::SUBMITTED,
        po_status::APPROVED,
        po_status::REJECTED,
        po_status::CANCELLED,
        po_status::CLOSED,
        po_status::COMPLETED,
        po_status::PARTIAL_RECEIVED,
    ];
    let allow_list = po_status::RECEIPT_ELIGIBLE.join("/");
    for current in all_po_states {
        let result =
            PurchaseReceiptService::validate_order_status_for_receipt("PO-LOCK-1", current);
        if po_status::RECEIPT_ELIGIBLE.contains(current) {
            assert!(
                result.is_ok(),
                "{current} 是可继续收货态，建单门不得误拦（判错过严即缺陷）"
            );
        } else {
            let err = match result {
                Err(err) => err,
                Ok(()) => panic!(
                    "不可收货态 {current} 建入库单必须被拒（原始缺陷：DRAFT 甚至 REJECTED 也能收货入账）"
                ),
            };
            assert_eq!(
                err.error_code(),
                "BUSINESS_ERROR",
                "来源订单状态门拒绝必须归 BUSINESS_ERROR（400），实得 {err:?}"
            );
            assert!(
                matches!(err, AppError::BusinessErrorDisplayable(_)),
                "来源订单状态门文案必须可外显，实得 {err:?}"
            );
            let msg = displayable_message(&err);
            assert!(
                msg.contains("PO-LOCK-1") && msg.contains(current),
                "拒绝必须点名来源订单号与当前态，实际文案: {msg}"
            );
            assert!(
                msg.contains(&allow_list),
                "拒绝必须点名允许清单（来自词表 RECEIPT_ELIGIBLE），实际文案: {msg}"
            );
        }
    }
}

/// 大小写敏感门：采购词表是大写 token，小写借值（销售订单词表形态）、带尾空格、
/// 中文 token、空串都不得被"归一后放行"，防止两套词表被 lowercase 糊在一起。
#[test]
fn receipt_gate_is_case_sensitive_and_rejects_foreign_tokens() {
    for bad in ["approved", "partial_received", "APPROVED ", "已审批", ""] {
        let err = PurchaseReceiptService::validate_order_status_for_receipt("PO-LOCK-2", bad)
            .expect_err("词表外取值不得被归一后放行（大写词表，大小写敏感）");
        assert_eq!(err.error_code(), "BUSINESS_ERROR", "实得 {err:?}");
        assert!(
            displayable_message(&err).contains(&po_status::RECEIPT_ELIGIBLE.join("/")),
            "拒绝必须点名允许清单"
        );
    }
}

// ---------------------------------------------------------------------------
// 四、写落点接线锁（静态源码对拍 + 变异注入自证检测力）
// ---------------------------------------------------------------------------

/// 销售订单写口接线判据，返回未满足项清单（空即接线成立）。
/// 判据只引用真实代码的字面（去空白化），不引入第二套词表判定。
fn so_wiring_missing(c: &str) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let create_gate = compact("Self::validate_create_initial_status(request.status.as_deref())?;");
    let insert = compact("DocumentNumberGenerator::insert_with_no_retry(");
    let initial_write =
        compact("status: sea_orm::ActiveValue::Set(so_status::CREATE_INITIAL.to_string()),");
    let current_read = compact("let current_status = order.status.clone();");
    let into_model = compact("let mut order_update: sales_order::ActiveModel = order.into();");
    let edit_gate = compact("Self::validate_status_edit_write(&current_status, status)?;");
    let status_write = compact("order_update.status = sea_orm::ActiveValue::Set(status.clone());");
    let edit_uses_vocab = compact("so_status::edit_transition_allowed(current, target)");
    let domain_uses_vocab = compact("so_status::ALL.contains(&target)");

    match (c.find(&create_gate), c.find(&insert)) {
        (Some(g), Some(i)) if g < i => {}
        _ => missing.push("建单门缺失或未先于取号插入（POST 可再直写任意状态）"),
    }
    if !c.contains(&initial_write) {
        missing.push("建单主表状态未写词表定初值 CREATE_INITIAL");
    }
    match (c.find(&current_read), c.find(&into_model)) {
        (Some(r), Some(m)) if r < m => {}
        _ => missing.push("当前态未先于 ActiveModel 转换读取（流转门失去比较基准）"),
    }
    match (c.find(&edit_gate), c.find(&status_write)) {
        (Some(g), Some(w)) if g < w => {}
        _ => missing.push("编辑门缺失或未先于状态列写入（PUT 可再跳级）"),
    }
    if !c.contains(&edit_uses_vocab) {
        missing.push("编辑门未引用词表出边判定 edit_transition_allowed（流转门被摘除）");
    }
    if !c.contains(&domain_uses_vocab) {
        missing.push("取值域门未引用词表 ALL（取值域来源被旁落）");
    }
    missing
}

#[test]
fn so_write_points_are_gated_before_any_status_write() {
    let src = compact(&read_src("services/so/order_crud.rs"));
    assert!(
        so_wiring_missing(&src).is_empty(),
        "销售订单写口接线判据未满足: {:?}",
        so_wiring_missing(&src)
    );
}

/// 检测力自证，对源码注入四种缺陷，接线判据必须逐项报缺。
/// 判据若对任何注入沉默，即"写了个永远不会红的假锁"，本用例当场红。
#[test]
fn so_wiring_checker_detects_injected_defects() {
    let src = compact(&read_src("services/so/order_crud.rs"));

    // 注入一，摘掉建单门（POST 直传状态重新放行）
    let create_gate = compact("Self::validate_create_initial_status(request.status.as_deref())?;");
    let m1 = src.replacen(&create_gate, "", 1);
    assert!(
        so_wiring_missing(&m1)
            .iter()
            .any(|item| item.contains("建单门")),
        "注入（删建单门）未被检出，接线锁是假锁"
    );

    // 注入二，把编辑门挪到状态列写入之后（门立在写入之后等于没门）
    let edit_gate = compact("Self::validate_status_edit_write(&current_status, status)?;");
    let status_write = compact("order_update.status = sea_orm::ActiveValue::Set(status.clone());");
    let m2 = src.replacen(&edit_gate, "", 1);
    let moved_after_write = status_write.clone() + &edit_gate;
    let m2 = m2.replacen(&status_write, &moved_after_write, 1);
    assert!(
        so_wiring_missing(&m2)
            .iter()
            .any(|item| item.contains("编辑门")),
        "注入（编辑门挪到写入之后）未被检出，接线锁是假锁"
    );

    // 注入三，流转门旁落词表出边表，改在门内手写同值比较（第二套定义的雏形，
    // 表与门从此各说各话）。调用点保留，仅把门体对词表判定的引用摘掉。
    let edit_uses_vocab = compact("so_status::edit_transition_allowed(current, target)");
    let m3 = src.replacen(&edit_uses_vocab, &compact("target == current"), 1);
    assert!(
        so_wiring_missing(&m3)
            .iter()
            .any(|item| item.contains("出边")),
        "注入（流转门旁落词表出边表）未被检出，接线锁是假锁"
    );

    // 注入四，建单初值改回读请求状态（unwrap_or 兜底，服务端定初值失效）
    let initial_write =
        compact("status: sea_orm::ActiveValue::Set(so_status::CREATE_INITIAL.to_string()),");
    let m4 = src.replacen(
        &initial_write,
        &compact(
            "status: sea_orm::ActiveValue::Set(request.status.clone().unwrap_or_else(|| so_status::DRAFT.to_string())),",
        ),
        1,
    );
    assert!(
        so_wiring_missing(&m4)
            .iter()
            .any(|item| item.contains("定初值")),
        "注入（初值改回读请求）未被检出，接线锁是假锁"
    );
}

/// 采购入库建单接线判据，来源订单行锁读与建单门必须先于主表插入，门体只认词表清单。
fn receipt_wiring_missing(c: &str) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let locked_read =
        compact("purchase_order::Entity::find_by_id(order_id).lock_exclusive().one(&txn)");
    // 门调用只取到实参列表为止，不钉收尾括号与 `?;`：rustfmt 会把多行调用的参数表补成
    // 带尾逗号形态，钉死闭合括号会让锁对格式重排敏感（判据要锁的是门用了哪一行）。
    let gate_call = compact(
        "Self::validate_order_status_for_receipt(&source_order.order_no, &source_order.order_status",
    );
    let main_insert =
        compact("Self::build_receipt_active_model(&req, receipt_no, user_id).insert(&txn)");
    let eligible_uses_vocab = compact("po_status::RECEIPT_ELIGIBLE.contains(&current)");

    let (Some(r), Some(g), Some(i)) = (
        c.find(&locked_read),
        c.find(&gate_call),
        c.find(&main_insert),
    ) else {
        missing.push("来源订单行锁读或建单门调用缺失（未获批订单可再收货入账）");
        return missing;
    };
    if r > g {
        missing.push("建单门未使用行锁读出的来源订单（可被并发撤单绕开）");
    }
    if g > i {
        missing.push("建单门未先于入库主表插入（拒绝路径会留下已落库的入库单）");
    }
    if !c.contains(&eligible_uses_vocab) {
        missing.push("建单门未引用词表 RECEIPT_ELIGIBLE（允许清单来源被旁落）");
    }
    missing
}

#[test]
fn receipt_create_point_is_gated_before_main_insert() {
    let src = compact(&read_src("services/purchase_receipt_ops/crud.rs"));
    assert!(
        receipt_wiring_missing(&src).is_empty(),
        "入库建单接线判据未满足: {:?}",
        receipt_wiring_missing(&src)
    );
}

/// 检测力自证，摘门、去行锁、门挪到插入之后，三种注入都必须被接线判据检出。
#[test]
fn receipt_wiring_checker_detects_injected_defects() {
    let src = compact(&read_src("services/purchase_receipt_ops/crud.rs"));
    let locked_read =
        compact("purchase_order::Entity::find_by_id(order_id).lock_exclusive().one(&txn)");
    let gate_call = compact(
        "Self::validate_order_status_for_receipt(&source_order.order_no, &source_order.order_status",
    );
    let main_insert =
        compact("Self::build_receipt_active_model(&req, receipt_no, user_id).insert(&txn)");

    // 注入一，摘掉建单门（回到不校验来源订单状态的缺陷形态）
    let m1 = src.replacen(&gate_call, "", 1);
    assert!(
        !receipt_wiring_missing(&m1).is_empty(),
        "注入（删来源订单状态门）未被检出，接线锁是假锁"
    );

    // 注入二，行锁退化（门基于过期快照判定，并发撤单可绕）
    let m2 = src.replacen(
        &locked_read,
        &compact("purchase_order::Entity::find_by_id(order_id).one(&txn)"),
        1,
    );
    assert!(
        !receipt_wiring_missing(&m2).is_empty(),
        "注入（去 lock_exclusive）未被检出，接线锁是假锁"
    );

    // 注入三，门挪到主表插入之后（拒绝路径留下已落库的入库单）
    let m3 = src.replacen(&gate_call, "", 1);
    let gate_after_insert = main_insert.clone() + &gate_call;
    let m3 = m3.replacen(&main_insert, &gate_after_insert, 1);
    assert!(
        !receipt_wiring_missing(&m3).is_empty(),
        "注入（门先于插入被倒置）未被检出，接线锁是假锁"
    );
}

// ---------------------------------------------------------------------------
// 五、出边与允许清单定义的唯一事实来源（词表）与门内禁写字面量
// ---------------------------------------------------------------------------

#[test]
fn transition_and_eligibility_definitions_live_only_in_status_vocabularies() {
    let sales_vocab = compact(&read_src("models/status/sales.rs"));
    assert!(
        sales_vocab.contains(&compact("pub const CREATE_INITIAL: &str = DRAFT;")),
        "建单定初值定义必须在 sales_order 词表模块内且逐字符引用 DRAFT 常量"
    );
    assert!(
        sales_vocab.contains(&compact(
            "pub const EDIT_OUT_EDGES: &[(&str, &[&str])] = &[];"
        )),
        "编辑口合法出边集合定义必须在 sales_order 词表模块内"
    );
    assert!(
        sales_vocab.contains(&compact(
            "pub fn edit_transition_allowed(current: &str, target: &str) -> bool"
        )),
        "出边判定函数必须在词表模块内（写口门与本锁共用同一实现，不存在第二套定义）"
    );
    let purchase_vocab = compact(&read_src("models/status/purchase_inventory.rs"));
    assert!(
        purchase_vocab.contains(&compact(
            "pub const RECEIPT_ELIGIBLE: &[&str] = &[APPROVED, PARTIAL_RECEIVED];"
        )),
        "收货允许清单定义必须在 purchase_order 词表模块内且逐条引用本 mod 常量"
    );

    // 写口文件内不得出现任何状态字符串字面量（比较与门控一律走词表常量或集合）
    let order_crud = read_src("services/so/order_crud.rs");
    let receipt_crud = read_src("services/purchase_receipt_ops/crud.rs");
    let po_states: &[&str] = &[
        po_status::DRAFT,
        po_status::PENDING_APPROVAL,
        po_status::SUBMITTED,
        po_status::APPROVED,
        po_status::REJECTED,
        po_status::CANCELLED,
        po_status::CLOSED,
        po_status::COMPLETED,
        po_status::PARTIAL_RECEIVED,
    ];
    for token in so_status::ALL
        .iter()
        .copied()
        .chain(po_states.iter().copied())
    {
        let literal = format!("\"{token}\"");
        assert!(
            !order_crud.contains(&literal),
            "order_crud.rs 不得手写状态字面量 {literal}（出边与取值唯一来源是词表）"
        );
        assert!(
            !receipt_crud.contains(&literal),
            "purchase_receipt_ops/crud.rs 不得手写状态字面量 {literal}（允许清单唯一来源是词表）"
        );
    }
}
