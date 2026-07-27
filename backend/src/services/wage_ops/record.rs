//! 工资记录 Service impl 子模块（wage_ops/record）
//!
//! 批次 490 D10-4a 拆分：从原 `wage_service.rs` L634-848 迁移。
//! 包含 WageRecordService 的 10 个方法（new 留在 facade）：
//! - generate_record_no（私有，生成单号 WR-YYYYMM-NNN）
//! - create / update / delete（CRUD + 业务校验）
//! - confirm / pay / cancel（状态机 draft→confirmed→paid/cancelled）
//! - get_by_id / get_by_no / list（查询）
//!
//! 业务规则：
//! - 周期结束 ≥ 周期开始
//! - 仅 draft 状态可更新/删除，确认前必须有明细
//! - confirm 需检查 detail_count > 0，pay 需 confirmed 状态
//! - 软删除（is_deleted = true）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::models::wage_record::{
    self, ActiveModel as RecordActiveModel, Entity as RecordEntity, Model as RecordModel,
};
use crate::models::wage_record_detail;
use crate::models::status::wage_record_status;
use crate::utils::error::AppError;

use crate::services::wage_service::{
    CreateWageRecordRequest, UpdateWageRecordRequest, WageRecordQuery, WageRecordService,
};

impl WageRecordService {
    /// 生成工资单号：WR-YYYYMM-NNN
    fn generate_record_no(period: chrono::NaiveDate) -> String {
        let ym = period.format("%Y%m");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("WR-{}-{:03}", ym, random)
    }

    /// 创建工资记录（仅创建空记录，需调用 calculate 触发计算）
    pub async fn create(&self, req: CreateWageRecordRequest) -> Result<RecordModel, AppError> {
        // 业务校验：周期结束必须 ≥ 周期开始
        if req.period_end < req.period_start {
            return Err(AppError::business("周期结束日期必须 ≥ 周期开始日期"));
        }

        let record_no = Self::generate_record_no(req.period_start);
        let now = crate::utils::date_utils::utc_now_fixed();

        let active = RecordActiveModel {
            id: Default::default(),
            record_no: Set(record_no),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            workshop: Set(req.workshop),
            total_workers: Set(0),
            total_step_records: Set(0),
            total_qualified_quantity: Set(Decimal::ZERO),
            total_duration_minutes: Set(0),
            total_amount: Set(Decimal::ZERO),
            status: Set(wage_record_status::DRAFT.to_string()),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            paid_by: Set(None),
            paid_at: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("工资记录创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新工资记录（仅 draft 状态可更新）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateWageRecordRequest,
    ) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可更新，当前状态: {}",
                model.status
            )));
        }

        let mut active: RecordActiveModel = model.into();

        if let Some(v) = req.workshop {
            active.workshop = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除工资记录（仅 draft 状态可删除，连带删除明细）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 确认工资（draft → confirmed）
    /// V15 Batch04-P1-3：确认时生成"应付工资"凭证并发布 WageConfirmed 事件
    pub async fn confirm(&self, id: i32, confirmed_by: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可确认，当前状态: {}",
                model.status
            )));
        }
        // 业务校验：必须有明细才能确认
        let detail_count = wage_record_detail::Entity::find()
            .filter(wage_record_detail::Column::WageRecordId.eq(id))
            .filter(wage_record_detail::Column::IsDeleted.eq(false))
            .count(&*self.db)
            .await?;
        if detail_count == 0 {
            return Err(AppError::business("工资记录无明细，请先调用 calculate 触发计算"));
        }

        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::CONFIRMED.to_string());
        active.confirmed_by = Set(Some(confirmed_by));
        active.confirmed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;

        // V15 Batch04-P1-3：生成"应付工资"凭证（借：生产成本-直接人工，贷：应付职工薪酬）
        if let Err(e) = self.create_wage_confirm_voucher(&updated, confirmed_by).await {
            tracing::warn!(wage_record_id = updated.id, "工资确认凭证生成失败：{}", e);
        }

        // V15 Batch04-P1-3：发布 WageConfirmed 事件，供成本核算服务监听并归集 direct_labor
        crate::services::event_bus::publish(crate::services::event_bus::BusinessEvent::WageConfirmed {
            wage_record_id: updated.id,
            record_no: updated.record_no.clone(),
            total_amount: updated.total_amount.unwrap_or(Decimal::ZERO),
            confirmed_by,
        });
        tracing::info!(wage_record_id = updated.id, "工资确认事件已发布");

        Ok(updated)
    }

    /// 发放工资（confirmed → paid）
    /// V15 Batch04-P1-3：发放时生成"工资发放"凭证并发布 WagePaid 事件
    pub async fn pay(&self, id: i32, paid_by: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::CONFIRMED {
            return Err(AppError::business(format!(
                "仅已确认(confirmed)状态可发放，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::PAID.to_string());
        active.paid_by = Set(Some(paid_by));
        active.paid_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;

        // V15 Batch04-P1-3：生成"工资发放"凭证（借：应付职工薪酬，贷：银行存款/现金）
        if let Err(e) = self.create_wage_pay_voucher(&updated, paid_by).await {
            tracing::warn!(wage_record_id = updated.id, "工资发放凭证生成失败：{}", e);
        }

        // V15 Batch04-P1-3：发布 WagePaid 事件
        crate::services::event_bus::publish(crate::services::event_bus::BusinessEvent::WagePaid {
            wage_record_id: updated.id,
            record_no: updated.record_no.clone(),
            total_amount: updated.total_amount.unwrap_or(Decimal::ZERO),
            paid_by,
        });
        tracing::info!(wage_record_id = updated.id, "工资发放事件已发布");

        Ok(updated)
    }

    /// V15 Batch04-P1-3：生成工资确认凭证（借：生产成本-直接人工，贷：应付职工薪酬）
    async fn create_wage_confirm_voucher(
        &self,
        record: &RecordModel,
        user_id: i32,
    ) -> Result<(), AppError> {
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        let amount = record.total_amount.unwrap_or(Decimal::ZERO);
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let voucher_date = chrono::Utc::now().date_naive();
        let req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "transfer".to_string(),
            voucher_date,
            source_type: Some("wage".to_string()),
            source_module: Some("wage_record".to_string()),
            source_bill_id: Some(record.id),
            source_bill_no: Some(record.record_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: None,
                    subject_code: Some("500101".to_string()),
                    subject_name: Some("生产成本-直接人工".to_string()),
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(format!("工资确认 {}", record.record_no)),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: None,
                    subject_code: Some("2211".to_string()),
                    subject_name: Some("应付职工薪酬".to_string()),
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(format!("工资确认 {}", record.record_no)),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
            ],
        };
        voucher_service.create_and_post(req, user_id).await?;
        Ok(())
    }

    /// V15 Batch04-P1-3：生成工资发放凭证（借：应付职工薪酬，贷：银行存款）
    async fn create_wage_pay_voucher(
        &self,
        record: &RecordModel,
        user_id: i32,
    ) -> Result<(), AppError> {
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        let amount = record.total_amount.unwrap_or(Decimal::ZERO);
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let voucher_date = chrono::Utc::now().date_naive();
        let req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "payment".to_string(),
            voucher_date,
            source_type: Some("wage".to_string()),
            source_module: Some("wage_record".to_string()),
            source_bill_id: Some(record.id),
            source_bill_no: Some(record.record_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: None,
                    subject_code: Some("2211".to_string()),
                    subject_name: Some("应付职工薪酬".to_string()),
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(format!("工资发放 {}", record.record_no)),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: None,
                    subject_code: Some("1002".to_string()),
                    subject_name: Some("银行存款".to_string()),
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(format!("工资发放 {}", record.record_no)),
                    assist_customer_id: None,
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
            ],
        };
        voucher_service.create_and_post(req, user_id).await?;
        Ok(())
    }

    /// 取消工资（draft/confirmed → cancelled）
    pub async fn cancel(&self, id: i32) -> Result<RecordModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != wage_record_status::DRAFT && model.status != wage_record_status::CONFIRMED
        {
            return Err(AppError::business(format!(
                "仅草稿(draft)或已确认(confirmed)状态可取消，当前状态: {}",
                model.status
            )));
        }
        let mut active: RecordActiveModel = model.into();
        active.status = Set(wage_record_status::CANCELLED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RecordModel, AppError> {
        let model = RecordEntity::find_by_id(id)
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资记录 {} 不存在", id)))?;
        Ok(model)
    }

    /// 按单号查询
    pub async fn get_by_no(&self, record_no: &str) -> Result<RecordModel, AppError> {
        let model = RecordEntity::find()
            .filter(wage_record::Column::RecordNo.eq(record_no))
            .filter(wage_record::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("工资单号 {} 不存在", record_no)))?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: WageRecordQuery,
    ) -> Result<(Vec<RecordModel>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RecordEntity::find().filter(wage_record::Column::IsDeleted.eq(false));

        if let Some(v) = query.record_no {
            q = q.filter(wage_record::Column::RecordNo.like(format!("%{}%", v)));
        }
        if let Some(v) = query.workshop {
            q = q.filter(wage_record::Column::Workshop.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(wage_record::Column::Status.eq(v));
        }
        if let Some(v) = query.period_start {
            q = q.filter(wage_record::Column::PeriodStart.gte(v));
        }
        if let Some(v) = query.period_end {
            q = q.filter(wage_record::Column::PeriodEnd.lte(v));
        }

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(wage_record::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
