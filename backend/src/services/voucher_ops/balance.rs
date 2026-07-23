//! 凭证服务-科目余额子模块（voucher_ops/balance）
//!
//! 批次 488 D10-2a 拆分：从原 `voucher_service.rs` L88-97 + L720-1045 迁移。
//! 包含 BalanceUpdateContext 内部 struct + 12 个余额更新方法：
//! - update_account_balances（pub(crate)，被 workflow::post 调用）
//! - fetch_voucher_and_items / compute_period_from_date / fetch_subjects_for_items
//! - aggregate_balance_by_subject / fetch_existing_balances / apply_balance_updates
//! - dispatch_balance_updates / update_existing_balance / compute_updated_balance_amounts
//! - create_new_balance / compute_ending_balance
//!
//! 业务规则：
//! - 期末余额按会计制度计算（借方科目：期初借+本期借-本期贷；贷方科目反之）
//! - 余额为正记同向，为负记反向
//! - v11 批次 37 修复：批量查询科目避免 N+1
//! - v16 批次 45 修复：批量 lock 查询现有余额避免 N+1

use chrono::Datelike;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect};
use tracing::info;

use crate::models::{account_subject, voucher, voucher_item};
use crate::utils::error::AppError;
use rust_decimal::Decimal;

use crate::services::voucher_service::VoucherService;

/// 余额更新上下文：封装科目列表、聚合发生额、锁定的现有余额记录
pub(super) struct BalanceUpdateContext {
    subjects: Vec<account_subject::Model>,
    balance_map: std::collections::HashMap<i32, (Decimal, Decimal)>,
    existing_balances: Vec<crate::models::account_balance::Model>,
}

impl VoucherService {
    /// 更新科目余额：按会计制度计算期末余额（借方=期初借+本期借-本期贷，贷方反之）
    pub(crate) async fn update_account_balances(
        &self,
        voucher_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        info!("更新科目余额 voucher_id={}", voucher_id);

        let (voucher, items) = Self::fetch_voucher_and_items(voucher_id, txn).await?;
        let period = Self::compute_period_from_date(voucher.voucher_date);
        let subjects = Self::fetch_subjects_for_items(&items, txn).await?;
        let balance_map = Self::aggregate_balance_by_subject(&items, &subjects)?;
        let subject_ids: Vec<i32> = balance_map.keys().copied().collect();
        let existing_balances =
            Self::fetch_existing_balances(&subject_ids, &period, txn).await?;
        let ctx = BalanceUpdateContext {
            subjects,
            balance_map,
            existing_balances,
        };
        Self::apply_balance_updates(ctx, &period, user_id, txn).await?;

        info!("科目余额更新成功");
        Ok(())
    }

    /// 获取凭证及其分录
    async fn fetch_voucher_and_items(
        voucher_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(voucher::Model, Vec<voucher_item::Model>), AppError> {
        // 获取凭证信息
        let voucher = voucher::Entity::find_by_id(voucher_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("凭证不存在"))?;

        // 获取凭证分录
        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(voucher_id))
            .all(txn)
            .await?;

        Ok((voucher, items))
    }

    /// 从凭证日期生成会计期间字符串 (YYYY-MM)
    fn compute_period_from_date(voucher_date: chrono::NaiveDate) -> String {
        format!("{:04}-{:02}", voucher_date.year(), voucher_date.month())
    }

    /// 批量查询分录涉及的所有科目（v11 批次 37 修复：避免循环内逐个查询 N+1）
    async fn fetch_subjects_for_items(
        items: &[voucher_item::Model],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<account_subject::Model>, AppError> {
        use crate::models::account_subject;

        let subject_codes: Vec<String> =
            items.iter().map(|item| item.subject_code.clone()).collect();
        let subjects = if subject_codes.is_empty() {
            Vec::new()
        } else {
            account_subject::Entity::find()
                .filter(account_subject::Column::Code.is_in(subject_codes))
                .all(txn)
                .await?
        };
        Ok(subjects)
    }

    /// 按科目聚合借贷发生额（v11 批次 37 修复：构建 code→Model 引用映射复用批量结果）
    fn aggregate_balance_by_subject(
        items: &[voucher_item::Model],
        subjects: &[account_subject::Model],
    ) -> Result<std::collections::HashMap<i32, (Decimal, Decimal)>, AppError> {
        use std::collections::HashMap;

        let subject_by_code: HashMap<&str, &account_subject::Model> =
            subjects.iter().map(|s| (s.code.as_str(), s)).collect();

        let mut balance_map: HashMap<i32, (Decimal, Decimal)> = HashMap::new();
        for item in items {
            // 查找科目 ID 和余额方向（从批量查询结果中取）
            let subject_code = &item.subject_code;
            let subject = subject_by_code
                .get(subject_code.as_str())
                // 批次 102 v6 P3-4 修复：科目不存在属于资源未找到，应为 not_found 而非 bad_request
                .ok_or_else(|| AppError::not_found(format!("科目不存在：{}", subject_code)))?;

            let entry = balance_map
                .entry(subject.id)
                .or_insert((Decimal::ZERO, Decimal::ZERO));

            // 累加借方和贷方发生额
            if !item.debit.is_zero() {
                entry.0 += item.debit;
            }
            if !item.credit.is_zero() {
                entry.1 += item.credit;
            }
        }
        Ok(balance_map)
    }

    /// 批量锁定查询现有余额记录（v16 批次 45 修复：避免循环内逐个 lock 查询 N+1）
    async fn fetch_existing_balances(
        subject_ids: &[i32],
        period: &str,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<crate::models::account_balance::Model>, AppError> {
        use crate::models::account_balance;

        if subject_ids.is_empty() {
            return Ok(Vec::new());
        }
        let balances = account_balance::Entity::find()
            .filter(account_balance::Column::SubjectId.is_in(subject_ids.to_vec()))
            .filter(account_balance::Column::Period.eq(period))
            .lock(sea_orm::sea_query::LockType::Update)
            .all(txn)
            .await?;
        Ok(balances)
    }

    /// 应用余额更新：构建查找映射并分发到更新或新建余额记录
    async fn apply_balance_updates(
        ctx: BalanceUpdateContext,
        period: &str,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::models::account_balance;
        use std::collections::HashMap;

        let BalanceUpdateContext {
            subjects,
            balance_map,
            existing_balances,
        } = ctx;
        // v11 批次 37 修复：构建 id→Model 引用映射，供更新循环复用
        let subject_by_id: HashMap<i32, &account_subject::Model> =
            subjects.iter().map(|s| (s.id, s)).collect();
        let mut balance_record_map: HashMap<i32, account_balance::Model> = existing_balances
            .into_iter()
            .map(|b| (b.subject_id, b))
            .collect();

        Self::dispatch_balance_updates(
            balance_map,
            &subject_by_id,
            &mut balance_record_map,
            period,
            user_id,
            txn,
        )
        .await
    }

    /// 遍历聚合发生额，按科目分发到更新现有余额或创建新余额记录
    async fn dispatch_balance_updates(
        balance_map: std::collections::HashMap<i32, (Decimal, Decimal)>,
        subject_by_id: &std::collections::HashMap<i32, &account_subject::Model>,
        balance_record_map: &mut std::collections::HashMap<i32, crate::models::account_balance::Model>,
        period: &str,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        for (subject_id, (debit_amount, credit_amount)) in balance_map {
            // 复用批量查询结果获取科目信息以确定余额方向，避免 N+1
            let subject = subject_by_id
                .get(&subject_id)
                // 批次 102 v6 P3-4 修复：科目不存在属于资源未找到，应为 not_found
                .ok_or_else(|| AppError::not_found(format!("科目不存在：{}", subject_id)))?;
            let balance_direction = subject.balance_direction.as_deref().unwrap_or("借");

            // v16 批次 45 修复：从批量查询结果获取余额记录（带行锁）
            if let Some(balance) = balance_record_map.remove(&subject_id) {
                Self::update_existing_balance(
                    balance, debit_amount, credit_amount, balance_direction, user_id, txn,
                )
                .await?;
            } else {
                Self::create_new_balance(
                    subject_id, period, debit_amount, credit_amount, balance_direction, txn,
                )
                .await?;
            }
        }
        Ok(())
    }

    /// 更新现有余额记录：累加本期发生额并重算期末余额
    async fn update_existing_balance(
        balance: crate::models::account_balance::Model,
        debit_amount: Decimal,
        credit_amount: Decimal,
        balance_direction: &str,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::models::account_balance;

        // 更新现有余额
        let mut active_model: account_balance::ActiveModel = balance.into_active_model();
        // D12 重构：期末余额计算提取到 compute_ending_balance，消除嵌套 4 分支
        let (ending_dr, ending_cr) = Self::compute_updated_balance_amounts(
            &mut active_model,
            debit_amount,
            credit_amount,
            balance_direction,
        );
        active_model.ending_balance_debit = sea_orm::Set(ending_dr);
        active_model.ending_balance_credit = sea_orm::Set(ending_cr);

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_model,
            // 批次 94 P2-10：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 从现有余额 ActiveModel 提取字段、累加本期发生额并返回期末余额（借,贷）
    fn compute_updated_balance_amounts(
        active_model: &mut crate::models::account_balance::ActiveModel,
        debit_amount: Decimal,
        credit_amount: Decimal,
        balance_direction: &str,
    ) -> (Decimal, Decimal) {
        let current_debit = active_model
            .current_period_debit
            .take()
            .unwrap_or(Decimal::ZERO);
        let current_credit = active_model
            .current_period_credit
            .take()
            .unwrap_or(Decimal::ZERO);
        // 获取期初余额
        let initial_debit = active_model
            .initial_balance_debit
            .take()
            .unwrap_or(Decimal::ZERO);
        let initial_credit = active_model
            .initial_balance_credit
            .take()
            .unwrap_or(Decimal::ZERO);
        // 计算新的本期发生额（累加）
        let new_period_debit = current_debit + debit_amount;
        let new_period_credit = current_credit + credit_amount;
        // 更新本期发生额
        active_model.current_period_debit = sea_orm::Set(new_period_debit);
        active_model.current_period_credit = sea_orm::Set(new_period_credit);
        Self::compute_ending_balance(
            balance_direction,
            initial_debit,
            initial_credit,
            new_period_debit,
            new_period_credit,
        )
    }

    /// 创建新余额记录：新账期首次出现该科目的余额记录
    async fn create_new_balance(
        subject_id: i32,
        period: &str,
        debit_amount: Decimal,
        credit_amount: Decimal,
        balance_direction: &str,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::models::account_balance;

        // 创建新余额记录
        // D12 重构：新账期末余额计算复用 compute_ending_balance（initial_dr=0, initial_cr=0）
        let (ending_debit, ending_credit) = Self::compute_ending_balance(
            balance_direction,
            Decimal::ZERO,
            Decimal::ZERO,
            debit_amount,
            credit_amount,
        );

        let active_model = account_balance::ActiveModel {
            subject_id: sea_orm::Set(subject_id),
            period: sea_orm::Set(period.to_string()),
            current_period_debit: sea_orm::Set(debit_amount),
            current_period_credit: sea_orm::Set(credit_amount),
            initial_balance_debit: sea_orm::Set(Decimal::ZERO),
            initial_balance_credit: sea_orm::Set(Decimal::ZERO),
            ending_balance_debit: sea_orm::Set(ending_debit),
            ending_balance_credit: sea_orm::Set(ending_credit),
            ..Default::default()
        };
        active_model.insert(txn).await?;
        Ok(())
    }

    /// 计算期末余额（借/贷双方向，余额为正记同向，为负记反向）
    /// 借方：期初借+本期借-本期贷；贷方：期初贷+本期贷-本期借
    fn compute_ending_balance(
        balance_direction: &str,
        initial_dr: Decimal,
        initial_cr: Decimal,
        period_dr: Decimal,
        period_cr: Decimal,
    ) -> (Decimal, Decimal) {
        if balance_direction == "借" {
            let ending = initial_dr + period_dr - period_cr;
            if ending >= Decimal::ZERO {
                (ending, Decimal::ZERO)
            } else {
                (Decimal::ZERO, ending.abs())
            }
        } else {
            let ending = initial_cr + period_cr - period_dr;
            if ending >= Decimal::ZERO {
                (Decimal::ZERO, ending)
            } else {
                (ending.abs(), Decimal::ZERO)
            }
        }
    }
}
