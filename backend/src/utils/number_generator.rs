use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseTransaction, EntityTrait, FromQueryResult, IntoActiveModel, QueryFilter, QuerySelect,
    SqlErr, Statement, TransactionSession, TransactionTrait,
};

/// 通用单号生成器
///
/// 标准格式：`{前缀}{YYYYMMDD}{左补零流水号}`（默认 3 位，可指定位数）。
///
/// ## 并发与唯一性约定（与调用方共同遵守）
/// - 取号前先取 `pg_advisory_xact_lock(prefix+date)` 事务咨询锁，
///   同前缀同日的并发取号在锁上串行化，锁持有到最外层事务结束。
/// - 候选号基数与真实号段同源：`max(当日已有单号后缀流水) + 1` 起逐位
///   探测占用（非 `count + 1`：行数会把软删/旁路行计入基数导致跳号）。
///   `{流水}-{派生行}` 形态的派生变体行（如 MRP 批次行号）**同样参与基数**：
///   基数头本身可能不落库，只有派生行落库，排除它们会重发同一基数导致派生行
///   直撞单据号列 UNIQUE（见 `base_seq_from_suffix` 判据）。
///   软删行仍占用号段，基数与探测都按全表真实单号计算；同事务内自己的
///   未提交插入对本连接可见，因此 `取号→插入→取号→插入` 的多次取号是安全的。
/// - 号段之外的旁路写入/人工输入重复号，最终由单据号列的数据库 UNIQUE
///   索引兜底（migration m0063 为 9 张无约束表补齐；更早建表的列自带 UNIQUE），
///   INSERT 撞唯一约束即 SQLSTATE 23505，由 `insert_with_no_retry` 捕获重试。
/// - `*_with_txn` 变体把锁、探测与调用方 INSERT 放在同一事务：锁持有到
///   调用方提交，其他走本生成器的会话拿不到锁就不会并发取同号。
/// - `generate_no`（自开事务）变体在提交后即释放锁，返回的号码只保证
///   “提交时刻未被占用”；若业务要求 INSERT 也不容冲突，用
///   `insert_with_no_retry` 在调用方事务内取号并重试。
/// - INSERT 撞 23505（如旁路写入/人工输入重复号）时：`insert_with_no_retry`
///   在保存点内回滚该次尝试、重新取号重试，可重试而不是裸 500；
///   非唯一约束类 SQL 错误原样显式上抛，不吞不兜底。
pub struct DocumentNumberGenerator;

/// 候选号连续被占用时的探测上限（防止病态存量数据导致无界循环；
/// 超限显式报错，由数据侧排查，不静默跳过）
const NO_PROBE_MAX: u64 = 100;

/// INSERT 撞唯一约束(23505)时的最大重试次数（每次在保存点内重新取号）
const NO_INSERT_MAX_RETRY: usize = 5;

impl DocumentNumberGenerator {
    /// 生成标准格式单号: {前缀}{YYYYMMDD}{3位流水号}
    pub async fn generate_no<'db, E, C>(
        db: &'db (impl ConnectionTrait + TransactionTrait),
        prefix: &str,
        _entity: E,
        column: C,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        Self::generate_no_with_width(db, prefix, _entity, column, 3).await
    }

    /// 生成可指定流水位数的单号
    pub async fn generate_no_with_width<'db, E, C>(
        db: &'db (impl ConnectionTrait + TransactionTrait),
        prefix: &str,
        _entity: E,
        column: C,
        width: usize,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let date_prefix = format!("{}{}", prefix, today);

        // 事务/保存点生命周期失败是真正的 DbErr：按语义归口 AppError::database
        // （分类与日志口径同 utils/error.rs 的 From<DbErr> 映射），不再降级成 internal；
        // 已返回 AppError 的调用（lock_prefix/allocate_no/insert）一律 `?`/原样透传，
        // 任何取号失败都必须显式上抛，不存在"回落成时间戳拼号"的路径。
        let txn = db
            .begin()
            .await
            .map_err(|e| AppError::database(format!("取号事务开启失败: {e}")))?;

        Self::lock_prefix(&txn, prefix, &today).await?;

        let result = Self::allocate_no::<E, C>(&txn, column, &date_prefix, width).await;

        // 探测过程只有 SELECT；无论成败都显式结束内层事务，错误路径回滚后原样上抛
        match result {
            Ok(no) => {
                txn.commit()
                    .await
                    .map_err(|e| AppError::database(format!("取号事务提交失败: {e}")))?;
                Ok(no)
            }
            Err(e) => {
                txn.rollback()
                    .await
                    .map_err(|re| AppError::database(format!("取号事务回滚失败: {re}")))?;
                Err(e)
            }
        }
    }

    /// 在外部事务内生成单号
    pub async fn generate_no_with_txn<'db, E, C>(
        txn: &'db DatabaseTransaction,
        prefix: &str,
        _entity: E,
        column: C,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        Self::generate_no_with_width_txn(txn, prefix, _entity, column, 3).await
    }

    /// 在外部事务内生成单号（可指定位数）
    pub async fn generate_no_with_width_txn<'db, E, C>(
        txn: &'db DatabaseTransaction,
        prefix: &str,
        _entity: E,
        column: C,
        width: usize,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        C: ColumnTrait,
    {
        let today = Utc::now().format("%Y%m%d").to_string();
        let date_prefix = format!("{}{}", prefix, today);

        Self::lock_prefix(txn, prefix, &today).await?;

        Self::allocate_no::<E, C>(txn, column, &date_prefix, width).await
    }

    /// 在调用方事务内「取号 + 构建 ActiveModel + INSERT」并对唯一约束冲突(23505)
    /// 做保存点级重试：冲突仅回滚本次尝试、重新取号，不使外层事务整体失败；
    /// 非唯一约束类 SQL 错误原样上抛（分类见 utils/error.rs 的 DbErr 映射）。
    ///
    /// 为什么需要它：`generate_no`（自开事务）提交后锁即释放，调用方 INSERT 前
    /// 存在并发窗口；人工输入/旁路写入的重复号在 INSERT 时才暴露。
    /// PostgreSQL 语句失败后事务进入 aborted 态，必须回滚到保存点才能继续，
    /// 因此每次尝试都在 `txn.begin()`（SeaORM 对嵌套 begin 发 SAVEPOINT）内执行。
    pub async fn insert_with_no_retry<'db, E, A, F>(
        txn: &'db DatabaseTransaction,
        prefix: &str,
        _entity: E,
        column: E::Column,
        build_active_model: F,
    ) -> Result<E::Model, AppError>
    where
        E: EntityTrait + Copy,
        E::Column: Copy,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult + 'db,
        A: ActiveModelTrait<Entity = E> + ActiveModelBehavior + Send,
        E::Model: IntoActiveModel<A>,
        F: Fn(String) -> A,
    {
        for attempt in 1..=NO_INSERT_MAX_RETRY {
            // SeaORM 对已开启事务的 begin() 发 SAVEPOINT，commit/rollback
            // 对应 RELEASE/ROLLBACK TO（见 sea-orm 2.0.2 transaction.rs:147
            // 与 sqlx-postgres 0.9 transaction.rs begin_ansi_transaction_sql）
            let sp = txn
                .begin()
                .await
                .map_err(|e| AppError::database(format!("创建取号保存点失败: {e}")))?;

            let candidate = match Self::generate_no_with_width_txn(
                &sp,
                prefix,
                _entity,
                column,
                Self::DEFAULT_SEQ_WIDTH,
            )
            .await
            {
                Ok(no) => no,
                Err(e) => {
                    // 取号失败的原始 AppError 必须原样透传；保存点回滚失败是
                    // 另一笔 DbErr，单独显式记录（归口 database），不得顶替真实原因。
                    if let Err(re) = sp.rollback().await {
                        tracing::error!(
                            error = %re,
                            original_error = %e,
                            "回滚取号保存点失败（取号原始错误继续上抛）"
                        );
                    }
                    return Err(e);
                }
            };

            match build_active_model(candidate.clone()).insert(&sp).await {
                Ok(model) => {
                    sp.commit()
                        .await
                        .map_err(|e| AppError::database(format!("释放取号保存点失败: {e}")))?;
                    return Ok(model);
                }
                Err(err) => {
                    let unique_conflict =
                        matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)));
                    // 尝试已失败，必须先回滚保存点才能在同一事务里继续；
                    // 回滚本身失败属 DbErr（连接级异常），归口 database 显式上抛。
                    if let Err(re) = sp.rollback().await {
                        tracing::error!(
                            error = %re,
                            insert_error = %err,
                            "回滚取号保存点失败，中止本次取号重试"
                        );
                        return Err(AppError::database(format!("回滚取号保存点失败: {re}")));
                    }
                    if !unique_conflict {
                        // 与取号无关的 SQL 错误（FK/CHECK/类型）重试无意义，直接暴露真实错误
                        tracing::error!(
                            prefix,
                            attempt,
                            doc_no = %candidate,
                            error = %err,
                            "单号插入遇到非唯一约束 SQL 错误，不重试直接上抛"
                        );
                        return Err(AppError::from(err));
                    }
                    tracing::warn!(
                        prefix,
                        attempt,
                        doc_no = %candidate,
                        error = %err,
                        "单号唯一约束冲突(23505)，保存点已回滚，重新取号重试"
                    );
                }
            }
        }
        tracing::error!(
            prefix,
            "单号插入连续 {} 次唯一约束冲突，停止重试",
            NO_INSERT_MAX_RETRY
        );
        Err(AppError::business_displayable(format!(
            "单号连续 {} 次生成冲突，已停止重试，请稍后重新提交",
            NO_INSERT_MAX_RETRY
        )))
    }

    /// `insert_with_no_retry` 使用的默认流水位数，与 `generate_no` 的默认一致
    const DEFAULT_SEQ_WIDTH: usize = 3;

    /// 取 `prefix+date` 的事务咨询锁（同会话重复获取不阻塞；
    /// 最外层事务提交时释放，保存点回滚会释放该保存点之后获取的部分，
    /// 重试路径重新调用本函数即可再次持有）
    async fn lock_prefix(
        txn: &impl ConnectionTrait,
        prefix: &str,
        today: &str,
    ) -> Result<(), AppError> {
        let lock_key = compute_advisory_lock_key(prefix, today);
        txn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await?;
        Ok(())
    }

    /// 从单号后缀解析出「基数流水」（`allocate_no` 的基数判据）。
    ///
    /// 两种合法形态都参与基数：
    /// - 纯流水 `{seq}`：常规单号行；
    /// - `{seq}-{派生行}` 派生形态（如 `12-0`、`12-0-1`）：以同一基数派生后
    ///   落在**同一 UNIQUE 单据号列**上的行号行。典型调用方
    ///   `services/mrp_engine_ops/calculation.rs::batch_calculate`：批次基数
    ///   本身不落库，落库的是 `{基数}-{行序}`（及其 BOM 子行 `{基数}-{行序}-{子行}`），
    ///   若把这类行排除在基数之外，第二次批量计算会重发同一基数，`{基数}-0`
    ///   直接撞 `mrp_results.calculation_no` UNIQUE（同一订单连算两次 MRP，
    ///   第二次即 500）。
    /// 其余后缀（含字母混排、人工输入）不视为基数派生态、不参与 max，
    /// 由调用方逐条 warn 显式暴露——不做“猜首段数字”，避免把真正的旁路号
    /// 误当基数导致无谓跳号。
    /// 本判据只影响基数取值（方向上只会抬高基数、不会压低），占用探测与
    /// 23505 保存点重试语义不变。
    fn base_seq_from_suffix(suffix: &str) -> Option<u64> {
        if let Ok(seq) = suffix.parse::<u64>() {
            return Some(seq);
        }
        // 仅认「连续数字开头且紧跟 `-`」的派生行形态（`12-0` / `12-0-1`）
        let (head, _rest) = suffix.split_once('-')?;
        head.parse::<u64>().ok()
    }

    /// 在持有咨询锁的同一连接/事务内分配一个未被占用的候选号。
    ///
    /// 基数与真实号段同源：投影当日单号列（`select_only + into_tuple`，不读整行），
    /// 从已有单号里解析后缀流水取 `max(seq) + 1` 起探测。
    /// 刻意**不用** `count + 1`：行数把软删行/旁路写入的非常规行也计入，
    /// 既会跳过实际空闲的号位，也可能从低于号段末尾处起探测。
    /// 软删行**参与**基数是刻意为之：软删行仍占用该号，且单据号列带 UNIQUE
    /// 索引（migration m0063），复用旧号会直接撞 23505——探测可见、索引兜底，
    /// 二者口径一致才叫"与真实号段同源"。
    /// 带 `-派生行` 后缀的基数派生行同样参与基数（判据见 `base_seq_from_suffix`）；
    /// 其余非数字后缀（旁路/人工输入的单号）不参与 max，但逐条 warn 显式暴露。
    /// 探测能看见本事务未提交的插入，因此事务内「取号→插入→取号」序列安全。
    async fn allocate_no<E, C>(
        conn: &impl ConnectionTrait,
        column: C,
        date_prefix: &str,
        width: usize,
    ) -> Result<String, AppError>
    where
        E: EntityTrait,
        <<E as EntityTrait>::Column as std::str::FromStr>::Err: std::fmt::Debug,
        E::Model: Sync + Send + FromQueryResult,
        C: ColumnTrait,
    {
        // DbErr 经 `?` 走 utils/error.rs 的 From<DbErr> 分类映射（database 系），
        // 查询失败显式中止取号，不回落。
        let existing_numbers: Vec<(String,)> = E::find()
            .select_only()
            .column(column)
            .filter(column.starts_with(date_prefix))
            .into_tuple::<(String,)>()
            .all(conn)
            .await?;

        let mut max_seq: Option<u64> = None;
        for (no,) in existing_numbers {
            let Some(suffix) = no.strip_prefix(date_prefix) else {
                tracing::warn!(
                    date_prefix,
                    doc_no = %no,
                    "单号无法剥离当日前缀（旁路写入格式异常），不参与序号基数"
                );
                continue;
            };
            match Self::base_seq_from_suffix(suffix) {
                Some(seq) => max_seq = Some(max_seq.map_or(seq, |m| std::cmp::max(m, seq))),
                None => tracing::warn!(
                    date_prefix,
                    doc_no = %no,
                    suffix,
                    "单号后缀既非纯数字流水也非 流水-派生行 派生态（旁路/人工输入），不参与序号基数"
                ),
            }
        }

        let width = std::cmp::Ord::max(width, 1);
        // saturating：后缀恰为 u64::MAX 的病态存量号不得让取号算术溢出 panic，
        // 探测循环与 23505 重试仍会把冲突显式暴露（探测到号位占用即失败可见）。
        let mut seq = max_seq.map_or(1, |m| m.saturating_add(1));
        for probe in 0..NO_PROBE_MAX {
            let candidate = format!("{}{:0width$}", date_prefix, seq, width = width);
            let taken = E::find()
                .filter(column.eq(candidate.as_str()))
                .one(conn)
                .await?
                .is_some();
            if !taken {
                if probe > 0 {
                    tracing::info!(
                        date_prefix,
                        skipped = probe,
                        doc_no = %candidate,
                        "单号探测跳过已占用号位（号段内软删占用或并发占用）"
                    );
                }
                return Ok(candidate);
            }
            // 与基数处的 saturating 同理：基数逼近 u64::MAX 的病态号段下，
            // 逐位探测的推进也不得溢出 panic（debug CI 会直接炸在这行），
            // 饱和后同一候选位被重复探测 100 次，最终走显式报错路径失败可见。
            seq = seq.saturating_add(1);
        }
        tracing::error!(date_prefix, "单号连续 {} 个候选位均被占用", NO_PROBE_MAX);
        Err(AppError::business_displayable(format!(
            "单号段 {} 连续 {} 个号位被占用，请联系管理员检查存量编号",
            date_prefix, NO_PROBE_MAX
        )))
    }
}

/// 单据号类型白名单注册表：doc_type → (实体表, 单号列) 的占用查询。
/// 调用方：`/document-no/check`（handlers/document_no_handler.rs）及各单据
/// 服务保存前的号段占用探测。
///
/// 为什么集中在这里：doc_type 白名单必须覆盖所有需要自动编码、且可能被前端
/// 预生成/查重的单据——漏登记会让前端取号查重直接 400、阻塞提交；
/// 新增单据时同步在此登记，保持白名单与实际单据一致。
/// 取值键 = 后端单据类型 snake_case 名。
pub async fn is_document_no_taken<C: ConnectionTrait>(
    db: &C,
    doc_type: &str,
    no: &str,
) -> Result<bool, AppError> {
    use crate::models::{
        ap_invoice, ap_payment, ap_payment_request, ap_reconciliation, ap_verification,
        ar_collection, ar_invoice, ar_reconciliation, bpm_process_instance, bpm_task,
        cost_collection, crm_lead, crm_opportunity, customer, customer_transfer_approval,
        inventory_adjustment, inventory_count, inventory_transfer, mrp_result, outsourcing_order,
        outsourcing_receipt, outsourcing_voucher, process_wage_rate, product, purchase_inspection,
        purchase_order, purchase_receipt, purchase_return, sales_delivery, sales_order,
        sales_quotation, sales_return, supplier, voucher, wage_record, warehouse,
    };

    let taken = match doc_type {
        // —— 销售域 ——
        "sales_order" => sales_order::Entity::find()
            .filter(sales_order::Column::OrderNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "sales_delivery" => sales_delivery::Entity::find()
            .filter(sales_delivery::Column::DeliveryNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "sales_return" => sales_return::Entity::find()
            .filter(sales_return::Column::ReturnNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "sales_quotation" => sales_quotation::Entity::find()
            .filter(sales_quotation::Column::QuotationNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 采购域 ——
        "purchase_order" => purchase_order::Entity::find()
            .filter(purchase_order::Column::OrderNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "purchase_receipt" => purchase_receipt::Entity::find()
            .filter(purchase_receipt::Column::ReceiptNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "purchase_inspection" => purchase_inspection::Entity::find()
            .filter(purchase_inspection::Column::InspectionNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "purchase_return" => purchase_return::Entity::find()
            .filter(purchase_return::Column::ReturnNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 委外域 ——
        "outsourcing_order" => outsourcing_order::Entity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "outsourcing_receipt" => outsourcing_receipt::Entity::find()
            .filter(outsourcing_receipt::Column::ReceiptNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // 委外加工凭证：四类前缀 OVIS/OVFE/OVRC/OVLS 同落 outsourcing_voucher.voucher_no
        // （取号/查重方 services/outsourcing_ops/voucher.rs），未登记时
        // /document-no/check 对该单据 400 ⇒ 前端预生成号无法查重。
        "outsourcing_voucher" => outsourcing_voucher::Entity::find()
            .filter(outsourcing_voucher::Column::VoucherNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 薪酬域（WR→wage_record.record_no、PWR→process_wage_rate.rate_no，
        //    取号方 services/wage_ops/{record,rate}.rs）——
        "wage_record" => wage_record::Entity::find()
            .filter(wage_record::Column::RecordNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "process_wage_rate" => process_wage_rate::Entity::find()
            .filter(process_wage_rate::Column::RateNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 库存域 ——
        "inventory_transfer" => inventory_transfer::Entity::find()
            .filter(inventory_transfer::Column::TransferNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "inventory_adjustment" => inventory_adjustment::Entity::find()
            .filter(inventory_adjustment::Column::AdjustmentNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "inventory_count" => inventory_count::Entity::find()
            .filter(inventory_count::Column::CountNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 财务域 ——
        "ar_invoice" => ar_invoice::Entity::find()
            .filter(ar_invoice::Column::InvoiceNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ar_reconciliation" => ar_reconciliation::Entity::find()
            .filter(ar_reconciliation::Column::ReconciliationNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ar_collection" => ar_collection::Entity::find()
            .filter(ar_collection::Column::CollectionNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ap_invoice" => ap_invoice::Entity::find()
            .filter(ap_invoice::Column::InvoiceNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ap_payment" => ap_payment::Entity::find()
            .filter(ap_payment::Column::PaymentNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ap_payment_request" => ap_payment_request::Entity::find()
            .filter(ap_payment_request::Column::RequestNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ap_reconciliation" => ap_reconciliation::Entity::find()
            .filter(ap_reconciliation::Column::ReconciliationNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "ap_verification" => ap_verification::Entity::find()
            .filter(ap_verification::Column::VerificationNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "voucher" => voucher::Entity::find()
            .filter(voucher::Column::VoucherNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "cost_collection" => cost_collection::Entity::find()
            .filter(cost_collection::Column::CollectionNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— BPM / CRM / 主数据 ——
        "bpm_process_instance" => bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::InstanceNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "bpm_task" => bpm_task::Entity::find()
            .filter(bpm_task::Column::TaskNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "crm_lead" => crm_lead::Entity::find()
            .filter(crm_lead::Column::LeadNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // 商机编号（OPP{YYYYMMDD}{3位流水}，取号方 services/crm/opp.rs）
        "crm_opportunity" => crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::OpportunityNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "customer" => customer::Entity::find()
            .filter(customer::Column::CustomerCode.eq(no))
            .one(db)
            .await?
            .is_some(),
        // 客户转移审批单号（TA{YYYYMMDD}{3位流水}，取号方
        // services/crm/customer_transfer_approval_service.rs；实体名单数、真实表
        // customer_transfer_approvals）
        "customer_transfer_approval" => customer_transfer_approval::Entity::find()
            .filter(customer_transfer_approval::Column::ApprovalNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "supplier" => supplier::Entity::find()
            .filter(supplier::Column::SupplierCode.eq(no))
            .one(db)
            .await?
            .is_some(),
        "product" => product::Entity::find()
            .filter(product::Column::Code.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 自动取号且列带 UNIQUE 的表（必须登记进占用探测，否则撞号）——
        // MRP 计算单号（mrp_results.calculation_no UNIQUE：
        // migration/src/domain/business/m0007_add_mrp_production_bom.rs:105；
        // 取号方 services/mrp_engine_ops/calculation.rs）
        "mrp_result" => mrp_result::Entity::find()
            .filter(mrp_result::Column::CalculationNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        // 仓库编码（warehouses.warehouse_code UNIQUE：
        // migration/src/domain/system/m0001_initial_schema.rs:281；
        // 取号方 services/warehouse_service.rs，人工传入码原样保留）
        "warehouse" => warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(no))
            .one(db)
            .await?
            .is_some(),
        // —— 其他登记单据类型（染色批次/配方、合同、发票）——
        "dye_batch" => crate::models::dye_batch::Entity::find()
            .filter(crate::models::dye_batch::Column::BatchNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "dye_recipe" => crate::models::dye_recipe::Entity::find()
            .filter(crate::models::dye_recipe::Column::RecipeNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "sales_contract" => crate::models::sales_contract::Entity::find()
            .filter(crate::models::sales_contract::Column::ContractNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "purchase_contract" => crate::models::purchase_contract::Entity::find()
            .filter(crate::models::purchase_contract::Column::ContractNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "labor_contract" => crate::models::labor_contract::Entity::find()
            .filter(crate::models::labor_contract::Column::ContractNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        "finance_invoice" => crate::models::finance_invoice::Entity::find()
            .filter(crate::models::finance_invoice::Column::InvoiceNo.eq(no))
            .one(db)
            .await?
            .is_some(),
        other => {
            return Err(AppError::bad_request(format!("未知的单据类型：{}", other)));
        }
    };

    Ok(taken)
}

/// 计算 `prefix+date` 的 `pg_advisory_xact_lock` key。
///
/// **前提显式声明（不许删锁、不许改成不安全近似）**：本函数用
/// `DefaultHasher`（SipHash），其取值仅保证**同一已编译二进制内稳定**——
/// 跨进程（不同构建产物的实例混跑）或未来 Rust std 变更 Hasher 实现后，
/// 同一 `(prefix, date)` 输入不保证得到同一 key，届时新旧进程可能各自
/// 持有"不同 key"而对同一号段失去互斥。因此本锁的定位是**同版本多会话间
/// 的取号串行化加速器**；跨进程/跨版本的最终正确性保证是单据号列的
/// 数据库 UNIQUE 索引 + `insert_with_no_retry` 对 23505 的保存点重试
/// （m0063 补齐 9 表约束）。key 漂移的最坏后果退化为"偶发撞号→重试"，
/// 不会落库重号，故维持现状并在此声明前提，供后续维护者评估是否升级为
/// 跨版本稳定的显式 key 方案。
fn compute_advisory_lock_key(prefix: &str, date: &str) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    date.hash(&mut hasher);
    hasher.finish() as i64
}
