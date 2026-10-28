//! 打印模板默认位部分唯一索引的双向契约锁（静态文本同源 + 连真实 PostgreSQL 活体）。
//!
//! 功能：钉死库侧唯一门 uq_print_template_active_default 与出件解析口径的同源关系。
//! 1. 正向静态锁：迁移 up 体内索引谓词文本 WHERE is_default AND status='active' 恰出现
//!    三处（探测统计段、探测样例段、建索引段），任何一处被简化（如只剩 WHERE is_default）
//!    即红；建索引语句整句逐字在场；谓词中的 status 取值由权威词表
//!    print_template_status::ACTIVE 运行时拼装比对，词表值与迁移文本单边改动即红；
//!    解析侧 load_active_default_template 的三条件过滤与按 id 升序取首行逐字在场。
//! 2. 正向活体锁：夹具库 pg_indexes 上索引真实存在，indexdef 键含 doc_type、谓词含
//!    is_default 与 active、不含 module（互斥域回退成 module 即红）。
//! 3. 反向活体锁：同一 doc_type 的第二行"启用且默认"直插必败，且违例归因约束名
//!    uq_print_template_active_default 本身（只断类别会把别的约束挡红误当本锁生效）。
//! 4. 谓词精度反向锁：同 doc_type 的停用默认行、同 module 不同 doc_type 的启用默认行
//!    都必须插入成功——谓词被放宽成全表唯一或误写 WHERE is_default、互斥域回退成
//!    module，这三类退化都会让本用例撞 23505 判红。
//! 5. 注册在场锁：域声明在 domain/mod.rs 链尾、迁移项在 lib.rs 向量末位、域台账名与
//!    域内委托与 down 对称回滚（撤索引并还原建表迁移 m0098 的列注释原文）逐处点名。
//! 6. 洗数据零形态锁：迁移 up 体内不存在 UPDATE 表名、DELETE FROM、SET 列名形态的
//!    数据操纵语句，存量违例只能走 fail-visible 中止通道。
//!
//! 调用方：CI 的 Rust 测试 job 与本仓活库门，连已迁移链的 PostgreSQL，
//! 与同族活测一样按单线程跑（--test-threads=1），避免并发互踩清表。
//! 入参：静态腿读迁移与 handler 与词表四份源文件原文；活体腿经
//! test_common::setup_test_db() 注入库连接，print_templates 逐用例清空，起点确定。
//! 传给谁：无下游，纯断言。
//! 存什么：活体腿只临时写 print_templates 的 it_ 前缀私有行，用自建自清；
//! doc_type 与 module 用 it_docidx 前缀私有 token，不与真实业务单据类型相撞。
//! 存哪里：PostgreSQL public.print_templates。
//!
//! 反空操作自证：解析侧过滤被删或改成按 module——静态腿 1 的逐字 needle 与活体腿 4
//! 的"同 module 不同 doc_type 必须可并存"必红；索引没建成或谓词不含启用位——活体腿 2
//! 与 3 必红；索引若退化为只按 is_default 判重——活体腿 4 的停用默认行插入必撞 23505 判红；
//! 迁移没注册进链——静态腿 5 必红；down 漏还原注释——静态腿 5 与 m0098 原文比对必红。
//! 断言不使用恒真式。

mod test_common;

use bingxi_backend::models::print_template;
use bingxi_backend::models::status::print::print_template_status;
use sea_orm::ActiveValue;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, Statement};
use std::sync::Arc;
use test_common::setup_test_db;

/// 被测迁移域内支原文（域目录与台账名同源，改名即红）。
fn m0120_src() -> String {
    include_str!(
        "../migration/src/domain/print_template_default_slot/m0120_add_print_template_active_default_unique.rs"
    )
    .replace('\r', "")
}

/// 被测迁移域聚合原文。
fn domain_mod_src() -> String {
    include_str!("../migration/src/domain/print_template_default_slot/mod.rs").replace('\r', "")
}

/// 域注册面原文（链尾锁读它）。
fn migration_root_mod_src() -> String {
    include_str!("../migration/src/domain/mod.rs").replace('\r', "")
}

/// 顶层迁移装配原文。
fn lib_src() -> String {
    include_str!("../migration/src/lib.rs").replace('\r', "")
}

/// 解析侧 handler 原文（谓词同源比对的对方）。
fn task_handler_src() -> String {
    include_str!("../src/handlers/print_task_handler.rs").replace('\r', "")
}

/// 打印模板状态词表原文。
fn status_vocab_src() -> String {
    include_str!("../src/models/status/print.rs").replace('\r', "")
}

/// 建表迁移原文（down 还原注释的比对基准）。
fn m0098_src() -> String {
    include_str!("../migration/src/domain/print_subsystem/m0098_create_print_templates.rs")
        .replace('\r', "")
}

/// 取 up 执行体区段（头注释也含谓词与探测描述文字，按整文件计数会误伤）。
fn up_body(src: &str) -> &str {
    let start = src.find("async fn up").expect("迁移缺少 up 实现");
    let end = src.find("async fn down").expect("迁移缺少 down 实现");
    &src[start..end]
}

/// 取 down 执行体区段。
fn down_body(src: &str) -> &str {
    let start = src.find("async fn down").expect("迁移缺少 down 实现");
    &src[start..]
}

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, Vec::<sea_orm::Value>::new())
}

/// 直插一行模板（绕过管理面校验，专造索引违例与并存形态）；布局列走 DDL 缺省。
async fn seed_template(
    db: &sea_orm::DatabaseConnection,
    code: &str,
    module: &str,
    doc_type: &str,
    status: &str,
    is_default: bool,
) -> Result<print_template::Model, DbErr> {
    let active = print_template::ActiveModel {
        template_code: ActiveValue::Set(code.to_string()),
        template_name: ActiveValue::Set(format!("默认位唯一锁模板 {code}")),
        module: ActiveValue::Set(module.to_string()),
        doc_type: ActiveValue::Set(doc_type.to_string()),
        status: ActiveValue::Set(status.to_string()),
        is_default: ActiveValue::Set(is_default),
        ..Default::default()
    };
    active.insert(db).await
}

/// 清掉本文件私有前缀行，起点确定（print_templates 已由夹具清表，这里再兜一层幂等）。
async fn sweep_private_rows(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(pg_stmt(
        "DELETE FROM print_templates \
          WHERE template_code LIKE 'it\\_docidx%' OR doc_type LIKE 'it\\_docidx%'",
    ))
    .await
    .unwrap_or_else(|e| panic!("清理私有模板行失败: {e}"));
}

/// 驱动上报的约束名（PostgreSQL 专用），把违例归因到本索引本身。
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(sea_orm::RuntimeErr::SqlxError(e))
        | DbErr::Query(sea_orm::RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 违例必为唯一冲突且归因到 uq_print_template_active_default，
/// 类别断言与约束名断言缺一不可（只断类别会把别的唯一约束挡红误当本锁生效）。
fn assert_unique_violation_on_active_default_index(err: DbErr, case: &str) {
    let constraint = constraint_of(&err);
    assert!(
        matches!(
            err.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ),
        "{case}：期望唯一冲突（23505），实际 sql_err()={:?}\n错误原文: {err}",
        err.sql_err()
    );
    assert_eq!(
        constraint.as_deref(),
        Some("uq_print_template_active_default"),
        "{case}：唯一违例必须归因到部分唯一索引 uq_print_template_active_default，\
         实际约束名={constraint:?}\n错误原文: {err}"
    );
}

// ---------------------------------------------------------------- 静态文本锁

#[test]
fn migration_up_predicate_text_three_copies_and_parser_same_source() {
    let src = m0120_src();
    let up = up_body(&src);

    // 谓词文本在 up 执行体恰三处：探测统计段、探测样例段、建索引段，一处不多不少。
    let pred = "WHERE is_default AND status='active'";
    assert_eq!(
        up.matches(pred).count(),
        3,
        "索引谓词文本应恰在探测统计段、探测样例段、建索引段各出现一次，实测 {} 处",
        up.matches(pred).count()
    );

    // 谓词中的启用 token 由权威词表拼装比对：词表值一改，迁移文本单边滞留即红。
    let pred_from_vocab = format!(
        "WHERE is_default AND status='{}'",
        print_template_status::ACTIVE
    );
    assert_eq!(
        up.matches(pred_from_vocab.as_str()).count(),
        3,
        "以词表 ACTIVE 拼装谓词后同样应命中三处（谓词与词表必须逐字符同源）"
    );

    // 建索引整句逐字钉死：键 doc_type、部分唯一、谓词三条件，缺一即红。
    let idx_stmt = "CREATE UNIQUE INDEX IF NOT EXISTS uq_print_template_active_default ON print_templates(doc_type) WHERE is_default AND status='active';";
    assert!(
        up.contains(idx_stmt),
        "建索引语句整句缺失或被改动:\n{idx_stmt}"
    );

    // 列注释更正真实落地在 up：COMMENT ON COLUMN 指向 is_default。
    assert!(
        up.contains("COMMENT ON COLUMN \"print_templates\".\"is_default\""),
        "up 段必须含 is_default 列注释更正语句"
    );

    // 洗数据零形态：up 体内不存在任何 UPDATE 表名、DELETE FROM、SET 列名的数据操纵。
    for banned in ["UPDATE \"", "DELETE FROM", "SET \"is_default\"", "RENAME"] {
        assert!(
            !src.contains(banned),
            "迁移文本出现数据操纵形态: {banned}（存量违例只能 fail-visible 中止）"
        );
    }

    // 探测 fail-closed 形态在场：HAVING 判重两处、RAISE EXCEPTION 一条。
    assert_eq!(
        up.matches("HAVING COUNT(*) > 1").count(),
        2,
        "探测应含统计与样例两处 HAVING，共 2 处"
    );
    assert_eq!(
        up.matches("RAISE EXCEPTION").count(),
        1,
        "应恰有一条 fail-closed RAISE EXCEPTION"
    );
    assert!(
        up.contains("USING HINT"),
        "拒绝段应带人工处置口径 HINT（同 m0082 范式）"
    );

    // 解析侧过滤三条件与升序取首行逐字在场（同源的对方）。
    let th = task_handler_src();
    for needle in [
        "pub async fn load_active_default_template",
        ".filter(print_template::Column::DocType.eq(doc_type))",
        ".filter(print_template::Column::Status.eq(print_template_status::ACTIVE))",
        ".filter(print_template::Column::IsDefault.eq(true))",
        ".order_by(print_template::Column::Id, Order::Asc)",
    ] {
        assert!(
            th.contains(needle),
            "解析侧 load_active_default_template 过滤形态缺失: {needle}"
        );
    }

    // 词表启用 token 字面值逐字钉住（与谓词中的 status 取值同源）。
    assert!(
        status_vocab_src().contains("pub const ACTIVE: &str = \"active\";"),
        "print_template_status::ACTIVE 的字面值必须是 active（与索引谓词逐字符一致）"
    );
}

#[test]
fn registration_at_migration_chain_tail_locked() {
    let dm = migration_root_mod_src();
    let lib = lib_src();
    let domain = domain_mod_src();

    // 域声明在 domain/mod.rs 且位于既有链尾 contract_template_schema 之后。
    assert!(
        dm.contains("pub mod print_template_default_slot;"),
        "domain/mod.rs 缺少本域声明"
    );
    let tail_anchor = dm
        .find("pub mod contract_template_schema;")
        .expect("domain/mod.rs 链尾锚点 contract_template_schema 缺失");
    let mine = dm
        .find("pub mod print_template_default_slot;")
        .expect("domain/mod.rs 本域声明缺失");
    assert!(
        mine > tail_anchor,
        "本域必须注册在 domain/mod.rs 链尾（晚于 contract_template_schema）"
    );

    // 顶层装配在 lib.rs 且是向量最后一个迁移项（链尾注册，复用库上必然执行）。
    let reg = "Box::new(domain::print_template_default_slot::Migration),";
    assert!(lib.contains(reg), "lib.rs 迁移向量缺少本域注册项");
    let last_box = lib
        .rfind("Box::new(domain::")
        .expect("lib.rs 迁移向量缺少注册项");
    assert!(
        lib[last_box..].starts_with("Box::new(domain::print_template_default_slot::"),
        "本域注册项必须是 lib.rs 迁移向量的末位（链尾），实测末位为其它域"
    );

    // 域台账名取域名（与目录名同源，改名即红）。
    assert!(
        domain.contains("\"m_print_template_default_slot\""),
        "域聚合缺少台账名 m_print_template_default_slot"
    );
    // 域内委托与回退对称：up/down 各调用域内支一次。
    let call = "m0120_add_print_template_active_default_unique::Migration";
    assert_eq!(
        domain.matches(call).count(),
        2,
        "域聚合应恰有 up 与 down 两次域内支调用，实测 {} 次",
        domain.matches(call).count()
    );
    assert_eq!(
        domain
            .matches("mod m0120_add_print_template_active_default_unique;")
            .count(),
        1,
        "域聚合应恰有一条域内支 mod 声明"
    );
    assert!(
        domain.contains(".up(manager)") && domain.contains(".down(manager)"),
        "域聚合缺少 up/down 委托"
    );
}

#[test]
fn down_symmetric_restores_index_and_m0098_comment() {
    let src = m0120_src();
    let down = down_body(&src);

    // down 显式撤索引（本仓纪律：down 不留空实现），不触碰数据行。
    assert!(
        down.contains("DROP INDEX IF EXISTS uq_print_template_active_default;"),
        "down 缺少显式撤索引语句"
    );
    assert!(
        !down.contains("DELETE FROM") && !down.contains("UPDATE \""),
        "down 不得操纵数据行"
    );

    // down 还原的列注释必须逐字等于建表迁移 m0098 的原文（还原基准从对方源文提取）。
    let baseline = m0098_src();
    let needle = "COMMENT ON COLUMN \"print_templates\".\"is_default\" IS ";
    let base_comment = {
        let start = baseline
            .find(needle)
            .expect("m0098 缺少 is_default 列注释语句（还原基准丢失）");
        let rest = &baseline[start + needle.len()..];
        let end = rest.find("';").expect("m0098 列注释语句未闭合");
        format!("{needle}{}'", &rest[..end])
    };
    assert!(
        down.contains(&base_comment),
        "down 还原的列注释与建表迁移 m0098 原文不一致，m0098 原文:\n{base_comment}"
    );
}

// ---------------------------------------------------------------- 活体锁

#[tokio::test]
async fn db_enforced_index_exists_with_parser_shaped_predicate() {
    let db = Arc::new(setup_test_db().await);

    // 正向形态锁：索引必须真实存在于夹具库（迁移注册生效的证明）。
    let idx = db
        .query_one_raw(pg_stmt(
            "SELECT indexdef FROM pg_indexes \
              WHERE tablename='print_templates' \
                AND indexname='uq_print_template_active_default'",
        ))
        .await
        .unwrap_or_else(|e| panic!("查 pg_indexes 失败: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "部分唯一索引 uq_print_template_active_default 不在夹具库上——\
                 迁移 m0120 未在测试库生效（检查域注册与迁移链是否跑到本域）"
            )
        });
    let indexdef: String = idx
        .try_get("", "indexdef")
        .unwrap_or_else(|e| panic!("indexdef 解码失败: {e}"));

    // 键是 doc_type（解析查找键），谓词带启用位与默认位且启用取值来自词表 active；
    // 互斥域不得是 module（module 形态即还原恒遮蔽缺陷）。
    assert!(
        indexdef.contains("UNIQUE")
            && indexdef.contains("(doc_type)")
            && indexdef.contains("is_default")
            && indexdef.contains(&format!("'{}'", print_template_status::ACTIVE))
            && indexdef.contains("WHERE"),
        "索引形态应为按 doc_type 的部分唯一且谓词含启用位与默认位，实测 indexdef={indexdef}"
    );
    assert!(
        !indexdef.contains("module"),
        "索引键与谓词都不得引用 module（互斥域是单据类型 doc_type），实测 indexdef={indexdef}"
    );
}

#[tokio::test]
async fn db_enforced_second_active_default_same_doc_type_rejects_23505() {
    let db = Arc::new(setup_test_db().await);
    sweep_private_rows(&db).await;

    // 在任行：启用且默认。
    let held = seed_template(
        &db,
        "it_docidx_held",
        "it_docidx_mod",
        "it_docidx_dt_a",
        print_template_status::ACTIVE,
        true,
    )
    .await
    .unwrap_or_else(|e| panic!("种在任启用默认行失败: {e}"));

    // 违例行：同 doc_type 第二行启用且默认（module 不同，证明互斥域与 module 无关）。
    let dup = seed_template(
        &db,
        "it_docidx_dup",
        "it_docidx_mod_other",
        "it_docidx_dt_a",
        print_template_status::ACTIVE,
        true,
    )
    .await;

    // 检测力自证：索引在场时此行必败且归因本索引；若迁移没生效，这里必是 Ok 并触发
    // 下面的断言红，拒绝把"插入成功"当通过。
    let err = dup.expect_err(
        "同 doc_type 的第二行启用且默认竟然插入成功——部分唯一索引 \
         uq_print_template_active_default 没有挡红先查后插竞态形态，本锁判红",
    );
    assert_unique_violation_on_active_default_index(err, "同键第二启用默认行");

    // 在任行原样留存（唯一门只拒新行，不吞既有行）。
    let after = print_template::Entity::find_by_id(held.id)
        .one(&*db)
        .await
        .unwrap_or_else(|e| panic!("回读在任行失败: {e}"))
        .unwrap_or_else(|| panic!("在任行 {id} 必须原样存在", id = held.id));
    assert!(
        after.is_default && after.status == print_template_status::ACTIVE,
        "撞索引拒绝后在任启用默认行的位与状态不得被改动"
    );
    sweep_private_rows(&db).await;
}

#[tokio::test]
async fn db_enforced_index_predicate_does_not_reject_non_conflicting_rows() {
    let db = Arc::new(setup_test_db().await);
    sweep_private_rows(&db).await;

    // 在任行：doc_type A 的启用默认。
    seed_template(
        &db,
        "it_docidx_base",
        "it_docidx_mod_x",
        "it_docidx_dt_a",
        print_template_status::ACTIVE,
        true,
    )
    .await
    .unwrap_or_else(|e| panic!("种在任启用默认行失败: {e}"));

    // 并存形态一：同 doc_type 的"停用且默认"第二行必须可插入。
    // 谓词被简化成只剩 WHERE is_default（丢掉启用位）时，这里会撞 23505 判红——
    // 停用行默认位只留存不参与解析，是本域既有裁定语义。
    let inactive_dup = seed_template(
        &db,
        "it_docidx_retained",
        "it_docidx_mod_x",
        "it_docidx_dt_a",
        print_template_status::INACTIVE,
        true,
    )
    .await;
    if let Err(e) = inactive_dup {
        panic!(
            "同 doc_type 的停用默认行被拒插入——索引谓词丢掉了启用位条件（应为 \
             WHERE is_default AND status='active'，不得简化成只按默认位判重）: {e}"
        );
    }

    // 并存形态二：同 module、不同 doc_type 的"启用且默认"第二行必须可插入。
    // 互斥域若回退成 module（恒遮蔽缺陷的索引侧形态），这里会撞 23505 判红。
    let cross_doc = seed_template(
        &db,
        "it_docidx_cross",
        "it_docidx_mod_x",
        "it_docidx_dt_b",
        print_template_status::ACTIVE,
        true,
    )
    .await;
    if let Err(e) = cross_doc {
        panic!(
            "同 module 不同 doc_type 的启用默认行被拒插入——索引互斥域回退成了 module，\
             互斥域必须是单据类型 doc_type: {e}"
        );
    }

    // 并存形态三：不同 doc_type 的启用默认行（另一模块）也必须可插入。
    seed_template(
        &db,
        "it_docidx_other_dt",
        "it_docidx_mod_y",
        "it_docidx_dt_c",
        print_template_status::ACTIVE,
        true,
    )
    .await
    .unwrap_or_else(|e| panic!("不同 doc_type 的启用默认行应可插入: {e}"));

    // 复核最终形态：三个 doc_type 各恰有一行"启用且默认"。
    for (dt, want) in [
        ("it_docidx_dt_a", 1_i64),
        ("it_docidx_dt_b", 1_i64),
        ("it_docidx_dt_c", 1_i64),
    ] {
        let row = db
            .query_one_raw(pg_stmt(&format!(
                "SELECT COUNT(*) AS cnt FROM print_templates \
                  WHERE doc_type='{dt}' AND is_default AND status='active'"
            )))
            .await
            .unwrap_or_else(|e| panic!("回读 {dt} 启用默认行数失败: {e}"));
        let cnt: i64 = row
            .expect("COUNT 恒有行")
            .try_get("", "cnt")
            .unwrap_or_else(|e| panic!("行数解码失败: {e}"));
        assert_eq!(cnt, want, "doc_type {dt} 的启用默认行数应为 {want}");
    }
    sweep_private_rows(&db).await;
}
