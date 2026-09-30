use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseTransaction, EntityTrait, FromQueryResult, IntoActiveModel, PaginatorTrait,
    QueryFilter, SqlErr, Statement, TransactionSession, TransactionTrait,
};

/// 通用单号生成器
///
/// 标准格式：`{前缀}{YYYYMMDD}{左补零流水号}`（默认 3 位，可指定位数）。
///
/// ## 并发与唯一性约定（与调用方共同遵守）
/// - 取号前先取 `pg_advisory_xact_lock(prefix+date)` 会话级事务咨询锁，
///   同前缀同日的并发取号在锁上串行化，锁持有到最外层事务结束。
/// - 候选号从 `count+1` 起逐位探测占用（`column.eq(candidate)`），
///   对硬删除造成的序号空洞不产生 23505 冲突；同事务内自己的未提交插入
///   对本连接可见，因此 `取号→插入→取号→插入` 的多次取号是安全的。
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

        let txn = db
            .begin()
            .await
            .map_err(|e| AppError::internal(format!("开始事务失败: {:?}", e)))?;

        Self::lock_prefix(&txn, prefix, &today).await?;

        let result = Self::allocate_no::<E, C>(&txn, column, &date_prefix, width).await;

        // 探测过程只有 SELECT；无论成败都显式结束内层事务，错误路径回滚后原样上抛
        match result {
            Ok(no) => {
                txn.commit()
                    .await
                    .map_err(|e| AppError::internal(format!("提交事务失败: {:?}", e)))?;
                Ok(no)
            }
            Err(e) => {
                txn.rollback()
                    .await
                    .map_err(|re| AppError::internal(format!("回滚取号事务失败: {:?}", re)))?;
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
                .map_err(|e| AppError::internal(format!("创建取号保存点失败: {:?}", e)))?;

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
                    sp.rollback().await.map_err(|re| {
                        AppError::internal(format!("回滚取号保存点失败: {:?}", re))
                    })?;
                    return Err(e);
                }
            };

            match build_active_model(candidate.clone()).insert(&sp).await {
                Ok(model) => {
                    sp.commit()
                        .await
                        .map_err(|e| AppError::internal(format!("释放取号保存点失败: {:?}", e)))?;
                    return Ok(model);
                }
                Err(err) => {
                    let unique_conflict =
                        matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)));
                    sp.rollback().await.map_err(|re| {
                        AppError::internal(format!("回滚取号保存点失败: {:?}", re))
                    })?;
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

    /// 在持有咨询锁的同一连接/事务内分配一个未被占用的候选号：
    /// 基数 = 当日已有号数 + 1，逐个探测占用直至空闲。
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
        let count = E::find()
            .filter(column.starts_with(date_prefix))
            .count(conn)
            .await?;

        let width = std::cmp::Ord::max(width, 1);
        let mut seq = count + 1;
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
                        "单号探测跳过已占用号位（存量序号空洞或并发占用）"
                    );
                }
                return Ok(candidate);
            }
            seq += 1;
        }
        tracing::error!(date_prefix, "单号连续 {} 个候选位均被占用", NO_PROBE_MAX);
        Err(AppError::business_displayable(format!(
            "单号段 {} 连续 {} 个号位被占用，请联系管理员检查存量编号",
            date_prefix, NO_PROBE_MAX
        )))
    }
}

/// 单据号类型白名单注册表：doc_type → (实体表, 单号列) 的占用查询。
///
/// 为什么集中在这里：`/document-no/check` 的 doc_type 白名单历史上只有
/// 7 个类型（handlers/document_no_handler.rs），新单据（如 outsourcing_receipt）
/// 前端取号后查重直接 400，阻塞提交。所有需要自动编码、且可能被前端
/// 预生成/查重的单据都必须在此登记；新增单据时同步登记，避免再次出现
/// 白名单与实际单据脱节。取值键 = 后端单据类型 snake_case 名。
pub async fn is_document_no_taken<C: ConnectionTrait>(
    db: &C,
    doc_type: &str,
    no: &str,
) -> Result<bool, AppError> {
    use crate::models::{
        ap_invoice, ap_payment, ap_payment_request, ap_reconciliation, ap_verification,
        ar_collection, ar_invoice, ar_reconciliation, bpm_process_instance, bpm_task,
        cost_collection, crm_lead, customer, inventory_adjustment, inventory_count,
        inventory_transfer, mrp_result, outsourcing_order, outsourcing_receipt, product,
        purchase_inspection, purchase_order, purchase_receipt, purchase_return, sales_delivery,
        sales_order, sales_quotation, sales_return, supplier, voucher, warehouse,
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
        "customer" => customer::Entity::find()
            .filter(customer::Column::CustomerCode.eq(no))
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
        // —— 任务 #158 残余收口新增自动取号点（两列均带 UNIQUE，必须登记）——
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
        // —— 原有登记类型（handlers/document_no_handler.rs 旧 match 全集保留）——
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

fn compute_advisory_lock_key(prefix: &str, date: &str) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    date.hash(&mut hasher);
    hasher.finish() as i64
}
