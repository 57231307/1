use crate::models::accounting_period;
use crate::utils::error::AppError;
use chrono::{TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

crate::define_service!(AccountingPeriodService);

impl AccountingPeriodService {
    /// 获取当前开放的会计期间
    pub async fn get_current_period(&self) -> Result<Option<accounting_period::Model>, AppError> {
        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::Status.eq("OPEN"))
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
            status: Set("OPEN".to_string()),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let period = active_model.insert(self.db.as_ref()).await?;
        Ok(period)
    }

    /// 执行月末结账
    pub async fn close_period(
        &self,
        period_id: i32,
        user_id: i32,
    ) -> Result<accounting_period::Model, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;

        let period = accounting_period::Entity::find_by_id(period_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("Accounting period {} not found", period_id))
            })?;

        if period.status == "CLOSED" {
            return Err(AppError::business("期间已经结账，不能重复结账".to_string()));
        }

        // 检查该期间内是否有未过账的凭证
        let start_date = period.start_date;
        let end_date = period.end_date;

        let unposted_vouchers = crate::models::voucher::Entity::find()
            .filter(crate::models::voucher::Column::VoucherDate.gte(start_date))
            .filter(crate::models::voucher::Column::VoucherDate.lte(end_date))
            .filter(crate::models::voucher::Column::Status.ne("posted"))
            .count(&txn)
            .await?;

        if unposted_vouchers > 0 {
            return Err(AppError::business(format!(
                "该期间有 {} 张凭证未过账，请先完成所有凭证的过账操作",
                unposted_vouchers
            )));
        }

        // 1. 将当前期间设置为 CLOSED（事务内更新并写入审计日志）
        let mut active_period: accounting_period::ActiveModel = period.clone().into();
        active_period.status = Set("CLOSED".to_string());
        active_period.closed_at = Set(Some(Utc::now()));
        active_period.closed_by = Set(Some(user_id));
        let closed_period = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_period,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // 2. 创建下一个期间并设置为 OPEN
        let next_month = if period.period == 12 {
            1
        } else {
            period.period + 1
        };
        let next_year = if period.period == 12 {
            period.year + 1
        } else {
            period.year
        };
        self.init_first_period(next_year, next_month as u32).await?;

        Ok(closed_period)
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
            if p.status == "CLOSED" {
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
    ///
    /// 原 check_date_locked 在 ar_collection_service 等调用方的事务外执行，
    /// 并发场景下可能在检查后、commit 前期间被关闭，导致历史数据被篡改。
    /// 新增 _txn 变体，在调用方事务内执行校验。
    #[allow(dead_code)] // TODO(tech-debt): ArCollectionService::create_collection 接入 handler 后移除
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
            if p.status == "CLOSED" {
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
