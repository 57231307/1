//! 后端安全「产地证建单人 / 色卡发放人只认服务端会话」行为级 + 形态级证明（批 3 A 组）
//!
//! 锁定的契约：
//! - `services/certificate_of_origin_service.rs::create` 的审计归属列 `created_by`
//!   （`certificate_of_origin.created_by`，NOT NULL 整数列）**唯一来源是调用方按服务端会话
//!   （`AuthContext.user_id`）传入的 `user_id` 形参**；请求 DTO `CreateCertificateReq`
//!   不再承载任何身份入参字段。
//!   本域当前**没有** create 端点（`routes/export_inspection_routes.rs` 只注册了两条 GET，
//!   `certificate_of_origin_handler.rs` 只有 list/get），故按批 1「环保税排放登记」同款口径
//!   走**服务层直证**：真库真落库后直读实体列，而不是伪造一条不存在的 HTTP 路径。
//! - 色卡发放 `POST /color-cards/issues` 的 `issued_by` **已是正确写法**（handler 用
//!   `auth.user_id` 构造 `IssueParams.issued_by`，请求 DTO `IssueColorCardDto` 无该字段）；
//!   本锁把它钉成形态，防止有人把发放人重新下放给请求体。
//!
//! 证明形态（夹具范式同 contract_wave11_created_by_from_session_batch1_test）：
//! ① 会话注入用户 A（=9511）作为唯一身份来源；
//! ② 伪造用户 B（=9472）作为「绝不允许出现在归属列里的值」显式反断言；
//! ③ 动作必须成功（身份不再由入参承载，缺省即可建单）；
//! ④ 直读实体行断言 `row.created_by == SESSION_A` 且 `!= FORGED_B`。
//!
//! 另含两条形态锁：
//! - `batch3a_request_dtos_have_no_identity_field`：两个请求 DTO 结构体不再含身份入参字段；
//! - `batch3a_identity_sources_are_the_session`：落库表达式与 handler 取值表达式形状锁。
//!
//! 检测力（改坏源码即红的点位）：
//! - service 退回 `created_by: Set(data.created_by)` / 加 `unwrap_or` 兜底 → 行为锁直读断言红；
//! - DTO 重新加回身份字段 → 形态锁当场红；
//! - handler 把 `issued_by` 改回从 body 取（`dto.issued_by`）→ 形状锁当场红；
//! - 形态锁解析不到目标结构体 → 地板断言当场红（禁止空集合=绿）。
//!
//! 批 3c 就地扩写（同族末段收口，不另建重名文件）新增两个站点：
//! - `POST /production/production-recipes`（`services/production_recipe_ops/recipe_crud.rs::create`）
//! - `POST /production/production-recipes/:id/additions`（`services/production_recipe_ops/addition.rs::create`）
//!   两站的 `issued_by`（开单人，可空 INTEGER 列）原先由请求体 `req.issued_by` 决定，现与 `created_by`
//!   同源取 handler 传入的 `AuthContext.user_id`；`CreateProductionRecipeRequest` /
//!   `CreateProductionRecipeAdditionRequest` 已删除 `issued_by` 入参字段（不留「接受但忽略」的 shim，
//!   本仓 DTO 无 `deny_unknown_fields`，老客户端多传该键由 serde 静默忽略）。

mod test_common;

use bingxi_backend::models::certificate_of_origin;
use bingxi_backend::models::production_recipe;
use bingxi_backend::models::production_recipe::RecipeMaterialItem;
use bingxi_backend::models::production_recipe_addition::{
    self, AdditionMaterialItem, Model as AdditionModel,
};
use bingxi_backend::models::status::production_recipe as recipe_status;
use bingxi_backend::models::status::production_recipe_addition as addition_status;
use bingxi_backend::services::certificate_of_origin_service::{
    CertificateOfOriginService, CreateCertificateReq,
};
use bingxi_backend::services::production_recipe_service::{
    CreateProductionRecipeAdditionRequest, CreateProductionRecipeRequest,
    ProductionRecipeAdditionService, ProductionRecipeService,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use std::sync::Arc;
use test_common::setup_test_db;

/// 会话用户 A：建单人唯一合法取值（私有 id 段 951x，与同族锁保持同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：永远不允许落进任何归属列的值
const FORGED_B: i32 = 9472;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("测试夹具日期常量必须合法")
}

/// 读 backend crate 内源码（相对 `CARGO_MANIFEST_DIR`）；取不到即判红，禁止静默跳过。
fn read_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()))
}

/// 在源码文本中定位 `pub struct NAME { ... }` 的结构体体文本；找不到返回 None，由调用方判红。
fn struct_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub struct {name} {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

/// 断言归属列取会话用户 A、绝不取伪造用户 B（列 NOT NULL，取值为裸 i32）。
fn assert_created_by_is_session(column: i32, what: &str) {
    assert_eq!(
        column, SESSION_A,
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {column}"
    );
    assert_ne!(
        column, FORGED_B,
        "直读实体：伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

// =========================================================
// 域 1：出口产地证建单（服务层直证：本域无 create 端点）
// =========================================================

#[tokio::test]
async fn certificate_of_origin_created_by_comes_from_caller_user_not_body() {
    let db = setup_test_db().await;
    let service = CertificateOfOriginService::new(Arc::new(db.clone()));
    // 建单入参已不含任何身份字段：唯一可能的归属来源就是 create 的 user_id 形参。
    let req = CreateCertificateReq {
        certificate_no: format!(
            "CO-W11CB3A-{}",
            Utc::now()
                .timestamp_nanos_opt()
                .expect("夹具取纳秒时间戳必须成功")
        ),
        inspection_id: None,
        product_name: "w11-cb3a-goods".to_string(),
        hs_code: "6302.21".to_string(),
        destination_country: "Germany".to_string(),
        quantity: Decimal::from(100_u8),
        unit: "M".to_string(),
        invoice_amount: Some(Decimal::from(2500_u32)),
        certificate_type: "FORM-A".to_string(),
        issue_date: date(2026, 10, 1),
        expiry_date: Some(date(2027, 1, 1)),
        remarks: Some("_createdby_b3a_lock".to_string()),
    };
    let created = service
        .create(req, SESSION_A)
        .await
        .expect("会话用户建单必须成功（created_by 不再是入参）");
    assert_created_by_is_session(created.created_by, "产地证（出参）");

    let row = certificate_of_origin::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap_or_else(|e| panic!("产地证 直读失败: {e}"))
        .unwrap_or_else(|| panic!("产地证 必须存在"));
    assert_created_by_is_session(row.created_by, "产地证（回读）");
    // 业务列不受身份改造牵连：状态与数量仍由服务真实写入
    assert_eq!(row.status, "active", "业务状态列不受身份改造牵连");
    assert_eq!(row.quantity, Decimal::from(100_u8), "业务数量列不受牵连");
    assert_eq!(row.product_name, "w11-cb3a-goods", "业务品名列不受牵连");
    // created_at/updated_at 是 NOT NULL 且建表 DDL 无默认值的两列：漏写即 INSERT 直接失败，
    // 两列同源（同一次 Utc::now()），故断等值而不是断与测试进程时钟的关系（避免跨主机时钟偏移假红）。
    assert_eq!(
        row.created_at, row.updated_at,
        "建单的两个审计时间戳必须由同一次写入同源落库"
    );
}

// =========================================================
// 形态锁 1：两个请求 DTO 不再承载身份入参字段
// =========================================================

#[test]
fn batch3a_request_dtos_have_no_identity_field() {
    // 任何身份入参键都不允许出现在建单 DTO 结构体体内（四个键一并钉，防止换个名字复活）
    const IDENTITY_KEYS: [&str; 4] = ["created_by", "issued_by", "user_id", "operator_id"];
    // (源码相对路径, DTO 名)
    let targets: [(&str, &str); 4] = [
        (
            "src/services/certificate_of_origin_service.rs",
            "CreateCertificateReq",
        ),
        ("src/handlers/color_card/issue.rs", "IssueColorCardDto"),
        (
            "src/services/production_recipe_service.rs",
            "CreateProductionRecipeRequest",
        ),
        (
            "src/services/production_recipe_service.rs",
            "CreateProductionRecipeAdditionRequest",
        ),
    ];
    let mut parsed = 0usize;
    for (file, dto) in targets.iter() {
        let src = read_src(file);
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（{file}，禁止空集合=绿）"));
        parsed += 1;
        for key in IDENTITY_KEYS.iter() {
            assert!(
                !body.contains(*key),
                "请求 DTO {dto}（{file}）不应承载身份入参字段 {key}，实际体: {body}"
            );
        }
    }
    assert_eq!(
        parsed,
        targets.len(),
        "四个请求 DTO 必须全部被解析到（检测力地板），实际 {parsed}"
    );
}

// =========================================================
// 形态锁 2：归属列取值来源只能是服务端会话（形状钉死，防回潮）
// =========================================================

#[test]
fn batch3a_identity_sources_are_the_session() {
    let co_src = read_src("src/services/certificate_of_origin_service.rs");
    assert!(
        co_src.contains("user_id: i32"),
        "产地证 create 必须以 user_id: i32 形参接收会话用户（范式②），实际未见该形参"
    );
    assert!(
        co_src.contains("created_by: Set(user_id)"),
        "产地证建单人必须直接落会话值（列 NOT NULL，范式④），实际未命中 Set(user_id)"
    );
    assert!(
        !co_src.contains("created_by: Set(data.created_by)"),
        "产地证建单人不得再回退为请求体决定（根因形态 Set(data.created_by)）"
    );

    let issue_src = read_src("src/handlers/color_card/issue.rs");
    assert!(
        issue_src.contains("let user_id = auth.user_id as i64;"),
        "色卡发放人必须从 AuthContext 会话取（范式③），实际未见该取值"
    );
    assert!(
        issue_src.contains("issued_by: user_id,"),
        "色卡 IssueParams.issued_by 必须由会话值构造（范式②③），实际未见该构造行"
    );
    assert!(
        !issue_src.contains("issued_by: dto.issued_by"),
        "色卡发放人不得从请求 DTO 取（回退形态 issued_by: dto.issued_by）"
    );

    let svc_src = read_src("src/services/color_card_issue_service.rs");
    assert!(
        svc_src.contains("issued_by: Set(params.issued_by)"),
        "色卡 service 必须保持「只透传 handler 构造的 IssueParams」形态，不得自造身份"
    );
}

// =========================================================
// 域 3（批 3c 追加）：大货处方 / 加料处方的开单人 issued_by
// 与建单人 created_by 都只认服务端会话（行为级直证 + 形态锁）
// =========================================================

/// 断言可空归属列取会话用户 A、绝不取伪造用户 B
/// （`production_recipe.issued_by` / `production_recipe_addition.issued_by`
/// 建表 DDL 为可空 INTEGER，migration/src/domain/v15/mod.rs:3429、:3451）。
fn assert_identity_column_is_session(column: Option<i32>, what: &str) {
    assert_eq!(
        column,
        Some(SESSION_A),
        "直读实体：{what} 必须等于会话用户 A，实际 {column:?}"
    );
    assert_ne!(
        column,
        Some(FORGED_B),
        "直读实体：伪造的 B 绝不许落进 {what}"
    );
}

/// 建一张可审核的大货处方入参（含一条明细，满足 approve 的前置校验）。
/// 注意：结构体内已无任何身份入参字段 —— 归属列唯一可能的来源就是 create 的会话形参。
fn recipe_req(remarks: &str) -> CreateProductionRecipeRequest {
    CreateProductionRecipeRequest {
        work_order_id: None,
        dye_batch_id: None,
        source_recipe_id: None,
        lab_dip_resample_id: None,
        customer_id: None,
        color_no: Some("W11-CB3C-RED".to_string()),
        fabric_name: Some("w11-cb3c-fabric".to_string()),
        fabric_spec: None,
        fabric_width: None,
        gram_weight: None,
        fabric_weight: Decimal::from(120_u16),
        equipment_no: None,
        liquor_ratio: "1:8".to_string(),
        bath_volume: None,
        adjustment_factor: None,
        recipe_detail: Some(vec![RecipeMaterialItem {
            material_code: "DYE-CB3C-01".to_string(),
            material_name: "w11-cb3c-dye".to_string(),
            concentration: Some(Decimal::from(2_u8)),
            unit: "kg".to_string(),
            amount: Decimal::from(3_u8),
            category: "dye".to_string(),
        }]),
        total_dye_cost: None,
        total_auxiliary_cost: None,
        remarks: Some(remarks.to_string()),
    }
}

/// 真实链路把大货处方推到 approved（不手改状态列，避免绕开被测状态机）
async fn approve_recipe(service: &ProductionRecipeService, id: i32) -> production_recipe::Model {
    service
        .approve(id, SESSION_A)
        .await
        .unwrap_or_else(|e| panic!("会话用户审核大货处方必须成功: {e}"))
}

#[tokio::test]
async fn production_recipe_issued_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let service = ProductionRecipeService::new(Arc::new(db.clone()));
    let created = service
        .create(recipe_req("_createdby_b3c_recipe"), SESSION_A)
        .await
        .expect("大货处方建单必须成功（issued_by/created_by 不再是入参）");
    assert_identity_column_is_session(created.issued_by, "大货处方 issued_by（出参）");
    assert_identity_column_is_session(created.created_by, "大货处方 created_by（出参）");

    let row = production_recipe::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap_or_else(|e| panic!("大货处方 直读失败: {e}"))
        .unwrap_or_else(|| panic!("大货处方 必须存在"));
    assert_identity_column_is_session(row.issued_by, "大货处方 issued_by（回读）");
    assert_identity_column_is_session(row.created_by, "大货处方 created_by（回读）");
    // 业务列不受身份改造牵连：状态、浴比、布重仍由服务真实写入
    assert_eq!(row.status, recipe_status::DRAFT, "业务状态列不受牵连");
    assert_eq!(row.liquor_ratio, "1:8", "业务浴比列不受牵连");
    assert_eq!(
        row.fabric_weight,
        Decimal::from(120_u16),
        "业务备布重量列不受牵连"
    );
    assert_eq!(
        row.color_no.as_deref(),
        Some("W11-CB3C-RED"),
        "业务色号列不受牵连"
    );
    assert!(
        row.approved_by.is_none(),
        "新建草稿的审批人列必须为空（审核身份不得在建单时被写入）"
    );
}

#[tokio::test]
async fn production_recipe_addition_issued_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let recipe_service = ProductionRecipeService::new(Arc::new(db.clone()));
    let addition_service = ProductionRecipeAdditionService::new(Arc::new(db.clone()));
    // 前置：加料处方要求关联大货处方为 approved —— 走真实建单 + 真实审核链路铺数据
    let recipe = recipe_service
        .create(recipe_req("_createdby_b3c_addition_parent"), SESSION_A)
        .await
        .expect("加料前置的大货处方建单必须成功");
    let approved = approve_recipe(&recipe_service, recipe.id).await;
    assert_eq!(
        approved.status,
        recipe_status::APPROVED,
        "前置数据必须真的进入 approved"
    );

    // 加料建单入参同样不含身份字段
    let req = CreateProductionRecipeAdditionRequest {
        production_recipe_id: approved.id,
        work_order_id: None,
        dye_batch_id: None,
        addition_reason: Some("色差补料".to_string()),
        addition_detail: Some(vec![AdditionMaterialItem {
            material_code: "AUX-CB3C-01".to_string(),
            material_name: "w11-cb3c-auxiliary".to_string(),
            amount: Decimal::from(5_u8),
            unit: "kg".to_string(),
            category: "auxiliary".to_string(),
        }]),
        total_cost: None,
        remarks: Some("_createdby_b3c_addition".to_string()),
    };
    let created: AdditionModel = addition_service
        .create(req, SESSION_A)
        .await
        .expect("加料处方建单必须成功（issued_by/created_by 不再是入参）");
    assert_identity_column_is_session(created.issued_by, "加料处方 issued_by（出参）");
    assert_identity_column_is_session(created.created_by, "加料处方 created_by（出参）");

    let row = production_recipe_addition::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap_or_else(|e| panic!("加料处方 直读失败: {e}"))
        .unwrap_or_else(|| panic!("加料处方 必须存在"));
    assert_identity_column_is_session(row.issued_by, "加料处方 issued_by（回读）");
    assert_identity_column_is_session(row.created_by, "加料处方 created_by（回读）");
    // 业务列不受牵连
    assert_eq!(row.status, addition_status::DRAFT, "加料业务状态列不受牵连");
    assert_eq!(
        row.production_recipe_id, approved.id,
        "加料关联的大货处方 ID 不受牵连"
    );
    assert_eq!(
        row.addition_reason.as_deref(),
        Some("色差补料"),
        "加料原因列不受牵连"
    );
    assert!(row.approved_by.is_none(), "新建加料草稿的审批人列必须为空");
}

// =========================================================
// 形态锁 3（批 3c）：两站点开单人取值来源钉死为会话，并含端点覆盖数地板
// =========================================================

#[test]
fn batch3c_recipe_identity_sources_are_the_session() {
    // 本组真实改动站点数（issued_by 由请求体决定 → 改取会话）
    const CHANGED_SITES: usize = 2;
    let mut verified_sites = 0usize;
    let ops_targets: [(&str, &str); CHANGED_SITES] = [
        (
            "src/services/production_recipe_ops/recipe_crud.rs",
            "production_recipe.issued_by",
        ),
        (
            "src/services/production_recipe_ops/addition.rs",
            "production_recipe_addition.issued_by",
        ),
    ];
    for (file, column) in ops_targets.iter() {
        let src = read_src(file);
        assert!(
            src.contains("user_id: i32"),
            "{file} 的 create 必须以 user_id: i32 形参接收会话用户（范式②），实际未见该形参"
        );
        assert!(
            src.contains("issued_by: Set(Some(user_id))"),
            "{column} 必须直接落会话值（列可空 INTEGER，范式④），{file} 未命中 Set(Some(user_id))"
        );
        assert!(
            !src.contains("issued_by: Set(req.issued_by)"),
            "{column} 不得再回退为请求体决定（根因形态 Set(req.issued_by)，{file}）"
        );
        verified_sites += 1;
    }
    assert!(
        verified_sites >= CHANGED_SITES,
        "两个 service 落库站点必须全部被校验到（检测力地板，空集合即判红），实际 {verified_sites}"
    );

    let handler_src = read_src("src/handlers/production_recipe_handler.rs");
    let session_bound = handler_src.matches(".create(req, auth.user_id)").count();
    assert!(
        session_bound >= CHANGED_SITES,
        "handler 必须把会话用户真实传入 service（范式③）：解析到的建单端点数 {session_bound} \
         必须 >= 改动站点数 {CHANGED_SITES}（欠集合即判红，禁止假绿）"
    );
    assert!(
        !handler_src.contains("req.issued_by") && !handler_src.contains("dto.issued_by"),
        "大货处方/加料处方 handler 不得从请求体取开单人（根因形态 req.issued_by / dto.issued_by），\
         身份只能由 service 落会话值"
    );
}
