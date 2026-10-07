//! ①「调拨主单更新允许任意状态写入」—— 真库契约锁（后端线）
//!
//! 被测事实（缺陷）：`services/inv/inventory_move.rs::apply_transfer_main_update`
//! 过去把调用方提交的 `status` 原样 Set 进非空状态列，任何人 PUT 一次就能拿到
//! `completed`（跳过审批/发货/收货，且一分钱库存都不动）——即"绕过状态机权威流转"。
//!
//! 收紧后的口径（本文件逐条钉死）：
//! - 取值域：只认权威词表 `models/status/purchase_inventory.rs::inventory_transfer::ALL`
//!   五个值（词表本身早已存在，只是没有闭合集合；本批补 `ALL`，不新造任何 token）。
//! - 落点唯一：`pending → approved / rejected`（审批）、`approved → shipped`（发货）、
//!   `shipped → completed`（收货）三条边各自绑着只在该操作里才有的必要副作用
//!   （审批人/审批层级/角色门；库存扣减 + 出库流水 + 事件；入库 + 收货量 + 事件），
//!   **编辑保存口（PUT）一条都不能履行** ⇒ 该入口的合法流转集合为空：
//!   只有与当前状态逐字符相同的幂等写入放行。这不是"忘了配白名单"，
//!   而是把"写状态"与"做那件事"重新合回一处（`validate_transfer_status_write` 逐条列了依据）。
//! - 三态口径：键缺席=不改、显式 null=拒（NOT NULL 列，既有门）、同值=幂等保持、
//!   异值=400 BUSINESS_ERROR。**没有任何一种形态被静默丢弃**。
//! - 建单口同理：状态是状态机位，新建只允许初始态（键缺席落缺省 pending）。
//!
//! 每个用例的"改坏什么必红"写在各自函数上方；最后一例是源码扫描防回潮锁：
//! 把门从写落点上摘掉（改回直接 Set）即红，即使有人重排了调用链。

mod test_common;

use bingxi_backend::models::role;
use bingxi_backend::models::status::inventory_transfer as transfer_status;
use bingxi_backend::models::{inventory_transfer, user, warehouse};
use bingxi_backend::services::inv::{
    CreateInventoryTransferRequest, InventoryTransferService, UpdateInventoryTransferRequest,
};
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

use test_common::setup_test_db;

const WH_FROM_ID: i32 = 9411;
const WH_TO_ID: i32 = 9412;
const OPERATOR_ID: i32 = 9413;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_ref_rows(db: &Arc<DatabaseConnection>) {
    for (id, code, name) in [
        (WH_FROM_ID, "WH-W8-FROM", "调拨门-调出仓"),
        (WH_TO_ID, "WH-W8-TO", "调拨门-调入仓"),
    ] {
        warehouse::ActiveModel {
            id: Set(id),
            warehouse_code: Set(code.to_string()),
            name: Set(name.to_string()),
            is_default: Set(false),
            is_active: Set(true),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("种子仓库 {code} 插入失败: {e}"));
    }

    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w8_transfer_gate".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("调拨状态门操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

/// 直接落一行调拨单（绕开服务口，夹具状态可控；列逐一对 models/inventory_transfer.rs 核对）
async fn seed_transfer(
    db: &Arc<DatabaseConnection>,
    transfer_no: &str,
    status: &str,
    notes: Option<&str>,
) -> inventory_transfer::Model {
    inventory_transfer::ActiveModel {
        transfer_no: Set(transfer_no.to_string()),
        from_warehouse_id: Set(WH_FROM_ID),
        to_warehouse_id: Set(WH_TO_ID),
        transfer_date: Set(now()),
        status: Set(status.to_string()),
        total_quantity: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::ZERO),
        notes: Set(notes.map(|s| s.to_string())),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子调拨单 {transfer_no} 插入失败: {e}"))
}

async fn reload(db: &Arc<DatabaseConnection>, id: i32) -> inventory_transfer::Model {
    inventory_transfer::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("调拨单 {id} 应已落库"))
}

/// 断言"状态门 = BUSINESS_ERROR 族且文案外显"（本批已锁口径）
fn expect_business_gate(err: AppError, keyword: &str) {
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门拒绝必须归 BUSINESS_ERROR（HTTP 400），实得 {err:?}"
    );
    let resp = err.to_response();
    assert_eq!(
        resp.code, "BUSINESS_ERROR",
        "出参 code 必须是 BUSINESS_ERROR，信封: {resp:?}"
    );
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "状态门文案是公开业务规则，必须可外显（不得走脱敏 business），实得 {err:?}"
    );
    assert!(
        resp.message.contains(keyword),
        "拒绝文案应点名「{keyword}」，让用户知道该走哪条路，实际: {}",
        resp.message
    );
}

// ---------------------------------------------------------------------------
// 1. 词表外取值：PUT status='draft'（历史界面上出现过的值）必 400 BUSINESS_ERROR
//    改坏什么必红：把 `validate_transfer_status_token` 摘掉 / 词表 `ALL` 少一个值 →
//    该写入被放行（或合法值被误拒），本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn put_status_outside_word_list_is_business_error_and_changes_nothing() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    let row = seed_transfer(
        &db,
        "TRF-W8-TOKEN",
        transfer_status::PENDING,
        Some("原备注"),
    )
    .await;

    let svc = InventoryTransferService::new(db.clone());
    for illegal in ["draft", "executed", "cancelled", "COMPLETED", ""] {
        let err = svc
            .update_transfer(
                row.id,
                UpdateInventoryTransferRequest {
                    status: Some(Some(illegal.to_string())),
                    notes: Some(Some("被状态门连带拦住".to_string())),
                    items: None,
                },
                OPERATOR_ID,
            )
            .await
            .expect_err("词表外的状态取值不得被写入主单（越界即绕过状态机）");
        expect_business_gate(err, "调拨状态取值无效");
    }

    // 零变化：状态与 notes 都必须保持（门在同一事务内先行，不存在"改了一半"）
    let after = reload(&db, row.id).await;
    assert_eq!(after.status, transfer_status::PENDING);
    assert_eq!(after.notes.as_deref(), Some("原备注"));
}

// ---------------------------------------------------------------------------
// 2. 词表内但真流转：pending → completed / approved 经 PUT 一律拒（唯一落点）
//    改坏什么必红：把门换成"只判词表不判流转"（本批缺陷的另一半）→ 本例红；
//    把流转白名单放宽到词表全集 → 本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn put_status_transition_via_edit_entry_is_business_error() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    let svc = InventoryTransferService::new(db.clone());

    let row = seed_transfer(&db, "TRF-W8-JUMP-COMPLETED", transfer_status::PENDING, None).await;
    let err = svc
        .update_transfer(
            row.id,
            UpdateInventoryTransferRequest {
                // 一步跳到终态：库存分文未动，"已完成"却是权威读到的值
                status: Some(Some(transfer_status::COMPLETED.to_string())),
                notes: None,
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect_err("编辑保存口不得驱动状态流转");
    expect_business_gate(err, "编辑保存不改变状态");
    assert_eq!(reload(&db, row.id).await.status, transfer_status::PENDING);

    // 借普通编辑拿到审批结果（绕开分级审批角色门与审批人落库）同样必拒
    let row2 = seed_transfer(&db, "TRF-W8-JUMP-APPROVED", transfer_status::PENDING, None).await;
    let err2 = svc
        .update_transfer(
            row2.id,
            UpdateInventoryTransferRequest {
                status: Some(Some(transfer_status::APPROVED.to_string())),
                notes: None,
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect_err("pending→approved 属审批操作的落点，不得经编辑口写入");
    expect_business_gate(err2, "编辑保存不改变状态");
    let after = reload(&db, row2.id).await;
    assert_eq!(after.status, transfer_status::PENDING);
    assert!(
        after.approved_by.is_none() && after.approved_at.is_none(),
        "被拒的伪审批不得留下审批人痕迹（approved_by/approved_at 只归 approve_transfer）"
    );

    // 回退式流转（shipped → approved）与终态改判（completed → shipped）同样无路可走
    for (current, target, no) in [
        (
            transfer_status::SHIPPED,
            transfer_status::APPROVED,
            "TRF-W8-BACK",
        ),
        (
            transfer_status::COMPLETED,
            transfer_status::SHIPPED,
            "TRF-W8-UNDO",
        ),
    ] {
        let r = seed_transfer(&db, no, current, None).await;
        let e = svc
            .update_transfer(
                r.id,
                UpdateInventoryTransferRequest {
                    status: Some(Some(target.to_string())),
                    notes: None,
                    items: None,
                },
                OPERATOR_ID,
            )
            .await
            .expect_err("状态流转不可逆行、不可经编辑口改判");
        expect_business_gate(e, "调拨单状态只能通过");
        assert_eq!(reload(&db, r.id).await.status, current);
    }
}

// ---------------------------------------------------------------------------
// 3. 三态口径（键缺席=不改 / 同值=幂等保持 / 显式 null=拒）
//    改坏什么必红：把状态字段"静默丢弃"（本批禁止的第三种修法）→ 幂等用例里 notes
//    仍能改但"同值 PUT"变红吗？不会——所以本例同时钉 notes 生效 + 状态保持，
//    并让"缺席即保持"与"显式 null 即拒"成对出现：任何把三态压成两态的实现都红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn put_status_three_state_semantics_are_preserved() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    let svc = InventoryTransferService::new(db.clone());
    let row = seed_transfer(
        &db,
        "TRF-W8-THREE",
        transfer_status::REJECTED,
        Some("旧备注"),
    )
    .await;

    // (a) 键缺席 ⇒ 状态保持、备注照常更新（编辑能力没被删掉）
    let detail = svc
        .update_transfer(
            row.id,
            UpdateInventoryTransferRequest {
                status: None,
                notes: Some(Some("只改备注".to_string())),
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect("仅改备注的编辑必须成功（不得为省事删掉更新能力）");
    assert_eq!(
        detail.status,
        transfer_status::REJECTED,
        "键缺席必须保持原状态"
    );
    let after = reload(&db, row.id).await;
    assert_eq!(after.notes.as_deref(), Some("只改备注"));

    // (b) 有值且与当前逐字符相同 ⇒ 幂等放行（"没改状态"的正确表达）
    let detail2 = svc
        .update_transfer(
            row.id,
            UpdateInventoryTransferRequest {
                status: Some(Some(transfer_status::REJECTED.to_string())),
                notes: Some(Some("幂等提交".to_string())),
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect("同值幂等写入不得被当成流转而拒绝");
    assert_eq!(detail2.status, transfer_status::REJECTED);
    assert_eq!(reload(&db, row.id).await.notes.as_deref(), Some("幂等提交"));

    // (c) 显式 null ⇒ NOT NULL 列的清空是调用方错误，既有门不变（回归锁）
    let err = svc
        .update_transfer(
            row.id,
            UpdateInventoryTransferRequest {
                status: Some(None),
                notes: None,
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect_err("status 映射非 Option 列，显式 null 必须被拒");
    expect_business_gate(err, "调拨状态不能清空");
    assert_eq!(reload(&db, row.id).await.status, transfer_status::REJECTED);
}

// ---------------------------------------------------------------------------
// 4. 建单口：状态只能是被授权词表内的初始态（键缺席落缺省 pending）
//    改坏什么必红：把 `validate_transfer_initial_status` 摘掉 → 以 completed 建单成功 → 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn create_transfer_rejects_non_initial_and_out_of_word_list_status() {
    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    let svc = InventoryTransferService::new(db.clone());

    // 两类拒绝分属两条门：词表外 ⇒ 取值域门；词表内但非初始态 ⇒ 建单初始态门。
    // 各自点名自己的文案，防止"一刀切成同一句话"把两条门混成一条（混了就没人发现少了一道）。
    for (submitted, keyword) in [
        (transfer_status::APPROVED, "初始状态只能是待审批"),
        (transfer_status::SHIPPED, "初始状态只能是待审批"),
        (transfer_status::COMPLETED, "初始状态只能是待审批"),
        ("draft", "调拨状态取值无效"),
    ] {
        let err = svc
            .create_transfer(
                CreateInventoryTransferRequest {
                    from_warehouse_id: Some(WH_FROM_ID),
                    to_warehouse_id: Some(WH_TO_ID),
                    transfer_date: None,
                    status: Some(submitted.to_string()),
                    notes: Some("建单期状态门".to_string()),
                    items: None,
                },
                OPERATOR_ID,
            )
            .await
            .expect_err("建单不得指定审批后/发货后/终态或词表外状态");
        expect_business_gate(err, keyword);
    }

    // 被拒的建单必须零落库（门在任何 DB 访问之前）
    let live = inventory_transfer::Entity::find()
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(
        live.is_empty(),
        "建单状态门必须在写库之前拦住，实得 {} 行",
        live.len()
    );

    // 对照：不送状态键 ⇒ 正常建单，初始态由权威缺省落 pending（读回为证）
    let created = svc
        .create_transfer(
            CreateInventoryTransferRequest {
                from_warehouse_id: Some(WH_FROM_ID),
                to_warehouse_id: Some(WH_TO_ID),
                transfer_date: None,
                status: None,
                notes: Some("正常建单".to_string()),
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect("不指定状态的建单必须成功");
    assert_eq!(created.status, transfer_status::PENDING);
    assert_eq!(
        reload(&db, created.id).await.status,
        transfer_status::PENDING
    );

    // 显式送初始态 pending 同样合法（幂等表达，不被误伤）
    let explicit = svc
        .create_transfer(
            CreateInventoryTransferRequest {
                from_warehouse_id: Some(WH_FROM_ID),
                to_warehouse_id: Some(WH_TO_ID),
                transfer_date: None,
                status: Some(transfer_status::PENDING.to_string()),
                notes: None,
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect("显式 pending 是合法建单形态");
    assert_eq!(explicit.status, transfer_status::PENDING);
}

// ---------------------------------------------------------------------------
// 5. 合法流转仍归权威操作（证明门没有拦腰截断权威路径）+ HTTP 层 400 形态
//    改坏什么必红：把编辑口的门误伤到 approve_transfer（例如在权威函数里也判同值）
//    → 本例红；把 BUSINESS_ERROR 改成其它族 → 第 (3) 段 HTTP 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn authoritative_approve_still_transitions_and_edit_entry_returns_400_over_http() {
    use axum::{
        Router,
        body::Body,
        http::{Method, Request, StatusCode, header},
        response::Response,
        routing::put,
    };
    use bingxi_backend::container::AppState;
    use bingxi_backend::handlers::inventory_transfer_handler;
    use bingxi_backend::middleware::auth_context::AuthContext;
    use serde_json::{Value as Json, json};
    use tower::ServiceExt;

    let db = Arc::new(setup_test_db().await);
    seed_ref_rows(&db).await;
    let svc = InventoryTransferService::new(db.clone());

    // (1) 权威落点：pending → approved（审批操作，附带审批人与层级落库）
    let admin_role = role::Entity::find()
        .filter(role::Column::Code.eq("admin"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("迁移种子应存在 admin 角色（roles 是夹具的不清空参照表）");
    let row = seed_transfer(&db, "TRF-W8-APPROVE", transfer_status::PENDING, None).await;
    let approved = svc
        .approve_transfer(
            row.id,
            true,
            Some("同意调拨".to_string()),
            OPERATOR_ID,
            Some(admin_role.id),
        )
        .await
        .expect("审批操作必须成功：编辑口的状态门不得波及权威流转");
    assert_eq!(approved.status, transfer_status::APPROVED);
    let after = reload(&db, row.id).await;
    assert_eq!(after.status, transfer_status::APPROVED, "回读必须已流转");
    assert_eq!(
        after.approved_by,
        Some(OPERATOR_ID),
        "审批人由权威函数落库（这正是 PUT 不能替它的原因）"
    );
    assert!(after.approved_at.is_some());

    // (2) 到了 approved 之后，经 PUT 直接写 shipped（跳过发货与库存扣减）仍必拒
    let err = svc
        .update_transfer(
            row.id,
            UpdateInventoryTransferRequest {
                status: Some(Some(transfer_status::SHIPPED.to_string())),
                notes: None,
                items: None,
            },
            OPERATOR_ID,
        )
        .await
        .expect_err("approved→shipped 归发货操作（要扣库存、写流水、发事件）");
    expect_business_gate(err, "编辑保存不改变状态");
    assert_eq!(reload(&db, row.id).await.status, transfer_status::APPROVED);

    // (3) 权威落点之二：pending → rejected（同一审批操作的另一分支）
    let row2 = seed_transfer(&db, "TRF-W8-REJECT", transfer_status::PENDING, None).await;
    let rejected = svc
        .approve_transfer(
            row2.id,
            false,
            Some("额度不符".to_string()),
            OPERATOR_ID,
            None,
        )
        .await
        .expect("驳回分支必须成功");
    assert_eq!(rejected.status, transfer_status::REJECTED);
    assert_eq!(reload(&db, row2.id).await.status, transfer_status::REJECTED);

    // (4) HTTP 层形态锁：非法写入回 400 + code=BUSINESS_ERROR + 真实文案外显（非脱敏常量）
    async fn inject_auth(
        auth: axum::extract::State<AuthContext>,
        mut request: Request<Body>,
        next: axum::middleware::Next,
    ) -> Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/inventory/transfers/{id}",
            put(inventory_transfer_handler::update_transfer),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            AuthContext {
                user_id: OPERATOR_ID,
                username: "w8_transfer_gate".to_string(),
                role_id: Some(admin_role.id),
                department_id: Some(1),
                data_scope: Some("all".to_string()),
                dept_ids: None,
                dept_member_user_ids: None,
            },
            inject_auth,
        ));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/inventory/transfers/{}", row.id))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "status": "completed" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "状态门必须 400");
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let v: Json = serde_json::from_slice(&bytes).expect("失败信封应是 JSON");
    assert_eq!(v["code"], "BUSINESS_ERROR", "信封: {v}");
    let msg = v["message"].as_str().unwrap_or_default();
    assert_ne!(msg, "请求参数验证失败", "必须外显真实原因，不得是脱敏常量");
    assert!(
        msg.contains("审批") && msg.contains("发货") && msg.contains("收货"),
        "文案要告诉用户该走哪条路，实际: {msg}"
    );
    assert_eq!(reload(&db, row.id).await.status, transfer_status::APPROVED);
}

// ---------------------------------------------------------------------------
// 6. 防回潮源码扫描锁（"把这道门删掉就必红"的结构性证明）
//    覆盖三处：写落点必须过门；词表必须有闭合集合且值数不变；发货/收货前置状态
//    不得再退回裸字面量（词表单一来源）。
// ---------------------------------------------------------------------------
#[test]
fn source_scan_transfer_status_gate_is_wired_at_the_write_point() {
    let move_src = std::fs::read_to_string("src/services/inv/inventory_move.rs")
        .expect("读取 inventory_move.rs 失败");
    let body_start = move_src
        .find("async fn apply_transfer_main_update")
        .expect("apply_transfer_main_update 必须仍在（更新能力不得被删）");
    let body = &move_src[body_start..];
    let body = body.split("\n    /// ").next().unwrap_or(body);
    assert!(
        body.contains("Self::validate_transfer_status_write("),
        "状态写落点必须就地过门（改回直接 Set 即视为把门摘掉，#246 ① 回归）"
    );
    assert!(
        body.contains("lock_exclusive()"),
        "状态门必须以事务内加锁读到的当前值为准（用事务外过期快照判定＝可被并发审批绕开）"
    );

    let batch_src =
        std::fs::read_to_string("src/services/inv/batch.rs").expect("读取 batch.rs 失败");
    for bare in ["!= \"approved\"", "!= \"shipped\""] {
        assert!(
            !batch_src.contains(bare),
            "发货/收货前置状态不得用裸字面量（词表单一来源），仍命中 {bare}"
        );
    }

    let status_src = std::fs::read_to_string("src/models/status/purchase_inventory.rs")
        .expect("读取 purchase_inventory.rs 失败");
    let module = status_src
        .split("pub mod inventory_transfer {")
        .nth(1)
        .expect("inventory_transfer 词表模块必须在")
        .split("\n}")
        .next()
        .unwrap();
    assert!(
        module.contains("pub const ALL"),
        "调拨词表必须有闭合取值集合 ALL（取值域校验的单一来源）"
    );
    assert_eq!(
        transfer_status::ALL,
        &[
            transfer_status::PENDING,
            transfer_status::APPROVED,
            transfer_status::REJECTED,
            transfer_status::SHIPPED,
            transfer_status::COMPLETED,
        ],
        "取值集合必须逐条等于既有五常量：新增/删除状态是产品口径变更，须走裁定而不是随手改这里"
    );
}
