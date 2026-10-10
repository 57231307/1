//! 凭证服务-CRUD 子模块（voucher_ops/crud）
//!
//! 批次 488 D10-2a 拆分：从原 `voucher_service.rs` L124-350 + L480-522 + L1258-1288 迁移。
//! 包含 11 个 CRUD 方法：
//! - create / create_and_post（公开 API）
//! - validate_voucher_create_req / precheck_subjects_exist_txn / insert_voucher_items_txn（create 的私有拆分）
//! - get_list / get_by_id / update / delete（公开 API）
//! - generate_voucher_no / available_voucher_types
//!
//! 业务规则：
//! - 创建凭证：期间锁定校验 + 借贷平衡 + 生成凭证编号 + 事务内批量校验科目 + 批量插入分录
//! - 状态门：仅 draft 状态可 update/delete（lock_exclusive 串行化并发）
//! - 凭证编号前缀：记→JZ / 收→SK / 付→FK / 转→ZZ
//!
//! 错误族口径（判据见 `utils/error.rs` 模块文档）：
//! - 借贷不平衡 = 提交明细金额自身不一致 → 校验族，文案含金额数字故脱敏
//! - 分录引用的科目查无 → NOT_FOUND；科目存在但已停用 = 前置状态门 → 业务族（文案含 code/ID，脱敏）
//! - 凭证状态不满足 update/delete 前置 → 业务族，纯公开规则文案可外显

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
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;

use crate::services::voucher_service::{
    CreateVoucherRequest, UpdateVoucherRequest, VoucherDetail, VoucherItemRequest,
    VoucherQueryParams, VoucherService, VoucherTypeDefinition,
};

impl VoucherService {
    /// 创建凭证（期间校验+借贷平衡+单号+事务+科目校验+分录批量插入）
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
        let txn = self.db.begin().await?;

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

        let voucher = active_model.insert(&txn).await?;
        info!("凭证创建成功：no={}", voucher.voucher_no);

        // 5. 批量校验分录引用的科目（存在性 + 可用状态）
        Self::precheck_subjects_exist_txn(&req.items, &txn).await?;

        // 6. 批量插入凭证分录
        Self::insert_voucher_items_txn(voucher.id, &req.items, &txn).await?;

        // 7. 提交事务
        txn.commit().await?;

        info!("凭证分录创建成功，共 {} 条", req.items.len());

        Ok(voucher)
    }

    /// 创建并自动过账凭证（DRAFT→SUBMITTED→REVIEWED→POSTED）
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
        // 非生产环境跳过期间锁定检查（CI 测试环境可能无会计期间数据）
        if !crate::utils::config::is_production() {
            // 验证借贷平衡
            let total_debit: Decimal = req.items.iter().map(|i| i.debit).sum();
            let total_credit: Decimal = req.items.iter().map(|i| i.credit).sum();

            if total_debit != total_credit {
                warn!("凭证借贷不平衡：借={}, 贷={}", total_debit, total_credit);
                return Err(Self::balance_error(total_debit, total_credit));
            }
            return Ok(());
        }

        // 校验期间锁定
        let period_svc =
            crate::services::accounting_period_service::AccountingPeriodService::new(db);
        period_svc.check_date_locked(req.voucher_date).await?;

        // 验证借贷平衡
        let total_debit: Decimal = req.items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = req.items.iter().map(|i| i.credit).sum();

        if total_debit != total_credit {
            warn!("凭证借贷不平衡：借={}, 贷={}", total_debit, total_credit);
            return Err(Self::balance_error(total_debit, total_credit));
        }

        Ok(())
    }

    /// 借贷不平衡的**唯一**错误装配点（create / update / workflow submit/review/post
    /// 全部路径共用；workflow.rs 经 `pub(super)` 在 voucher_ops 模块树内共享，禁止另起炉灶）。
    ///
    /// 判族：借贷是否平衡完全由提交进来的分录金额决定，属「提交数据一致性校验」，
    /// 归 **VALIDATION 族**（不是状态门，也不是 BAD_REQUEST 兜底族）。
    /// 保密分层：文案携带借/贷合计金额数字，按 `utils/error.rs` 安全边界走**脱敏**
    /// `AppError::validation`（出参 message 恒为常量「请求参数验证失败」），
    /// 真实金额只进 tracing 日志，不外显。
    pub(super) fn balance_error(total_debit: Decimal, total_credit: Decimal) -> AppError {
        AppError::validation(format!(
            "凭证借贷不平衡：借方 {} != 贷方 {}",
            total_debit, total_credit
        ))
    }

    /// P2 1-6 修复：批量校验凭证分录引用的科目（从 create 抽取）
    ///
    /// 契约对齐扩展：除按 subject_code 校验外，同时校验按 subject_id 提交的科目
    /// （前端 VoucherEntry 仅带科目 ID，不带 code）
    ///
    /// 这里的两种拒绝语义不同，必须分属两族（族口径判据）
    /// - 查无此科目（分录引用了库里不存在的 code / id）→ 引用存在性缺失，
    ///   归 [`AppError::not_found`]（HTTP 404 / `NOT_FOUND`），与 `voucher_ops/balance.rs`
    ///   同族同文案；
    /// - 科目存在但状态非 `active` → 引用主数据的**前置状态门**未满足，归业务族。
    ///   文案携带科目 code / 记录 ID（内部编码），按 `utils/error.rs` 的安全边界走
    ///   **脱敏** [`AppError::business`]，真实 code 只进 WARN 日志，不出 HTTP 出参。
    async fn precheck_subjects_exist_txn(
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let mut subject_codes = std::collections::HashSet::new();
        let mut subject_ids = std::collections::HashSet::new();
        for item_req in items {
            if let Some(ref subject_code) = item_req.subject_code {
                if !subject_code.is_empty() {
                    subject_codes.insert(subject_code.clone());
                }
            }
            if let Some(sid) = item_req.subject_id {
                subject_ids.insert(sid);
            }
        }
        if subject_codes.is_empty() && subject_ids.is_empty() {
            return Ok(());
        }

        // 连状态一起读出被引用的科目（不能用 Status=ACTIVE 过滤后再判），否则「不存在」与
        // 「已停用」会被压成同一个分支，族归类就无从谈起了。
        let mut status_by_code: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        // value = (科目编码, 状态)：id 提交分支也要能在日志/Display 侧留痕真实科目编码，
        // 只有主键的话排障时无法定位到具体科目。
        let mut status_by_id: std::collections::HashMap<i32, (String, String)> =
            std::collections::HashMap::new();

        if !subject_codes.is_empty() {
            let found = account_subject::Entity::find()
                .filter(
                    account_subject::Column::Code
                        .is_in(subject_codes.iter().cloned().collect::<Vec<_>>()),
                )
                .all(txn)
                .await?;
            for s in found {
                status_by_code.insert(s.code.clone(), s.status.clone());
                status_by_id.insert(s.id, (s.code.clone(), s.status));
            }
        }

        if !subject_ids.is_empty() {
            let found = account_subject::Entity::find()
                .filter(
                    account_subject::Column::Id
                        .is_in(subject_ids.iter().copied().collect::<Vec<_>>()),
                )
                .all(txn)
                .await?;
            for s in found {
                status_by_id.insert(s.id, (s.code.clone(), s.status.clone()));
                status_by_code.insert(s.code, s.status);
            }
        }

        for code in subject_codes {
            match status_by_code.get(&code) {
                None => {
                    warn!("凭证分录引用的科目不存在：code={}", code);
                    return Err(AppError::not_found(format!("科目不存在：{}", code)));
                }
                Some(status) if status != master_data::ACTIVE => {
                    warn!("凭证分录引用的科目已停用：code={}, status={}", code, status);
                    return Err(AppError::business(format!("科目已停用：{}", code)));
                }
                Some(_) => {}
            }
        }
        for sid in subject_ids {
            match status_by_id.get(&sid) {
                None => {
                    warn!("凭证分录引用的科目不存在：id={}", sid);
                    return Err(AppError::not_found(format!("科目不存在：ID={}", sid)));
                }
                Some((code, status)) if status != master_data::ACTIVE => {
                    warn!(
                        "凭证分录引用的科目已停用：id={}, code={}, status={}",
                        sid, code, status
                    );
                    return Err(AppError::business(format!(
                        "科目已停用：ID={}, code={}",
                        sid, code
                    )));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// 契约对齐：按科目 ID 批量反查科目（code/name），用于请求只带 subject_id 的分录补全
    async fn load_subject_map_by_ids(
        subject_ids: &[i32],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<std::collections::HashMap<i32, (String, String)>, AppError> {
        if subject_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let subjects = account_subject::Entity::find()
            .filter(account_subject::Column::Id.is_in(subject_ids.to_vec()))
            .all(txn)
            .await?;
        Ok(subjects
            .into_iter()
            .map(|s| (s.id, (s.code, s.name)))
            .collect())
    }

    /// 批量插入凭证分录（从 create 抽取）
    ///
    /// 契约对齐：请求只带 subject_id 时，反查科目补全 code/name 落库并持久化 subject_id
    async fn insert_voucher_items_txn(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let subject_ids: Vec<i32> = items.iter().filter_map(|i| i.subject_id).collect();
        let subject_map = Self::load_subject_map_by_ids(&subject_ids, txn).await?;

        for (index, item_req) in items.iter().enumerate() {
            let (code, name) = match item_req.subject_id.and_then(|id| subject_map.get(&id)) {
                Some((c, n)) => (Some(c.clone()), Some(n.clone())),
                None => (item_req.subject_code.clone(), item_req.subject_name.clone()),
            };
            let item_active_model = voucher_item::ActiveModel {
                voucher_id: sea_orm::Set(voucher_id),
                line_no: sea_orm::Set(item_req.line_no.unwrap_or((index + 1) as i32)),
                subject_id: sea_orm::Set(item_req.subject_id),
                subject_code: sea_orm::Set(code.unwrap_or_default()),
                subject_name: sea_orm::Set(name.unwrap_or_default()),
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

            item_active_model.insert(txn).await?;
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

        if let Some(voucher_no) = &params.voucher_no {
            if !voucher_no.is_empty() {
                query = query.filter(voucher::Column::VoucherNo.contains(voucher_no));
            }
        }

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

        let txn = self.db.begin().await?;

        let voucher_model = Self::load_voucher_for_update(id, &txn).await?;
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
            Self::replace_voucher_items(id, &items, &txn).await?;
        }

        txn.commit().await?;

        info!("凭证更新成功：no={}", updated_voucher.voucher_no);
        Ok(updated_voucher)
    }

    /// 加载凭证并校验 draft 状态（txn 内 lock_exclusive）
    async fn load_voucher_for_update(
        id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<voucher::Model, AppError> {
        let voucher_model = voucher::Entity::find_by_id(id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("凭证不存在：{}", id)))?;

        if voucher_model.status != crate::models::status::voucher::VOUCHER_DRAFT {
            warn!("只有草稿状态的凭证可以更新：{}", voucher_model.voucher_no);
            // 状态门：凭证当前状态不满足更新前置，归业务族；出参文案只述公开规则
            // （真实状态仅进 warn 日志），可外显。
            return Err(AppError::business_displayable(
                "只有草稿状态的凭证可以更新".to_string(),
            ));
        }
        Ok(voucher_model)
    }

    /// 替换凭证分录：校验借贷平衡 + 删除旧分录 + 插入新分录
    async fn replace_voucher_items(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        Self::validate_voucher_items_balance(items)?;

        vi::Entity::delete_many()
            .filter(vi::Column::VoucherId.eq(voucher_id))
            .exec(txn)
            .await?;

        Self::insert_voucher_items_for_update(voucher_id, items, txn).await
    }

    /// 校验凭证分录借贷平衡
    fn validate_voucher_items_balance(items: &[VoucherItemRequest]) -> Result<(), AppError> {
        let total_debit: Decimal = items.iter().map(|i| i.debit).sum();
        let total_credit: Decimal = items.iter().map(|i| i.credit).sum();
        if total_debit != total_credit {
            warn!(
                "凭证借贷不平衡（更新分录路径）：借={}, 贷={}",
                total_debit, total_credit
            );
            return Err(Self::balance_error(total_debit, total_credit));
        }
        Ok(())
    }

    /// 插入凭证分录（update 路径，含 created_at）
    ///
    /// 契约对齐：请求只带 subject_id 时，反查科目补全 code/name 落库并持久化 subject_id
    async fn insert_voucher_items_for_update(
        voucher_id: i32,
        items: &[VoucherItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let subject_ids: Vec<i32> = items.iter().filter_map(|i| i.subject_id).collect();
        let subject_map = Self::load_subject_map_by_ids(&subject_ids, txn).await?;

        for (index, item_req) in items.iter().enumerate() {
            let (code, name) = match item_req.subject_id.and_then(|id| subject_map.get(&id)) {
                Some((c, n)) => (Some(c.clone()), Some(n.clone())),
                None => (item_req.subject_code.clone(), item_req.subject_name.clone()),
            };
            let item_active = vi::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                voucher_id: sea_orm::Set(voucher_id),
                line_no: sea_orm::Set(item_req.line_no.unwrap_or((index + 1) as i32)),
                subject_id: sea_orm::Set(item_req.subject_id),
                subject_code: sea_orm::Set(code.unwrap_or_default()),
                subject_name: sea_orm::Set(name.unwrap_or_default()),
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
            item_active.insert(txn).await?;
        }
        Ok(())
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
            // 状态门：凭证当前状态不满足删除前置，归业务族；真实状态只进日志，出参可外显
            return Err(AppError::business_displayable(
                "只有草稿状态的凭证可以删除".to_string(),
            ));
        }

        // 保留凭证号用于日志
        let voucher_no = voucher.voucher_no.clone();

        // 先删分录：voucher_items 的 FK（fk_voucher_items_voucher）无 ON DELETE CASCADE，
        // 直接删主表会 FK violation（DATABASE_ERROR）
        voucher_item::Entity::delete_many()
            .filter(voucher_item::Column::VoucherId.eq(id))
            .exec(&txn)
            .await?;

        // 删除凭证（含审计日志）
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
