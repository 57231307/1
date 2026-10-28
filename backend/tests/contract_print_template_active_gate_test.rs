//! 打印出口启用位与默认位的活体契约锁（连真实 PostgreSQL）。
//!
//! 功能：在真已迁移、按用例清表的 PostgreSQL 上钉死打印子系统两侧口径。
//! 1. 出件解析侧：`load_active_default_template` 与 `resolve_print_docx_layout` 只选中
//!    「status=启用 且 is_default=真」的模板行；唯一启用默认行停用后解析必 404（消息点名
//!    doc_type 与启用成因）且零副作用（不写 `print_tasks`、模板行原样留存停用态）；
//!    重新启用后必须又能命中同一行（停用-启用往返闭环）。
//! 2. 默认位竞位侧：新建/更新/设默认都以 doc_type（渲染查找键）为互斥域、只在显式置默认位
//!    且本行生效为启用时竞位，且只降「同键且未停用」的在任默认行；内置种子形态行
//!    （module=doc_type）被用户自建默认行降位是预期语义；停用行不得把在任启用默认行打下来；
//!    单纯状态位切换（含重新启用）一律不清任何行的默认位；带默认位的停用行重新启用若会与
//!    在任启用默认行撞位，管理面非竞位腿按生效值点名拒绝且零写入，并存形态不再落库。
//! 3. 显式模板 id 出件侧：`load_printable_template` 只接受启用行，停用与不存在都 404 并
//!    点名模板 id 与成因（锁 POST /print-tasks 与 POST /print-templates/{id}/print 的受理门）。
//! 4. 管理面口径：详情读取必须仍能看到停用行（与出件侧有意不同，防止后续"顺手"把管理面
//!    也过滤掉，运营要靠它核对待启用模板）。
//!
//! 调用方：集成测试 crate（CI 与本地对已跑完迁移链的 PostgreSQL 执行，
//! `cargo test --offline --test contract_print_template_active_gate_test -- --test-threads=1`）。
//! 入参：`test_common::setup_test_db()` 注入库连接（业务表逐用例清空，`print_templates`
//! 无启动种子残留，起点确定）；模板行全部由本文件自建自清，module 用 it_ 前缀私有 token，
//! 不与任何真实业务模块相撞。
//! 传给谁：断言落在 `AppError` 变体、模板行 `status`/`is_default` 列值与 `print_tasks`
//! 行数上；不派发真实打印目标，不生成文件。
//! 存什么：只临时写 `print_templates` 行与经由建模板链路的少量事务写；404 分支验证零写入。
//! 存哪里：PostgreSQL `print_templates`（`print_tasks` 只读计数）。
//!
//! 反空操作自证：若出件解析去掉启用位过滤（还原"建了就一直可用"），用例 1 的停用段与
//! 用例 2 的停用段必红；若竞位过滤退回按 module 清（还原遮蔽缺陷），用例 2 的"种子形态
//! 行被降位/停用后 404"必红（种子行仍以最小 id 出件，停用无效）；若竞位条件放宽为
//! "保存后本行将成为启用默认行"（即单纯状态切换也竞位），用例 3 的"重新启用后在任行保位"
//! 必红；若显式 id 分支不查启用位，用例 4 停用段必红；若管理面详情被顺手加上启用位过滤，
//! 用例 5 必红。断言不使用恒真式。

mod test_common;

use axum::Json;
use axum::extract::{Path as AxPath, State};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{print_handler, print_task_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::print_task;
use bingxi_backend::models::print_template;
use bingxi_backend::models::status::print::print_template_status;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;

/// 出件解析往返用例的单据类型（真实受支持 token，库已清表，行内容全部自建）。
const DT_ROUNDTRIP: &str = "inventory_write_down";
/// 种子遮蔽复刻用例的单据类型（与判红的打样单打印链同一 token）。
const DT_DEMOTION: &str = "lab_dip_request";

fn auth() -> AuthContext {
    AuthContext {
        user_id: 9527,
        username: "it_print_gate".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

fn state(db: &Arc<sea_orm::DatabaseConnection>) -> AppState {
    AppState {
        db: db.clone(),
        ..Default::default()
    }
}

/// 直插一行模板（绕过管理面校验，用于构造存量形态）；布局列走 DDL 缺省。
async fn seed_template(
    db: &sea_orm::DatabaseConnection,
    code: &str,
    module: &str,
    doc_type: &str,
    status: &str,
    is_default: bool,
) -> print_template::Model {
    use sea_orm::ActiveValue;
    let active = print_template::ActiveModel {
        template_code: ActiveValue::Set(code.to_string()),
        template_name: ActiveValue::Set(format!("打印启用位锁模板 {code}")),
        module: ActiveValue::Set(module.to_string()),
        doc_type: ActiveValue::Set(doc_type.to_string()),
        status: ActiveValue::Set(status.to_string()),
        is_default: ActiveValue::Set(is_default),
        ..Default::default()
    };
    active
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("直插模板行 {code} 失败: {e}"))
}

async fn reload(db: &sea_orm::DatabaseConnection, id: i32) -> print_template::Model {
    print_template::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("回读模板行 {id} 失败: {e}"))
        .unwrap_or_else(|| panic!("模板行 {id} 必须存在"))
}

async fn set_status(db: &sea_orm::DatabaseConnection, id: i32, status: &str) {
    use sea_orm::IntoActiveModel;
    let row = reload(db, id).await;
    let mut am: print_template::ActiveModel = row.into_active_model();
    am.status = Set(status.to_string());
    am.update(db)
        .await
        .unwrap_or_else(|e| panic!("改模板行 {id} 状态失败: {e}"));
}

fn create_req(
    code: &str,
    module: &str,
    doc_type: &str,
    status: &str,
    is_default: bool,
) -> print_handler::CreatePrintTemplateRequest {
    print_handler::CreatePrintTemplateRequest {
        template_name: format!("建口锁模板 {code}"),
        doc_type: doc_type.to_string(),
        template_code: Some(code.to_string()),
        description: None,
        module: Some(module.to_string()),
        template_type: None,
        paper_size: None,
        orientation: None,
        copies_default: None,
        default_target_name: None,
        content: None,
        css_styles: None,
        variables: None,
        is_default: Some(is_default),
        status: Some(status.to_string()),
    }
}

fn update_req_none() -> print_handler::UpdatePrintTemplateRequest {
    print_handler::UpdatePrintTemplateRequest {
        template_name: None,
        doc_type: None,
        template_code: None,
        description: None,
        module: None,
        template_type: None,
        paper_size: None,
        orientation: None,
        copies_default: None,
        default_target_name: None,
        content: None,
        css_styles: None,
        variables: None,
        is_default: None,
        status: None,
    }
}

fn assert_not_found(err: AppError, expect_in_msg: &[&str]) {
    let rendered = format!("{err}");
    assert!(
        matches!(
            err,
            AppError::NotFound(_) | AppError::NotFoundDisplayable(_)
        ),
        "必须返回 404 not_found 族，实际渲染文本: {rendered}"
    );
    for token in expect_in_msg {
        assert!(
            rendered.contains(token),
            "404 消息必须点名 {token}，实际: {rendered}"
        );
    }
}

/// 用例 1：出件解析以「启用且默认」为硬条件，停用 404 零副作用，再启用恢复命中。
#[tokio::test]
async fn print_default_resolution_roundtrip_active_gate() {
    let db = test_common::setup_test_db().await;
    let tpl = seed_template(
        &db,
        "IT_GATE_RT_A",
        "it_print_gate",
        DT_ROUNDTRIP,
        print_template_status::ACTIVE,
        true,
    )
    .await;

    let hit = print_task_handler::load_active_default_template(&db, DT_ROUNDTRIP)
        .await
        .expect("启用且默认的模板行必须可被打印口解析");
    assert_eq!(hit.id, tpl.id, "解析必须命中本用例自建行");

    let layout = print_task_handler::resolve_print_docx_layout(&db, DT_ROUNDTRIP)
        .await
        .expect("布局解析与默认行解析同源，必须同为成功");
    assert_eq!(
        layout.paper_size, hit.paper_size,
        "布局必须取自同一行的持久化列"
    );

    // 停用：出件解析立即失去候选，404 点名 doc_type 与启用成因。
    set_status(&db, tpl.id, print_template_status::INACTIVE).await;
    let err = print_task_handler::load_active_default_template(&db, DT_ROUNDTRIP)
        .await
        .expect_err("唯一默认行停用后打印口必须拒绝解析");
    assert_not_found(err, &[DT_ROUNDTRIP, "启用"]);

    // 零副作用：解析拒绝是只读段，不产生任务行，模板行原样留存停用态。
    let tasks = print_task::Entity::find()
        .filter(print_task::Column::DocType.eq(DT_ROUNDTRIP))
        .count(&db)
        .await
        .expect("统计打印任务行数失败");
    assert_eq!(tasks, 0, "停用后解析拒绝不得产生任何打印任务行");
    let after = reload(&db, tpl.id).await;
    assert_eq!(
        after.status,
        print_template_status::INACTIVE,
        "停用行必须原样留存"
    );
    assert!(after.is_default, "停用不应抹掉默认位（留存给运营核对）");

    // 再启用：同一行必须重新可解析（启用位真实参与解析的闭环）。
    set_status(&db, tpl.id, print_template_status::ACTIVE).await;
    let hit2 = print_task_handler::load_active_default_template(&db, DT_ROUNDTRIP)
        .await
        .expect("重新启用后必须再次命中同一模板行");
    assert_eq!(hit2.id, tpl.id);
}

/// 用例 2：经建模板管理口复刻种子遮蔽场景，钉"默认位按 doc_type 互斥、停用即断供"。
#[tokio::test]
async fn new_active_default_demotes_builtin_style_default_of_same_doc_type() {
    let db = test_common::setup_test_db().await;
    let db = Arc::new(db);
    let st = state(&db);

    // 先建"内置种子形态"行：module=doc_type、启用、默认（与启动种子的落库形态一致）。
    let seed_resp = print_handler::create_print_template(
        State(st.clone()),
        auth(),
        Json(create_req(
            "IT_GATE_DM_SEED",
            DT_DEMOTION,
            DT_DEMOTION,
            print_template_status::ACTIVE,
            true,
        )),
    )
    .await
    .expect("建种子形态默认模板必须成功");
    let seed_id = seed_resp.0.data.expect("建模板返回体必须携带模板行").id;

    // 再建用户私有 module 的启用默认行：竞位必须按 doc_type 把种子形态行降位，
    // 否则解析按最小 id 恒命中种子行，用户行的停用/启用对出件无效（判红根因）。
    let user_resp = print_handler::create_print_template(
        State(st.clone()),
        auth(),
        Json(create_req(
            "IT_GATE_DM_MINE",
            "it_cc_private",
            DT_DEMOTION,
            print_template_status::ACTIVE,
            true,
        )),
    )
    .await
    .expect("建用户私有模块默认模板必须成功");
    let user_id = user_resp.0.data.expect("建模板返回体必须携带模板行").id;

    let seed_row = reload(&db, seed_id).await;
    assert!(
        !seed_row.is_default,
        "同 doc_type 的用户新启用默认行必须把内置种子形态行降位（预期语义：自定义默认取代内置默认）"
    );
    let hit = print_task_handler::load_active_default_template(&db, DT_DEMOTION)
        .await
        .expect("解析必须命中在任启用默认行");
    assert_eq!(
        hit.id, user_id,
        "解析必须命中用户自建行而非被降位的种子形态行"
    );

    // 停用用户行：该 doc_type 再无「启用且默认」行，打印口必须 404（种子形态行已无默认位）。
    let mut off = update_req_none();
    off.status = Some(print_template_status::INACTIVE.to_string());
    let rec_off =
        print_handler::update_print_template(AxPath(user_id), State(st.clone()), auth(), Json(off))
            .await
            .expect("停用用户默认模板必须成功");
    assert_eq!(
        rec_off.0.data.expect("停用更新返回体必须携带模板行").status,
        print_template_status::INACTIVE,
        "停用更新必须回读 inactive 状态"
    );
    let err = print_task_handler::load_active_default_template(&db, DT_DEMOTION)
        .await
        .expect_err("用户默认行停用后打印口必须退回 404");
    assert_not_found(err, &[DT_DEMOTION, "启用"]);

    // 重新启用用户行：同一行恢复出件；已降位的种子形态行不得借机抢回默认位。
    let mut on = update_req_none();
    on.status = Some(print_template_status::ACTIVE.to_string());
    let rec_on =
        print_handler::update_print_template(AxPath(user_id), State(st.clone()), auth(), Json(on))
            .await
            .expect("重新启用用户默认模板必须成功");
    assert_eq!(
        rec_on.0.data.expect("重新启用返回体必须携带模板行").status,
        print_template_status::ACTIVE,
        "重新启用更新必须回读 active 状态"
    );
    let hit2 = print_task_handler::load_active_default_template(&db, DT_DEMOTION)
        .await
        .expect("重新启用后打印口必须再次命中用户行");
    assert_eq!(hit2.id, user_id);
    let seed_row2 = reload(&db, seed_id).await;
    assert!(
        !seed_row2.is_default,
        "竞位只降「当时在任的启用默认行」，已降位行不应被改动"
    );
}

/// 用例 3：停用行不得把在任启用默认行打下来，重新启用不发起竞位；带默认位的停用行
/// 重新启用会与在任启用默认行撞位，管理面点名拒绝且零写入，在任行保位。
#[tokio::test]
async fn inactive_row_never_demotes_in_position_default() {
    let db = test_common::setup_test_db().await;
    let db = Arc::new(db);
    let st = state(&db);

    let keeper = seed_template(
        &db,
        "IT_GATE_IX_KEEP",
        "it_print_gate",
        DT_ROUNDTRIP,
        print_template_status::ACTIVE,
        true,
    )
    .await;
    let claimant = seed_template(
        &db,
        "IT_GATE_IX_CLAIM",
        "it_print_gate",
        DT_ROUNDTRIP,
        print_template_status::INACTIVE,
        false,
    )
    .await;

    // 对停用行设默认：只留自己的默认位，不清在任启用默认行（停用行打不下在任行）。
    let set_resp =
        print_handler::set_default_print_template(AxPath(claimant.id), State(st.clone()), auth())
            .await
            .expect("对停用行设默认必须成功");
    assert!(
        set_resp.0.data.is_some(),
        "停用行设默认的管理面写入必须正常返回"
    );
    let keeper_after = reload(&db, keeper.id).await;
    assert!(
        keeper_after.is_default,
        "停用行竞位不得生效：在任启用默认行的默认位必须保持"
    );
    let claimant_after = reload(&db, claimant.id).await;
    assert!(
        claimant_after.is_default,
        "停用行自身默认位应留存（管理面照常可见），但停用行不参与出件解析"
    );
    let hit = print_task_handler::load_active_default_template(&db, DT_ROUNDTRIP)
        .await
        .expect("解析必须仍命中在任启用默认行");
    assert_eq!(hit.id, keeper.id);

    // 停用行（is_default=true）重新启用且未显式置默认位：单纯状态切换不发起竞位。
    // schema 层同一单据类型至多一行启用且默认，直写会与在任启用默认行撞位，管理面
    // 非竞位腿必须点名拒绝，而不是把裸的库侧重复错误冒到界面上。
    let mut on = update_req_none();
    on.status = Some(print_template_status::ACTIVE.to_string());
    let err_on = print_handler::update_print_template(
        AxPath(claimant.id),
        State(st.clone()),
        auth(),
        Json(on),
    )
    .await
    .expect_err("撞位在任启用默认行的停用默认行重新启用必须被点名拒绝");
    assert!(
        matches!(err_on, AppError::BusinessErrorDisplayable(_)),
        "撞位拒绝必须是可外显的业务拒绝，实际形态 {err_on:?}"
    );
    // 拒绝不得部分生效、零写入：在任行保位保状态，被拒行留存停用态与自身默认位。
    let keeper_still = reload(&db, keeper.id).await;
    assert!(
        keeper_still.is_default,
        "单纯状态位切换一律不清任何行的默认位，在任启用默认行必须保位"
    );
    assert_eq!(
        keeper_still.status,
        print_template_status::ACTIVE,
        "拒绝路径不得动在任行的启用状态"
    );
    let claimant_still = reload(&db, claimant.id).await;
    assert_eq!(
        claimant_still.status,
        print_template_status::INACTIVE,
        "拒绝不得部分生效，被拒行必须仍为停用态"
    );
    assert!(
        claimant_still.is_default,
        "拒绝不得顺手清掉被拒行自身的默认位"
    );
    let hit2 = print_task_handler::load_active_default_template(&db, DT_ROUNDTRIP)
        .await
        .expect("撞位重新启用被拒后解析必须仍命中在任启用默认行");
    assert_eq!(
        hit2.id, keeper.id,
        "重新启用被点名拒绝，在任行保位且解析结果不变"
    );
    // 全程零副作用：出件门拒绝段不产生任何打印任务行。
    let tasks = print_task::Entity::find()
        .filter(print_task::Column::DocType.eq(DT_ROUNDTRIP))
        .count(db.as_ref())
        .await
        .expect("统计打印任务行数失败");
    assert_eq!(tasks, 0, "本用例只动模板配置，不得产生打印任务");
}

/// 用例 4：显式模板 id 出件门只接受启用行；停用与不存在都 404 且点名模板 id 与成因。
#[tokio::test]
async fn explicit_template_id_gate_requires_active() {
    let db = test_common::setup_test_db().await;
    let tpl = seed_template(
        &db,
        "IT_GATE_EX_A",
        "it_print_gate",
        DT_ROUNDTRIP,
        print_template_status::ACTIVE,
        false,
    )
    .await;

    let ok = print_task_handler::load_printable_template(&db, tpl.id)
        .await
        .expect("启用模板行必须可经显式 id 出件");
    assert_eq!(ok.id, tpl.id);

    set_status(&db, tpl.id, print_template_status::INACTIVE).await;
    let err = print_task_handler::load_printable_template(&db, tpl.id)
        .await
        .expect_err("停用模板行经显式 id 必须被拒");
    assert_not_found(err, &[&tpl.id.to_string(), "停用"]);

    let err2 = print_task_handler::load_printable_template(&db, 987_654_321)
        .await
        .expect_err("不存在的模板 id 必须被拒");
    assert_not_found(err2, &["987654321", "不存在"]);
}

/// 用例 5：管理面详情必须仍读得到停用行（与出件门口径有意不同，锁死"别顺手改管理面"）。
#[tokio::test]
async fn admin_detail_still_reads_inactive_template() {
    let db = test_common::setup_test_db().await;
    let db = Arc::new(db);
    let st = state(&db);

    let tpl = seed_template(
        &db,
        "IT_GATE_AD_A",
        "it_print_gate",
        DT_ROUNDTRIP,
        print_template_status::INACTIVE,
        true,
    )
    .await;
    let resp = print_handler::get_print_template(AxPath(tpl.id), State(st), auth())
        .await
        .expect("管理面详情对停用行必须正常返回");
    let record = resp.0.data.expect("详情返回体必须携带模板行");
    assert_eq!(record.status, print_template_status::INACTIVE);
    assert_eq!(record.doc_type, DT_ROUNDTRIP);
}
