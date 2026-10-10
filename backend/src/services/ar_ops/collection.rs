//! 应收账款-收款管理子模块（ar_ops/collection）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L112-751 迁移。
//! 包含收款管理方法：
//! - list_payments / get_payment / create_payment（公开 API）
//! - validate_payment_amount / check_payment_period_locked / load_customer_for_payment
//! - generate_collection_no / build_collection_active_model / build_and_insert_collection
//! - link_invoices_to_payment / allocate_payment_to_invoice / publish_payment_events
//! - active_receipt_ledger_items / receipt_verify_totals / payment_verified_total
//!   （收款单级"已核销分配额"读数的唯一实现，核销门与可核销列表同源引用）
//! - update_payment / confirm_payment（公开 API）
//! - cancel_collection（公开 API）/ ensure_collection_cancellable / build_cancelled_active
//!
//! 业务规则：
//! - 收款基于 ar_collection 表
//! - 状态机：pending → confirmed（不可逆）/ pending → cancelled
//! - 金额校验 round_dp(2) 限制货币精度
//! - 期间锁定检查通过 AccountingPeriodService::check_date_locked_txn
//! - 所有写操作在事务内执行，状态变更加 lock_exclusive 串行化

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
use crate::utils::error::AppError;

use super::json_helpers::collection_to_json;
use super::types::{CollectionBuildContext, CreateArPaymentParams, ReconciliationItemContext};
use crate::services::ar_service::ArService;

impl ArService {
    /// 获取收款列表
    /// 基于 ar_collection 表分页查询，支持状态/客户/收款单号过滤
    pub async fn list_payments(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        customer_id: Option<i32>,
        payment_no: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        let mut query = ar_collection::Entity::find();

        if let Some(s) = status {
            query = query.filter(ar_collection::Column::Status.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(ar_collection::Column::CustomerId.eq(cid));
        }
        if let Some(no) = payment_no {
            query = query.filter(ar_collection::Column::CollectionNo.eq(no));
        }

        // V15 P0-S01：行级数据权限过滤
        // ar_collection 表无 department_id，Dept 退化为 Self，使用 created_by（i32 必填）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                ar_collection::Column::CreatedBy,
                ar_collection::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }

        let total = query.clone().count(&*self.db).await? as i64;
        let items = query
            .order_by(ar_collection::Column::CollectionDate, Order::Desc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        let list = items.into_iter().map(collection_to_json).collect();
        Ok((list, total))
    }

    /// 获取收款详情
    pub async fn get_payment(
        &self,
        payment_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let payment = ar_collection::Entity::find_by_id(payment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // ar_collection 表无 department_id，使用成员归属集合判定；
        // ar_collection.created_by 是 i32（必填）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner_by_member_scope(ctx, Some(payment.created_by)) {
                return Err(AppError::permission_denied(
                    "无权访问收款单（数据范围限制）",
                ));
            }
        }
        Ok(collection_to_json(payment))
    }

    /// 创建收款：金额校验→期间锁定→客户校验→单号生成→插入收款→关联发票→事件发布
    /// 批次 488 D08-1 拆分：主函数仅做协调，细节逻辑提取到 helper
    pub async fn create_payment(
        &self,
        params: CreateArPaymentParams,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_payment_amount(params.customer_id, params.amount)?;

        let txn = (*self.db).begin().await?;

        // 期间锁定检查（事务内，避免 TOCTOU）
        self.check_payment_period_locked(&txn, params.payment_date)
            .await?;

        let customer = Self::load_customer_for_payment(&txn, params.customer_id).await?;

        let collection_no = Self::generate_collection_no(&txn).await?;

        let now = Utc::now();
        let collection_model = Self::build_and_insert_collection(
            &txn,
            collection_no,
            &params,
            customer.customer_name,
            user_id,
            now,
        )
        .await?;

        // 关联多张发票：累加 received_amount、扣减 unpaid_amount、按需更新状态，
        // 并与手工/自动核销共用同一写账本入口落收款单级核销明细（同事务，见函数文档）
        let linked_invoices = self
            .link_invoices_to_payment(
                &txn,
                &collection_model,
                params.invoice_ids,
                params.amount,
                params.customer_id,
                user_id,
                now,
            )
            .await?;

        txn.commit().await?;

        Self::publish_payment_events(
            &collection_model,
            &linked_invoices,
            params.amount,
            user_id,
            params.payment_date,
        );

        Ok(collection_to_json(collection_model))
    }

    /// 金额校验：必须大于零且精度不超过 2 位小数
    fn validate_payment_amount(customer_id: i32, amount: Decimal) -> Result<(), AppError> {
        if amount <= Decimal::ZERO {
            // 批次 389 P2-2：金额校验失败记录 warn 日志，便于审计异常收款行为
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_INVALID_AMOUNT",
                customer_id = customer_id,
                amount = %amount,
                "AR 收款金额校验失败：金额必须大于零"
            );
            return Err(AppError::validation_displayable("收款金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_INVALID_PRECISION",
                customer_id = customer_id,
                amount = %amount,
                "AR 收款金额校验失败：精度超过 2 位小数"
            );
            return Err(AppError::validation_displayable(
                "收款金额精度不能超过 2 位小数",
            ));
        }
        Ok(())
    }

    /// 期间锁定检查（事务内，避免 TOCTOU）
    async fn check_payment_period_locked(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        payment_date: NaiveDate,
    ) -> Result<(), AppError> {
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc.check_date_locked_txn(txn, payment_date).await
    }

    /// 客户存在性校验 + 名称查询（事务内）
    async fn load_customer_for_payment(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
    ) -> Result<crate::models::customer::Model, AppError> {
        crate::models::customer::Entity::find_by_id(customer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))
    }

    /// 单号生成（事务内，advisory_xact_lock 串行化）
    async fn generate_collection_no(
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<String, AppError> {
        crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            txn,
            "COL",
            ar_collection::Entity,
            ar_collection::Column::CollectionNo,
        )
        .await
    }

    /// 构造收款单 ActiveModel（纯函数，无 IO）
    fn build_collection_active_model(ctx: CollectionBuildContext) -> ar_collection::ActiveModel {
        ar_collection::ActiveModel {
            collection_no: Set(ctx.collection_no),
            collection_date: Set(ctx.payment_date),
            customer_id: Set(ctx.customer_id),
            customer_name: Set(ctx.customer_name),
            collection_amount: Set(ctx.amount),
            collection_method: Set(Some(ctx.payment_method)),
            bank_account: Set(ctx.bank_account),
            remark: Set(ctx.remark),
            status: Set(crate::models::status::ar::COLLECTION_PENDING.to_string()),
            created_by: Set(ctx.user_id),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
    }

    /// 构造收款单上下文并插入：复用 CollectionBuildContext + build_collection_active_model
    async fn build_and_insert_collection(
        txn: &sea_orm::DatabaseTransaction,
        collection_no: String,
        params: &CreateArPaymentParams,
        customer_name: String,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<ar_collection::Model, AppError> {
        let ctx = CollectionBuildContext {
            collection_no,
            payment_date: params.payment_date,
            customer_id: params.customer_id,
            customer_name: Some(customer_name),
            amount: params.amount,
            payment_method: params.payment_method.clone(),
            bank_account: params.bank_account.clone(),
            remark: params.remark.clone(),
            user_id,
            now,
        };
        Ok(Self::build_collection_active_model(ctx).insert(txn).await?)
    }

    /// 关联多张发票：累加 received_amount、扣减 unpaid_amount、按需更新状态，并在
    /// 同一事务内经核销明细唯一写入口（verification_ops/manual.rs::create_reconciliation_record
    /// + create_reconciliation_items，与手工核销同函数同参数口径）为每张实际分配的发票
    /// 落一条核销单主记录 + INVOICE/RECEIPT 明细对——创建直连路径核销掉的金额因此对
    /// payment_verified_total 可见，修改收款金额的下限拒绝门据此覆盖本路径；
    /// cancel_collection 同事务回滚发票侧并回收这些明细。任一步失败事务整体回滚，
    /// 发票侧金额与收款单级账本行不允许只写一半。
    async fn link_invoices_to_payment(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        payment: &ar_collection::Model,
        invoice_ids: Option<Vec<i32>>,
        amount: Decimal,
        customer_id: i32,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<i32>, AppError> {
        let mut linked_invoices: Vec<i32> = Vec::new();
        if let Some(inv_ids) = invoice_ids {
            // 批量查询所有发票并加锁，避免循环内 N+1
            let invoice_map: std::collections::HashMap<i32, ar_invoice::Model> =
                if inv_ids.is_empty() {
                    std::collections::HashMap::new()
                } else {
                    ar_invoice::Entity::find()
                        .filter(ar_invoice::Column::Id.is_in(inv_ids.clone()))
                        .lock_exclusive()
                        .all(txn)
                        .await?
                        .into_iter()
                        .map(|inv| (inv.id, inv))
                        .collect()
                };

            // 按比例分摊收款金额到各发票
            // 简化策略：按发票顺序扣减，每张发票扣减 min(剩余收款, 发票未收金额)
            let mut remaining = amount;
            for inv_id in inv_ids {
                if remaining <= Decimal::ZERO {
                    break;
                }
                let invoice = invoice_map
                    .get(&inv_id)
                    .ok_or_else(|| AppError::not_found(format!("应收单 {} 不存在", inv_id)))?;
                let allocated = Self::allocate_payment_to_invoice(
                    invoice,
                    &mut remaining,
                    customer_id,
                    user_id,
                    now,
                    txn,
                )
                .await?;
                if allocated <= Decimal::ZERO {
                    continue;
                }
                linked_invoices.push(inv_id);
                // 与手工核销同口径落核销单主记录 + INVOICE/RECEIPT 分配明细
                let reconciliation = self
                    .create_reconciliation_record(invoice, allocated, user_id, now, txn)
                    .await?;
                self.create_reconciliation_items(
                    ReconciliationItemContext {
                        reconciliation: &reconciliation,
                        invoice,
                        payment,
                        amount: allocated,
                        remark: None,
                        now,
                    },
                    txn,
                )
                .await?;
            }
        }
        Ok(linked_invoices)
    }

    /// 单张发票的扣减分配：校验 + 计算 allocate + 更新 received/unpaid/status。
    /// 返回本次实际分配金额（0 表示未发生扣减，如发票已结清）。
    async fn allocate_payment_to_invoice(
        invoice: &ar_invoice::Model,
        remaining: &mut Decimal,
        customer_id: i32,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Decimal, AppError> {
        if invoice.status == crate::models::status::common::STATUS_CANCELLED {
            // 状态门：应收单已处于取消终态，收款关联前置未满足，归业务族；
            // 文案含内部应收单 ID，按安全边界保持脱敏 business。
            return Err(AppError::business(format!(
                "应收单 {} 已取消，无法关联收款",
                invoice.id
            )));
        }
        if invoice.customer_id != customer_id {
            // 一致性门：应收单与收款分属不同客户，属业务前置不满足，归业务族；含内部 ID 保持脱敏
            return Err(AppError::business(format!(
                "应收单 {} 客户与收款客户不一致",
                invoice.id
            )));
        }
        let allocate = (*remaining).min(invoice.unpaid_amount);
        if allocate <= Decimal::ZERO {
            return Ok(Decimal::ZERO);
        }
        let new_received = invoice.received_amount + allocate;
        let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
        let new_status = if new_unpaid == Decimal::ZERO {
            crate::models::status::payment::PAYMENT_PAID.to_string()
        } else if new_received > Decimal::ZERO {
            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
        } else {
            invoice.status.clone()
        };

        let mut active: ar_invoice::ActiveModel = invoice.clone().into();
        active.received_amount = Set(new_received);
        active.unpaid_amount = Set(new_unpaid);
        active.status = Set(new_status);
        active.updated_at = Set(now);
        crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_invoice::Entity,
            _,
            _,
        >(txn, "ar_invoice", active, Some(user_id))
        .await?;

        *remaining -= allocate;
        Ok(allocate)
    }

    /// 收款单级有效核销账本明细（"已核销分配额"读口径的 crate 内唯一实现）：
    /// ar_reconciliation_items 中同时满足三个条件的明细行——
    /// - item_type = RECEIPT；
    /// - document_type = AR_COLLECTION：核销账本明细的收款单引用形态（与写入口
    ///   verification_ops/manual.rs::create_reconciliation_items、
    ///   verification_ops/auto.rs::make_receipt_verify_item 落值逐字符相同）。
    ///   对账单域明细（document_type = COLLECTION，由对账生成与自动匹配落账）的 amount
    ///   记单据全额、matched_amount 记匹配额，属于对账单口径，与本"核销分配额"口径
    ///   分族，不得混入同一求和（口径分族判据即该列取值）；
    /// - 所属核销单 reconciliation_status = 核销状态词表常量 RECONCILIATION_CLOSED：
    ///   核销单生命周期为 closed↔cancelled（创建路径落 closed，取消门控仅 closed→cancelled），
    ///   有效核销集合恰为 closed，已取消核销不再占用分配额；按词表正向等值过滤，
    ///   不使用 `!= cancelled` 之类反向判据。
    /// 金额按核销记账方向存负，调用方取绝对值求和。
    pub(crate) async fn active_receipt_ledger_items<C: ConnectionTrait>(
        db: &C,
        payment_ids: &[i32],
    ) -> Result<Vec<ar_reconciliation_item::Model>, AppError> {
        if payment_ids.is_empty() {
            return Ok(Vec::new());
        }
        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentType.eq("AR_COLLECTION"))
            .filter(ar_reconciliation_item::Column::DocumentId.is_in(payment_ids.to_vec()))
            .join(
                JoinType::InnerJoin,
                ar_reconciliation_item::Relation::Reconciliation.def(),
            )
            .filter(
                ar_reconciliation::Column::ReconciliationStatus
                    .eq(crate::models::status::ar::RECONCILIATION_CLOSED),
            )
            .all(db)
            .await?;
        Ok(items)
    }

    /// 收款单级已核销分配汇总（批量）：委托 active_receipt_ledger_items，
    /// 按收款单 ID 分组对明细金额取绝对值求和；无有效账本行的收款单不进映射，
    /// 调用方按缺省零处理。手工核销可用余额门、自动核销已核销汇总
    /// （verification_ops/auto.rs::load_auto_verify_data）与可核销收款列表
    /// （verification_ops/query.rs::get_unverified_payments）全部引用本函数，
    /// 禁止各处再内联第二套求和。
    pub(crate) async fn receipt_verify_totals<C: ConnectionTrait>(
        db: &C,
        payment_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, Decimal>, AppError> {
        let items = Self::active_receipt_ledger_items(db, payment_ids).await?;
        let mut totals: std::collections::HashMap<i32, Decimal> = std::collections::HashMap::new();
        for item in items {
            if let Some(doc_id) = item.document_id {
                *totals.entry(doc_id).or_insert(Decimal::ZERO) += item.amount.abs();
            }
        }
        Ok(totals)
    }

    /// 收款单级已核销分配合计（单笔读口）：委托收款单维度"已核销分配额"的唯一读数
    /// receipt_verify_totals（求和口径与有效核销过滤条件见其函数文档）；
    /// 修改收款金额的下限拒绝门只引用本函数，与核销可用余额门
    /// （verification_ops/manual.rs::check_payment_available_balance）同源。
    pub(super) async fn payment_verified_total(
        txn: &sea_orm::DatabaseTransaction,
        payment_id: i32,
    ) -> Result<Decimal, AppError> {
        let totals = Self::receipt_verify_totals(txn, std::slice::from_ref(&payment_id)).await?;
        Ok(totals.get(&payment_id).copied().unwrap_or(Decimal::ZERO))
    }

    /// 事件发布（commit 后，避免事件处理器回写导致事务膨胀）
    fn publish_payment_events(
        collection: &ar_collection::Model,
        linked_invoices: &[i32],
        amount: Decimal,
        user_id: i32,
        payment_date: NaiveDate,
    ) {
        info!(
            "AR 收款创建成功：collection_no={}, customer_id={}, amount={}, 关联发票数={}",
            collection.collection_no,
            collection.customer_id,
            amount,
            linked_invoices.len()
        );
        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        for inv_id in linked_invoices {
            EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
                collection_id: collection.id,
                invoice_id: Some(*inv_id),
                amount,
                user_id,
            });
        }
        let period = format!("{:04}-{:02}", payment_date.year(), payment_date.month());
        EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source: format!("ar_collection_completed:{}", collection.collection_no),
        });
    }

    /// 更新收款
    /// 仅 pending 状态可修改。可空列（payment_method/bank_account/check_no/remark）
    /// 按三态落列；NOT NULL 列 amount/payment_date 亦为三态但显式 null 一律业务拒绝：
    /// amount 覆盖前执行与创建同源的金额校验（validate_payment_amount）与一致性门
    /// （新金额 < 已核销分配合计 payment_verified_total 时拒绝，库内分配账本不被破坏）；
    /// payment_date 覆盖前复用创建路径的所属期间关账检查（check_payment_period_locked）。
    /// pending 状态门天然排除凭证/核销联动（凭证仅确认时生成、核销要求 confirmed），见模块文档。
    pub async fn update_payment(
        &self,
        payment_id: i32,
        payload: serde_json::Value,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;

        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_UPDATE_REJECTED",
                payment_id = payment_id,
                status = %collection.status,
                "AR 收款更新被拒：状态非 pending"
            );
            return Err(AppError::bad_request("非 pending 状态的收款单不可修改"));
        }

        // 金额业务校验的审计维度与一致性门在 collection 被搬入 ActiveModel 前取存
        let customer_id = collection.customer_id;
        let mut active: ar_collection::ActiveModel = collection.into();

        // 可空业务列统一三态（键缺席=保持原值、显式 null=清空为 NULL、字符串值=覆盖）：
        // 收款方式落 collection_method 列、银行账号落 bank_account 列、备注落 remark 列、
        // 支票号只接受 check_no 入参，各列互不覆盖。
        if let Some(v) = payload.get("payment_method") {
            if v.is_null() {
                active.collection_method = Set(None);
            } else if let Some(s) = v.as_str() {
                active.collection_method = Set(Some(s.to_string()));
            } else {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "payment_method",
                    "AR 收款更新 payment_method 类型非字符串/null，保持原值不覆盖"
                );
            }
        }
        if let Some(v) = payload.get("bank_account") {
            if v.is_null() {
                active.bank_account = Set(None);
            } else if let Some(s) = v.as_str() {
                active.bank_account = Set(Some(s.to_string()));
            } else {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "bank_account",
                    "AR 收款更新 bank_account 类型非字符串/null，保持原值不覆盖"
                );
            }
        }
        if let Some(v) = payload.get("remark") {
            if v.is_null() {
                active.remark = Set(None);
            } else if let Some(s) = v.as_str() {
                active.remark = Set(Some(s.to_string()));
            } else {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "remark",
                    "AR 收款更新 remark 类型非字符串/null，保持原值不覆盖"
                );
            }
        }
        if let Some(v) = payload.get("check_no") {
            if v.is_null() {
                active.check_no = Set(None);
            } else if let Some(s) = v.as_str() {
                active.check_no = Set(Some(s.to_string()));
            } else {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "check_no",
                    "AR 收款更新 check_no 类型非字符串/null，保持原值不覆盖"
                );
            }
        }
        // amount/payment_date 落 NOT NULL 列（collection_amount/collection_date）：
        // 键缺席=保持原值、有值=覆盖、显式 null=业务拒绝（不落默认值，先例判据见
        // handlers/ar_payment_handler.rs::UpdateArPaymentRequest 文档与
        // inventory_adjustment_service.rs::update_adjustment 的 NOT NULL 列门控）。
        if let Some(v) = payload.get("amount") {
            if v.is_null() {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_NOT_NULL_CLEARED",
                    payment_id = payment_id,
                    field = "amount",
                    "AR 收款更新被拒：amount 显式 null 指向 NOT NULL 列 collection_amount"
                );
                return Err(AppError::business_displayable(
                    "收款金额不能清空：该字段为必填项",
                ));
            }
            // DTO 已把该键约束为 Decimal（number 或数字字符串），到这里仍解析失败
            // 只可能是 service 直调方载荷异常，显式 400 不吞不兜底。
            let new_amount: Decimal = serde_json::from_value(v.clone()).map_err(|e| {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "amount",
                    error = %e,
                    "AR 收款更新 amount 载荷形态非法（非数字金额）"
                );
                AppError::validation_displayable("收款金额格式非法：须为数字或数字字符串")
            })?;
            // 与创建路径同源的金额值域/精度校验
            Self::validate_payment_amount(customer_id, new_amount)?;
            // 一致性门：发票侧已核销分配（收款单维度账本）必须被新金额覆盖得住，
            // 新金额小于已分配额直接拒绝，绝不静默重分配关联发票金额。
            let verified_total = Self::payment_verified_total(&txn, payment_id).await?;
            if new_amount < verified_total {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_BELOW_VERIFIED",
                    payment_id = payment_id,
                    new_amount = %new_amount,
                    verified_total = %verified_total,
                    "AR 收款更新被拒：新金额小于已核销分配金额"
                );
                return Err(AppError::business_displayable(
                    "新收款金额小于该收款单已核销分配金额，不可下调",
                ));
            }
            active.collection_amount = Set(new_amount);
        }
        if let Some(v) = payload.get("payment_date") {
            if v.is_null() {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_NOT_NULL_CLEARED",
                    payment_id = payment_id,
                    field = "payment_date",
                    "AR 收款更新被拒：payment_date 显式 null 指向 NOT NULL 列 collection_date"
                );
                return Err(AppError::business_displayable(
                    "收款日期不能清空：该字段为必填项",
                ));
            }
            let new_date: NaiveDate = serde_json::from_value(v.clone()).map_err(|e| {
                warn!(
                    target: "business_audit",
                    event = "AR_PAYMENT_UPDATE_FIELD_TYPE_MISMATCH",
                    payment_id = payment_id,
                    field = "payment_date",
                    error = %e,
                    "AR 收款更新 payment_date 载荷形态非法（非 YYYY-MM-DD 日期）"
                );
                AppError::validation_displayable("收款日期格式非法：须为 YYYY-MM-DD 日期字符串")
            })?;
            // 所属期间关账检查与创建路径同源（事务内，避免 TOCTOU）；
            // 关账拒绝文案沿用 AccountingPeriodService::check_date_locked_txn 的业务外显。
            self.check_payment_period_locked(&txn, new_date).await?;
            active.collection_date = Set(new_date);
        }
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(&txn, "ar_collection", active, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(collection_to_json(updated))
    }

    /// 确认收款：状态门 pending → confirmed，lock_exclusive 串行化并发
    pub async fn confirm_payment(
        &self,
        payment_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;
        let collection = Self::lock_and_validate_for_confirm(&txn, payment_id, user_id).await?;
        let updated = Self::confirm_collection_status(&txn, collection, user_id).await?;
        txn.commit().await?;
        self.generate_collection_voucher(&updated, user_id).await;
        Ok(collection_to_json(updated))
    }

    // ===== confirm_payment 私有 helpers（D08-1 拆分）=====

    /// 锁定并校验收款单状态（仅 pending 可确认）
    async fn lock_and_validate_for_confirm(
        txn: &sea_orm::DatabaseTransaction,
        payment_id: i32,
        user_id: i32,
    ) -> Result<ar_collection::Model, AppError> {
        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_CONFIRM_REJECTED",
                payment_id = payment_id,
                status = %collection.status,
                operator = user_id,
                "AR 收款确认被拒：状态非 pending"
            );
            return Err(AppError::bad_request(format!(
                "收款单状态为 {}，仅 pending 状态可确认",
                collection.status
            )));
        }
        Ok(collection)
    }

    /// 应用确认状态并 update_with_audit
    async fn confirm_collection_status(
        txn: &sea_orm::DatabaseTransaction,
        collection: ar_collection::Model,
        user_id: i32,
    ) -> Result<ar_collection::Model, AppError> {
        let mut active: ar_collection::ActiveModel = collection.into();
        active.status = Set(crate::models::status::ar::COLLECTION_CONFIRMED.to_string());
        active.confirmed_by = Set(Some(user_id));
        active.confirmed_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(txn, "ar_collection", active, Some(user_id))
        .await?;
        Ok(updated)
    }

    /// 生成收款凭证（best-effort，失败仅 warn 不阻塞主流程）
    async fn generate_collection_voucher(&self, updated: &ar_collection::Model, user_id: i32) {
        let collection_amount = updated.collection_amount;
        let collection_method = updated
            .collection_method
            .as_deref()
            .unwrap_or("BANK_TRANSFER");
        let (debit_code, debit_name) = match collection_method {
            "CASH" => ("1001", "库存现金"),
            _ => ("1002", "银行存款"),
        };
        let voucher_req = Self::build_collection_voucher_request(
            updated,
            collection_amount,
            debit_code,
            debit_name,
        );
        let voucher_service =
            crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "收款单 {} 确认成功，但生成收款凭证失败：{}",
                updated.collection_no,
                e
            );
        }
    }

    /// 构造收款凭证请求（借：银行存款/库存现金，贷：应收账款）
    fn build_collection_voucher_request(
        updated: &ar_collection::Model,
        collection_amount: Decimal,
        debit_code: &str,
        debit_name: &str,
    ) -> crate::services::voucher_service::CreateVoucherRequest {
        let summary = format!("收款确认-{}", updated.collection_no);
        crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "收".to_string(),
            voucher_date: updated.collection_date,
            source_type: Some("AR_COLLECTION".to_string()),
            source_module: Some("ar".to_string()),
            source_bill_id: Some(i64::from(updated.id)),
            source_bill_no: Some(updated.collection_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                Self::build_voucher_item(
                    1,
                    debit_code,
                    debit_name,
                    collection_amount,
                    true,
                    summary.clone(),
                    updated.customer_id,
                ),
                Self::build_voucher_item(
                    2,
                    "1122",
                    "应收账款",
                    collection_amount,
                    false,
                    summary,
                    updated.customer_id,
                ),
            ],
        }
    }

    /// 构造凭证分录（借/贷方向由 is_debit 决定）
    fn build_voucher_item(
        line_no: i32,
        subject_code: &str,
        subject_name: &str,
        amount: Decimal,
        is_debit: bool,
        summary: String,
        customer_id: i32,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        let (debit, credit) = if is_debit {
            (amount, Decimal::ZERO)
        } else {
            (Decimal::ZERO, amount)
        };
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(line_no),
            subject_id: None,
            subject_code: Some(subject_code.to_string()),
            subject_name: Some(subject_name.to_string()),
            debit,
            credit,
            summary: Some(summary),
            assist_customer_id: Some(customer_id),
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

    /// 取消收款单。业务规则：仅 `pending` 状态的收款单可直接取消；`confirmed` 状态
    /// 需先经核销模块取消关联核销单（verification_ops/manual.rs::cancel_verification）
    /// 后再操作。取消与"回收本收款单的有效核销账本"在同一事务内原子完成：
    /// - 读本收款单的有效核销账本明细（与 payment_verified_total 同一读口径，
    ///   见 active_receipt_ledger_items），锁定其所属核销单；
    /// - 逐核销单回滚所涉发票 received_amount/unpaid_amount/status（与取消核销同一
    ///   回滚实现 rollback_invoices，按本核销单 INVOICE 明细逐条冲减）；
    /// - 核销单 closed→cancelled 并写审计；明细不删行留痕，主单状态是有效性的唯一
    ///   权威标记，退出有效核销集合后收款占用额随之释放；
    /// - 收款单状态置 `cancelled`，清空 confirmed_by/confirmed_at（pending 状态本应为
    ///   空，防御性清理）。
    /// 任一步失败（? 上抛、事务未提交即回滚），绝不出现"收款单已取消而发票侧仍记
    /// 已收、核销单仍 closed"的半成品态。加锁顺序为核销单→发票→收款单，与取消核销
    /// 路径同向、与核销路径（发票→收款单）不交叉，避免并发死锁。
    /// 全部状态变更经 update_with_audit 记录审计日志。
    pub async fn cancel_collection(
        &self,
        payment_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        // 不加锁快速失败预检（存在性 + pending），权威判定在取得全部行锁后的复核
        let precheck = ar_collection::Entity::find_by_id(payment_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        Self::ensure_collection_cancellable(&precheck, user_id, "precheck")?;

        // 1. 读本收款单有效核销账本明细，定位待回收的核销单（ID 排序后加锁，保证并发下顺序一致）
        let ledger_items =
            Self::active_receipt_ledger_items(&txn, std::slice::from_ref(&payment_id)).await?;
        let mut rec_ids: Vec<i32> = ledger_items.iter().map(|i| i.reconciliation_id).collect();
        rec_ids.sort_unstable();
        rec_ids.dedup();

        // 2. 锁核销单；已被并发取消（非 closed）的跳过，避免重复回滚发票金额
        let mut recoverable = Vec::new();
        for rec_id in rec_ids {
            let reconciliation = Self::lock_reconciliation_for_cancel(&txn, rec_id).await?;
            if reconciliation.reconciliation_status.as_deref()
                != Some(crate::models::status::ar::RECONCILIATION_CLOSED)
            {
                warn!(
                    target: "business_audit",
                    event = "AR_COLLECTION_CANCEL_REC_ALREADY_RECOVERED",
                    payment_id = payment_id,
                    reconciliation_id = rec_id,
                    status = ?reconciliation.reconciliation_status,
                    operator = user_id,
                    "AR 收款取消：关联核销单已非 closed（并发取消），跳过重复回收"
                );
                continue;
            }
            recoverable.push(reconciliation);
        }

        // 3. 汇总各待回收核销单的 INVOICE 明细并批量锁定发票
        let mut invoice_items: Vec<ar_reconciliation_item::Model> = Vec::new();
        for reconciliation in &recoverable {
            invoice_items.extend_from_slice(
                &Self::load_cancel_invoice_items(&txn, reconciliation.id).await?,
            );
        }
        let inv_ids: Vec<i32> = invoice_items.iter().filter_map(|i| i.document_id).collect();
        let mut inv_map = Self::lock_invoices_for_cancel(&txn, inv_ids).await?;

        // 4. 回滚发票侧金额与状态（缺失发票即报错整体回滚，不吞不跳过）
        Self::rollback_invoices(&txn, &invoice_items, &mut inv_map, user_id).await?;

        // 5. 核销单置 cancelled（留痕不删明细，账本读数随主单状态退出有效集合）
        let now = Utc::now();
        for reconciliation in recoverable {
            Self::mark_reconciliation_cancelled(&txn, reconciliation, now, user_id).await?;
        }

        // 6. 加锁复核收款状态（权威门）后置 cancelled
        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        Self::ensure_collection_cancellable(&collection, user_id, "locked")?;

        let active = Self::build_cancelled_active(collection);

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(&txn, "ar_collection", active, Some(user_id))
        .await?;

        txn.commit().await?;

        info!(
            "AR 收款单取消成功：payment_id={}, operator={}, 回收核销明细数={}",
            payment_id,
            user_id,
            invoice_items.len()
        );

        Ok(collection_to_json(updated))
    }

    /// 收款单取消状态门（预检与加锁复核共用）：仅 pending 可取消，confirmed 请先走
    /// 核销模块取消关联核销单；拒绝记录 warn 审计日志。收款单取消路径不再因"已被
    /// 核销明细引用"拒绝——账本回收本身是同事务职责（见 cancel_collection）。
    fn ensure_collection_cancellable(
        collection: &ar_collection::Model,
        user_id: i32,
        phase: &str,
    ) -> Result<(), AppError> {
        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            warn!(
                target: "business_audit",
                event = "AR_COLLECTION_CANCEL_REJECTED",
                payment_id = collection.id,
                status = %collection.status,
                operator = user_id,
                phase = phase,
                "AR 收款取消被拒：状态非 pending"
            );
            return Err(AppError::bad_request(format!(
                "收款单状态为 {}，仅 pending 状态可直接取消；confirmed 状态请先取消关联核销单",
                collection.status
            )));
        }
        Ok(())
    }

    /// 构建取消状态的 ActiveModel（status=cancelled，清空 confirmed_by/at）
    fn build_cancelled_active(collection: ar_collection::Model) -> ar_collection::ActiveModel {
        let now = Utc::now();
        let mut active: ar_collection::ActiveModel = collection.into();
        active.status = Set(crate::models::status::ar::COLLECTION_CANCELLED.to_string());
        active.confirmed_by = Set(None);
        active.confirmed_at = Set(None);
        active.updated_at = Set(now);
        active
    }
}
