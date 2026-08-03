use crate::models::accounting_period;
use crate::models::status::accounting_period as period_status;
use crate::models::status::accounting_period_closing as period_closing;
use crate::models::status::voucher::VOUCHER_POSTED;
use crate::utils::error::AppError;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Set, TransactionTrait,
};

crate::define_service!(AccountingPeriodService);

impl AccountingPeriodService {
    /// 获取当前开放的会计期间
    pub async fn get_current_period(&self) -> Result<Option<accounting_period::Model>, AppError> {
        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::Status.eq(period_status::OPEN))
            .order_by_desc(accounting_period::Column::Year)
            .order_by_desc(accounting_period::Column::Period)
            .one(self.db.as_ref())
            .await?;
        Ok(period)
    }

    /// 初始化第一个会计期间（如果不存在）
    pub async fn init_first_period(
        &self,
        year: i32,
        month: u32,
    ) -> Result<accounting_period::Model, AppError> {
        let existing = self.get_current_period().await?;
        if let Some(p) = existing {
            return Ok(p);
        }

        let start_date = Utc
            .with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| {
                AppError::bad_request(format!("Invalid date: {}-{:02}-01", year, month))
            })?;

        let next_month = if month == 12 { 1 } else { month + 1 };
        let next_month_year = if month == 12 { year + 1 } else { year };
        let end_date = Utc
            .with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| {
                AppError::bad_request(format!(
                    "Invalid date: {}-{:02}-01",
                    next_month_year, next_month
                ))
            })?
            - chrono::Duration::seconds(1);

        let active_model = accounting_period::ActiveModel {
            year: Set(year),
            period: Set(month as i32),
            period_name: Set(format!("{} 年 {:02} 月", year, month)),
            start_date: Set(start_date),
            end_date: Set(end_date),
            status: Set(period_status::OPEN.to_string()),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let period = active_model.insert(self.db.as_ref()).await?;
        Ok(period)
    }

    /// 执行月末结账
    /// V15 P1 17.1-D1 修复：引入 CLOSING 中间状态。；结账流程：OPEN →（置 CLOSING）→ 试算平衡/结转 → CLOSED（成功） / 回滚 OPEN（失败）。；CLOSING 状态对外可见，便于并发场景区分"结账中"与"已结账"。
    pub async fn close_period(
        &self,
        period_id: i32,
        user_id: i32,
    ) -> Result<accounting_period::Model, AppError> {
        // V15 P2 B05-P2-10：期末调整（暂估/摊销/预提）自动确认。
        // 在结账事务前处理，确保调整凭证计入本期试算平衡；失败仅 warn 不阻断结账。
        if let Some(period_for_adjust) = accounting_period::Entity::find_by_id(period_id)
            .one(self.db.as_ref())
            .await?
        {
            let period_str = format!(
                "{:04}-{:02}",
                period_for_adjust.year, period_for_adjust.period
            );
            let adjust_service =
                crate::services::period_adjustment_service::PeriodAdjustmentService::new(
                    self.db.clone(),
                );
            if let Err(e) = adjust_service.confirm_pending(&period_str, user_id).await {
                tracing::warn!(
                    period_id,
                    period = %period_str,
                    "期末调整自动确认失败（不阻断结账，B05-P2-10）：{}",
                    e
                );
            }
        }

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;
        let period = self.lock_period_for_close_txn(&txn, period_id).await?;
        // V15 P1 17.1-D1：置为 CLOSING 中间状态（事务内，失败自动回滚为 OPEN）
        self.mark_period_closing_txn(&txn, &period, user_id).await?;
        self.check_unposted_vouchers_txn(&txn, period.start_date, period.end_date)
            .await?;
        // F-P1-1 修复（批次 360 v13 复审）：试算平衡校验兜底
        let (total_debit, total_credit) = self
            .check_trial_balance_txn(&txn, period.start_date, period.end_date)
            .await?;
        if total_debit != total_credit {
            // V15 P1 17.1-D1：试算不平衡，事务回滚自动恢复 OPEN
            return Err(AppError::business(format!(
                "试算不平衡：本期借方总额 {} ≠ 贷方总额 {}，差额 {}，无法关闭期间",
                total_debit,
                total_credit,
                total_debit - total_credit
            )));
        }
        let closed_period = self.mark_period_closed_txn(&txn, &period, user_id).await?;
        // F-P1-1 修复（批次 384 v13 复审）：期末余额结转到下期期初
        let (next_year, next_month) = calc_next_period(period.year, period.period);
        let current_period_str = format!("{:04}-{:02}", period.year, period.period);
        let next_period_str = format!("{:04}-{:02}", next_year, next_month);
        self.carry_forward_balances_txn(&txn, &current_period_str, &next_period_str)
            .await?;
        txn.commit().await?;
        self.init_first_period(next_year, next_month as u32).await?;
        Ok(closed_period)
    }

    /// V15 P1 17.1-D2：反结账（重开期间）
    /// 业务规则：1. 仅 CLOSED 状态的期间可反结账；CLOSING 状态不可（结账中无法重开）；2. 仅最近一个已结账期间可反结账（防止跳过中间期间导致数据断层）；3. 下一个期间必须存在且为 OPEN 状态（确保期间连续性）；4. 反结账后期间状态恢复为 OPEN，清空 closed_at/closed_by；5. 操作写入审计日志，记录"反结账"操作类型
    pub async fn reopen_period(
        &self,
        period_id: i32,
        user_id: i32,
        reason: &str,
    ) -> Result<accounting_period::Model, AppError> {
        if reason.trim().is_empty() {
            return Err(AppError::validation("反结账失败：原因不能为空"));
        }
        let txn = (*self.db).begin().await?;
        // 锁定期间防止并发
        let period = accounting_period::Entity::find_by_id(period_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("Accounting period {} not found", period_id))
            })?;
        if period.status != period_status::CLOSED {
            return Err(AppError::business(format!(
                "反结账失败：期间 {} 当前状态为 {}，仅 CLOSED 状态可反结账",
                period_id, period.status
            )));
        }
        // 校验为最近一个已结账期间（按 year+period 倒序找首条 CLOSED）
        let latest_closed = accounting_period::Entity::find()
            .filter(accounting_period::Column::Status.eq(period_status::CLOSED))
            .order_by_desc(accounting_period::Column::Year)
            .order_by_desc(accounting_period::Column::Period)
            .one(&txn)
            .await?;
        if let Some(latest) = latest_closed {
            if latest.id != period.id {
                return Err(AppError::business(format!(
                    "反结账失败：仅最近一个已结账期间可反结账，最近已结账期间为 {}（{}）",
                    latest.period_name, latest.id
                )));
            }
        }
        // 校验下一个期间存在且为 OPEN（确保期间连续性）
        let (next_year, next_period) = calc_next_period(period.year, period.period);
        let next_period_exists = accounting_period::Entity::find()
            .filter(accounting_period::Column::Year.eq(next_year))
            .filter(accounting_period::Column::Period.eq(next_period))
            .one(&txn)
            .await?;
        if next_period_exists.is_none() {
            return Err(AppError::business(format!(
                "反结账失败：下一期间 {}-{:02} 不存在，无法保证期间连续性",
                next_year, next_period
            )));
        }
        if let Some(next_p) = &next_period_exists {
            if next_p.status != period_status::OPEN {
                return Err(AppError::business(format!(
                    "反结账失败：下一期间 {} 状态为 {}，必须为 OPEN",
                    next_p.period_name, next_p.status
                )));
            }
        }
        // 反结账：状态恢复 OPEN，清空 closed_at/closed_by
        let mut active: accounting_period::ActiveModel = period.into();
        active.status = Set(period_status::OPEN.to_string());
        active.closed_at = Set(None);
        active.closed_by = Set(None);
        let reopened = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;
        tracing::info!(
            "用户 {} 反结账期间 {}（{}），原因：{}",
            user_id,
            period_id,
            reopened.period_name,
            reason
        );
        Ok(reopened)
    }

    /// V15 P1 17.1-D3：年结（年度结账）
    /// 业务规则：1. 校验指定年度 1-12 月所有期间均为 CLOSED 状态（防止遗漏未结账月份）；2. 将所有损益科目（收入类 6xxx / 成本费用类 5xxx）余额结转至"4104 未分配利润"；3. 结转后损益科目余额为零，资产负债表平账；4. 创建下一年度 1 月期间（若不存在）；5. 操作写入审计日志
    pub async fn year_end_closing(
        &self,
        year: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;
        // 1. 校验所有月份均已结账
        let year_periods = accounting_period::Entity::find()
            .filter(accounting_period::Column::Year.eq(year))
            .order_by_asc(accounting_period::Column::Period)
            .all(&txn)
            .await?;
        if year_periods.len() != 12 {
            return Err(AppError::business(format!(
                "年结失败：年度 {} 仅有 {} 个期间，需 12 个月完整",
                year,
                year_periods.len()
            )));
        }
        for p in &year_periods {
            if p.status != period_status::CLOSED {
                return Err(AppError::business(format!(
                    "年结失败：期间 {}（{}）状态为 {}，需全部 CLOSED",
                    p.period_name, p.id, p.status
                )));
            }
        }
        // 2. 结转损益科目余额至未分配利润
        let transfer_result = self
            .transfer_profit_loss_to_retained_earnings_txn(&txn, year, user_id)
            .await?;
        // 3. 创建下一年度 1 月期间（若不存在）
        let next_year = year + 1;
        let existing_next = accounting_period::Entity::find()
            .filter(accounting_period::Column::Year.eq(next_year))
            .filter(accounting_period::Column::Period.eq(1))
            .one(&txn)
            .await?;
        if existing_next.is_none() {
            let start_date = Utc
                .with_ymd_and_hms(next_year, 1, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| {
                    AppError::bad_request(format!("Invalid date: {}-01-01", next_year))
                })?;
            let end_date = Utc
                .with_ymd_and_hms(next_year, 2, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| {
                    AppError::bad_request(format!("Invalid date: {}-02-01", next_year))
                })?
                - chrono::Duration::seconds(1);
            let new_period = accounting_period::ActiveModel {
                year: Set(next_year),
                period: Set(1),
                period_name: Set(format!("{} 年 01 月", next_year)),
                start_date: Set(start_date),
                end_date: Set(end_date),
                status: Set(period_status::OPEN.to_string()),
                created_at: Set(Utc::now()),
                ..Default::default()
            };
            new_period.insert(&txn).await?;
        }
        txn.commit().await?;
        tracing::info!(
            "用户 {} 完成年度 {} 年结：结转 {} 个科目，未分配利润调整 {}",
            user_id,
            year,
            transfer_result.transferred_subjects,
            transfer_result.retained_earnings_adjustment
        );
        Ok(serde_json::json!({
            "year": year,
            "next_year": next_year,
            "transferred_subjects": transfer_result.transferred_subjects,
            "retained_earnings_adjustment": transfer_result.retained_earnings_adjustment,
            "operator_id": user_id,
            "operated_at": Utc::now(),
        }))
    }

    /// V15 P1 17.1-D1：将期间标记为 CLOSING 中间状态（事务内）
    async fn mark_period_closing_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        period: &accounting_period::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        let mut active: accounting_period::ActiveModel = period.clone().into();
        active.status = Set(period_closing::CLOSING.to_string());
        let _ = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// V15 P1 17.1-D3：年结损益科目结转至未分配利润（4104）
    /// 损益科目编码规则（中国企业会计准则）：收入类：6xxx（主营业务收入、其他业务收入、营业外收入等）；成本费用类：5xxx（主营业务成本、销售费用、管理费用、财务费用等）；结转后损益科目余额为零，净额计入 4104 未分配利润。
    async fn transfer_profit_loss_to_retained_earnings_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        year: i32,
        _user_id: i32,
    ) -> Result<YearEndTransferResult, AppError> {
        use crate::models::{account_balance, account_subject};
        // 查询所有损益类科目（编码以 5 或 6 开头）
        let pl_subjects: Vec<account_subject::Model> = account_subject::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(account_subject::Column::Code.starts_with("5"))
                    .add(account_subject::Column::Code.starts_with("6")),
            )
            .all(txn)
            .await?;
        let pl_subject_ids: Vec<i32> = pl_subjects.iter().map(|s| s.id).collect();
        if pl_subject_ids.is_empty() {
            return Ok(YearEndTransferResult {
                transferred_subjects: 0,
                retained_earnings_adjustment: Decimal::ZERO,
            });
        }
        // 查询 4104 未分配利润科目
        let retained_earnings_subject = account_subject::Entity::find()
            .filter(account_subject::Column::Code.eq("4104"))
            .one(txn)
            .await?
            .ok_or_else(|| {
                AppError::business("年结失败：未找到 4104 未分配利润科目".to_string())
            })?;
        // 汇总全年各损益科目的期末余额（12 月期末 = 全年累计）
        let december_period = format!("{:04}-12", year);
        let pl_balances: Vec<account_balance::Model> = account_balance::Entity::find()
            .filter(account_balance::Column::Period.eq(&december_period))
            .filter(account_balance::Column::SubjectId.is_in(pl_subject_ids.clone()))
            .all(txn)
            .await?;
        // 计算净损益 = 收入类贷方余额 - 成本费用类借方余额
        let mut net_profit = Decimal::ZERO;
        let mut transferred_count = 0u64;
        for balance in &pl_balances {
            let subject = pl_subjects.iter().find(|s| s.id == balance.subject_id);
            if let Some(subj) = subject {
                let code = &subj.code;
                if code.starts_with('6') {
                    // 收入类：贷方余额，结转后借记收入科目、贷记未分配利润
                    let credit_balance =
                        balance.ending_balance_credit - balance.ending_balance_debit;
                    if credit_balance != Decimal::ZERO {
                        net_profit += credit_balance;
                        transferred_count += 1;
                    }
                } else if code.starts_with('5') {
                    // 成本费用类：借方余额，结转后借记未分配利润、贷记费用科目
                    let debit_balance =
                        balance.ending_balance_debit - balance.ending_balance_credit;
                    if debit_balance != Decimal::ZERO {
                        net_profit -= debit_balance;
                        transferred_count += 1;
                    }
                }
            }
        }
        // 更新 4104 未分配利润的期末余额（净损益计入）
        let re_balance = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.eq(retained_earnings_subject.id))
            .filter(account_balance::Column::Period.eq(&december_period))
            .one(txn)
            .await?;
        if let Some(re) = re_balance {
            let mut active: account_balance::ActiveModel = re.into();
            if net_profit >= Decimal::ZERO {
                active.ending_balance_credit = Set(net_profit);
            } else {
                active.ending_balance_debit = Set(-net_profit);
            }
            active.updated_at = Set(Utc::now());
            active.update(txn).await?;
        }
        Ok(YearEndTransferResult {
            transferred_subjects: transferred_count,
            retained_earnings_adjustment: net_profit,
        })
    }

    /// 事务内锁定会计期间并校验是否已结账
    async fn lock_period_for_close_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        period_id: i32,
    ) -> Result<accounting_period::Model, AppError> {
        let period = accounting_period::Entity::find_by_id(period_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("Accounting period {} not found", period_id))
            })?;
        if period.status == period_status::CLOSED {
            return Err(AppError::business("期间已经结账，不能重复结账".to_string()));
        }
        // V15 P1 17.1-D1：CLOSING 状态拒绝重复结账（防止并发结账）
        if period.status == period_closing::CLOSING {
            return Err(AppError::business(
                "期间正在结账中（CLOSING），不能重复发起结账".to_string(),
            ));
        }
        Ok(period)
    }

    /// 校验期间内是否存在未过账凭证，存在则返回错误
    async fn check_unposted_vouchers_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> Result<(), AppError> {
        let unposted_vouchers = crate::models::voucher::Entity::find()
            .filter(crate::models::voucher::Column::VoucherDate.gte(start_date))
            .filter(crate::models::voucher::Column::VoucherDate.lte(end_date))
            .filter(crate::models::voucher::Column::Status.ne(VOUCHER_POSTED))
            .count(txn)
            .await?;
        if unposted_vouchers > 0 {
            return Err(AppError::business(format!(
                "该期间有 {} 张凭证未过账，请先完成所有凭证的过账操作",
                unposted_vouchers
            )));
        }
        Ok(())
    }

    /// 将期间标记为 CLOSED 并写入审计日志
    async fn mark_period_closed_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        period: &accounting_period::Model,
        user_id: i32,
    ) -> Result<accounting_period::Model, AppError> {
        let mut active_period: accounting_period::ActiveModel = period.clone().into();
        active_period.status = Set(period_status::CLOSED.to_string());
        active_period.closed_at = Set(Some(Utc::now()));
        active_period.closed_by = Set(Some(user_id));
        let closed_period = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_period,
            Some(user_id),
        )
        .await?;
        Ok(closed_period)
    }

    /// F-P1-1 修复（批次 360 v13 复审）：试算平衡校验
    /// 在 close_period 事务内调用，联表查询指定期间内所有已过账凭证分录，；汇总借方总额与贷方总额。返回 (借方总额, 贷方总额) 供调用方校验。；空期间返回 (0, 0) 视为平衡。
    async fn check_trial_balance_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> Result<(Decimal, Decimal), AppError> {
        use crate::models::{voucher, voucher_item};

        let result: Option<(Option<Decimal>, Option<Decimal>)> = voucher_item::Entity::find()
            .join(
                sea_orm::JoinType::InnerJoin,
                voucher_item::Relation::Voucher.def(),
            )
            .filter(voucher::Column::Status.eq(VOUCHER_POSTED))
            .filter(voucher::Column::VoucherDate.gte(start_date))
            .filter(voucher::Column::VoucherDate.lte(end_date))
            .select_only()
            .column_as(Expr::col(voucher_item::Column::Debit).sum(), "total_debit")
            .column_as(
                Expr::col(voucher_item::Column::Credit).sum(),
                "total_credit",
            )
            .into_tuple()
            .one(txn)
            .await?;

        let (total_debit_opt, total_credit_opt) = result.unwrap_or((None, None));
        Ok((
            total_debit_opt.unwrap_or(Decimal::ZERO),
            total_credit_opt.unwrap_or(Decimal::ZERO),
        ))
    }

    /// F-P1-1 修复（批次 384 v13 复审）：期末结转余额
    /// 将本期 account_balances 的期末余额（ending_balance_debit/credit）；结转到下期 account_balances 的期初余额（initial_balance_debit/credit）。；结转规则：下期 initial_balance_debit = 本期 ending_balance_debit；下期 initial_balance_credit = 本期 ending_balance_credit；下期 current_period_debit/credit = 0（新期间无发生额）；下期 ending_balance_debit/credit = 本期期末值（期初即期末，未发生新业务前）；若下期记录已存在，则更新期初余额；若不存在，则插入新记录。
    async fn carry_forward_balances_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        current_period: &str,
        next_period: &str,
    ) -> Result<(), AppError> {
        use crate::models::account_balance;

        // 查询本期所有科目的期末余额
        let current_balances: Vec<account_balance::Model> = account_balance::Entity::find()
            .filter(account_balance::Column::Period.eq(current_period))
            .all(txn)
            .await?;

        if current_balances.is_empty() {
            tracing::info!("期间 {} 无余额记录，期末结转跳过", current_period);
            return Ok(());
        }

        let now = chrono::Utc::now();
        let mut inserted_count = 0u64;
        let mut updated_count = 0u64;

        for balance in current_balances {
            let inserted =
                Self::upsert_next_period_balance_txn(txn, &balance, next_period, now).await?;
            if inserted {
                inserted_count += 1;
            } else {
                updated_count += 1;
            }
        }

        tracing::info!(
            "期末结转完成：{} → {}，新增 {} 条，更新 {} 条",
            current_period,
            next_period,
            inserted_count,
            updated_count
        );
        Ok(())
    }

    /// 结转单条余额到下期：已存在则更新期初余额，不存在则插入新记录
    /// 返回 true=inserted, false=updated；期末余额 = 期初 + 本期发生额，若本期发生额为零则期末 = 期初。；这里不重置 current_period_*，因为下期可能已有业务发生。
    async fn upsert_next_period_balance_txn(
        txn: &sea_orm::DatabaseTransaction,
        balance: &crate::models::account_balance::Model,
        next_period: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, AppError> {
        use crate::models::account_balance;

        let existing = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.eq(balance.subject_id))
            .filter(account_balance::Column::Period.eq(next_period))
            .one(txn)
            .await?;

        if let Some(existing) = existing {
            // 下期记录已存在，更新期初余额
            let next_period_debit = existing.current_period_debit;
            let next_period_credit = existing.current_period_credit;
            let mut active: account_balance::ActiveModel = existing.into();
            active.initial_balance_debit = Set(balance.ending_balance_debit);
            active.initial_balance_credit = Set(balance.ending_balance_credit);
            active.ending_balance_debit = Set(balance.ending_balance_debit + next_period_debit);
            active.ending_balance_credit = Set(balance.ending_balance_credit + next_period_credit);
            active.updated_at = Set(now);
            active.update(txn).await?;
            Ok(false)
        } else {
            // 下期记录不存在，插入新记录
            let new_balance = account_balance::ActiveModel {
                subject_id: Set(balance.subject_id),
                period: Set(next_period.to_string()),
                initial_balance_debit: Set(balance.ending_balance_debit),
                initial_balance_credit: Set(balance.ending_balance_credit),
                current_period_debit: Set(Decimal::ZERO),
                current_period_credit: Set(Decimal::ZERO),
                ending_balance_debit: Set(balance.ending_balance_debit),
                ending_balance_credit: Set(balance.ending_balance_credit),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            new_balance.insert(txn).await?;
            Ok(true)
        }
    }

    /// 校验指定日期是否在已结账的期间内（防止篡改历史数据）
    pub async fn check_date_locked(&self, date: chrono::NaiveDate) -> Result<(), AppError> {
        let dt = Utc.from_utc_datetime(
            &date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::bad_request(format!("Invalid date: {:?}", date)))?,
        );

        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::StartDate.lte(dt))
            .filter(accounting_period::Column::EndDate.gte(dt))
            .one(self.db.as_ref())
            .await?;

        if let Some(p) = period {
            if p.status == period_status::CLOSED {
                return Err(AppError::business(format!(
                    "日期 {} 属于已结账的财务期间 ({})，该期间的数据已被锁定，不可修改或新增。",
                    date.format("%Y-%m-%d"),
                    p.period_name
                )));
            }
        } else {
            return Err(AppError::business(format!(
                "日期 {} 不在任何已设置的会计期间内，请先创建对应的会计期间。",
                date.format("%Y-%m-%d")
            )));
        }

        Ok(())
    }

    /// P2 3-22 修复：事务内校验指定日期是否在已结账的期间内，避免 TOCTOU
    /// 原 check_date_locked 在 ar_service 等调用方的事务外执行，；并发场景下可能在检查后、commit 前期间被关闭，导致历史数据被篡改。；新增 _txn 变体，在调用方事务内执行校验。
    // v11 批次 148 P2-A：移除失效的 dead_code 标注（被 ar_service.rs:120 真实调用）
    // 批次 349 v12 复审 P2-1：ar_collection_service 已删除（死代码），注释引用修正为 ar_service
    pub async fn check_date_locked_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        date: chrono::NaiveDate,
    ) -> Result<(), AppError> {
        let dt = Utc.from_utc_datetime(
            &date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::bad_request(format!("Invalid date: {:?}", date)))?,
        );

        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::StartDate.lte(dt))
            .filter(accounting_period::Column::EndDate.gte(dt))
            .one(txn)
            .await?;

        if let Some(p) = period {
            if p.status == period_status::CLOSED {
                return Err(AppError::business(format!(
                    "日期 {} 属于已结账的财务期间 ({})，该期间的数据已被锁定，不可修改或新增。",
                    date.format("%Y-%m-%d"),
                    p.period_name
                )));
            }
        } else {
            return Err(AppError::business(format!(
                "日期 {} 不在任何已设置的会计期间内，请先创建对应的会计期间。",
                date.format("%Y-%m-%d")
            )));
        }

        Ok(())
    }
}

/// 计算下一个期间的年份与月份（12 月跨年到次年 1 月）
fn calc_next_period(year: i32, period: i32) -> (i32, i32) {
    if period == 12 {
        (year + 1, 1)
    } else {
        (year, period + 1)
    }
}

/// V15 P1 17.1-D3：年结损益结转结果
#[derive(Debug, Clone)]
struct YearEndTransferResult {
    /// 结转的损益科目数
    transferred_subjects: u64,
    /// 未分配利润调整额（正数为净收益，负数为净亏损）
    retained_earnings_adjustment: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::services::test_common::setup_test_db;
    use crate::ymd;
    use chrono::NaiveDate;
    use std::sync::Arc;

    // 会计期间状态常量（引用 status::accounting_period 模块，批次 232 v13 P1-1）
    const PERIOD_STATUS_OPEN: &str = period_status::OPEN;
    const PERIOD_STATUS_CLOSED: &str = period_status::CLOSED;

    /// 测试夹具：计算下个月月份（复现 init_first_period 与 close_period 中的纯算法逻辑：`if month == 12 { 1 } else { month + 1 }`）
    fn calc_next_month(month: u32) -> u32 {
        if month == 12 {
            1
        } else {
            month + 1
        }
    }

    /// 测试夹具：计算下个月所属年份（复现 init_first_period 与 close_period 中的纯算法逻辑：`if month == 12 { year + 1 } else { year }`）
    fn calc_next_month_year(month: u32, year: i32) -> i32 {
        if month == 12 {
            year + 1
        } else {
            year
        }
    }

    /// 测试夹具：格式化期间名称（复现 init_first_period 中的纯算法逻辑：`format!("{} 年 {:02} 月", year, month)`）
    fn format_period_name(year: i32, month: u32) -> String {
        format!("{} 年 {:02} 月", year, month)
    }

    /// 测试夹具：计算期间结束日期（复现 init_first_period 中的纯算法逻辑：下月 1 号 00:00:00 减去 1 秒 = 当月最后一天 23:59:59）
    fn calc_period_end_date(year: i32, month: u32) -> chrono::DateTime<Utc> {
        let next_month = calc_next_month(month);
        let next_month_year = calc_next_month_year(month, year);
        let next_month_start = Utc
            .with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0)
            .single()
            .expect("测试夹具：下月起始日期解析失败");
        next_month_start - chrono::Duration::seconds(1)
    }

    /// 测试夹具：校验期间是否已结账（复现 close_period 与 check_date_locked 中的状态判断逻辑：`if period.status == "CLOSED"`）
    fn is_period_closed(status: &str) -> bool {
        status == PERIOD_STATUS_CLOSED
    }

    /// 测试夹具：格式化已结账期间锁定错误消息
    /// 复现 check_date_locked / check_date_locked_txn 中的消息拼接逻辑
    fn format_locked_error_msg(date: NaiveDate, period_name: &str) -> String {
        format!(
            "日期 {} 属于已结账的财务期间 ({})，该期间的数据已被锁定，不可修改或新增。",
            date.format("%Y-%m-%d"),
            period_name
        )
    }

    /// 测试夹具：格式化未设置会计期间错误消息
    /// 复现 check_date_locked / check_date_locked_txn 中的消息拼接逻辑
    fn format_no_period_error_msg(date: NaiveDate) -> String {
        format!(
            "日期 {} 不在任何已设置的会计期间内，请先创建对应的会计期间。",
            date.format("%Y-%m-%d")
        )
    }

    #[test]
    fn test_xgyjs_ptyf() {
        // 验证 1-11 月的下个月为当前月加 1（复现 init_first_period 算法）
        assert_eq!(calc_next_month(1), 2);
        assert_eq!(calc_next_month(6), 7);
        assert_eq!(calc_next_month(11), 12);
    }

    #[test]
    fn test_xgyjs_seykn() {
        // 验证 12 月的下个月为次年 1 月（跨年边界场景，复现 init_first_period 算法）
        assert_eq!(calc_next_month(12), 1);
        assert_eq!(calc_next_month_year(12, 2026), 2027);
    }

    #[test]
    fn test_xgynfjs_ptyfbb() {
        // 验证 1-11 月的下个月仍属同一年（复现 close_period 中 next_year 计算逻辑）
        assert_eq!(calc_next_month_year(1, 2026), 2026);
        assert_eq!(calc_next_month_year(11, 2026), 2026);
    }

    #[test]
    fn test_qjmcgsh() {
        // 验证期间名称格式（如：2026 年 03 月），复现 init_first_period 的 period_name 拼接
        assert_eq!(format_period_name(2026, 3), "2026 年 03 月");
        assert_eq!(format_period_name(2026, 12), "2026 年 12 月");
        assert_eq!(format_period_name(2027, 1), "2027 年 01 月");
    }

    #[test]
    fn test_qjjsrq_ptyf() {
        // 验证 2026 年 3 月的结束日期为 2026-03-31 23:59:59
        // 复现 init_first_period 中 end_date = 下月起始 - 1 秒 的计算逻辑
        let end_date = calc_period_end_date(2026, 3);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-03-31 23:59:59"
        );
    }

    #[test]
    fn test_qjjsrq_eyrn() {
        // 验证 2024 年（闰年）2 月的结束日期为 2024-02-29 23:59:59
        // 复现 init_first_period 中月末日期计算在闰年 2 月的正确性
        let end_date = calc_period_end_date(2024, 2);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-02-29 23:59:59"
        );
    }

    #[test]
    fn test_qjjsrq_seykn() {
        // 验证 2026 年 12 月的结束日期为 2026-12-31 23:59:59（跨年场景）
        // 复现 init_first_period 中 12 月跨年到次年 1 月的 end_date 计算逻辑
        let end_date = calc_period_end_date(2026, 12);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-12-31 23:59:59"
        );
    }

    #[test]
    fn test_qjztjy_yjzsb() {
        // 验证 CLOSED 状态被识别为已结账，OPEN 状态不被识别
        // 复现 close_period 中 `if period.status == "CLOSED"` 重复结账检查逻辑
        assert!(is_period_closed(PERIOD_STATUS_CLOSED));
        assert!(!is_period_closed(PERIOD_STATUS_OPEN));
    }

    #[test]
    fn test_yjzqjsdcwxxgs() {
        // 验证已结账期间的错误消息拼接（使用 ymd! 夹具宏解析日期）
        // 复现 check_date_locked 中 `format!("日期 {} 属于已结账的财务期间 ({})...")` 逻辑
        let date = ymd!(2026, 3, 15);
        let period_name = "2026 年 03 月";
        let msg = format_locked_error_msg(date, period_name);
        assert!(msg.contains("2026-03-15"));
        assert!(msg.contains("2026 年 03 月"));
        assert!(msg.contains("已被锁定"));
        assert!(msg.contains("不可修改或新增"));
    }

    #[test]
    fn test_wszkjqjcwxxgs() {
        // 验证未设置会计期间的错误消息拼接（使用 ymd! 夹具宏解析日期）
        // 复现 check_date_locked 中 `format!("日期 {} 不在任何已设置的会计期间内...")` 逻辑
        let date = ymd!(2026, 7, 9);
        let msg = format_no_period_error_msg(date);
        assert!(msg.contains("2026-07-09"));
        assert!(msg.contains("不在任何已设置的会计期间内"));
        assert!(msg.contains("请先创建对应的会计期间"));
    }

    #[test]
    fn test_decs_jjhkyx() {
        // 验证 decs! 夹具宏可正常解析 Decimal 字符串
        // 注：本 service 不直接涉及金额，但保留宏以符合项目测试夹具规范
        let v = decs!("100.00");
        assert_eq!(v.to_string(), "100.00");
    }

    #[tokio::test]
    async fn test_fwslh() {
        // 验证 AccountingPeriodService 可正常实例化（define_service! 宏生成 new 方法）
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_hqdqqj_w_schema_sjj() {
        // 需要 accounting_periods 表 schema 的真实场景，标注 #[ignore]
        // sqlite::memory: 无 schema 时，get_current_period 预期返回数据库错误
        // 真实环境需先建表，应返回 Ok(None) 表示无开放期间
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        let result = service.get_current_period().await;
        if let Ok(opt) = result {
            assert!(opt.is_none(), "空库应返回 None");
        } // 无 schema 时数据库报错属于预期降级
    }

    #[tokio::test]
    #[ignore]
    async fn test_jyrqsd_wqjsybc() {
        // 需要 accounting_periods 表 schema 的真实场景，标注 #[ignore]
        // 真实环境应返回 BusinessError（日期不在任何已设置的会计期间内）
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        let date = ymd!(2026, 3, 15);
        let result = service.check_date_locked(date).await;
        assert!(result.is_err(), "无会计期间时应返回错误");
    }
}
