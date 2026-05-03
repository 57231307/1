use crate::models::accounting_period;
use crate::utils::error::AppError;
use chrono::{Datelike, NaiveDate, Utc, TimeZone};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, QueryOrder,
};
use std::sync::Arc;

pub struct AccountingPeriodService {
    db: Arc<DatabaseConnection>,
}

impl AccountingPeriodService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取当前开放的会计期间
    pub async fn get_current_period(&self) -> Result<Option<accounting_period::Model>, AppError> {
        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::Status.eq("OPEN"))
            .filter(accounting_period::Column::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await?;
        Ok(period)
    }

    /// 初始化第一个会计期间（如果不存在）
    pub async fn init_first_period(&self, year: i32, month: u32) -> Result<accounting_period::Model, AppError> {
        let existing = self.get_current_period().await?;
        if let Some(p) = existing {
            return Ok(p);
        }

        let start_date = Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap();
        let next_month = if month == 12 { 1 } else { month + 1 };
        let next_month_year = if month == 12 { year + 1 } else { year };
        let end_date = Utc.with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0).unwrap() - chrono::Duration::seconds(1);

        let active_model = accounting_period::ActiveModel {
            year: Set(year),
            period: Set(month as i32),
            period_name: Set(format!("{} 年 {:02} 月", year, month)),
            start_date: Set(start_date),
            end_date: Set(end_date),
            status: Set("OPEN".to_string()),
            is_deleted: Set(false),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        let period = active_model.insert(self.db.as_ref()).await?;
        Ok(period)
    }

    /// 执行月末结账
    pub async fn close_period(&self, period_id: i32, user_id: i32) -> Result<accounting_period::Model, AppError> {
        let period = accounting_period::Entity::find_by_id(period_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Accounting period {} not found", period_id)))?;

        if period.status == "CLOSED" {
            return Err(AppError::BusinessError("期间已经结账，不能重复结账".to_string()));
        }

        // 1. 将当前期间设置为 CLOSED
        let mut active_period: accounting_period::ActiveModel = period.clone().into();
        active_period.status = Set("CLOSED".to_string());
        active_period.closed_at = Set(Some(Utc::now()));
        active_period.closed_by = Set(Some(user_id));
        let closed_period = active_period.update(self.db.as_ref()).await?;

        // 2. 创建下一个期间并设置为 OPEN
        let next_month = if period.period == 12 { 1 } else { period.period + 1 };
        let next_year = if period.period == 12 { period.year + 1 } else { period.year };
        self.init_first_period(next_year, next_month as u32).await?;

        Ok(closed_period)
    }

    /// 校验指定日期是否在已结账的期间内（防止篡改历史数据）
    pub async fn check_date_locked(&self, date: chrono::NaiveDate) -> Result<(), AppError> {
        let dt = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        
        let period = accounting_period::Entity::find()
            .filter(accounting_period::Column::StartDate.lte(dt))
            .filter(accounting_period::Column::EndDate.gte(dt))
            .filter(accounting_period::Column::IsDeleted.eq(false))
            .one(self.db.as_ref())
            .await?;

        if let Some(p) = period {
            if p.status == "CLOSED" {
                return Err(AppError::BusinessError(format!(
                    "日期 {} 属于已结账的财务期间 ({})，该期间的数据已被锁定，不可修改或新增。",
                    date.format("%Y-%m-%d"), p.period_name
                )));
            }
        }
        
        Ok(())
    }
}
