//! 契约锁（wave12），凭证分录形态门与草稿凭证写入口期间锁与期末结转竞态形态
//!
//! 来源为运行期真实性审计 D5 后端、D6、D10 三条缺陷的修复形态，三条都是"代码形态"级
//! 不变量，用 include_str! 固化源码文本静态跑（无需 PostgreSQL），结构与
//! `contract_wave5_delete_precheck_and_status_literal_lock_test.rs` 的源码扫描锁一致。
//!
//! 锁定的契约。
//! 一，D5，分录形态门（行数至少 1 行，借贷平衡，合计金额非零）在创建与更新与提交审核
//!     过账全部写口共用同一内核 `validate_entry_shape_core`，拒绝走可外显的
//!     `AppError::validation_displayable` 点名缺什么，0 分录与全零金额凭证不得再过账。
//! 二，D6，凭证 update 与 delete 两个写口在事务内接 `check_date_locked_txn`，
//!     已关账期间内的草稿凭证不得再被改写或删除，不得使用事务外的 `check_date_locked`
//!     重新引入检查与写入之间的 TOCTOU。
//! 三，D10，期末结转 `upsert_next_period_balance_txn` 不得再是先查后插裸 insert，
//!     必须为 INSERT ON CONFLICT DO NOTHING 占位加冲突后 lock_exclusive 读改写，
//!     并发撞 `idx_account_balances_unique` 的整单失败由形态上消除。
//!
//! 检测力自证，每条锁的判定谓词同时跑两份输入，真实源码必须零命中（修复在场），
//! 内嵌的修复前坏样本必须逐条命中全部预期违规（证明谓词真的能拦住退化，不是恒真断言）。
//!
//! 写不出的部分（如实声明），D10 的真并发时序（两个会话对同一科目同时结转撞唯一索引）
//! 无法用静态源码锁或单线程测试确定性地复现，本锁只钉死消除竞态的写口形态。
//! 行为级并发证明需要双连接交错编排，建议交集成测试或在运行期加撞索引哨兵计数。
//! 另，D5 中 `subject_code.unwrap_or_default()` 的空串行写入本波未动，31b 删除矩阵
//! e2e 依赖无科目行建单的现状，需要科目必填契约决策后另波处理。

/// 取函数体，找不到签名直接 None，由调用方按地板判红或记为违规。
fn try_fn_body(src: &str, signature: &str) -> Option<String> {
    let anchor = src.find(signature)?;
    let tail = &src[anchor..];
    let next = [
        "\n    pub(super) async fn ",
        "\n    pub(super) fn ",
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    Some(tail[..next].to_string())
}

/// 取函数体，从签名处开始，到下一个同级条目边界（impl 内四空格缩进的 fn 声明）之前为止。
/// 找不到签名直接 panic，这是检测力地板，禁止"取不到就当通过"。
fn fn_body(src: &str, signature: &str) -> String {
    try_fn_body(src, signature)
        .unwrap_or_else(|| panic!("待锁函数签名不存在（检测力地板）: {signature}"))
}

fn read(path: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let full = std::path::Path::new(manifest_dir).join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", full.display()))
        .replace('\r', "")
}

// ---------------------------------------------------------------------------
// 谓词一，D5 分录形态门
// ---------------------------------------------------------------------------

/// 判定分录形态门退化点，返回命中的违规名清单（空清单表示修复形态在场）。
fn d5_shape_gate_violations(crud_src: &str, workflow_src: &str) -> Vec<&'static str> {
    let mut violations = Vec::new();

    let create_validate = fn_body(crud_src, "async fn validate_voucher_create_req(");
    if !create_validate.contains("validate_voucher_items_shape(") {
        violations.push("创建侧未接分录形态门");
    }

    let replace_items = fn_body(crud_src, "async fn replace_voucher_items(");
    if !replace_items.contains("validate_voucher_items_shape(") {
        violations.push("更新分量替换未接分录形态门");
    }

    match try_fn_body(crud_src, "fn validate_entry_shape_core(") {
        None => violations.push("分录形态门内核不存在"),
        Some(core) => {
            if !core.contains(".is_empty()") {
                violations.push("形态门内核缺行数门");
            }
            if !(core.contains("Decimal::ZERO") || core.contains("is_zero")) {
                violations.push("形态门内核缺非零金额门");
            }
            if !core.contains("Self::balance_error(") {
                violations.push("形态门内核未复用 balance_error 唯一装配点");
            }
            if core.matches("AppError::validation_displayable(").count() < 2 {
                violations.push("形态门内核拒绝未点名缺什么（可外显装配点不足两处）");
            }
        }
    }

    if !fn_body(workflow_src, "async fn validate_voucher(").contains("validate_entry_shape_core(") {
        violations.push("提交审核校验未走形态门内核");
    }
    if !fn_body(workflow_src, "async fn validate_voucher_in_transaction(")
        .contains("validate_entry_shape_core(")
    {
        violations.push("过账校验未走形态门内核");
    }

    violations
}

/// D5 真实源码零命中。
#[test]
fn d5_voucher_shape_gate_present_in_real_sources() {
    let crud = read("src/services/voucher_ops/crud.rs");
    let workflow = read("src/services/voucher_ops/workflow.rs");
    assert!(
        d5_shape_gate_violations(&crud, &workflow).is_empty(),
        "D5 分录形态门退化点命中: {:?}",
        d5_shape_gate_violations(&crud, &workflow)
    );
}

/// D5 检测力自证，修复前的坏样本必须逐条命中全部违规点。
#[test]
fn d5_predicate_detects_pre_fix_shape() {
    const BAD_CRUD: &str = r#"
impl VoucherService {
    async fn validate_voucher_create_req(
        req: &CreateVoucherRequest,
        db: Arc<DatabaseConnection>,
    ) -> Result<(), AppError> {
        if !crate::utils::config::is_production() {
            let total_debit: Decimal = req.items.iter().map(|i| i.debit).sum();
            let total_credit: Decimal = req.items.iter().map(|i| i.credit).sum();
            if total_debit != total_credit {
                return Err(Self::balance_error(total_debit, total_credit));
            }
            return Ok(());
        }
        let period_svc = AccountingPeriodService::new(db);
        period_svc.check_date_locked(req.voucher_date).await?;
        let total_debit: Decimal = req.items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = req.items.iter().map(|i| i.credit).sum();
        if total_debit != total_credit {
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }

    pub(super) fn balance_error(total_debit: Decimal, total_credit: Decimal) -> AppError {
        AppError::validation(format!("借 {} != 贷 {}", total_debit, total_credit))
    }

    async fn replace_voucher_items(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        Self::validate_voucher_items_balance(items)?;
        Ok(())
    }

    fn validate_voucher_items_balance(items: &[VoucherItemRequest]) -> Result<(), AppError> {
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        if total_debit != total_credit {
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }
}
"#;
    const BAD_WORKFLOW: &str = r#"
impl VoucherService {
    async fn validate_voucher(&self, id: i32) -> Result<(), AppError> {
        let items = voucher_item::Entity::find().all(&*self.db).await?;
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        if total_debit != total_credit {
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }

    async fn validate_voucher_in_transaction(
        &self,
        id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let items = voucher_item::Entity::find().all(txn).await?;
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        if total_debit != total_credit {
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }
}
"#;
    let violations = d5_shape_gate_violations(BAD_CRUD, BAD_WORKFLOW);
    let expected = [
        "创建侧未接分录形态门",
        "更新分量替换未接分录形态门",
        "分录形态门内核不存在",
        "提交审核校验未走形态门内核",
        "过账校验未走形态门内核",
    ];
    for name in expected {
        assert!(
            violations.contains(&name),
            "D5 坏样本应命中违规 {name}，实际命中: {violations:?}"
        );
    }
    assert_eq!(
        violations.len(),
        expected.len(),
        "D5 坏样本命中集合必须与预期逐项相等"
    );

    // 半退化自证，内核在但只查平衡（无行数门与非零门与可外显装配点）也必须命中
    const HALF_CRUD: &str = r#"
impl VoucherService {
    async fn validate_voucher_create_req(
        req: &CreateVoucherRequest,
        db: Arc<DatabaseConnection>,
    ) -> Result<(), AppError> {
        Self::validate_voucher_items_shape(&req.items)?;
        Ok(())
    }

    pub(super) fn balance_error(total_debit: Decimal, total_credit: Decimal) -> AppError {
        AppError::validation(format!("借 {} != 贷 {}", total_debit, total_credit))
    }

    pub(super) fn validate_voucher_items_shape(
        items: &[VoucherItemRequest],
    ) -> Result<(), AppError> {
        let rows: Vec<(i32, Decimal, Decimal)> = items.iter().map(|i| (1, i.debit, i.credit)).collect();
        Self::validate_entry_shape_core(&rows)
    }

    pub(super) fn validate_entry_shape_core(
        rows: &[(i32, Decimal, Decimal)],
    ) -> Result<(), AppError> {
        let total_debit: Decimal = rows.iter().map(|(_, d, _)| *d).sum();
        let total_credit: Decimal = rows.iter().map(|(_, _, c)| *c).sum();
        if total_debit != total_credit {
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }

    async fn replace_voucher_items(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        Self::validate_voucher_items_shape(items)?;
        Ok(())
    }
}
"#;
    let half = d5_shape_gate_violations(HALF_CRUD, BAD_WORKFLOW_REAL_GATE);
    assert!(
        half.contains(&"形态门内核缺行数门")
            && half.contains(&"形态门内核缺非零金额门")
            && half.contains(&"形态门内核拒绝未点名缺什么（可外显装配点不足两处）"),
        "半退化内核样本应命中三道子门缺失，实际命中: {half:?}"
    );
}

/// 内核接线正确但内核自身退化的样本，workflow 侧用真实形态门调用铺路。
const BAD_WORKFLOW_REAL_GATE: &str = r#"
impl VoucherService {
    async fn validate_voucher(&self, id: i32) -> Result<(), AppError> {
        let items = voucher_item::Entity::find().all(&*self.db).await?;
        let rows: Vec<(i32, Decimal, Decimal)> = items.iter().map(|i| (i.line_no, i.debit, i.credit)).collect();
        Self::validate_entry_shape_core(&rows)
    }

    async fn validate_voucher_in_transaction(
        &self,
        id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let items = voucher_item::Entity::find().all(txn).await?;
        Self::validate_entry_shape_core(&[])
    }
}
"#;

// ---------------------------------------------------------------------------
// 谓词二，D6 已关账期间草稿凭证写入口
// ---------------------------------------------------------------------------

/// 判定 update 与 delete 的期间锁退化点（空清单表示修复形态在场）。
fn d6_period_gate_violations(crud_src: &str) -> Vec<&'static str> {
    let mut violations = Vec::new();

    let update_body = fn_body(crud_src, "pub async fn update(");
    if !update_body.contains("check_date_locked_txn(&txn,") {
        violations.push("凭证更新缺事务内期间锁门");
    }
    if update_body.contains("check_date_locked(") {
        violations.push("凭证更新用了事务外期间锁，TOCTOU 形态回潮");
    }

    let delete_body = fn_body(crud_src, "pub async fn delete(");
    if !delete_body.contains("check_date_locked_txn(&txn,") {
        violations.push("凭证删除缺事务内期间锁门");
    }
    if delete_body.contains("check_date_locked(") {
        violations.push("凭证删除用了事务外期间锁，TOCTOU 形态回潮");
    }

    violations
}

/// D6 真实源码零命中。
#[test]
fn d6_closed_period_gates_present_in_real_sources() {
    let crud = read("src/services/voucher_ops/crud.rs");
    assert!(
        d6_period_gate_violations(&crud).is_empty(),
        "D6 期间锁退化点命中: {:?}",
        d6_period_gate_violations(&crud)
    );
}

/// D6 检测力自证，修复前无期间锁的坏样本必须逐条命中。
#[test]
fn d6_predicate_detects_pre_fix_shape() {
    const BAD_CRUD: &str = r#"
impl VoucherService {
    pub async fn update(
        &self,
        id: i32,
        req: UpdateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        let txn = self.db.begin().await?;
        let voucher_model = Self::load_voucher_for_update(id, &txn).await?;
        let mut active_model: voucher::ActiveModel = voucher_model.into_active_model();
        txn.commit().await?;
        Ok(updated_voucher)
    }

    async fn load_voucher_for_update(
        id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<voucher::Model, AppError> {
        Ok(voucher::Entity::find_by_id(id).one(txn).await?.unwrap())
    }

    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id).lock_exclusive().one(&txn).await?;
        txn.commit().await?;
        Ok(())
    }
}
"#;
    let violations = d6_period_gate_violations(BAD_CRUD);
    for name in ["凭证更新缺事务内期间锁门", "凭证删除缺事务内期间锁门"] {
        assert!(
            violations.contains(&name),
            "D6 坏样本应命中违规 {name}，实际命中: {violations:?}"
        );
    }
    assert_eq!(violations.len(), 2, "D6 坏样本命中集合必须恰为两条缺锁违规");

    // TOCTOU 变体自证，接了事务外 check_date_locked 的样本必须命中回潮违规
    const TOCTOU_CRUD: &str = r#"
impl VoucherService {
    pub async fn update(
        &self,
        id: i32,
        req: UpdateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        let txn = self.db.begin().await?;
        period_svc.check_date_locked(date).await?;
        txn.commit().await?;
        Ok(voucher)
    }

    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        period_svc.check_date_locked(date).await?;
        txn.commit().await?;
        Ok(())
    }
}
"#;
    let toctou = d6_period_gate_violations(TOCTOU_CRUD);
    assert!(toctou.contains(&"凭证更新用了事务外期间锁，TOCTOU 形态回潮"));
    assert!(toctou.contains(&"凭证删除用了事务外期间锁，TOCTOU 形态回潮"));
}

// ---------------------------------------------------------------------------
// 谓词三，D10 期末结转竞态形态
// ---------------------------------------------------------------------------

/// 判定期末结转写口退化点（空清单表示修复形态在场）。
fn d10_carry_forward_violations(period_src: &str) -> Vec<&'static str> {
    let mut violations = Vec::new();
    let body = fn_body(period_src, "async fn upsert_next_period_balance_txn(");

    if !body.contains(".on_conflict(") {
        violations.push("结转写口缺 on_conflict 仲裁");
    }
    if !body.contains("do_nothing()") {
        violations.push("结转写口缺占位跳过（撞索引会整单失败）");
    }
    if !body.contains("lock_exclusive()") {
        violations.push("冲突后的读改写缺行锁");
    }
    if body.contains("if let Some(existing)") {
        violations.push("结转写口回潮为先查后分支形态");
    }

    violations
}

/// D10 真实源码零命中。
#[test]
fn d10_carry_forward_shape_present_in_real_source() {
    let period = read("src/services/accounting_period_service.rs");
    assert!(
        d10_carry_forward_violations(&period).is_empty(),
        "D10 结转竞态退化点命中: {:?}",
        d10_carry_forward_violations(&period)
    );
}

/// D10 检测力自证，修复前 find-then-insert 裸 insert 的坏样本必须逐条命中。
#[test]
fn d10_predicate_detects_pre_fix_shape() {
    const BAD_PERIOD: &str = r#"
impl AccountingPeriodService {
    async fn upsert_next_period_balance_txn(
        txn: &sea_orm::DatabaseTransaction,
        balance: &crate::models::account_balance::Model,
        next_period: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, AppError> {
        let existing = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.eq(balance.subject_id))
            .one(txn)
            .await?;
        if let Some(existing) = existing {
            active.update(txn).await?;
            Ok(false)
        } else {
            new_balance.insert(txn).await?;
            Ok(true)
        }
    }

    pub async fn check_date_locked(&self, date: chrono::NaiveDate) -> Result<(), AppError> {
        Ok(())
    }
}
"#;
    let violations = d10_carry_forward_violations(BAD_PERIOD);
    for name in [
        "结转写口缺 on_conflict 仲裁",
        "结转写口缺占位跳过（撞索引会整单失败）",
        "冲突后的读改写缺行锁",
        "结转写口回潮为先查后分支形态",
    ] {
        assert!(
            violations.contains(&name),
            "D10 坏样本应命中违规 {name}，实际命中: {violations:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 前端静默丢行侧的静态钉（与后端 D5 锁同文件登记，检测口径为源码文本）
// ---------------------------------------------------------------------------

/// useVchr.ts 提交路径不得再出现"未选科目行被 filter 静默删除"的形态，
/// 且必须存在点名行号的显式拦截。前端行为级锁不在本后端文件职责内，
/// 此处钉的是源码形态，eslint 与 vue-tsc 门禁在同波次另行执行。
#[test]
fn d5_frontend_usevchr_no_silent_row_drop() {
    let src = read("../frontend/src/views/finance/tabs/composables/useVchr.ts");

    assert!(
        !src.contains(".filter(e => e.subject_id)"),
        "useVchr.ts 不得回潮未选科目行静默丢行的 filter 形态"
    );
    assert!(
        src.contains("missingSubjectLines") && src.contains("entryMissingSubject"),
        "useVchr.ts 必须保留点名行号的显式科目校验与拦截提示"
    );
    assert!(
        src.contains("return false"),
        "校验失败必须阻止提交（存在 return false 出口）"
    );
}
