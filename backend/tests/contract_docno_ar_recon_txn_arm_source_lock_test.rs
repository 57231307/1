//! 应收对账单取号臂形态与临界区接线的源码级防回潮锁（无需数据库）
//!
//! 钉「取号与插入同临界区」的接线。共用 helper generate_reconciliation_no
//! （services/ar/mod.rs）必须收 &DatabaseTransaction 形参并调用
//! DocumentNumberGenerator::generate_no_with_txn，让前缀级 advisory 锁随
//! 建单事务提交才释放；建单两条路径（vfy_ops/reconciliation.rs 的
//! generate_reconciliation、vfy_ops/match.rs 的 process_customer_match）
//! 必须在已开启的事务内取号、在事务内 insert 落 ar_reconciliations 的
//! reconciliation_no 列、再显式 commit。
//! 正向逐文件断言 begin、helper 带事务实参调用、insert 落同一事务与
//! commit 全部在场；反向断言缺陷形态一旦回潮即红，包括自开事务的
//! DocumentNumberGenerator::generate_no( 直调、helper 形参改回
//! ConnectionTrait 加 TransactionTrait 的泛型连接形态（该形态会在事务内
//! 自开保存点使锁提前释放）、调用点把取号传回事务外连接、以及取号或
//! 插入不落事务的裸连接形态。
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

#[test]
fn ar_reconciliation_no_helper_uses_txn_arm_generator() {
    let text = src("src/services/ar/mod.rs");
    assert!(
        text.contains(
            "pub(crate) async fn generate_reconciliation_no(\n    txn: &DatabaseTransaction,"
        ),
        "ar/mod.rs 的 generate_reconciliation_no 必须是收 &DatabaseTransaction 形参的同域臂签名"
    );
    assert!(
        text.contains("DocumentNumberGenerator::generate_no_with_txn("),
        "ar/mod.rs 的 helper 必须走带事务臂生成器 generate_no_with_txn"
    );
    // 反向钉，缺陷形态回潮即红
    assert!(
        !text.contains("DocumentNumberGenerator::generate_no("),
        "ar/mod.rs 出现自开事务生成器 generate_no 直调，取号与插入的临界区被拆回两段"
    );
    assert!(
        !text.contains("impl ConnectionTrait + TransactionTrait"),
        "ar/mod.rs 的 helper 形参不得改回泛型连接形态，该形态允许在事务内自开保存点使锁提前释放"
    );
}

#[test]
fn ar_reconciliation_create_numbering_and_insert_share_txn() {
    let text = src("src/services/ar/vfy_ops/reconciliation.rs");
    for (needle, desc) in [
        ("(*self.db).begin().await?", "建单事务自开在场"),
        (
            "generate_reconciliation_no(&txn)",
            "取号走事务内 helper 实参形态",
        ),
        ("reconciliation.insert(&txn)", "对账单主行插入落在同一事务"),
        ("txn.commit().await?", "建单事务显式提交"),
    ] {
        assert!(
            text.contains(needle),
            "vfy_ops/reconciliation.rs 缺少{}的接线，应含 {:?}",
            desc,
            needle
        );
    }
    assert!(
        !text.contains("generate_reconciliation_no(&*self.db"),
        "vfy_ops/reconciliation.rs 出现把取号传回事务外连接的形态，锁会随 helper 自开事务提交提前释放"
    );
}

#[test]
fn ar_auto_match_numbering_and_insert_share_txn() {
    let text = src("src/services/ar/vfy_ops/match.rs");
    for (needle, desc) in [
        ("(*self.db).begin().await?", "自动对账主流程自开事务在场"),
        (
            "txn: &sea_orm::DatabaseTransaction",
            "匹配子流程签名收事务形参",
        ),
        (
            "generate_reconciliation_no(txn)",
            "取号在事务内以事务实参调用 helper",
        ),
        ("reconciliation.insert(txn)", "对账单主行插入落在同一事务"),
        ("txn.commit().await?", "自动对账事务显式提交"),
    ] {
        assert!(
            text.contains(needle),
            "vfy_ops/match.rs 缺少{}的接线，应含 {:?}",
            desc,
            needle
        );
    }
    assert!(
        !text.contains("generate_reconciliation_no(&*self.db"),
        "vfy_ops/match.rs 出现把取号传回事务外连接的形态，锁会随 helper 自开事务提交提前释放"
    );
    assert!(
        !text.contains("DocumentNumberGenerator::generate_no("),
        "vfy_ops/match.rs 出现自开事务生成器直调，同临界区接线被拆回两段"
    );
}
