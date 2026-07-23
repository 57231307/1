//! 凭证服务-CRUD 子模块（voucher_ops/crud）
//!
//! 批次 488 D10-4 拆分：从原 `voucher_service.rs` L116-521 迁移。
//! 包含 9 个 CRUD 方法：
//! - create / create_and_post（公开 API）
//! - validate_voucher_create_req / precheck_subjects_exist_txn / insert_voucher_items_txn（create 辅助）
//! - get_list / get_by_id / update / delete（公开 API）

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::sync::Arc;
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::voucher_item as vi;
use crate::models::{account_subject, voucher, voucher_item};
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;

use crate::services::voucher_service::{
    CreateVoucherRequest, UpdateVoucherRequest, VoucherDetail, VoucherItemRequest,
    VoucherQueryParams, VoucherService,
};

impl VoucherService {
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
}
