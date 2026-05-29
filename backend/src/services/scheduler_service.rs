//! 定时任务调度器 Service
//!
//! 提供报表订阅定时任务的管理和执行功能

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

use sea_orm::DatabaseConnection;

use crate::models::report_subscription::{
    Entity as ReportSubscriptionEntity, Model as ReportSubscriptionModel,
};
use crate::services::report_template_service::ReportTemplateService;
use crate::utils::error::AppError;

/// 定时任务调度器
#[allow(dead_code)]
pub struct SchedulerService {
    db: Arc<DatabaseConnection>,
    scheduler: JobScheduler,
}

#[allow(dead_code)]
impl SchedulerService {
    /// 创建新的调度器实例
    pub async fn new(db: Arc<DatabaseConnection>) -> Result<Self, AppError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| AppError::BusinessError(format!("创建调度器失败: {}", e)))?;

        Ok(Self { db, scheduler })
    }

    /// 启动调度器
    pub async fn start(&mut self) -> Result<(), AppError> {
        // 添加报表订阅定时任务
        self.add_report_subscription_job().await?;

        // 添加每日财务指标计算任务
        self.add_financial_indicator_job().await?;

        // 启动调度器
        self.scheduler
            .start()
            .await
            .map_err(|e| AppError::BusinessError(format!("启动调度器失败: {}", e)))?;

        tracing::info!("定时任务调度器已启动");
        Ok(())
    }

    /// 停止调度器
    pub async fn stop(&mut self) -> Result<(), AppError> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| AppError::BusinessError(format!("停止调度器失败: {}", e)))?;

        tracing::info!("定时任务调度器已停止");
        Ok(())
    }

    /// 添加报表订阅定时任务
    async fn add_report_subscription_job(&mut self) -> Result<(), AppError> {
        let db = self.db.clone();

        // 每分钟检查一次待执行的订阅
        let job = Job::new("0 * * * * *", move |_, _| {
            let db = db.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::execute_pending_subscriptions(&db).await {
                    tracing::error!("执行报表订阅任务失败: {}", e);
                }
            });
        })
        .map_err(|e| AppError::BusinessError(format!("创建定时任务失败: {}", e)))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| AppError::BusinessError(format!("添加定时任务失败: {}", e)))?;

        tracing::info!("已添加报表订阅定时任务");
        Ok(())
    }

    /// 添加每日财务指标计算任务
    ///
    /// 每天凌晨 00:05 自动计算上月的财务指标
    async fn add_financial_indicator_job(&mut self) -> Result<(), AppError> {
        let db = self.db.clone();

        // 每天凌晨 00:05 执行
        let job = Job::new("0 5 0 * * *", move |_, _| {
            let db = db.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::execute_financial_indicator_calculation(&db).await {
                    tracing::error!("执行财务指标计算任务失败: {}", e);
                }
            });
        })
        .map_err(|e| AppError::BusinessError(format!("创建财务指标计算任务失败: {}", e)))?;

        self.scheduler
            .add(job)
            .await
            .map_err(|e| AppError::BusinessError(format!("添加财务指标计算任务失败: {}", e)))?;

        tracing::info!("已添加每日财务指标计算任务（每天凌晨 00:05 执行）");
        Ok(())
    }

    /// 执行财务指标计算
    async fn execute_financial_indicator_calculation(
        db: &Arc<DatabaseConnection>,
    ) -> Result<(), AppError> {
        use chrono::Datelike;

        // 计算上月的期间
        let now = chrono::Utc::now();
        let last_month = if now.month() == 1 {
            now.with_year(now.year() - 1)
                .and_then(|d| d.with_month(12))
                .unwrap_or(now)
        } else {
            now.with_month(now.month() - 1).unwrap_or(now)
        };
        let period = format!("{:04}-{:02}", last_month.year(), last_month.month());

        tracing::info!("开始执行每日财务指标计算，期间: {}", period);

        let fa_service =
            crate::services::financial_analysis_service::FinancialAnalysisService::new(db.clone());

        match fa_service.calculate_indicators(&period, 0).await {
            Ok(results) => {
                tracing::info!(
                    "财务指标定时计算完成: 期间={}, 计算 {} 个指标",
                    period,
                    results.len()
                );
            }
            Err(e) => {
                tracing::error!("财务指标定时计算失败: 期间={}, 错误={}", period, e);
            }
        }

        Ok(())
    }

    /// 执行待处理的订阅
    async fn execute_pending_subscriptions(db: &Arc<DatabaseConnection>) -> Result<(), AppError> {
        let now = Utc::now();

        // 查询待执行的订阅
        let subscriptions = ReportSubscriptionEntity::find()
            .filter(crate::models::report_subscription::Column::IsEnabled.eq(true))
            .filter(crate::models::report_subscription::Column::Status.eq("ACTIVE"))
            .filter(crate::models::report_subscription::Column::NextRunAt.lte(now))
            .all(db.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if subscriptions.is_empty() {
            return Ok(());
        }

        tracing::info!("找到 {} 个待执行的报表订阅", subscriptions.len());

        for subscription in subscriptions {
            if let Err(e) = Self::execute_subscription(db, &subscription).await {
                tracing::error!("执行订阅 {} 失败: {}", subscription.id, e);

                // 更新执行状态为失败
                let _ = Self::update_subscription_status(
                    db,
                    subscription.id,
                    "failed",
                    Some(e.to_string()),
                )
                .await;
            }
        }

        Ok(())
    }

    /// 执行单个订阅
    async fn execute_subscription(
        db: &Arc<DatabaseConnection>,
        subscription: &ReportSubscriptionModel,
    ) -> Result<(), AppError> {
        tracing::info!(
            "开始执行报表订阅: {} (ID: {})",
            subscription.name,
            subscription.id
        );

        let template_service = ReportTemplateService::new(db.clone());

        // 获取报表模板
        let template = template_service
            .get_by_id(subscription.template_id)
            .await?
            .ok_or_else(|| AppError::NotFound("报表模板不存在".to_string()))?;

        // 执行报表
        let (headers, data, _total) = template_service
            .execute_custom_report(subscription.template_id, 1, 10000)
            .await?;

        // 根据导出格式生成文件
        let (filename, _content) = match subscription.export_format.as_str() {
            "csv" => {
                let csv =
                    crate::services::import_export_service::ImportExportService::generate_csv(
                        &headers, &data,
                    )?;
                (format!("report_{}.csv", template.code), csv.into_bytes())
            }
            "excel" => {
                // 简化实现：使用CSV格式
                let csv =
                    crate::services::import_export_service::ImportExportService::generate_csv(
                        &headers, &data,
                    )?;
                (format!("report_{}.xlsx", template.code), csv.into_bytes())
            }
            _ => {
                return Err(AppError::BusinessError(format!(
                    "不支持的导出格式: {}",
                    subscription.export_format
                )));
            }
        };

        // 获取收件人列表
        let recipients: Vec<String> =
            serde_json::from_value(subscription.recipients.clone()).unwrap_or_default();

        // 发送邮件（如果有收件人）
        if !recipients.is_empty() {
            if let Some(email_service) = crate::services::email_service::EmailService::from_env() {
                let subject = format!("报表订阅 - {}", template.name);
                let body = format!(
                    "<h2>报表订阅通知</h2><p>报表 <strong>{}</strong> 已生成。</p><p>附件: {}</p>",
                    template.name, filename
                );

                match email_service
                    .send_html_email(recipients.clone(), subject, body)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("报表 {} 邮件发送成功，收件人: {:?}", filename, recipients);
                    }
                    Err(e) => {
                        tracing::error!("报表 {} 邮件发送失败: {:?}", filename, e);
                    }
                }
            } else {
                tracing::warn!("邮件服务未配置，跳过发送报表邮件");
            }
        }

        // 更新订阅状态
        let next_run = subscription.calculate_next_run();
        Self::update_subscription_after_execution(db, subscription.id, "success", None, next_run)
            .await?;

        tracing::info!("报表订阅 {} 执行完成", subscription.name);

        Ok(())
    }

    /// 更新订阅执行状态
    async fn update_subscription_status(
        db: &Arc<DatabaseConnection>,
        subscription_id: i32,
        status: &str,
        error: Option<String>,
    ) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(subscription_id)
            .one(db.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        let mut active_model: crate::models::report_subscription::ActiveModel = model.into();
        active_model.last_run_status = Set(Some(status.to_string()));
        active_model.last_run_error = Set(error);
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 更新订阅执行后的状态
    async fn update_subscription_after_execution(
        db: &Arc<DatabaseConnection>,
        subscription_id: i32,
        status: &str,
        error: Option<String>,
        next_run: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(subscription_id)
            .one(db.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        let run_count = model.run_count + 1;
        let mut active_model: crate::models::report_subscription::ActiveModel = model.into();
        active_model.last_run_at = Set(Some(Utc::now()));
        active_model.last_run_status = Set(Some(status.to_string()));
        active_model.last_run_error = Set(error);
        active_model.next_run_at = Set(next_run);
        active_model.run_count = Set(run_count);
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
