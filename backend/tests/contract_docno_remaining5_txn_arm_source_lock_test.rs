//! 五站点取号臂形态与临界区接线的源码级防回潮锁（无需数据库）
//!
//! 钉住取号与插入同临界区的接线，活体锁
//! （contract_docno_remaining5_lock_scope_test.rs）测的是运行时行为，
//! 本锁测的是源码形态，两者互补。正向逐文件断言 create 里
//! begin、生成器带 txn 形参调用、insert 落在同一 txn、commit、
//! 唯一冲突映射分支与先判重读路径全部在场；反向断言缺陷形态一旦回潮即红，
//! 包括裸无参 self.generate_*_no() 调用、自开事务的
//! DocumentNumberGenerator::generate_no( 直调、无事务裸 insert(&*self.db)、
//! 以及取号侧把真实错误拍平成请稍后重试类业务文案的映射。
//! 覆盖文件，cost_collection_service.rs（宏调用切带 txn 臂的实参形态）、
//! production_recipe_ops/recipe_crud.rs、production_recipe_ops/addition.rs、
//! wage_ops/rate.rs、wage_ops/record.rs，以及 facade
//! production_recipe_service.rs 的两个生成器函数体。
//! 判据是纯文本 contains 对规范化（去回车）后的源码，失败消息点名文件与
//! 被钉形态。调用方为集成测试 crate 的静态档，无入参、无下游、不落数据。

/// 读取并规范化源文件（去 \r，兼容 autocrlf 工作树）
fn src(path: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(path);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
        .replace('\r', "")
}

fn assert_txn_scoped_create(file: &str, text: &str, gen_call: &str) {
    for (needle, desc) in [
        ("(*self.db).begin().await?", "create 自开建单事务"),
        (gen_call, "取号走带事务形参调用"),
        ("insert(&txn)", "插入落在同一事务"),
        ("txn.commit().await?", "建单事务显式提交"),
        ("SqlErr::UniqueConstraintViolation", "唯一冲突映射分支在场"),
    ] {
        assert!(
            text.contains(needle),
            "{} 缺少{}形态的接线，应含 {:?}",
            file,
            desc,
            needle
        );
    }
}

fn deny_defect_shapes(file: &str, text: &str, bare_gen_call: &str) {
    for (needle, desc) in [
        (bare_gen_call, "裸无参取号调用（自开事务臂回潮）"),
        ("DocumentNumberGenerator::generate_no(", "自开事务生成器直调"),
        (".insert(&*self.db)", "无事务裸插入"),
        ("请稍后重试", "取号失败拍平为请稍后重试类文案"),
    ] {
        assert!(
            !text.contains(needle),
            "{} 出现缺陷形态{}",
            file,
            desc
        );
    }
}

#[test]
fn cost_collection_create_is_txn_scoped_and_macro_invocation_uses_txn_arm() {
    let text = src("src/services/cost_collection_service.rs");
    assert_txn_scoped_create("cost_collection_service.rs", &text, "self.generate_collection_no(&txn)");
    assert!(
        text.contains("cost_collection::Column::CollectionNo,\n        txn\n    );"),
        "cost_collection_service.rs 的 impl_generate_no 调用必须是带 txn 形参的宏臂，\
         实参表末位应是独立成行的 txn"
    );
    deny_defect_shapes("cost_collection_service.rs", &text, "self.generate_collection_no()");
}

#[test]
fn recipe_crud_create_is_txn_scoped() {
    let text = src("src/services/production_recipe_ops/recipe_crud.rs");
    assert_txn_scoped_create("recipe_crud.rs", &text, "self.generate_recipe_no(&txn)");
    deny_defect_shapes("recipe_crud.rs", &text, "self.generate_recipe_no()");
}

#[test]
fn addition_create_is_txn_scoped() {
    let text = src("src/services/production_recipe_ops/addition.rs");
    assert_txn_scoped_create("addition.rs", &text, "self.generate_addition_no(&txn)");
    deny_defect_shapes("addition.rs", &text, "self.generate_addition_no()");
}

#[test]
fn recipe_facade_generators_use_txn_arm_generator() {
    let text = src("src/services/production_recipe_service.rs");
    assert_eq!(
        text.matches("DocumentNumberGenerator::generate_no_with_txn(").count(),
        2,
        "facade 的大货处方与加料处方两个生成器必须都走带事务臂生成器，实得 {} 处",
        text.matches("DocumentNumberGenerator::generate_no_with_txn(").count()
    );
    assert!(
        text.contains("txn: &sea_orm::DatabaseTransaction"),
        "facade 生成器签名必须收 &DatabaseTransaction 形参"
    );
    deny_defect_shapes("production_recipe_service.rs", &text, "self.generate_recipe_no()");
}

#[test]
fn wage_rate_create_is_txn_scoped_and_get_by_no_dedups() {
    let text = src("src/services/wage_ops/rate.rs");
    assert_txn_scoped_create("rate.rs", &text, "self.generate_rate_no(&txn)");
    assert!(
        text.contains("DocumentNumberGenerator::generate_no_with_txn("),
        "rate.rs 的生成器必须走带事务臂"
    );
    assert!(
        text.contains("条活跃记录，数据异常"),
        "rate.rs 的 get_by_no 必须先判重再取行，撞重复显式报数据异常"
    );
    deny_defect_shapes("rate.rs", &text, "self.generate_rate_no()");
}

#[test]
fn wage_record_create_is_txn_scoped_and_get_by_no_dedups() {
    let text = src("src/services/wage_ops/record.rs");
    assert_txn_scoped_create("record.rs", &text, "self.generate_record_no(&txn)");
    assert!(
        text.contains("DocumentNumberGenerator::generate_no_with_txn("),
        "record.rs 的生成器必须走带事务臂"
    );
    assert!(
        text.contains("条活跃记录，数据异常"),
        "record.rs 的 get_by_no 必须先判重再取行，撞重复显式报数据异常"
    );
    deny_defect_shapes("record.rs", &text, "self.generate_record_no()");
}
