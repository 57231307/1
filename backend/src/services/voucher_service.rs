use crate::utils::number_generator::DocumentNumberGenerator;
// 凭证管理 Service
//
// 凭证业务逻辑层（核心）

use chrono::Datelike;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::voucher_item as vi;
use crate::models::{account_subject, voucher, voucher_item};
use crate::utils::error::AppError;
use rust_decimal::Decimal;

/// 创建凭证请求
#[derive(Debug, Clone)]
pub struct CreateVoucherRequest {
    pub voucher_type: String,
    pub voucher_date: chrono::NaiveDate,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemRequest>,
}

/// 凭证分录请求
#[derive(Debug, Clone)]
pub struct VoucherItemRequest {
    pub line_no: Option<i32>,
    pub subject_code: Option<String>,
    pub subject_name: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub summary: Option<String>,
    pub assist_customer_id: Option<i32>,
    pub assist_supplier_id: Option<i32>,
    pub assist_department_id: Option<i32>,
    pub assist_employee_id: Option<i32>,
    pub assist_project_id: Option<i32>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub assist_dye_lot_id: Option<i32>,
    pub assist_grade: Option<String>,
    pub assist_workshop_id: Option<i32>,
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,
}

/// 更新凭证请求
#[derive(Debug, Clone)]
pub struct UpdateVoucherRequest {
    pub voucher_type: Option<String>,
    pub voucher_date: Option<chrono::NaiveDate>,
    pub items: Option<Vec<VoucherItemRequest>>,
}

/// 凭证查询参数
#[derive(Debug, Clone)]
pub struct VoucherQueryParams {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 凭证 Service
pub struct VoucherService {
    db: Arc<DatabaseConnection>,
}

impl VoucherService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建凭证
    pub async fn create(
        &self,
        req: CreateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        // 校验期间锁定
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc.check_date_locked(req.voucher_date).await?;

        info!(
            "创建凭证：type={}, date={}",
            req.voucher_type, req.voucher_date
        );

        // 验证借贷平衡
        let total_debit: Decimal = req.items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = req.items.iter().map(|i| i.credit).sum();

        if total_debit != total_credit {
            warn!("凭证借贷不平衡：借={}, 贷={}", total_debit, total_credit);
            return Err(AppError::bad_request(format!(
                "凭证借贷不平衡：借方 {} != 贷方 {}",
                total_debit, total_credit
            )));
        }

        // 生成凭证编号
        let voucher_no = self
            .generate_voucher_no(&req.voucher_type, req.voucher_date)
            .await?;

        // 开启事务
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        // 创建凭证主表
        let active_model = voucher::ActiveModel {
            voucher_no: sea_orm::Set(voucher_no),
            voucher_type: sea_orm::Set(req.voucher_type),
            voucher_date: sea_orm::Set(req.voucher_date),
            source_type: sea_orm::Set(req.source_type),
            source_module: sea_orm::Set(req.source_module),
            source_bill_id: sea_orm::Set(req.source_bill_id),
            source_bill_no: sea_orm::Set(req.source_bill_no),
            batch_no: sea_orm::Set(req.batch_no),
            color_no: sea_orm::Set(req.color_no),
            status: sea_orm::Set("draft".to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        };

        let voucher = active_model
            .insert(&txn)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        info!("凭证创建成功：no={}", voucher.voucher_no);

        // 批量校验科目是否存在（优化N+1查询）
        {
            let mut subject_codes = std::collections::HashSet::new();
            for item_req in &req.items {
                if let Some(ref subject_code) = item_req.subject_code {
                    if !subject_code.is_empty() {
                        subject_codes.insert(subject_code.clone());
                    }
                }
            }
            if !subject_codes.is_empty() {
                let existing_subjects = account_subject::Entity::find()
                    .filter(
                        account_subject::Column::Code
                            .is_in(subject_codes.iter().cloned().collect::<Vec<_>>()),
                    )
                    .filter(account_subject::Column::Status.eq("active"))
                    .all(&txn)
                    .await
                    .map_err(|e| {
                        tracing::error!("批量查询科目失败: {}", e);
                        AppError::internal(format!("批量查询科目失败: {}", e))
                    })?;
                let existing_codes: std::collections::HashSet<String> =
                    existing_subjects.into_iter().map(|s| s.code).collect();
                for code in subject_codes {
                    if !existing_codes.contains(&code) {
                        return Err(AppError::bad_request(format!(
                            "科目不存在或已停用：{}",
                            code
                        )));
                    }
                }
            }
        }

        // 创建凭证分录
        for (index, item_req) in req.items.iter().enumerate() {
            let item_active_model = voucher_item::ActiveModel {
                voucher_id: sea_orm::Set(voucher.id),
                line_no: sea_orm::Set(item_req.line_no.unwrap_or((index + 1) as i32)),
                subject_code: sea_orm::Set(item_req.subject_code.clone().unwrap_or_default()),
                subject_name: sea_orm::Set(item_req.subject_name.clone().unwrap_or_default()),
                debit: sea_orm::Set(item_req.debit),
                credit: sea_orm::Set(item_req.credit),
                summary: sea_orm::Set(item_req.summary.clone()),
                assist_customer_id: sea_orm::Set(item_req.assist_customer_id),
                assist_supplier_id: sea_orm::Set(item_req.assist_supplier_id),
                assist_department_id: sea_orm::Set(item_req.assist_department_id),
                assist_employee_id: sea_orm::Set(item_req.assist_employee_id),
                assist_project_id: sea_orm::Set(item_req.assist_project_id),
                assist_batch_id: sea_orm::Set(item_req.assist_batch_id),
                assist_color_no_id: sea_orm::Set(item_req.assist_color_no_id),
                assist_dye_lot_id: sea_orm::Set(item_req.assist_dye_lot_id),
                assist_grade: sea_orm::Set(item_req.assist_grade.clone()),
                assist_workshop_id: sea_orm::Set(item_req.assist_workshop_id),
                quantity_meters: sea_orm::Set(item_req.quantity_meters),
                quantity_kg: sea_orm::Set(item_req.quantity_kg),
                unit_price: sea_orm::Set(item_req.unit_price),
                ..Default::default()
            };

            item_active_model
                .insert(&txn)
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
        }

        // 提交事务
        txn.commit()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        info!("凭证分录创建成功，共 {} 条", req.items.len());

        Ok(voucher)
    }

    /// 查询凭证列表
    pub async fn get_list(
        &self,
        params: VoucherQueryParams,
    ) -> Result<(Vec<voucher::Model>, u64), AppError> {
        info!("查询凭证列表");

        let mut query = voucher::Entity::find();

        if let Some(voucher_type) = params.voucher_type {
            query = query.filter(voucher::Column::VoucherType.eq(voucher_type));
        }

        if let Some(status) = params.status {
            query = query.filter(voucher::Column::Status.eq(status));
        }

        if let Some(start_date) = params.start_date {
            query = query.filter(voucher::Column::VoucherDate.gte(start_date));
        }

        if let Some(end_date) = params.end_date {
            query = query.filter(voucher::Column::VoucherDate.lte(end_date));
        }

        let total = query.clone().count(&*self.db).await?;
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS
        let vouchers = query
            .order_by(voucher::Column::VoucherDate, Order::Desc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        info!("凭证列表查询成功，共 {} 条", total);
        Ok((vouchers, total))
    }

    /// 查询凭证详情
    pub async fn get_by_id(&self, id: i32) -> Result<VoucherDetail, AppError> {
        info!("查询凭证详情 ID: {}", id);

        let voucher = voucher::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(id))
            .order_by(voucher_item::Column::LineNo, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(VoucherDetail { voucher, items })
    }

    /// 更新凭证
    pub async fn update(
        &self,
        id: i32,
        req: UpdateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        info!("更新凭证 ID: {}, 操作用户: {}", id, user_id);

        let voucher_record = self.get_by_id(id).await?;
        let voucher_model = voucher_record.voucher;

        if voucher_model.status != "draft" {
            warn!("只有草稿状态的凭证可以更新：{}", voucher_model.voucher_no);
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以更新".to_string(),
            ));
        }

        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let mut active_model: voucher::ActiveModel = voucher_model.clone().into_active_model();

        if let Some(voucher_type) = req.voucher_type {
            active_model.voucher_type = sea_orm::Set(voucher_type);
        }

        if let Some(voucher_date) = req.voucher_date {
            active_model.voucher_date = sea_orm::Set(voucher_date);
        }

        let updated_voucher =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;

        if let Some(items) = req.items {
            // 验证更新后的分录借贷平衡
            let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
            let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
            if total_debit != total_credit {
                return Err(AppError::bad_request(format!(
                    "凭证借贷不平衡：借方 {} != 贷方 {}",
                    total_debit, total_credit
                )));
            }

            vi::Entity::delete_many()
                .filter(vi::Column::VoucherId.eq(id))
                .exec(&txn)
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;

            for (index, item_req) in items.iter().enumerate() {
                let item_active = vi::ActiveModel {
                    id: sea_orm::Set(0),
                    voucher_id: sea_orm::Set(id),
                    line_no: sea_orm::Set(item_req.line_no.unwrap_or((index + 1) as i32)),
                    subject_code: sea_orm::Set(item_req.subject_code.clone().unwrap_or_default()),
                    subject_name: sea_orm::Set(item_req.subject_name.clone().unwrap_or_default()),
                    debit: sea_orm::Set(item_req.debit),
                    credit: sea_orm::Set(item_req.credit),
                    summary: sea_orm::Set(item_req.summary.clone()),
                    assist_customer_id: sea_orm::Set(item_req.assist_customer_id),
                    assist_supplier_id: sea_orm::Set(item_req.assist_supplier_id),
                    assist_department_id: sea_orm::Set(item_req.assist_department_id),
                    assist_employee_id: sea_orm::Set(item_req.assist_employee_id),
                    assist_project_id: sea_orm::Set(item_req.assist_project_id),
                    assist_batch_id: sea_orm::Set(item_req.assist_batch_id),
                    assist_color_no_id: sea_orm::Set(item_req.assist_color_no_id),
                    assist_dye_lot_id: sea_orm::Set(item_req.assist_dye_lot_id),
                    assist_grade: sea_orm::Set(item_req.assist_grade.clone()),
                    assist_workshop_id: sea_orm::Set(item_req.assist_workshop_id),
                    quantity_meters: sea_orm::Set(item_req.quantity_meters),
                    quantity_kg: sea_orm::Set(item_req.quantity_kg),
                    unit_price: sea_orm::Set(item_req.unit_price),
                    created_at: sea_orm::Set(chrono::Utc::now()),
                };
                item_active
                    .insert(&txn)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        info!("凭证更新成功：no={}", updated_voucher.voucher_no);
        Ok(updated_voucher)
    }

    /// 删除凭证
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("删除凭证 ID: {}", id);

        let voucher = self.get_by_id(id).await?.voucher;

        // 只有草稿状态可以删除
        if voucher.status != "draft" {
            warn!("只有草稿状态的凭证可以删除：{}", voucher.voucher_no);
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以删除".to_string(),
            ));
        }

        // 保留凭证号用于日志
        let voucher_no = voucher.voucher_no.clone();

        // 删除分录（CASCADE 会自动删除）
        let _ = voucher.delete(&*self.db).await?;

        info!("凭证删除成功：no={}", voucher_no);
        Ok(())
    }

    /// 提交凭证
    // 批次 24 v6 P1-4 修复：将 _user_id 改为 user_id，传入审计日志追溯操作人。
    // 原签名用下划线前缀表示未使用，导致审计日志 user_id 硬编码为 0 无法追溯。
    pub async fn submit(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("提交凭证 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发提交可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != "draft" {
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以提交".to_string(),
            ));
        }

        // 提交前验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set("submitted".to_string());

        // 批次 11（2026-06-28）：事务包裹"凭证状态更新 + 审计日志"，保证原子性
        // 原 update_with_audit(&*self.db, ...) 内部 2 次独立写入非原子，
        // 审计插入失败会导致"凭证已提交但审计缺失"
        // 批次 24 v6 P1-4 修复：user_id 从 0 改为传入的真实值
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;

        info!("凭证提交成功：no={}", updated.voucher_no);
        Ok(updated)
    }

    /// 审核凭证
    pub async fn review(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("审核凭证 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发审核可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != "submitted" {
            return Err(AppError::bad_request("只有已提交的凭证可以审核"));
        }

        // 验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set("reviewed".to_string());
        active_model.reviewed_by = sea_orm::Set(Some(user_id));
        active_model.reviewed_at = sea_orm::Set(Some(chrono::Utc::now()));

        // 批次 11（2026-06-28）：事务包裹"凭证审核状态更新 + 审计日志"，保证原子性
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;
        txn.commit().await?;

        info!("凭证审核成功：no={}", updated.voucher_no);
        Ok(updated)
    }

    /// 凭证过账（核心功能）
    pub async fn post(&self, id: i32, user_id: i32) -> Result<voucher::Model, AppError> {
        info!("凭证过账 ID: {}", id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_by_id 裸查询，未加行锁，并发过账可能双写状态。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = (*self.db).begin().await?;
        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher.status != "reviewed" {
            return Err(AppError::bad_request("只有已审核的凭证可以过账"));
        }

        // 检查期间锁定
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc.check_date_locked(voucher.voucher_date).await?;

        // 1. 验证凭证
        self.validate_voucher_in_transaction(id, &txn).await?;

        // 2. 更新科目余额
        self.update_account_balances(id, &txn).await?;

        // 3. 更新凭证状态
        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set("posted".to_string());
        active_model.posted_by = sea_orm::Set(Some(user_id));
        active_model.posted_at = sea_orm::Set(Some(chrono::Utc::now()));
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        info!("凭证过账成功：no={}", updated.voucher_no);

        // 触发财务指标更新事件
        let period = format!(
            "{:04}-{:02}",
            updated.voucher_date.year(),
            updated.voucher_date.month()
        );
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source: format!("voucher_posted:{}", updated.voucher_no),
            },
        );

        Ok(updated)
    }

    /// 验证凭证（借贷平衡）
    async fn validate_voucher(&self, id: i32) -> Result<(), AppError> {
        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(id))
            .all(&*self.db)
            .await?;

        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();

        if total_debit != total_credit {
            return Err(AppError::bad_request(format!(
                "凭证借贷不平衡：借方 {} != 贷方 {}",
                total_debit, total_credit
            )));
        }

        Ok(())
    }

    /// 验证凭证（事务内）
    async fn validate_voucher_in_transaction(
        &self,
        id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(id))
            .all(txn)
            .await?;

        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();

        if total_debit != total_credit {
            return Err(AppError::bad_request("凭证借贷不平衡"));
        }

        Ok(())
    }

    /// 更新科目余额（核心逻辑）
    /// 根据会计制度正确计算期末余额
    /// - 借方科目：期末余额 = 期初余额(借) + 本期借方发生 - 本期贷方发生
    /// - 贷方科目：期末余额 = 期初余额(贷) + 本期贷方发生 - 本期借方发生
    async fn update_account_balances(
        &self,
        voucher_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        info!("更新科目余额 voucher_id={}", voucher_id);

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

        // 提取会计期间
        let period = format!(
            "{:04}-{:02}",
            voucher.voucher_date.year(),
            voucher.voucher_date.month()
        );

        // 按科目分组汇总借贷发生额
        use std::collections::HashMap;
        let mut balance_map: HashMap<i32, (Decimal, Decimal)> = HashMap::new();

        for item in &items {
            // 查找科目 ID 和余额方向
            use crate::models::account_subject;
            let subject_code = &item.subject_code;
            let subject = account_subject::Entity::find()
                .filter(account_subject::Column::Code.eq(subject_code))
                .one(txn)
                .await?
                .ok_or_else(|| AppError::bad_request(format!("科目不存在：{}", subject_code)))?;

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

        // 更新或创建余额记录
        use crate::models::account_balance;
        for (subject_id, (debit_amount, credit_amount)) in balance_map {
            // 获取科目信息以确定余额方向
            let subject = account_subject::Entity::find_by_id(subject_id)
                .one(txn)
                .await?
                .ok_or_else(|| AppError::bad_request(format!("科目不存在：{}", subject_id)))?;

            let balance_direction = subject.balance_direction.as_deref().unwrap_or("借");

            // 使用 SELECT FOR UPDATE 锁定余额记录，防止并发更新
            let existing = account_balance::Entity::find()
                .filter(account_balance::Column::SubjectId.eq(subject_id))
                .filter(account_balance::Column::Period.eq(&period))
                .lock(sea_orm::sea_query::LockType::Update)
                .one(txn)
                .await?;

            if let Some(balance) = existing {
                // 更新现有余额
                let mut active_model: account_balance::ActiveModel = balance.into_active_model();
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

                // 根据余额方向计算期末余额
                // 借方科目：期末余额 = 期初借方 + 本期借方发生 - 本期贷方发生
                // 贷方科目：期末余额 = 期初贷方 + 本期贷方发生 - 本期借方发生
                if balance_direction == "借" {
                    let ending_balance = initial_debit + new_period_debit - new_period_credit;
                    if ending_balance >= Decimal::ZERO {
                        active_model.ending_balance_debit = sea_orm::Set(ending_balance);
                        active_model.ending_balance_credit = sea_orm::Set(Decimal::ZERO);
                    } else {
                        active_model.ending_balance_debit = sea_orm::Set(Decimal::ZERO);
                        active_model.ending_balance_credit = sea_orm::Set(ending_balance.abs());
                    }
                } else {
                    let ending_balance = initial_credit + new_period_credit - new_period_debit;
                    if ending_balance >= Decimal::ZERO {
                        active_model.ending_balance_debit = sea_orm::Set(Decimal::ZERO);
                        active_model.ending_balance_credit = sea_orm::Set(ending_balance);
                    } else {
                        active_model.ending_balance_debit = sea_orm::Set(ending_balance.abs());
                        active_model.ending_balance_credit = sea_orm::Set(Decimal::ZERO);
                    }
                }

                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    active_model,
                    Some(0),
                )
                .await?;
            } else {
                // 创建新余额记录
                // 根据余额方向设置期末余额
                let (ending_debit, ending_credit) = if balance_direction == "借" {
                    // 借方科目新账：期末余额 = 本期借方 - 本期贷方
                    let net = debit_amount - credit_amount;
                    if net >= Decimal::ZERO {
                        (net, Decimal::ZERO)
                    } else {
                        // 借方科目出现贷方余额，记录在贷方
                        (Decimal::ZERO, net.abs())
                    }
                } else {
                    // 贷方科目新账：期末余额 = 本期贷方 - 本期借方
                    let net = credit_amount - debit_amount;
                    if net >= Decimal::ZERO {
                        (Decimal::ZERO, net)
                    } else {
                        // 贷方科目出现借方余额，记录在借方
                        (net.abs(), Decimal::ZERO)
                    }
                };

                let active_model = account_balance::ActiveModel {
                    subject_id: sea_orm::Set(subject_id),
                    period: sea_orm::Set(period.clone()),
                    current_period_debit: sea_orm::Set(debit_amount),
                    current_period_credit: sea_orm::Set(credit_amount),
                    initial_balance_debit: sea_orm::Set(Decimal::ZERO),
                    initial_balance_credit: sea_orm::Set(Decimal::ZERO),
                    ending_balance_debit: sea_orm::Set(ending_debit),
                    ending_balance_credit: sea_orm::Set(ending_credit),
                    ..Default::default()
                };
                active_model.insert(txn).await?;
            }
        }

        info!("科目余额更新成功");
        Ok(())
    }

    /// 生成凭证编号
    async fn generate_voucher_no(
        &self,
        voucher_type: &str,
        _voucher_date: chrono::NaiveDate,
    ) -> Result<String, AppError> {
        let prefix = match voucher_type {
            "记" => "JZ",
            "收" => "SK",
            "付" => "FK",
            "转" => "ZZ",
            _ => "JZ",
        };

        DocumentNumberGenerator::generate_no(
            &*self.db,
            prefix,
            voucher::Entity,
            voucher::Column::VoucherNo,
        )
        .await
    }
}

/// 凭证详情（包含分录）
#[allow(dead_code)] // TODO(tech-debt): 预留 API，待凭证详情查询接入后移除
#[derive(Debug, Clone)]
pub struct VoucherDetail {
    pub voucher: voucher::Model,
    pub items: Vec<voucher_item::Model>,
}
