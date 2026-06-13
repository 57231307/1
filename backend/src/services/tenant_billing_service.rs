#![allow(dead_code)]

use crate::models::tenant::{self, Entity as Tenant};
use crate::models::tenant_invoice::{self, Entity as TenantInvoice};
use crate::models::tenant_plan::{self, Entity as TenantPlan};
use crate::models::tenant_subscription::{self, Entity as TenantSubscription};
use crate::models::tenant_usage::{self, Entity as TenantUsage};
use crate::utils::error::AppError;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

pub struct TenantBillingService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanInfo {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub max_users: i32,
    pub max_storage_mb: i32,
    pub max_api_calls_per_day: i32,
    pub price_monthly: Decimal,
    pub price_yearly: Decimal,
    pub features: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentPlanInfo {
    pub plan: PlanInfo,
    pub subscription: SubscriptionInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionInfo {
    pub id: i32,
    pub status: String,
    pub billing_cycle: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub auto_renew: bool,
    pub current_price: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub tenant_id: i32,
    pub tenant_name: String,
    pub plan_name: String,
    pub api_calls_today: i64,
    pub max_api_calls_per_day: i32,
    pub storage_used_mb: i64,
    pub max_storage_mb: i32,
    pub current_users: i64,
    pub max_users: i32,
    pub usage_percentages: UsagePercentages,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsagePercentages {
    pub users: f64,
    pub storage: f64,
    pub api_calls: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoiceItem {
    pub id: i32,
    pub invoice_number: String,
    pub billing_period_start: String,
    pub billing_period_end: String,
    pub amount: Decimal,
    pub status: String,
    pub paid_at: Option<String>,
    pub due_date: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradePlanRequest {
    pub plan_id: i32,
    pub billing_cycle: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenewSubscriptionRequest {
    pub billing_cycle: Option<String>,
}

impl TenantBillingService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_all_plans(&self) -> Result<Vec<PlanInfo>, AppError> {
        let plans = TenantPlan::find()
            .filter(tenant_plan::Column::IsActive.eq(true))
            .order_by_asc(tenant_plan::Column::PriceMonthly)
            .all(self.db.as_ref())
            .await?;

        Ok(plans
            .into_iter()
            .map(|p| PlanInfo {
                id: p.id,
                code: p.code,
                name: p.name,
                description: p.description,
                max_users: p.max_users,
                max_storage_mb: p.max_storage_mb,
                max_api_calls_per_day: p.max_api_calls_per_day,
                price_monthly: p.price_monthly,
                price_yearly: p.price_yearly,
                features: p.features,
            })
            .collect())
    }

    pub async fn get_current_plan(
        &self,
        tenant_id: i32,
    ) -> Result<Option<CurrentPlanInfo>, AppError> {
        let subscription = TenantSubscription::find()
            .filter(tenant_subscription::Column::TenantId.eq(tenant_id))
            .filter(tenant_subscription::Column::Status.eq("ACTIVE"))
            .order_by_desc(tenant_subscription::Column::CreatedAt)
            .one(self.db.as_ref())
            .await?;

        match subscription {
            Some(sub) => {
                let plan = TenantPlan::find_by_id(sub.plan_id)
                    .one(self.db.as_ref())
                    .await?;

                match plan {
                    Some(p) => Ok(Some(CurrentPlanInfo {
                        plan: PlanInfo {
                            id: p.id,
                            code: p.code,
                            name: p.name,
                            description: p.description,
                            max_users: p.max_users,
                            max_storage_mb: p.max_storage_mb,
                            max_api_calls_per_day: p.max_api_calls_per_day,
                            price_monthly: p.price_monthly,
                            price_yearly: p.price_yearly,
                            features: p.features,
                        },
                        subscription: SubscriptionInfo {
                            id: sub.id,
                            status: sub.status,
                            billing_cycle: sub.billing_cycle,
                            start_date: sub.start_date.to_rfc3339(),
                            end_date: sub.end_date.map(|d| d.to_rfc3339()),
                            auto_renew: sub.auto_renew,
                            current_price: sub.current_price,
                        },
                    })),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    pub async fn get_usage_stats(&self, tenant_id: i32) -> Result<Option<UsageStats>, AppError> {
        let tenant = Tenant::find_by_id(tenant_id).one(self.db.as_ref()).await?;

        let tenant_info = match tenant {
            Some(t) => t,
            None => return Ok(None),
        };

        let current_plan = self.get_current_plan(tenant_id).await?;
        let plan_name = current_plan
            .as_ref()
            .map(|c| c.plan.name.clone())
            .unwrap_or_else(|| "免费".to_string());
        let max_users = current_plan
            .as_ref()
            .map(|c| c.plan.max_users)
            .unwrap_or(10);
        let max_storage_mb = current_plan
            .as_ref()
            .map(|c| c.plan.max_storage_mb)
            .unwrap_or(1024);
        let max_api_calls = current_plan
            .as_ref()
            .map(|c| c.plan.max_api_calls_per_day)
            .unwrap_or(1000);

        let _now = Utc::now();
        let today_start = crate::utils::date_utils::today_start_utc();

        let today_usage = TenantUsage::find()
            .filter(tenant_usage::Column::TenantId.eq(tenant_id))
            .filter(tenant_usage::Column::StatDate.gte(today_start))
            .one(self.db.as_ref())
            .await?;

        let api_calls_today = today_usage.as_ref().map(|u| u.api_calls).unwrap_or(0);
        let storage_used_mb = today_usage.as_ref().map(|u| u.storage_used_mb).unwrap_or(0);
        let current_users = today_usage
            .as_ref()
            .map(|u| u.user_count as i64)
            .unwrap_or(0);

        let user_pct = if max_users > 0 {
            (current_users as f64 / max_users as f64) * 100.0
        } else {
            0.0
        };
        let storage_pct = if max_storage_mb > 0 {
            (storage_used_mb as f64 / max_storage_mb as f64) * 100.0
        } else {
            0.0
        };
        let api_pct = if max_api_calls > 0 {
            (api_calls_today as f64 / max_api_calls as f64) * 100.0
        } else {
            0.0
        };

        Ok(Some(UsageStats {
            tenant_id,
            tenant_name: tenant_info.name,
            plan_name,
            api_calls_today,
            max_api_calls_per_day: max_api_calls,
            storage_used_mb,
            max_storage_mb,
            current_users,
            max_users,
            usage_percentages: UsagePercentages {
                users: (user_pct * 100.0).round() / 100.0,
                storage: (storage_pct * 100.0).round() / 100.0,
                api_calls: (api_pct * 100.0).round() / 100.0,
            },
        }))
    }

    pub async fn check_usage_limits(
        &self,
        tenant_id: i32,
    ) -> Result<Vec<LimitViolation>, AppError> {
        let usage_stats = self.get_usage_stats(tenant_id).await?;
        let mut violations = Vec::new();

        if let Some(stats) = usage_stats {
            if stats.usage_percentages.users >= 100.0 {
                violations.push(LimitViolation {
                    resource: "users".to_string(),
                    current: stats.current_users,
                    limit: stats.max_users as i64,
                    message: format!(
                        "用户数已达上限 ({}/{})",
                        stats.current_users, stats.max_users
                    ),
                });
            }
            if stats.usage_percentages.storage >= 100.0 {
                violations.push(LimitViolation {
                    resource: "storage".to_string(),
                    current: stats.storage_used_mb,
                    limit: stats.max_storage_mb as i64,
                    message: format!(
                        "存储空间已达上限 ({}/{} MB)",
                        stats.storage_used_mb, stats.max_storage_mb
                    ),
                });
            }
            if stats.usage_percentages.api_calls >= 100.0 {
                violations.push(LimitViolation {
                    resource: "api_calls".to_string(),
                    current: stats.api_calls_today,
                    limit: stats.max_api_calls_per_day as i64,
                    message: format!(
                        "API 调用次数已达上限 ({}/{})",
                        stats.api_calls_today, stats.max_api_calls_per_day
                    ),
                });
            }
        }

        Ok(violations)
    }

    pub async fn upgrade_plan(
        &self,
        tenant_id: i32,
        req: UpgradePlanRequest,
    ) -> Result<CurrentPlanInfo, AppError> {
        let plan = TenantPlan::find_by_id(req.plan_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found("套餐不存在"))?;

        if !plan.is_active {
            return Err(AppError::business("该套餐已停用"));
        }

        if req.billing_cycle != "MONTHLY" && req.billing_cycle != "YEARLY" {
            return Err(AppError::bad_request(
                "计费周期必须为 MONTHLY 或 YEARLY".to_string(),
            ));
        }

        let price = if req.billing_cycle == "YEARLY" {
            plan.price_yearly
        } else {
            plan.price_monthly
        };
        let now = Utc::now();
        let end_date = if req.billing_cycle == "YEARLY" {
            now + Duration::days(365)
        } else {
            now + Duration::days(30)
        };

        let billing_cycle = req.billing_cycle.clone();

        let existing_subscription = TenantSubscription::find()
            .filter(tenant_subscription::Column::TenantId.eq(tenant_id))
            .filter(tenant_subscription::Column::Status.eq("ACTIVE"))
            .one(self.db.as_ref())
            .await?;

        let subscription = if let Some(existing) = existing_subscription {
            let mut active: tenant_subscription::ActiveModel = existing.into();
            active.plan_id = Set(plan.id);
            active.billing_cycle = Set(billing_cycle.clone());
            active.start_date = Set(now);
            active.end_date = Set(Some(end_date));
            active.current_price = Set(price);
            active.updated_at = Set(now);
            active.update(self.db.as_ref()).await?
        } else {
            let active = tenant_subscription::ActiveModel {
                id: Default::default(),
                tenant_id: Set(tenant_id),
                plan_id: Set(plan.id),
                status: Set("ACTIVE".to_string()),
                billing_cycle: Set(billing_cycle.clone()),
                start_date: Set(now),
                end_date: Set(Some(end_date)),
                auto_renew: Set(true),
                current_price: Set(price),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(self.db.as_ref()).await?
        };

        let mut tenant_active: tenant::ActiveModel = Tenant::find_by_id(tenant_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found("租户不存在"))?
            .into();
        tenant_active.plan_id = Set(Some(plan.id));
        tenant_active.expired_at = Set(Some(end_date));
        tenant_active.updated_at = Set(now);
        tenant_active.update(self.db.as_ref()).await?;

        self.generate_invoice(
            tenant_id,
            subscription.id,
            plan.id,
            price,
            billing_cycle,
            now,
            end_date,
        )
        .await?;

        Ok(CurrentPlanInfo {
            plan: PlanInfo {
                id: plan.id,
                code: plan.code,
                name: plan.name,
                description: plan.description,
                max_users: plan.max_users,
                max_storage_mb: plan.max_storage_mb,
                max_api_calls_per_day: plan.max_api_calls_per_day,
                price_monthly: plan.price_monthly,
                price_yearly: plan.price_yearly,
                features: plan.features,
            },
            subscription: SubscriptionInfo {
                id: subscription.id,
                status: subscription.status,
                billing_cycle: subscription.billing_cycle,
                start_date: subscription.start_date.to_rfc3339(),
                end_date: subscription.end_date.map(|d| d.to_rfc3339()),
                auto_renew: subscription.auto_renew,
                current_price: subscription.current_price,
            },
        })
    }

    pub async fn renew_subscription(
        &self,
        tenant_id: i32,
        req: RenewSubscriptionRequest,
    ) -> Result<CurrentPlanInfo, AppError> {
        let current_plan = self
            .get_current_plan(tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("当前无有效订阅"))?;

        let billing_cycle = req
            .billing_cycle
            .unwrap_or(current_plan.subscription.billing_cycle);
        let now = Utc::now();
        let end_date = if billing_cycle == "YEARLY" {
            now + Duration::days(365)
        } else {
            now + Duration::days(30)
        };

        let price = if billing_cycle == "YEARLY" {
            current_plan.plan.price_yearly
        } else {
            current_plan.plan.price_monthly
        };

        let subscription = TenantSubscription::find()
            .filter(tenant_subscription::Column::TenantId.eq(tenant_id))
            .filter(tenant_subscription::Column::Status.eq("ACTIVE"))
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found("当前无有效订阅"))?;

        let mut active: tenant_subscription::ActiveModel = subscription.into();
        active.billing_cycle = Set(billing_cycle.clone());
        active.start_date = Set(now);
        active.end_date = Set(Some(end_date));
        active.current_price = Set(price);
        active.updated_at = Set(now);
        let subscription = active.update(self.db.as_ref()).await?;

        let mut tenant_active: tenant::ActiveModel = Tenant::find_by_id(tenant_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found("租户不存在"))?
            .into();
        tenant_active.expired_at = Set(Some(end_date));
        tenant_active.updated_at = Set(now);
        tenant_active.update(self.db.as_ref()).await?;

        self.generate_invoice(
            tenant_id,
            subscription.id,
            current_plan.plan.id,
            price,
            billing_cycle,
            now,
            end_date,
        )
        .await?;

        Ok(CurrentPlanInfo {
            plan: current_plan.plan,
            subscription: SubscriptionInfo {
                id: subscription.id,
                status: subscription.status,
                billing_cycle: subscription.billing_cycle,
                start_date: subscription.start_date.to_rfc3339(),
                end_date: subscription.end_date.map(|d| d.to_rfc3339()),
                auto_renew: subscription.auto_renew,
                current_price: subscription.current_price,
            },
        })
    }

    pub async fn list_invoices(
        &self,
        tenant_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<InvoiceItem>, u64), AppError> {
        let paginator = TenantInvoice::find()
            .filter(tenant_invoice::Column::TenantId.eq(tenant_id))
            .order_by_desc(tenant_invoice::Column::CreatedAt)
            .paginate(self.db.as_ref(), page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        let invoice_items = items
            .into_iter()
            .map(|inv| InvoiceItem {
                id: inv.id,
                invoice_number: inv.invoice_number,
                billing_period_start: inv.billing_period_start.to_rfc3339(),
                billing_period_end: inv.billing_period_end.to_rfc3339(),
                amount: inv.amount,
                status: inv.status,
                paid_at: inv.paid_at.map(|d| d.to_rfc3339()),
                due_date: inv.due_date.to_rfc3339(),
                created_at: inv.created_at.to_rfc3339(),
            })
            .collect();

        Ok((invoice_items, total))
    }

    pub async fn record_api_call(&self, tenant_id: i32) -> Result<(), AppError> {
        let now = Utc::now();
        let today_start = crate::utils::date_utils::today_start_utc();

        let existing = TenantUsage::find()
            .filter(tenant_usage::Column::TenantId.eq(tenant_id))
            .filter(tenant_usage::Column::StatDate.gte(today_start))
            .one(self.db.as_ref())
            .await?;

        if let Some(usage) = existing {
            let api_calls = usage.api_calls;
            let mut active: tenant_usage::ActiveModel = usage.into();
            active.api_calls = Set(api_calls + 1);
            active.updated_at = Set(now);
            active.update(self.db.as_ref()).await?;
        } else {
            let active = tenant_usage::ActiveModel {
                id: Default::default(),
                tenant_id: Set(tenant_id),
                stat_date: Set(today_start),
                api_calls: Set(1),
                storage_used_mb: Set(0),
                user_count: Set(0),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(self.db.as_ref()).await?;
        }

        Ok(())
    }

    pub async fn update_storage_usage(
        &self,
        tenant_id: i32,
        storage_mb: i64,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let today_start = crate::utils::date_utils::today_start_utc();

        let existing = TenantUsage::find()
            .filter(tenant_usage::Column::TenantId.eq(tenant_id))
            .filter(tenant_usage::Column::StatDate.gte(today_start))
            .one(self.db.as_ref())
            .await?;

        if let Some(usage) = existing {
            let mut active: tenant_usage::ActiveModel = usage.into();
            active.storage_used_mb = Set(storage_mb);
            active.updated_at = Set(now);
            active.update(self.db.as_ref()).await?;
        } else {
            let active = tenant_usage::ActiveModel {
                id: Default::default(),
                tenant_id: Set(tenant_id),
                stat_date: Set(today_start),
                api_calls: Set(0),
                storage_used_mb: Set(storage_mb),
                user_count: Set(0),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(self.db.as_ref()).await?;
        }

        Ok(())
    }

    pub async fn update_user_count(&self, tenant_id: i32, count: i32) -> Result<(), AppError> {
        let now = Utc::now();
        let today_start = crate::utils::date_utils::today_start_utc();

        let existing = TenantUsage::find()
            .filter(tenant_usage::Column::TenantId.eq(tenant_id))
            .filter(tenant_usage::Column::StatDate.gte(today_start))
            .one(self.db.as_ref())
            .await?;

        if let Some(usage) = existing {
            let mut active: tenant_usage::ActiveModel = usage.into();
            active.user_count = Set(count);
            active.updated_at = Set(now);
            active.update(self.db.as_ref()).await?;
        } else {
            let active = tenant_usage::ActiveModel {
                id: Default::default(),
                tenant_id: Set(tenant_id),
                stat_date: Set(today_start),
                api_calls: Set(0),
                storage_used_mb: Set(0),
                user_count: Set(count),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(self.db.as_ref()).await?;
        }

        Ok(())
    }

    pub async fn process_auto_renewals(&self) -> Result<usize, AppError> {
        let now = Utc::now();
        let threshold = now + Duration::hours(24);

        let expiring_subscriptions = TenantSubscription::find()
            .filter(tenant_subscription::Column::Status.eq("ACTIVE"))
            .filter(tenant_subscription::Column::AutoRenew.eq(true))
            .filter(tenant_subscription::Column::EndDate.lt(threshold))
            .all(self.db.as_ref())
            .await?;

        let mut renewed_count = 0;

        for sub in expiring_subscriptions {
            let plan = TenantPlan::find_by_id(sub.plan_id)
                .one(self.db.as_ref())
                .await?;

            if let Some(plan) = plan {
                let duration_days = if sub.billing_cycle == "YEARLY" {
                    365
                } else {
                    30
                };
                let new_end_date = sub.end_date.unwrap_or(now) + Duration::days(duration_days);
                let price = if sub.billing_cycle == "YEARLY" {
                    plan.price_yearly
                } else {
                    plan.price_monthly
                };

                let mut active: tenant_subscription::ActiveModel = sub.clone().into();
                active.start_date = Set(sub.end_date.unwrap_or(now));
                active.end_date = Set(Some(new_end_date));
                active.current_price = Set(price);
                active.updated_at = Set(now);
                active.update(self.db.as_ref()).await?;

                let tenant_record = Tenant::find_by_id(sub.tenant_id)
                    .one(self.db.as_ref())
                    .await?
                    .ok_or_else(|| AppError::not_found(format!("租户 {} 不存在", sub.tenant_id)))?;
                let mut tenant_active: tenant::ActiveModel = tenant_record.into();
                tenant_active.expired_at = Set(Some(new_end_date));
                tenant_active.updated_at = Set(now);
                tenant_active.update(self.db.as_ref()).await?;

                self.generate_invoice(
                    sub.tenant_id,
                    sub.id,
                    sub.plan_id,
                    price,
                    sub.billing_cycle,
                    sub.end_date.unwrap_or(now),
                    new_end_date,
                )
                .await?;

                renewed_count += 1;
            }
        }

        Ok(renewed_count)
    }

    #[allow(clippy::too_many_arguments)]
    async fn generate_invoice(
        &self,
        tenant_id: i32,
        subscription_id: i32,
        _plan_id: i32,
        amount: Decimal,
        _billing_cycle: String,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<tenant_invoice::Model, AppError> {
        let now = Utc::now();
        let invoice_number = format!(
            "INV-{}-{}",
            now.format("%Y%m%d"),
            crate::utils::random::random_6_digit()
        );
        let due_date = now + Duration::days(30);

        let active = tenant_invoice::ActiveModel {
            id: Default::default(),
            tenant_id: Set(tenant_id),
            subscription_id: Set(subscription_id),
            invoice_number: Set(invoice_number),
            billing_period_start: Set(period_start),
            billing_period_end: Set(period_end),
            amount: Set(amount),
            status: Set("PENDING".to_string()),
            paid_at: Set(None),
            due_date: Set(due_date),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let invoice = active.insert(self.db.as_ref()).await?;
        Ok(invoice)
    }
}

use sea_orm::Set;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct LimitViolation {
    pub resource: String,
    pub current: i64,
    pub limit: i64,
    pub message: String,
}
