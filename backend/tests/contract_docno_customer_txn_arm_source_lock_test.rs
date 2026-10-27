//! 客户编码取号与 INSERT 同临界区的源码级防回潮锁（无需数据库）
//!
//! 钉住 customers.customer_code 自动取号腿的接线形态：create_customer 必须
//! 自开一个写入事务，自动编码走 DocumentNumberGenerator::insert_with_no_retry
//! （advisory 锁在本事务内取号并持有到提交、撞 23505 在保存点内重新取号重试），
//! 取号结果经 build_customer_active_model(&args, code) 闭包直接喂给 INSERT，
//! 最后显式提交。正向逐文件断言 begin、insert_with_no_retry、"CUS" 前缀、
//! customer::Column::CustomerCode、闭包传 (&args, code)、commit 全部在场；
//! 反向断言缺陷形态一旦回潮即判红，覆盖 crud.rs 里重新出现的自开事务生成器
//! 直调 DocumentNumberGenerator::generate_no( 与 generate_customer_code( 调用、
//! update.rs 里重新出现的 generate_customer_code 定义体与自开事务直调、
//! handler 里重新出现的 customer_service.generate_customer_code( 预生成调用，
//! 以及取号侧把真实 SQL 错误拍平成请稍后重试类业务文案的映射。
//! 回退（改回事务外自开取号 + 另起事务 INSERT）会同时命中反向钉，立即判红，
//! 这即是本锁的变异自证，无需再执行数据库。
//! 与活体锁 contract_docno_customer_concurrent_create_test.rs 互补：本锁测源码形态，
//! 活体锁测并发与撞键重试的真实行为。
//! 判据是纯文本 contains 对规范化（去回车）后的源码，失败消息点名文件与被钉形态。
//! 调用方为集成测试 crate 的静态档，无入参、无下游、不落数据。

/// 读取并规范化源文件（去 \r，兼容 autocrlf 工作树）
fn src(path: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(path);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
        .replace('\r', "")
}

/// 正向钉，create_customer 里取号与 INSERT 同临界区的接线逐条在场
#[test]
fn customer_create_wires_generation_and_insert_in_same_txn() {
    let text = src("src/services/customer_ops/crud.rs");
    for (needle, desc) in [
        ("(*self.db).begin().await?", "create_customer 自开建单事务"),
        (
            "DocumentNumberGenerator::insert_with_no_retry(",
            "自动编码走同事务取号加保存点重试的生成器",
        ),
        ("\"CUS\"", "取号前缀为客户域权威 CUS"),
        (
            "customer::Column::CustomerCode",
            "取号占用探测落在 customer_code 列",
        ),
        (
            "|code| Self::build_customer_active_model(&args, code)",
            "取号结果经闭包直接喂给 INSERT（生成与插入同一临界区）",
        ),
        ("txn.commit().await?", "建单事务显式提交"),
    ] {
        assert!(
            text.contains(needle),
            "crud.rs 缺少{}的接线，应含 {:?}",
            desc,
            needle
        );
    }
}

/// 反向钉，crud.rs 不得出现事务外自开取号或被拍平的取号失败文案
#[test]
fn customer_create_deny_defect_shapes_in_crud() {
    let text = src("src/services/customer_ops/crud.rs");
    for (needle, desc) in [
        (
            "DocumentNumberGenerator::generate_no(",
            "自开事务生成器直调（提交即放锁，与 INSERT 不在同一临界区）",
        ),
        ("generate_customer_code(", "旧事务外自开取号方法调用回潮"),
        ("请稍后重试", "取号失败拍平为请稍后重试类文案"),
    ] {
        assert!(!text.contains(needle), "crud.rs 出现缺陷形态{}", desc);
    }
}

/// 反向钉，update.rs 不得重新出现事务外自开取号的定义与直调
#[test]
fn customer_ops_update_deny_self_opening_generator() {
    let text = src("src/services/customer_ops/update.rs");
    for (needle, desc) in [
        (
            "pub async fn generate_customer_code",
            "自开事务取号方法定义回潮",
        ),
        (
            "DocumentNumberGenerator::generate_no(",
            "自开事务生成器直调回潮",
        ),
    ] {
        assert!(!text.contains(needle), "update.rs 出现缺陷形态{}", desc);
    }
    // 正向附带，build_customer_active_model 必须是收 (&args, code) 的复用形态
    assert!(
        text.contains("fn build_customer_active_model(")
            && text.contains("args: &CreateCustomerArgs")
            && text.contains("customer_code: String"),
        "update.rs 的 build_customer_active_model 必须收 (&CreateCustomerArgs, String)，\
         以便取号闭包在同一事务内多次重建 ActiveModel 而不消耗 args"
    );
}

/// 反向钉，handler 不得在事务外预生成客户编码
#[test]
fn customer_handler_deny_pre_txn_generation() {
    let text = src("src/handlers/customer_handler.rs");
    assert!(
        !text.contains("generate_customer_code("),
        "customer_handler.rs 出现缺陷形态：handler 里重新调用 customer_service.generate_customer_code\
         在事务外预生成编码，会把取号与 INSERT 重新拆成两个临界区"
    );
}
