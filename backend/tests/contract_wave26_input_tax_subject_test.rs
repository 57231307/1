//! 进项税额科目分账的三通道同源锁
//!
//! 背景：采购入库应付凭证的进项税额分录一直记在 `222101` 下，而 `222101` 的种子名称是
//! "应交税费-应交增值税-销项税额"。销售销项与采购进项同挂一枚科目码，明细账同码互抵，
//! 进项税额被销项吃掉。修法是新开 `222102 进项税额` 并把采购侧分录改指过去。
//!
//! 三处必须同口径，本测逐条钉死，防止一边改一边忘：
//! ① 迁移域 `backend/migration/src/domain/ap_input_tax_subject`（新库与存量库共用）
//! ② 迁移链注册位（`domain/mod.rs` 声明加 `lib.rs` 链尾挂载）
//! ③ 运行时写入侧 `services/ap_invoice_ops/receipt.rs` 的进项分录科目码
//!
//! 另钉两条反向漂移：销售发货开票的销项分录必须仍是 222101；finance 域种子对 222101 的
//! 名称必须仍是销项税额（不许靠改名把进项塞进销项科目）。判据全部取自源文件原文，
//! 与 CI 的 cargo 分片同一一份实现，不依赖运行期工作目录。

const MIGRATION: &str = include_str!("../migration/src/domain/ap_input_tax_subject/mod.rs");
const DOMAIN_MOD: &str = include_str!("../migration/src/domain/mod.rs");
const LIB_RS: &str = include_str!("../migration/src/lib.rs");
const FINANCE_MOD: &str = include_str!("../migration/src/domain/finance/mod.rs");
const RECEIPT: &str = include_str!("../src/services/ap_invoice_ops/receipt.rs");
const SHIP: &str = include_str!("../src/services/so/delivery_ops/ship.rs");

/// 去掉全部空白，使判据不受排版折行影响
fn flat(src: &str) -> String {
    src.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 迁移域必须建出 222102 进项税额：父科目 2221、方向 debit、幂等、缺父即中止、不洗存量
#[test]
fn migration_creates_input_tax_subject_idempotently() {
    let src = MIGRATION.replace('\r', "");
    assert!(
        src.contains("222102"),
        "迁移必须建 222102 进项税额科目码，否则分录无处可落"
    );
    assert!(
        src.contains("应交税费-应交增值税-进项税额"),
        "科目名称必须与销项 222101 同族且明确写进项，不得与销项同名"
    );
    assert!(
        src.contains("2221"),
        "必须挂在应交税费父科目 2221 下，建成孤儿一级科目会绕过余额汇总口径"
    );
    assert!(
        src.contains("'debit'"),
        "进项税额是应交增值税的借方明细，余额方向必须 debit"
    );
    assert!(
        src.contains("ON CONFLICT") || src.contains("NOT EXISTS"),
        "重跑必须等价：缺幂等守卫会在复用库上撞科目码唯一约束"
    );
    assert!(
        src.contains("RAISE EXCEPTION"),
        "父科目缺失必须中止而非静默跳过，否则进项分录继续落错码且无人知道"
    );
    assert!(
        src.contains("RAISE NOTICE"),
        "存量错码进项行必须以 NOTICE 可见化报出，不得静默吞掉"
    );
    assert!(
        !flat(&src).contains("UPDATE\"voucher_items\""),
        "禁止在迁移里 UPDATE 已入账凭证行改科目码：历史更正须走凭证调整流程留痕"
    );
}

/// 迁移必须真挂进链尾：域声明加 lib.rs 挂载缺一即"写了文件不跑迁移"
#[test]
fn migration_is_declared_and_mounted_at_chain_tail() {
    let dm = DOMAIN_MOD.replace('\r', "");
    assert!(
        dm.contains("pub mod ap_input_tax_subject;"),
        "domain/mod.rs 必须声明本域，否则不参与迁移链"
    );
    let lb = flat(&LIB_RS.replace('\r', ""));
    let mounted = "Box::new(domain::ap_input_tax_subject::Migration),";
    assert!(lb.contains(mounted), "lib.rs 必须挂载本域迁移");
    let prev = lb
        .find("Box::new(domain::system_admin_bpm_config_grant::Migration),")
        .expect("链尾既有域必须可定位，它是判挂载顺序的锚点");
    let now = lb.find(mounted).expect("本域必须挂在迁移列表内");
    assert!(
        now > prev,
        "本域必须挂在迁移链尾：建表与科目种子域先行完成，才解析得到父科目 2221 的 id"
    );
}

/// 运行时写入侧改指 222102，且采购应付链路不得再出现销项科目码
#[test]
fn receipt_posts_input_tax_to_new_subject_only() {
    let src = RECEIPT.replace('\r', "");
    assert!(
        src.contains("Some(\"222102\".to_string())"),
        "进项税额分录的科目码必须是 222102"
    );
    assert!(
        !src.contains("Some(\"222101\".to_string())"),
        "采购应付链路不得再写销项科目码 222101，否则同码互抵又回来了"
    );
    assert!(
        src.contains("应交税费-应交增值税-进项税额"),
        "分录行上的科目名称与科目码必须同源，不得一名一码互相矛盾"
    );
}

/// 反向漂移防护：销项侧仍归 222101，种子名称未被改名成进项
#[test]
fn sales_output_tax_and_seed_names_stay_unchanged() {
    assert!(
        SHIP.contains("222101"),
        "销售发货开票的销项税额必须继续落 222101，本次改动不得波及销项"
    );
    let finance = flat(&FINANCE_MOD.replace('\r', ""));
    assert!(
        finance.contains("('222101','应交税费-应交增值税-销项税额'"),
        "222101 的种子名称必须仍是销项税额：不得靠改名把进项塞进同一枚科目"
    );
}
