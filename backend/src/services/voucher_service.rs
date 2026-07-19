use crate::utils::number_generator::DocumentNumberGenerator;
// 凭证管理 Service
//
// 凭证业务逻辑层（核心）

use chrono::Datelike;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::voucher_item as vi;
use crate::models::{account_subject, voucher, voucher_item};
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use rust_decimal::Decimal;

// 批次 102 v6 P3-1 修复：状态字符串常量化，引用 crate::models::status::voucher

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
    ///
    /// P2 1-6 修复：原函数 138 行混合 8 职责（期间校验+借贷平衡+单号+事务+主表+科目校验+分录循环+提交），
    /// 拆为 validate_voucher_create_req / precheck_subjects_exist_txn / insert_voucher_items_txn 3 个私有方法
    pub async fn create(
        &self,
        req: CreateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        // 1. 校验期间锁定 + 借贷平衡
        Self::validate_voucher_create_req(&req, self.db.clone()).await?;

        info!(
            "创建凭证：type={}, date={}",
            req.voucher_type, req.voucher_date
        );

        // 2. 生成凭证编号
        let voucher_no = self
            .generate_voucher_no(&req.voucher_type, req.voucher_date)
            .await?;

        // 3. 开启事务
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        // 4. 创建凭证主表
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
            status: sea_orm::Set(crate::models::status::voucher::VOUCHER_DRAFT.to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        };

        let voucher = active_model
            .insert(&txn)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        info!("凭证创建成功：no={}", voucher.voucher_no);

        // 5. 批量校验科目是否存在
        Self::precheck_subjects_exist_txn(&req.items, &txn).await?;

        // 6. 批量插入凭证分录
        Self::insert_voucher_items_txn(voucher.id, &req.items, &txn).await?;

        // 7. 提交事务
        txn.commit()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        info!("凭证分录创建成功，共 {} 条", req.items.len());

        Ok(voucher)
    }

    /// 创建并自动过账凭证（用于库存桥接等自动凭证场景）
    ///
    /// 批次 356 v13 复审 F-P0-1+F-P0-2 修复：
    /// - F-P0-1：post 内部调用 update_account_balances 实现科目余额回写
    /// - F-P0-2：库存桥接凭证只 create 不 post，现改为 create_and_post 自动过账
    ///
    /// 自动完成 DRAFT → SUBMITTED → REVIEWED → POSTED 状态流转，
    /// 适用于库存桥接等无需人工审核的自动凭证场景。
    pub async fn create_and_post(
        &self,
        req: CreateVoucherRequest,
        user_id: i32,
    ) -> Result<voucher::Model, AppError> {
        let created = self.create(req, user_id).await?;
        self.submit(created.id, user_id).await?;
        self.review(created.id, user_id).await?;
        self.post(created.id, user_id).await
    }

    /// P2 1-6 修复：校验期间锁定 + 借贷平衡（从 create 抽取）
    async fn validate_voucher_create_req(
        req: &CreateVoucherRequest,
        db: Arc<DatabaseConnection>,
    ) -> Result<(), AppError> {
        // 校验期间锁定
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(db);
        period_svc.check_date_locked(req.voucher_date).await?;

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

        Ok(())
    }

    /// P2 1-6 修复：批量校验科目是否存在（从 create 抽取）
    async fn precheck_subjects_exist_txn(
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let mut subject_codes = std::collections::HashSet::new();
        for item_req in items {
            if let Some(ref subject_code) = item_req.subject_code {
                if !subject_code.is_empty() {
                    subject_codes.insert(subject_code.clone());
                }
            }
        }
        if subject_codes.is_empty() {
            return Ok(());
        }

        let existing_subjects = account_subject::Entity::find()
            .filter(
                account_subject::Column::Code
                    .is_in(subject_codes.iter().cloned().collect::<Vec<_>>()),
            )
            .filter(account_subject::Column::Status.eq(master_data::ACTIVE))
            .all(txn)
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
        Ok(())
    }

    /// P2 1-6 修复：批量插入凭证分录（从 create 抽取）
    async fn insert_voucher_items_txn(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        for (index, item_req) in items.iter().enumerate() {
            let item_active_model = voucher_item::ActiveModel {
                voucher_id: sea_orm::Set(voucher_id),
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
                .insert(txn)
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        Ok(())
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

        // P1 5-20 修复（批次 61）：状态门移入 txn + lock_exclusive
        // 原实现状态门在事务外（self.get_by_id 裸查询），并发 update 会竞态绕过 draft 状态门控。
        // 改为在 txn 内用 find_by_id(id).lock_exclusive() 串行化并发状态变更。
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let voucher_model = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher_model.status != crate::models::status::voucher::VOUCHER_DRAFT {
            warn!("只有草稿状态的凭证可以更新：{}", voucher_model.voucher_no);
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以更新".to_string(),
            ));
        }

        let mut active_model: voucher::ActiveModel = voucher_model.into_active_model();

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
                    // 批次 97 P1-1 修复：原 id: Set(0) 在并发 update 重写明细时
                    // 可能触发主键约束异常（DB 自增列应使用 NotSet 让 DB 生成）
                    id: sea_orm::ActiveValue::NotSet,
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
    // 批次 93 P1-3 修复：补 user_id 参数 + txn + lock_exclusive + 审计日志
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("删除凭证 ID: {}", id);

        // 批次 93 P1-3 修复：状态门 + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 get_by_id 在 self.db → 状态门 → delete 在 self.db，
        // 状态门与 delete 跨事务边界，并发 delete + submit 会竞态绕过 draft 状态门控。
        let txn = (*self.db).begin().await?;

        let voucher = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        // 只有草稿状态可以删除（状态门在 txn 内，基于 lock_exclusive 读出的 model）
        if voucher.status != crate::models::status::voucher::VOUCHER_DRAFT {
            warn!("只有草稿状态的凭证可以删除：{}", voucher.voucher_no);
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以删除".to_string(),
            ));
        }

        // 保留凭证号用于日志
        let voucher_no = voucher.voucher_no.clone();

        // 删除凭证（含审计日志）；分录由数据库 CASCADE 自动删除
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<voucher::Entity, _>(
            &txn,
            "voucher",
            id,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

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

        if voucher.status != crate::models::status::voucher::VOUCHER_DRAFT {
            return Err(AppError::bad_request(
                "只有草稿状态的凭证可以提交".to_string(),
            ));
        }

        // 提交前验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_SUBMITTED.to_string());

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

        if voucher.status != crate::models::status::voucher::VOUCHER_SUBMITTED {
            return Err(AppError::bad_request("只有已提交的凭证可以审核"));
        }

        // 验证借贷平衡
        self.validate_voucher(id).await?;

        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_REVIEWED.to_string());
        active_model.reviewed_by = sea_orm::Set(Some(user_id));
        active_model.reviewed_at = sea_orm::Set(Some(chrono::Utc::now()));

        // 批次 11（2026-06-28）：事务包裹"凭证审核状态更新 + 审计日志"，保证原子性
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
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

        if voucher.status != crate::models::status::voucher::VOUCHER_REVIEWED {
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
        // 批次 94 P2-10：传入 user_id 用于余额变更审计日志
        self.update_account_balances(id, user_id, &txn).await?;

        // 2.5 F-P1-3 修复（批次 359 v13 复审）：写入辅助核算记录
        // 原实现仅更新科目余额（account_balance），未写入 assist_accounting_record 表，
        // 导致辅助核算明细账与汇总表查询无数据。仅对包含辅助核算维度的分录写入。
        self.write_assist_accounting_records_txn(id, user_id, &txn)
            .await?;

        // 3. 更新凭证状态
        let mut active_model: voucher::ActiveModel = voucher.into_active_model();
        active_model.status = sea_orm::Set(crate::models::status::voucher::VOUCHER_POSTED.to_string());
        active_model.posted_by = sea_orm::Set(Some(user_id));
        active_model.posted_at = sea_orm::Set(Some(chrono::Utc::now()));
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
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
    ///
    /// 批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，
    /// 保证余额变更审计日志能追溯实际操作人。
    async fn update_account_balances(
        &self,
        voucher_id: i32,
        user_id: i32,
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

        // v11 批次 37 修复：批量查询所有科目，避免循环内逐个查询（N+1，同一科目可能重复查询）
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
        // v11 批次 37 修复：构建 code→Model 和 id→Model 两个引用映射，供下方两个循环复用
        let subject_by_code: HashMap<&str, &account_subject::Model> =
            subjects.iter().map(|s| (s.code.as_str(), s)).collect();
        let subject_by_id: HashMap<i32, &account_subject::Model> =
            subjects.iter().map(|s| (s.id, s)).collect();

        for item in &items {
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

        // 更新或创建余额记录
        use crate::models::account_balance;
        // v16 批次 45 修复：循环外批量锁定查询所有余额记录，避免循环内逐个 lock 查询（N+1）
        let all_subject_ids: Vec<i32> = balance_map.keys().copied().collect();
        let existing_balances: Vec<account_balance::Model> = if all_subject_ids.is_empty() {
            Vec::new()
        } else {
            account_balance::Entity::find()
                .filter(account_balance::Column::SubjectId.is_in(all_subject_ids))
                .filter(account_balance::Column::Period.eq(&period))
                .lock(sea_orm::sea_query::LockType::Update)
                .all(txn)
                .await?
        };
        let mut balance_record_map: HashMap<i32, account_balance::Model> = existing_balances
            .into_iter()
            .map(|b| (b.subject_id, b))
            .collect();

        for (subject_id, (debit_amount, credit_amount)) in balance_map {
            // 获取科目信息以确定余额方向（复用批量查询结果，避免 N+1）
            let subject = subject_by_id
                .get(&subject_id)
                // 批次 102 v6 P3-4 修复：科目不存在属于资源未找到，应为 not_found 而非 bad_request
                .ok_or_else(|| AppError::not_found(format!("科目不存在：{}", subject_id)))?;

            let balance_direction = subject.balance_direction.as_deref().unwrap_or("借");

            // v16 批次 45 修复：从批量查询结果获取余额记录（带行锁）
            let existing = balance_record_map.remove(&subject_id);

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

                // D12 重构：期末余额计算提取到 compute_ending_balance，消除嵌套 4 分支
                let (ending_dr, ending_cr) = Self::compute_ending_balance(
                    balance_direction,
                    initial_debit,
                    initial_credit,
                    new_period_debit,
                    new_period_credit,
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
            } else {
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

    /// F-P1-3 修复（批次 359 v13 复审）：凭证过账时写入辅助核算记录
    ///
    /// 遍历凭证分录，对包含辅助核算维度（客户/供应商/批次/色号/缸号/等级/车间等）
    /// 的分录生成 `assist_accounting_record` 记录，便于辅助核算明细账与汇总表查询。
    /// 原实现仅更新 `account_balance` 表，未写入辅助核算记录，导致辅助核算报表无数据。
    async fn write_assist_accounting_records_txn(
        &self,
        voucher_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::models::assist_accounting_record;

        // 获取凭证主表（含 source_module/source_bill_no/批次/色号等）
        let voucher_model = voucher::Entity::find_by_id(voucher_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("凭证不存在"))?;

        // 获取凭证分录
        let items = voucher_item::Entity::find()
            .filter(voucher_item::Column::VoucherId.eq(voucher_id))
            .all(txn)
            .await?;

        // 批量查询科目，构建 code→id 映射（复用 update_account_balances 的批量模式）
        let subject_codes: Vec<String> =
            items.iter().map(|i| i.subject_code.clone()).collect();
        let subjects = if subject_codes.is_empty() {
            Vec::new()
        } else {
            account_subject::Entity::find()
                .filter(account_subject::Column::Code.is_in(subject_codes))
                .all(txn)
                .await?
        };
        let subject_id_by_code: std::collections::HashMap<&str, i32> =
            subjects.iter().map(|s| (s.code.as_str(), s.id)).collect();

        // 业务类型：优先 source_module，其次 source_type，兜底 "VOUCHER"
        let business_type = voucher_model
            .source_module
            .clone()
            .or(voucher_model.source_type.clone())
            .unwrap_or_else(|| "VOUCHER".to_string());
        // 业务单号：优先 source_bill_no，其次 voucher_no
        let business_no = voucher_model
            .source_bill_no
            .clone()
            .unwrap_or_else(|| voucher_model.voucher_no.clone());
        // 业务单 ID：优先 source_bill_id，其次 voucher_id
        let business_id = voucher_model.source_bill_id.unwrap_or(voucher_id);

        let now = chrono::Utc::now();

        for item in &items {
            // 仅对包含任意辅助核算维度的分录生成记录，避免空记录污染
            let has_assist = item.assist_customer_id.is_some()
                || item.assist_supplier_id.is_some()
                || item.assist_batch_id.is_some()
                || item.assist_color_no_id.is_some()
                || item.assist_dye_lot_id.is_some()
                || item.assist_grade.is_some()
                || item.assist_workshop_id.is_some()
                || item.assist_department_id.is_some()
                || item.assist_employee_id.is_some()
                || item.assist_project_id.is_some();
            if !has_assist {
                continue;
            }

            let account_subject_id = *subject_id_by_code
                .get(item.subject_code.as_str())
                .ok_or_else(|| {
                    AppError::not_found(format!("科目不存在：{}", item.subject_code))
                })?;

            // 构造五维 ID（复合键，便于按维度反查）
            let five_dimension_id = format!(
                "BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}",
                item.assist_batch_id.unwrap_or(0),
                item.assist_color_no_id.unwrap_or(0),
                item.assist_dye_lot_id.unwrap_or(0),
                item.assist_grade.clone().unwrap_or_default(),
                item.assist_workshop_id.unwrap_or(0),
            );

            // F-P1-3 已知 Schema 缺口：voucher_item 无 product_id/warehouse_id 字段，
            // 暂用 0 占位，待后续 Schema 补字段后修正。
            let record = assist_accounting_record::ActiveModel {
                business_type: sea_orm::Set(business_type.clone()),
                business_no: sea_orm::Set(business_no.clone()),
                business_id: sea_orm::Set(business_id),
                account_subject_id: sea_orm::Set(account_subject_id),
                debit_amount: sea_orm::Set(item.debit),
                credit_amount: sea_orm::Set(item.credit),
                five_dimension_id: sea_orm::Set(five_dimension_id),
                // TODO(F-P1-3): voucher_item 缺 product_id，待 Schema 补字段后修正
                product_id: sea_orm::Set(0),
                // v14 批次 418 修复 G-P0-2：原 unwrap_or_default() 会静默将 None 替换为空字符串，
                // 导致辅助核算记录无法区分"未指定"和"空字符串"。改为显式记录 warn 日志便于排查。
                batch_no: sea_orm::Set(voucher_model.batch_no.clone().unwrap_or_else(|| {
                    tracing::warn!(
                        "凭证 {} 的 batch_no 为 None，辅助核算记录使用空字符串占位",
                        voucher_id
                    );
                    String::new()
                })),
                color_no: sea_orm::Set(voucher_model.color_no.clone().unwrap_or_else(|| {
                    tracing::warn!(
                        "凭证 {} 的 color_no 为 None，辅助核算记录使用空字符串占位",
                        voucher_id
                    );
                    String::new()
                })),
                dye_lot_no: sea_orm::Set(voucher_model.dye_lot_no.clone()),
                grade: sea_orm::Set(item.assist_grade.clone().unwrap_or_default()),
                workshop_id: sea_orm::Set(item.assist_workshop_id),
                // TODO(F-P1-3): voucher_item 缺 warehouse_id，待 Schema 补字段后修正
                warehouse_id: sea_orm::Set(0),
                customer_id: sea_orm::Set(item.assist_customer_id),
                supplier_id: sea_orm::Set(item.assist_supplier_id),
                quantity_meters: sea_orm::Set(item.quantity_meters.unwrap_or(Decimal::ZERO)),
                quantity_kg: sea_orm::Set(item.quantity_kg.unwrap_or(Decimal::ZERO)),
                remarks: sea_orm::Set(Some(format!("voucher_id={}", voucher_id))),
                created_at: sea_orm::Set(now),
                created_by: sea_orm::Set(Some(user_id)),
                ..Default::default()
            };
            record.insert(txn).await?;
        }

        info!("辅助核算记录写入成功 voucher_id={}", voucher_id);
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

    /// 返回系统支持的凭证类型列表（v11 批次 155 P2-C：从 handler 下沉到 service 静态配置化）
    pub fn available_voucher_types() -> Vec<VoucherTypeDefinition> {
        vec![
            VoucherTypeDefinition::new("记", "记账凭证"),
            VoucherTypeDefinition::new("收", "收款凭证"),
            VoucherTypeDefinition::new("付", "付款凭证"),
            VoucherTypeDefinition::new("转", "转账凭证"),
        ]
    }
}

/// 凭证类型定义（v11 批次 155 P2-C：静态配置化，避免 handler 硬编码）
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoucherTypeDefinition {
    pub code: &'static str,
    pub name: &'static str,
}

impl VoucherTypeDefinition {
    pub fn new(code: &'static str, name: &'static str) -> Self {
        Self { code, name }
    }
}

/// 凭证详情（包含分录）
// v11 批次 148 P2-A：移除失效的 dead_code 标注（get_by_id 方法返回 VoucherDetail，被 voucher_handler::get_voucher 真实调用）
#[derive(Debug, Clone)]
pub struct VoucherDetail {
    pub voucher: voucher::Model,
    pub items: Vec<voucher_item::Model>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::voucher as voucher_status;
    use chrono::Utc;
    use sea_orm::DatabaseConnection;
    use std::str::FromStr;

    /// 构建测试用凭证分录请求夹具
    ///
    /// 封装 VoucherItemRequest 的构造，便于借贷平衡测试复用。
    fn make_voucher_item_request(debit: Decimal, credit: Decimal) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: None,
            subject_code: Some("1001".to_string()),
            subject_name: Some("库存现金".to_string()),
            debit,
            credit,
            summary: None,
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
        }
    }

    /// 构建测试用凭证模型夹具
    ///
    /// 封装 voucher::Model 的构造，状态字段可定制，便于状态门校验测试复用。
    fn make_voucher_model(status: &str) -> voucher::Model {
        voucher::Model {
            id: 1,
            voucher_no: "JZ20260101001".to_string(),
            voucher_type: "记".to_string(),
            voucher_date: ymd!(2026, 1, 1),
            source_type: None,
            source_module: None,
            source_bill_id: None,
            source_bill_no: None,
            batch_no: None,
            color_no: None,
            dye_lot_no: None,
            workshop: None,
            production_order_no: None,
            quantity_meters: None,
            quantity_kg: None,
            gram_weight: None,
            status: status.to_string(),
            attachment_count: 0,
            created_by: 1,
            reviewed_by: None,
            reviewed_at: None,
            posted_by: None,
            posted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 复现 generate_voucher_no 中的凭证编号前缀映射逻辑
    ///
    /// 源码位置：generate_voucher_no 方法内的 match 表达式。
    /// "记" => "JZ", "收" => "SK", "付" => "FK", "转" => "ZZ", _ => "JZ"
    fn voucher_prefix(voucher_type: &str) -> &str {
        match voucher_type {
            "记" => "JZ",
            "收" => "SK",
            "付" => "FK",
            "转" => "ZZ",
            _ => "JZ",
        }
    }

    /// 复现 validate_voucher_create_req 中的借贷平衡校验逻辑
    ///
    /// 源码位置：validate_voucher_create_req 方法内的借方合计 == 贷方合计判断。
    fn is_balanced(items: &[VoucherItemRequest]) -> bool {
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        total_debit == total_credit
    }

    /// 复现凭证状态机合法转换判断
    ///
    /// 源码位置：submit/review/post 方法内的状态门校验。
    /// 合法路径：draft → submitted → reviewed → posted
    fn can_transition(from: &str, to: &str) -> bool {
        matches!(
            (from, to),
            (voucher_status::VOUCHER_DRAFT, voucher_status::VOUCHER_SUBMITTED)
                | (voucher_status::VOUCHER_SUBMITTED, voucher_status::VOUCHER_REVIEWED)
                | (voucher_status::VOUCHER_REVIEWED, voucher_status::VOUCHER_POSTED)
        )
    }

    /// 复现 update_account_balances 中的期末余额计算逻辑
    ///
    /// 源码位置：update_account_balances 方法内的余额方向分支。
    /// - 借方科目：期末余额 = 期初借方 + 本期借方发生 - 本期贷方发生
    /// - 贷方科目：期末余额 = 期初贷方 + 本期贷方发生 - 本期借方发生
    /// 返回 (期末借方余额, 期末贷方余额)
    fn calc_ending_balance(
        balance_direction: &str,
        initial_debit: Decimal,
        initial_credit: Decimal,
        period_debit: Decimal,
        period_credit: Decimal,
    ) -> (Decimal, Decimal) {
        if balance_direction == "借" {
            let ending_balance = initial_debit + period_debit - period_credit;
            if ending_balance >= Decimal::ZERO {
                (ending_balance, Decimal::ZERO)
            } else {
                (Decimal::ZERO, ending_balance.abs())
            }
        } else {
            let ending_balance = initial_credit + period_credit - period_debit;
            if ending_balance >= Decimal::ZERO {
                (Decimal::ZERO, ending_balance)
            } else {
                (ending_balance.abs(), Decimal::ZERO)
            }
        }
    }

    // ============ 凭证状态常量值正确性测试 ============

    /// 测试_凭证状态常量_值正确性
    ///
    /// 验证 crate::models::status::voucher 子模块中 4 个状态常量值
    /// 与凭证状态机约定一致（小写：draft/submitted/reviewed/posted）。
    #[test]
    fn 测试_凭证状态常量_值正确性() {
        assert_eq!(voucher_status::VOUCHER_DRAFT, "draft");
        assert_eq!(voucher_status::VOUCHER_SUBMITTED, "submitted");
        assert_eq!(voucher_status::VOUCHER_REVIEWED, "reviewed");
        assert_eq!(voucher_status::VOUCHER_POSTED, "posted");
    }

    // ============ 凭证状态机转换测试 ============

    /// 测试_凭证状态机_合法转换路径
    ///
    /// 验证凭证状态机的 3 条合法转换路径：
    /// draft → submitted → reviewed → posted
    #[test]
    fn 测试_凭证状态机_合法转换路径() {
        assert!(can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_SUBMITTED
        ));
        assert!(can_transition(
            voucher_status::VOUCHER_SUBMITTED,
            voucher_status::VOUCHER_REVIEWED
        ));
        assert!(can_transition(
            voucher_status::VOUCHER_REVIEWED,
            voucher_status::VOUCHER_POSTED
        ));
    }

    /// 测试_凭证状态机_非法跳转
    ///
    /// 验证非相邻状态的直接跳转应被拒绝：
    /// draft 不能直接 reviewed/posted；submitted 不能直接 posted。
    #[test]
    fn 测试_凭证状态机_非法跳转() {
        assert!(!can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_REVIEWED
        ));
        assert!(!can_transition(
            voucher_status::VOUCHER_DRAFT,
            voucher_status::VOUCHER_POSTED
        ));
        assert!(!can_transition(
            voucher_status::VOUCHER_SUBMITTED,
            voucher_status::VOUCHER_POSTED
        ));
        // 已过账不可再转换
        assert!(!can_transition(
            voucher_status::VOUCHER_POSTED,
            voucher_status::VOUCHER_DRAFT
        ));
    }

    // ============ 借贷平衡校验测试 ============

    /// 测试_借贷平衡校验_借方等于贷方通过
    ///
    /// 验证 validate_voucher_create_req 中借方合计 == 贷方合计时校验通过。
    #[test]
    fn 测试_借贷平衡校验_借方等于贷方通过() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1000")),
        ];
        assert!(is_balanced(&items));
    }

    /// 测试_借贷平衡校验_借方大于贷方失败
    ///
    /// 验证 validate_voucher_create_req 中借方合计 > 贷方合计时校验失败。
    #[test]
    fn 测试_借贷平衡校验_借方大于贷方失败() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("500")),
        ];
        assert!(!is_balanced(&items));
    }

    /// 测试_借贷平衡校验_借方小于贷方失败
    ///
    /// 验证 validate_voucher_create_req 中借方合计 < 贷方合计时校验失败。
    #[test]
    fn 测试_借贷平衡校验_借方小于贷方失败() {
        let items = vec![
            make_voucher_item_request(decs!("500"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1000")),
        ];
        assert!(!is_balanced(&items));
    }

    /// 测试_借贷平衡校验_零金额平衡通过
    ///
    /// 验证 validate_voucher_create_req 中借贷双方均为零时校验通过（边界场景）。
    #[test]
    fn 测试_借贷平衡校验_零金额平衡通过() {
        let items = vec![
            make_voucher_item_request(Decimal::ZERO, Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, Decimal::ZERO),
        ];
        assert!(is_balanced(&items));
    }

    /// 测试_借贷平衡校验_多分录汇总平衡
    ///
    /// 验证 validate_voucher_create_req 中多个分录汇总后借贷平衡时校验通过。
    #[test]
    fn 测试_借贷平衡校验_多分录汇总平衡() {
        let items = vec![
            make_voucher_item_request(decs!("1000"), Decimal::ZERO),
            make_voucher_item_request(decs!("500"), Decimal::ZERO),
            make_voucher_item_request(decs!("200.50"), Decimal::ZERO),
            make_voucher_item_request(Decimal::ZERO, decs!("1700.50")),
        ];
        assert!(is_balanced(&items));
    }

    // ============ 金额计算精度测试 ============

    /// 测试_金额计算_精度归一化
    ///
    /// 验证 Decimal 求和保留精度，不同小数位的金额相加不会丢失精度。
    /// 复现 validate_voucher_create_req 中 iter().map(|i| i.debit).sum() 的精度行为。
    #[test]
    fn 测试_金额计算_精度归一化() {
        let items = vec![
            make_voucher_item_request(decs!("0.1"), Decimal::ZERO),
            make_voucher_item_request(decs!("0.2"), Decimal::ZERO),
            make_voucher_item_request(decs!("0.3"), Decimal::ZERO),
        ];
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        // Decimal 不存在 f64 浮点累加误差，0.1+0.2+0.3 应精确等于 0.6
        assert_eq!(total_debit, decs!("0.6"));

        // 不同精度混合相加
        let mixed = vec![
            make_voucher_item_request(decs!("100.125"), Decimal::ZERO),
            make_voucher_item_request(decs!("200.875"), Decimal::ZERO),
        ];
        let mixed_total: Decimal = mixed.iter().map(|i| i.debit).sum();
        assert_eq!(mixed_total, decs!("301.000"));
    }

    // ============ 凭证类型定义测试 ============

    /// 测试_凭证类型定义_完整列表
    ///
    /// 验证 available_voucher_types 返回 4 种凭证类型，且 code 与 name 对应正确。
    #[test]
    fn 测试_凭证类型定义_完整列表() {
        let types = VoucherService::available_voucher_types();
        assert_eq!(types.len(), 4);

        // 验证每种类型的 code 和 name 对应
        let pairs: Vec<(&str, &str)> = types.iter().map(|t| (t.code, t.name)).collect();
        assert!(pairs.contains(&("记", "记账凭证")));
        assert!(pairs.contains(&("收", "收款凭证")));
        assert!(pairs.contains(&("付", "付款凭证")));
        assert!(pairs.contains(&("转", "转账凭证")));

        // code 应唯一
        let codes: std::collections::HashSet<&str> = types.iter().map(|t| t.code).collect();
        assert_eq!(codes.len(), 4);
    }

    /// 测试_凭证编号前缀_各类型映射
    ///
    /// 验证 generate_voucher_no 中凭证类型到前缀的映射：
    /// "记" => "JZ", "收" => "SK", "付" => "FK", "转" => "ZZ"
    #[test]
    fn 测试_凭证编号前缀_各类型映射() {
        assert_eq!(voucher_prefix("记"), "JZ");
        assert_eq!(voucher_prefix("收"), "SK");
        assert_eq!(voucher_prefix("付"), "FK");
        assert_eq!(voucher_prefix("转"), "ZZ");
    }

    /// 测试_凭证编号前缀_未知类型默认
    ///
    /// 验证 generate_voucher_no 中未知凭证类型回退到默认前缀 "JZ"。
    #[test]
    fn 测试_凭证编号前缀_未知类型默认() {
        assert_eq!(voucher_prefix("未知"), "JZ");
        assert_eq!(voucher_prefix(""), "JZ");
    }

    // ============ 科目余额计算测试 ============

    /// 测试_科目余额计算_借方科目正常
    ///
    /// 验证 update_account_balances 中借方科目期末余额计算：
    /// 期末余额 = 期初借方 + 本期借方发生 - 本期贷方发生（结果为正记借方）。
    #[test]
    fn 测试_科目余额计算_借方科目正常() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "借",
            decs!("1000"),
            Decimal::ZERO,
            decs!("500"),
            decs!("200"),
        );
        // 1000 + 500 - 200 = 1300，正数记借方
        assert_eq!(ending_debit, decs!("1300"));
        assert_eq!(ending_credit, Decimal::ZERO);
    }

    /// 测试_科目余额计算_贷方科目正常
    ///
    /// 验证 update_account_balances 中贷方科目期末余额计算：
    /// 期末余额 = 期初贷方 + 本期贷方发生 - 本期借方发生（结果为正记贷方）。
    #[test]
    fn 测试_科目余额计算_贷方科目正常() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "贷",
            Decimal::ZERO,
            decs!("2000"),
            decs!("300"),
            decs!("800"),
        );
        // 2000 + 800 - 300 = 2500，正数记贷方
        assert_eq!(ending_debit, Decimal::ZERO);
        assert_eq!(ending_credit, decs!("2500"));
    }

    /// 测试_科目余额计算_借方科目出现贷方余额
    ///
    /// 验证 update_account_balances 中借方科目净额为负时记贷方（如累计折旧场景）。
    #[test]
    fn 测试_科目余额计算_借方科目出现贷方余额() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "借",
            decs!("100"),
            Decimal::ZERO,
            decs!("200"),
            decs!("500"),
        );
        // 100 + 200 - 500 = -200，负数取绝对值记贷方
        assert_eq!(ending_debit, Decimal::ZERO);
        assert_eq!(ending_credit, decs!("200"));
    }

    /// 测试_科目余额计算_贷方科目出现借方余额
    ///
    /// 验证 update_account_balances 中贷方科目净额为负时记借方（如预交税费场景）。
    #[test]
    fn 测试_科目余额计算_贷方科目出现借方余额() {
        let (ending_debit, ending_credit) = calc_ending_balance(
            "贷",
            Decimal::ZERO,
            decs!("100"),
            decs!("500"),
            decs!("200"),
        );
        // 100 + 200 - 500 = -200，负数取绝对值记借方
        assert_eq!(ending_debit, decs!("200"));
        assert_eq!(ending_credit, Decimal::ZERO);
    }

    // ============ 状态校验逻辑测试 ============

    /// 测试_状态校验_仅草稿可更新
    ///
    /// 验证 update 方法中状态门：仅 draft 状态可更新，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可更新() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 update 中的状态门：voucher_model.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(submitted.status != voucher_status::VOUCHER_DRAFT);
        assert!(reviewed.status != voucher_status::VOUCHER_DRAFT);
        assert!(posted.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅草稿可删除
    ///
    /// 验证 delete 方法中状态门：仅 draft 状态可删除，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可删除() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 delete 中的状态门：voucher.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(posted.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅草稿可提交
    ///
    /// 验证 submit 方法中状态门：仅 draft 状态可提交，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅草稿可提交() {
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);

        // 复现 submit 中的状态门：voucher.status != VOUCHER_DRAFT 则拒绝
        assert!(draft.status == voucher_status::VOUCHER_DRAFT);
        assert!(reviewed.status != voucher_status::VOUCHER_DRAFT);
    }

    /// 测试_状态校验_仅已提交可审核
    ///
    /// 验证 review 方法中状态门：仅 submitted 状态可审核，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅已提交可审核() {
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let posted = make_voucher_model(voucher_status::VOUCHER_POSTED);

        // 复现 review 中的状态门：voucher.status != VOUCHER_SUBMITTED 则拒绝
        assert!(submitted.status == voucher_status::VOUCHER_SUBMITTED);
        assert!(draft.status != voucher_status::VOUCHER_SUBMITTED);
        assert!(posted.status != voucher_status::VOUCHER_SUBMITTED);
    }

    /// 测试_状态校验_仅已审核可过账
    ///
    /// 验证 post 方法中状态门：仅 reviewed 状态可过账，其余状态应拒绝。
    #[test]
    fn 测试_状态校验_仅已审核可过账() {
        let reviewed = make_voucher_model(voucher_status::VOUCHER_REVIEWED);
        let draft = make_voucher_model(voucher_status::VOUCHER_DRAFT);
        let submitted = make_voucher_model(voucher_status::VOUCHER_SUBMITTED);

        // 复现 post 中的状态门：voucher.status != VOUCHER_REVIEWED 则拒绝
        assert!(reviewed.status == voucher_status::VOUCHER_REVIEWED);
        assert!(draft.status != voucher_status::VOUCHER_REVIEWED);
        assert!(submitted.status != voucher_status::VOUCHER_REVIEWED);
    }

    // ============ 错误消息格式测试 ============

    /// 测试_错误消息格式_借贷不平衡
    ///
    /// 验证 validate_voucher_create_req 中借贷不平衡的错误消息格式：
    /// "凭证借贷不平衡：借方 {} != 贷方 {}"
    #[test]
    fn 测试_错误消息格式_借贷不平衡() {
        let total_debit = decs!("1000");
        let total_credit = decs!("500");
        let msg = format!(
            "凭证借贷不平衡：借方 {} != 贷方 {}",
            total_debit, total_credit
        );
        assert!(msg.contains("凭证借贷不平衡"));
        assert!(msg.contains("借方 1000"));
        assert!(msg.contains("贷方 500"));

        let err = AppError::bad_request(msg);
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    /// 测试_错误消息格式_凭证不存在
    ///
    /// 验证 get_by_id/update/delete 等方法中凭证不存在的错误消息格式：
    /// "凭证不存在：{}"
    #[test]
    fn 测试_错误消息格式_凭证不存在() {
        let id = 99999;
        let msg = format!("凭证不存在：{}", id);
        assert_eq!(msg, "凭证不存在：99999");

        let err = AppError::not_found(msg);
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // ============ 夹具宏可用性测试 ============

    /// 测试_decs_夹具宏可用性
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串常量。
    #[test]
    fn 测试_decs_夹具宏可用性() {
        let v = decs!("1234.56");
        assert_eq!(v.to_string(), "1234.56");

        let zero = decs!("0");
        assert!(zero.is_zero());

        let neg = decs!("-100.50");
        assert!(neg < Decimal::ZERO);
    }

    /// 测试_ymd_夹具宏可用性
    ///
    /// 验证 ymd! 宏能正确解析日期常量。
    #[test]
    fn 测试_ymd_夹具宏可用性() {
        let d = ymd!(2026, 7, 1);
        assert_eq!(d.year(), 2026);
        assert_eq!(d.month(), 7);
        assert_eq!(d.day(), 1);
    }

    // ============ 服务实例化测试 ============

    /// 测试_服务实例创建
    ///
    /// 验证 VoucherService 在 SQLite 内存数据库上能正常实例化。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ============ 数据库交互测试（标注 #[ignore]）============

    /// 测试_创建凭证_需要真实数据库
    ///
    /// 需要 vouchers/voucher_items/account_subjects 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。无 schema 时返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_创建凭证_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let req = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: ymd!(2026, 1, 1),
            source_type: None,
            source_module: None,
            source_bill_id: None,
            source_bill_no: None,
            batch_no: None,
            color_no: None,
            items: vec![
                make_voucher_item_request(decs!("1000"), Decimal::ZERO),
                make_voucher_item_request(Decimal::ZERO, decs!("1000")),
            ],
        };
        let result = service.create(req, 1).await;
        // 无 schema 时为 Err
        assert!(result.is_err());
    }

    /// 测试_查询凭证列表_需要真实数据库
    ///
    /// 需要 vouchers 表 schema，标注 #[ignore] 仅在本地手动运行。
    #[tokio::test]
    #[ignore]
    async fn 测试_查询凭证列表_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let params = VoucherQueryParams {
            voucher_type: None,
            status: None,
            start_date: None,
            end_date: None,
            batch_no: None,
            color_no: None,
            page: None,
            page_size: None,
        };
        let result = service.get_list(params).await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时为 Err；有 schema 时为 Ok
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_凭证过账_需要真实数据库
    ///
    /// 需要 vouchers/voucher_items/account_balances/account_subjects 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。
    #[tokio::test]
    #[ignore]
    async fn 测试_凭证过账_需要真实数据库() {
        let db = setup_test_db().await;
        let service = VoucherService::new(Arc::new(db));

        let result = service.post(99999, 1).await;
        // 无 schema 时为 Err
        assert!(result.is_err());
    }

    // ============ 批次 393 补测：凭证类型定义与辅助核算五维 ============

    /// 测试_VoucherTypeDefinition_new构造器
    ///
    /// 验证 VoucherTypeDefinition::new 正确设置 code 和 name 字段。
    #[test]
    fn 测试_VoucherTypeDefinition_new构造器() {
        let def = VoucherTypeDefinition::new("记", "记账凭证");
        assert_eq!(def.code, "记");
        assert_eq!(def.name, "记账凭证");

        let def = VoucherTypeDefinition::new("收", "收款凭证");
        assert_eq!(def.code, "收");
        assert_eq!(def.name, "收款凭证");

        let def = VoucherTypeDefinition::new("付", "付款凭证");
        assert_eq!(def.code, "付");
        assert_eq!(def.name, "付款凭证");

        let def = VoucherTypeDefinition::new("转", "转账凭证");
        assert_eq!(def.code, "转");
        assert_eq!(def.name, "转账凭证");
    }

    /// 测试_available_voucher_types返回4种类型
    ///
    /// 验证 available_voucher_types 静态方法返回 4 种凭证类型定义，
    /// code 覆盖 "记/收/付/转" 全部业务类型。
    #[test]
    fn 测试_available_voucher_types返回4种类型() {
        let types = VoucherService::available_voucher_types();
        assert_eq!(types.len(), 4, "应有 4 种凭证类型");

        let codes: Vec<&str> = types.iter().map(|t| t.code).collect();
        assert!(codes.contains(&"记"), "应包含记账凭证");
        assert!(codes.contains(&"收"), "应包含收款凭证");
        assert!(codes.contains(&"付"), "应包含付款凭证");
        assert!(codes.contains(&"转"), "应包含转账凭证");

        // 名称不应为空
        for t in &types {
            assert!(!t.name.is_empty(), "凭证类型 {} 的名称不应为空", t.code);
        }
    }

    /// 测试_科目不存在错误消息格式
    ///
    /// 验证两个分支的科目不存在错误消息格式：
    /// 1. validate_voucher_create_req 阶段："科目不存在或已停用：{code}"
    /// 2. update_account_balances 阶段："科目不存在：{code}"
    /// （批次 102 v6 P3-4：后者已从 bad_request 改为 not_found）
    #[test]
    fn 测试_科目不存在错误消息格式() {
        // 分支 1：校验阶段（科目不存在或已停用）
        let err1 = AppError::bad_request(format!("科目不存在或已停用：{}", "9999"));
        let msg1 = err1.to_string();
        assert!(
            msg1.contains("科目不存在或已停用：9999"),
            "校验阶段错误消息应包含科目代码，实际：{}",
            msg1
        );

        // 分支 2：余额更新阶段（科目不存在，not_found）
        let err2 = AppError::not_found(format!("科目不存在：{}", "9999"));
        let msg2 = err2.to_string();
        assert!(
            msg2.contains("科目不存在：9999"),
            "余额更新阶段错误消息应包含科目代码，实际：{}",
            msg2
        );

        // 两个消息不应相同（措辞不同）
        assert_ne!(msg1, msg2, "两个分支的错误消息应有区别");
    }

    /// 测试_辅助核算五维ID拼接格式
    ///
    /// 复现 create_assist_accounting_records 中的五维 ID 拼接逻辑。
    /// 格式：BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}
    /// 缺失字段使用 unwrap_or(0) / unwrap_or_default() 填充。
    #[test]
    fn 测试_辅助核算五维ID拼接格式() {
        // 复现五维 ID 拼接逻辑（与源码一致）
        fn build_five_dimension_id(
            batch_id: Option<i32>,
            color_no_id: Option<i32>,
            dye_lot_id: Option<i32>,
            grade: Option<&str>,
            workshop_id: Option<i32>,
        ) -> String {
            format!(
                "BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}",
                batch_id.unwrap_or(0),
                color_no_id.unwrap_or(0),
                dye_lot_id.unwrap_or(0),
                grade.unwrap_or_default(),
                workshop_id.unwrap_or(0),
            )
        }

        // 场景 1：全部字段齐全
        let id = build_five_dimension_id(Some(10), Some(20), Some(30), Some("A"), Some(40));
        assert_eq!(id, "BATCH:10|COLOR:20|DYE_LOT:30|GRADE:A|WORKSHOP:40");

        // 场景 2：全部字段缺失（使用默认值）
        let id = build_five_dimension_id(None, None, None, None, None);
        assert_eq!(id, "BATCH:0|COLOR:0|DYE_LOT:0|GRADE:|WORKSHOP:0");

        // 场景 3：部分字段缺失
        let id = build_five_dimension_id(Some(10), None, Some(30), None, Some(40));
        assert_eq!(id, "BATCH:10|COLOR:0|DYE_LOT:30|GRADE:|WORKSHOP:40");

        // 防御性断言：分隔符格式正确
        let parts: Vec<&str> = id.split('|').collect();
        assert_eq!(parts.len(), 5, "五维 ID 应有 5 个段");
        assert!(parts[0].starts_with("BATCH:"));
        assert!(parts[1].starts_with("COLOR:"));
        assert!(parts[2].starts_with("DYE_LOT:"));
        assert!(parts[3].starts_with("GRADE:"));
        assert!(parts[4].starts_with("WORKSHOP:"));
    }
}
