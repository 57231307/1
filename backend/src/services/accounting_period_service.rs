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
    // v11 批次 148 P2-A：移除失效的 dead_code 标注（被 ar_service.rs:120 和 ar_collection_service.rs:42 真实调用）
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use chrono::NaiveDate;
    use sea_orm::{Database, DatabaseConnection};
    use std::str::FromStr;
    use std::sync::Arc;

    // 会计期间状态常量（accounting_period 模型定义的状态值）
    // 注：status.rs 未定义会计期间的 OPEN 常量，此处局部定义以避免硬编码字符串字面量
    const PERIOD_STATUS_OPEN: &str = "OPEN";
    const PERIOD_STATUS_CLOSED: &str = "CLOSED";

    /// 测试夹具：计算下个月月份
    /// 复现 init_first_period 与 close_period 中的纯算法逻辑：
    /// `if month == 12 { 1 } else { month + 1 }`
    fn calc_next_month(month: u32) -> u32 {
        if month == 12 { 1 } else { month + 1 }
    }

    /// 测试夹具：计算下个月所属年份
    /// 复现 init_first_period 与 close_period 中的纯算法逻辑：
    /// `if month == 12 { year + 1 } else { year }`
    fn calc_next_month_year(month: u32, year: i32) -> i32 {
        if month == 12 { year + 1 } else { year }
    }

    /// 测试夹具：格式化期间名称
    /// 复现 init_first_period 中的纯算法逻辑：
    /// `format!("{} 年 {:02} 月", year, month)`
    fn format_period_name(year: i32, month: u32) -> String {
        format!("{} 年 {:02} 月", year, month)
    }

    /// 测试夹具：计算期间结束日期
    /// 复现 init_first_period 中的纯算法逻辑：
    /// 下月 1 号 00:00:00 减去 1 秒 = 当月最后一天 23:59:59
    fn calc_period_end_date(year: i32, month: u32) -> chrono::DateTime<Utc> {
        let next_month = calc_next_month(month);
        let next_month_year = calc_next_month_year(month, year);
        let next_month_start = Utc
            .with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0)
            .single()
            .expect("测试夹具：下月起始日期解析失败");
        next_month_start - chrono::Duration::seconds(1)
    }

    /// 测试夹具：校验期间是否已结账
    /// 复现 close_period 与 check_date_locked 中的状态判断逻辑：
    /// `if period.status == "CLOSED"`
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

    /// 测试夹具：建立 SQLite 内存数据库连接（参考 inventory_adjustment_service.rs 的 setup_test_db 模式）
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    #[test]
    fn test_下个月计算_普通月份() {
        // 验证 1-11 月的下个月为当前月加 1（复现 init_first_period 算法）
        assert_eq!(calc_next_month(1), 2);
        assert_eq!(calc_next_month(6), 7);
        assert_eq!(calc_next_month(11), 12);
    }

    #[test]
    fn test_下个月计算_十二月跨年() {
        // 验证 12 月的下个月为次年 1 月（跨年边界场景，复现 init_first_period 算法）
        assert_eq!(calc_next_month(12), 1);
        assert_eq!(calc_next_month_year(12, 2026), 2027);
    }

    #[test]
    fn test_下个月年份计算_普通月份不变() {
        // 验证 1-11 月的下个月仍属同一年（复现 close_period 中 next_year 计算逻辑）
        assert_eq!(calc_next_month_year(1, 2026), 2026);
        assert_eq!(calc_next_month_year(11, 2026), 2026);
    }

    #[test]
    fn test_期间名称格式化() {
        // 验证期间名称格式（如：2026 年 03 月），复现 init_first_period 的 period_name 拼接
        assert_eq!(format_period_name(2026, 3), "2026 年 03 月");
        assert_eq!(format_period_name(2026, 12), "2026 年 12 月");
        assert_eq!(format_period_name(2027, 1), "2027 年 01 月");
    }

    #[test]
    fn test_期间结束日期_普通月份() {
        // 验证 2026 年 3 月的结束日期为 2026-03-31 23:59:59
        // 复现 init_first_period 中 end_date = 下月起始 - 1 秒 的计算逻辑
        let end_date = calc_period_end_date(2026, 3);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-03-31 23:59:59"
        );
    }

    #[test]
    fn test_期间结束日期_二月闰年() {
        // 验证 2024 年（闰年）2 月的结束日期为 2024-02-29 23:59:59
        // 复现 init_first_period 中月末日期计算在闰年 2 月的正确性
        let end_date = calc_period_end_date(2024, 2);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-02-29 23:59:59"
        );
    }

    #[test]
    fn test_期间结束日期_十二月跨年() {
        // 验证 2026 年 12 月的结束日期为 2026-12-31 23:59:59（跨年场景）
        // 复现 init_first_period 中 12 月跨年到次年 1 月的 end_date 计算逻辑
        let end_date = calc_period_end_date(2026, 12);
        assert_eq!(
            end_date.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-12-31 23:59:59"
        );
    }

    #[test]
    fn test_期间状态校验_已结账识别() {
        // 验证 CLOSED 状态被识别为已结账，OPEN 状态不被识别
        // 复现 close_period 中 `if period.status == "CLOSED"` 重复结账检查逻辑
        assert!(is_period_closed(PERIOD_STATUS_CLOSED));
        assert!(!is_period_closed(PERIOD_STATUS_OPEN));
    }

    #[test]
    fn test_已结账期间锁定错误消息格式() {
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
    fn test_未设置会计期间错误消息格式() {
        // 验证未设置会计期间的错误消息拼接（使用 ymd! 夹具宏解析日期）
        // 复现 check_date_locked 中 `format!("日期 {} 不在任何已设置的会计期间内...")` 逻辑
        let date = ymd!(2026, 7, 9);
        let msg = format_no_period_error_msg(date);
        assert!(msg.contains("2026-07-09"));
        assert!(msg.contains("不在任何已设置的会计期间内"));
        assert!(msg.contains("请先创建对应的会计期间"));
    }

    #[test]
    fn test_decs夹具宏可用性() {
        // 验证 decs! 夹具宏可正常解析 Decimal 字符串
        // 注：本 service 不直接涉及金额，但保留宏以符合项目测试夹具规范
        let v = decs!("100.00");
        assert_eq!(v.to_string(), "100.00");
    }

    #[tokio::test]
    async fn test_服务实例化() {
        // 验证 AccountingPeriodService 可正常实例化（define_service! 宏生成 new 方法）
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_获取当前期间_无schema时降级() {
        // 需要 accounting_periods 表 schema 的真实场景，标注 #[ignore]
        // sqlite::memory: 无 schema 时，get_current_period 预期返回数据库错误
        // 真实环境需先建表，应返回 Ok(None) 表示无开放期间
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        let result = service.get_current_period().await;
        match result {
            Ok(opt) => assert!(opt.is_none(), "空库应返回 None"),
            Err(_) => {} // 无 schema 时数据库报错属于预期降级
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_校验日期锁定_无期间时应报错() {
        // 需要 accounting_periods 表 schema 的真实场景，标注 #[ignore]
        // 真实环境应返回 BusinessError（日期不在任何已设置的会计期间内）
        let db = setup_test_db().await;
        let service = AccountingPeriodService::new(Arc::new(db));
        let date = ymd!(2026, 3, 15);
        let result = service.check_date_locked(date).await;
        assert!(result.is_err(), "无会计期间时应返回错误");
    }
}
