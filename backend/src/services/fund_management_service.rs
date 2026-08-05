use crate::models::fund_management;
use crate::models::fund_transfer_record;
// 批次 210 P2-5 修复（v12 复审）：资金账户状态字符串替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::{Duration, Local, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::info;

/// V15 P1 17.6-D3：资金账户类型常量（不同账户类型对账方式与风控规则不同，需差异化处理。）
pub mod account_type {
    /// 银行账户（需银企对账，支持大额验证）
    pub const BANK: &str = "bank";
    /// 现金账户（无对账，仅手工盘点）
    pub const CASH: &str = "cash";
    /// 支付宝账户（第三方支付对账）
    pub const ALIPAY: &str = "alipay";
    /// 微信支付账户（第三方支付对账）
    pub const WECHAT: &str = "wechat";
}

/// V15 P1 17.6-D3：判断账户类型是否需要银企对账
pub fn requires_reconciliation(account_type: &str) -> bool {
    matches!(
        account_type,
        account_type::BANK | account_type::ALIPAY | account_type::WECHAT
    )
}

/// 资金账户查询参数
#[derive(Debug, Clone, Default)]
pub struct FundAccountQueryParams {
    pub account_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建资金账户请求
#[derive(Debug, Clone)]
pub struct CreateFundAccountRequest {
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub opened_date: Option<chrono::NaiveDate>,
    pub remark: Option<String>,
}

/// 更新资金账户请求
#[derive(Debug, Clone)]
pub struct UpdateFundAccountRequest {
    pub account_name: Option<String>,
    pub bank_name: Option<String>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// V15 P0-B05：大额调拨阈值（§17.6-D1）
/// 金额超过此阈值的调拨必须由前端二次确认（弹窗 + 用户显式确认），；后端校验 `TransferFundRequest.confirm_large == true` 才放行。；注意：rust_decimal 1.x 中 `Decimal::new` 不是 `const fn`，；故使用普通 `fn` 而非 `const`（参考批次 481 经验）。
fn large_transfer_threshold() -> Decimal {
    // 10 万（100,000.00）
    Decimal::new(100_000, 0)
}

/// V15 P1 17.6-D5：免审批阈值（金额 <= 此值自动审批）
fn auto_approve_threshold() -> Decimal {
    // 1 万（10,000.00）
    Decimal::new(10_000, 0)
}

/// V15 P1 17.6-D5：两级审批阈值（金额 > 此值需两级审批）
fn two_level_approval_threshold() -> Decimal {
    // 10 万（100,000.00）
    Decimal::new(100_000, 0)
}

pub struct FundManagementService {
    db: Arc<DatabaseConnection>,
}

impl FundManagementService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取资金账户列表
    pub async fn get_accounts_list(
        &self,
        params: FundAccountQueryParams,
    ) -> Result<(Vec<fund_management::Model>, u64), AppError> {
        let mut query = fund_management::Entity::find();

        if let Some(account_type) = &params.account_type {
            query = query.filter(fund_management::Column::AccountType.eq(account_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(fund_management::Column::Status.eq(status));
        }

        // 批次 266：接入 paginate_with_total，消除手写 count + offset/limit 重复
        // 补 page_size.clamp(1, 100) 防 DoS（原实现仅 clamp page，page_size 无上限保护）
        let paginator = query
            .order_by(fund_management::Column::Id, Order::Desc)
            .paginate(&*self.db, params.page_size.clamp(1, 100) as u64);
        let (accounts, total) =
            paginate_with_total(paginator, params.page.clamp(1, 1000) as u64).await?;

        Ok((accounts, total))
    }

    /// 创建资金账户
    pub async fn create_account(
        &self,
        req: CreateFundAccountRequest,
        user_id: i32,
    ) -> Result<fund_management::Model, AppError> {
        info!("用户 {} 正在创建资金账户：{}", user_id, req.account_no);

        let active_account = fund_management::ActiveModel {
            account_name: Set(req.account_name),
            account_no: Set(req.account_no),
            account_type: Set(req.account_type),
            bank_name: Set(req.bank_name),
            currency: Set(req.currency),
            balance: Set(Decimal::ZERO),
            available_balance: Set(Decimal::ZERO),
            frozen_balance: Set(Decimal::ZERO),
            status: Set(master_data::ACTIVE.to_string()),
            opened_date: Set(req.opened_date),
            remark: Set(req.remark),
            ..Default::default()
        };

        let account = active_account.insert(&*self.db).await?;
        info!("资金账户创建成功：{}", account.account_no);
        Ok(account)
    }

    /// 获取账户详情
    pub async fn get_account_by_id(&self, id: i32) -> Result<fund_management::Model, AppError> {
        let account = fund_management::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("资金账户不存在：{}", id)))?;
        Ok(account)
    }

    /// 更新资金账户
    pub async fn update_account(
        &self,
        id: i32,
        req: UpdateFundAccountRequest,
    ) -> Result<fund_management::Model, AppError> {
        info!("更新资金账户 ID: {}", id);

        let account = self.get_account_by_id(id).await?;
        let mut active: fund_management::ActiveModel = account.into_active_model();

        if let Some(account_name) = req.account_name {
            active.account_name = Set(account_name);
        }
        if let Some(bank_name) = req.bank_name {
            active.bank_name = Set(Some(bank_name));
        }
        if let Some(currency) = req.currency {
            active.currency = Set(currency);
        }
        if let Some(status) = req.status {
            active.status = Set(status);
        }
        if let Some(remark) = req.remark {
            active.remark = Set(Some(remark));
        }

        let updated = active.update(&*self.db).await?;
        info!("资金账户更新成功：{}", updated.account_no);
        Ok(updated)
    }

    /// 账户存款
    pub async fn deposit(
        &self,
        account_id: i32,
        amount: Decimal,
        user_id: i32,
        _remark: Option<String>,
    ) -> Result<(), AppError> {
        // 输入校验：金额必须大于零，防止 0 或负数入账破坏账户余额一致性
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("金额必须大于零"));
        }
        // P2-4 修复（批次 84 v1 复审）：金额精度校验，最多 2 位小数（货币精度）
        if amount.round_dp(2) != amount {
            return Err(AppError::validation("金额精度不能超过 2 位小数"));
        }

        info!(
            "用户 {} 正在向账户 {} 存款 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if account.status != master_data::ACTIVE {
            return Err(AppError::validation("账户状态非活跃"));
        }

        let new_balance = account.balance + amount;
        let new_available_balance = account.available_balance + amount;

        let mut account_active: fund_management::ActiveModel = account.into();
        account_active.balance = Set(new_balance);
        account_active.available_balance = Set(new_available_balance);
        account_active.save(&*self.db).await?;

        info!("账户 {} 存款成功，新余额：{}", account_id, new_balance);
        Ok(())
    }

    /// 账户取款
    pub async fn withdraw(
        &self,
        account_id: i32,
        amount: Decimal,
        user_id: i32,
        _remark: Option<String>,
    ) -> Result<(), AppError> {
        // 输入校验：金额必须大于零，防止 0 或负数取款破坏账户余额一致性
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("金额必须大于零"));
        }
        // P2-4 修复（批次 84 v1 复审）：金额精度校验，最多 2 位小数（货币精度）
        if amount.round_dp(2) != amount {
            return Err(AppError::validation("金额精度不能超过 2 位小数"));
        }

        info!(
            "用户 {} 正在从账户 {} 取款 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if account.status != master_data::ACTIVE {
            return Err(AppError::validation("账户状态非活跃"));
        }

        if amount > account.available_balance {
            return Err(AppError::validation("可用余额不足"));
        }

        let new_balance = account.balance - amount;
        let new_available_balance = account.available_balance - amount;

        let mut account_active: fund_management::ActiveModel = account.into();
        account_active.balance = Set(new_balance);
        account_active.available_balance = Set(new_available_balance);
        account_active.save(&*self.db).await?;

        info!("账户 {} 取款成功，新余额：{}", account_id, new_balance);
        Ok(())
    }

    /// 冻结账户资金
    pub async fn freeze_funds(
        &self,
        account_id: i32,
        amount: Decimal,
        user_id: i32,
        reason: String,
    ) -> Result<(), AppError> {
        // 输入校验：冻结金额必须大于零，防止 0 或负数冻结破坏余额一致性
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("金额必须大于零"));
        }

        info!(
            "用户 {} 正在冻结账户 {} 资金 {:.2}，原因：{}",
            user_id, account_id, amount, reason
        );

        let account = self.get_account_by_id(account_id).await?;

        if amount > account.available_balance {
            return Err(AppError::validation("可用余额不足"));
        }

        let new_available_balance = account.available_balance - amount;
        let new_frozen_balance = account.frozen_balance + amount;

        let mut account_active: fund_management::ActiveModel = account.into();
        account_active.available_balance = Set(new_available_balance);
        account_active.frozen_balance = Set(new_frozen_balance);
        account_active.save(&*self.db).await?;

        info!("账户 {} 资金冻结成功", account_id);
        Ok(())
    }

    /// 解冻账户资金
    pub async fn unfreeze_funds(
        &self,
        account_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 输入校验：解冻金额必须大于零，防止 0 或负数解冻破坏余额一致性
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("金额必须大于零"));
        }

        info!(
            "用户 {} 正在解冻账户 {} 资金 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if amount > account.frozen_balance {
            return Err(AppError::validation("冻结余额不足"));
        }

        let new_available_balance = account.available_balance + amount;
        let new_frozen_balance = account.frozen_balance - amount;

        let mut account_active: fund_management::ActiveModel = account.into();
        account_active.available_balance = Set(new_available_balance);
        account_active.frozen_balance = Set(new_frozen_balance);
        account_active.save(&*self.db).await?;

        info!("账户 {} 资金解冻成功", account_id);
        Ok(())
    }

    /// 删除账户（仅支持无余额账户）
    pub async fn delete_account(&self, account_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除账户 {}", user_id, account_id);

        let account = self.get_account_by_id(account_id).await?;

        if account.balance != Decimal::ZERO {
            return Err(AppError::validation("账户余额不为零，无法删除".to_string()));
        }

        fund_management::Entity::delete_many()
            .filter(fund_management::Column::Id.eq(account_id))
            .exec(&*self.db)
            .await?;

        info!("账户 {} 删除成功", account_id);
        Ok(())
    }

    pub async fn transfer_fund(
        &self,
        req: crate::models::dto::fund_dto::TransferFundRequest,
        user_id: i32,
    ) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        Self::validate_transfer_request(&req)?;

        // V15 P1 17.6-D5：根据金额判断是否需要审批
        let status = if req.amount <= auto_approve_threshold() {
            // 小额免审批，直接执行
            "APPROVED".to_string()
        } else {
            // 需要审批，创建待审批记录
            "PENDING".to_string()
        };

        let record = self.create_transfer_record(&req, user_id, &status).await?;

        // 小额自动审批，直接执行转账
        if status == "APPROVED" {
            self.execute_transfer(record.id, user_id).await?;
            return self.get_transfer_record(record.id).await;
        }

        Ok(record)
    }

    /// V15 P1 17.6-D5：创建转账记录（不执行实际转账）
    async fn create_transfer_record(
        &self,
        req: &crate::models::dto::fund_dto::TransferFundRequest,
        user_id: i32,
        status: &str,
    ) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        let transfer_no = format!("TR{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let record = crate::models::fund_transfer_record::ActiveModel {
            transfer_no: sea_orm::Set(transfer_no),
            from_account_id: sea_orm::Set(Some(req.from_account_id)),
            to_account_id: sea_orm::Set(Some(req.to_account_id)),
            transfer_date: sea_orm::Set(chrono::Local::now().naive_local().date()),
            amount: sea_orm::Set(req.amount),
            transfer_type: sea_orm::Set("TRANSFER".to_string()),
            status: sea_orm::Set(Some(status.to_string())),
            purpose: sea_orm::Set(req.reason.clone()),
            applied_by: sea_orm::Set(Some(user_id)),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;
        Ok(record)
    }

    /// V15 P1 17.6-D5：获取转账记录
    async fn get_transfer_record(
        &self,
        transfer_id: i32,
    ) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        crate::models::fund_transfer_record::Entity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("转账记录不存在：{}", transfer_id)))
    }

    /// V15 P1 17.6-D5：审批通过转账
    pub async fn approve_transfer(
        &self,
        transfer_id: i32,
        approver_id: i32,
    ) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        let record = self.get_transfer_record(transfer_id).await?;

        // 只有待审批状态的记录才能审批
        if record.status.as_deref() != Some("PENDING") {
            return Err(AppError::validation(format!(
                "转账记录 {} 状态为 {:?}，无法审批",
                transfer_id, record.status
            )));
        }

        // 判断是否需要两级审批
        let current_approval_count = record.approved_by.map(|_| 1).unwrap_or(0);
        let needs_two_level = record.amount > two_level_approval_threshold();

        if needs_two_level && current_approval_count == 0 {
            // 第一级审批：更新审批人，保持 pending 状态
            let mut active: crate::models::fund_transfer_record::ActiveModel = record.into();
            active.approved_by = sea_orm::Set(Some(approver_id));
            active.approved_at = sea_orm::Set(Some(chrono::Utc::now().into()));
            let updated = active.update(&*self.db).await?;
            info!("转账 {} 第一级审批通过", transfer_id);
            return Ok(updated);
        }

        // 单级审批完成或第二级审批通过，执行转账
        self.execute_transfer(transfer_id, approver_id).await?;
        self.get_transfer_record(transfer_id).await
    }

    /// V15 P1 17.6-D5：拒绝转账
    pub async fn reject_transfer(
        &self,
        transfer_id: i32,
        _rejector_id: i32,
    ) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        let record = self.get_transfer_record(transfer_id).await?;

        // 只有待审批状态的记录才能拒绝
        if record.status.as_deref() != Some("PENDING") {
            return Err(AppError::validation(format!(
                "转账记录 {} 状态为 {:?}，无法拒绝",
                transfer_id, record.status
            )));
        }

        let mut active: crate::models::fund_transfer_record::ActiveModel = record.into();
        active.status = sea_orm::Set(Some("REJECTED".to_string()));
        let updated = active.update(&*self.db).await?;
        info!("转账 {} 已被拒绝", transfer_id);
        Ok(updated)
    }

    /// V15 P1 17.6-D5：执行实际转账（内部方法）
    async fn execute_transfer(
        &self,
        transfer_id: i32,
        approver_id: i32,
    ) -> Result<(), AppError> {
        let record = self.get_transfer_record(transfer_id).await?;

        let from_account_id = record
            .from_account_id
            .ok_or_else(|| AppError::validation("转出账户ID缺失"))?;
        let to_account_id = record
            .to_account_id
            .ok_or_else(|| AppError::validation("转入账户ID缺失"))?;

        use sea_orm::TransactionTrait;
        let txn = self.db.begin().await?;

        // 从转出账户扣款
        Self::deduct_from_account_txn(&txn, from_account_id, record.amount, None).await?;
        // 向转入账户加款
        Self::credit_to_account_txn(&txn, to_account_id, record.amount).await?;

        // 更新转账记录状态为已完成
        let mut active: crate::models::fund_transfer_record::ActiveModel = record.into();
        active.status = sea_orm::Set(Some("COMPLETED".to_string()));
        active.approved_by = sea_orm::Set(Some(approver_id));
        active.approved_at = sea_orm::Set(Some(chrono::Utc::now().into()));
        active.executed_at = sea_orm::Set(Some(chrono::Utc::now().into()));
        active.update(&txn).await?;

        txn.commit().await?;
        info!("转账 {} 已执行完成", transfer_id);
        Ok(())
    }

    /// V15 P1 17.6-D5：获取待审批转账列表
    pub async fn get_pending_transfers(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<fund_transfer_record::Model>, AppError> {
        let records = fund_transfer_record::Entity::find()
            .filter(fund_transfer_record::Column::Status.eq("PENDING"))
            .order_by(fund_transfer_record::Column::TransferDate, Order::Desc)
            .paginate(&*self.db, page_size)
            .fetch_page(page.saturating_sub(1))
            .await?;

        Ok(records)
    }

    /// 校验转账请求（金额、手续费、大额确认）
    fn validate_transfer_request(
        req: &crate::models::dto::fund_dto::TransferFundRequest,
    ) -> Result<(), AppError> {
        if req.amount <= Decimal::ZERO {
            return Err(AppError::validation("转账金额必须大于零"));
        }
        if let Some(fee) = req.fee {
            if fee < Decimal::ZERO {
                return Err(AppError::validation("手续费不能为负"));
            }
        }
        if req.amount > large_transfer_threshold() && !req.confirm_large {
            return Err(AppError::validation(format!(
                "大额调拨（>{})必须二次确认，请通过 confirm_large=true 显式确认（V15 P0-B05 强制拦截）",
                large_transfer_threshold()
            )));
        }
        Ok(())
    }

    /// 从转出账户扣减金额+手续费
    async fn deduct_from_account_txn(
        txn: &sea_orm::DatabaseTransaction,
        account_id: i32,
        amount: Decimal,
        fee: Option<Decimal>,
    ) -> Result<(), AppError> {
        let acc = crate::models::fund_management::Entity::find_by_id(account_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("From account not found"))?;
        let total_deduct = amount + fee.unwrap_or_default();
        if acc.available_balance < total_deduct {
            return Err(AppError::validation("Insufficient balance"));
        }
        let mut active: crate::models::fund_management::ActiveModel = acc.clone().into();
        active.balance = sea_orm::Set(acc.balance - total_deduct);
        active.available_balance = sea_orm::Set(acc.available_balance - total_deduct);
        active.update(txn).await?;
        Ok(())
    }

    /// 向转入账户增加金额
    async fn credit_to_account_txn(
        txn: &sea_orm::DatabaseTransaction,
        account_id: i32,
        amount: Decimal,
    ) -> Result<(), AppError> {
        let acc = crate::models::fund_management::Entity::find_by_id(account_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("To account not found"))?;
        let mut active: crate::models::fund_management::ActiveModel = acc.clone().into();
        active.balance = sea_orm::Set(acc.balance + amount);
        active.available_balance = sea_orm::Set(acc.available_balance + amount);
        active.update(txn).await?;
        Ok(())
    }

    /// 查询转账记录列表
    pub async fn list_transfer_records(
        &self,
        from_account_id: Option<i32>,
        to_account_id: Option<i32>,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<fund_transfer_record::Model>, AppError> {
        let mut query = fund_transfer_record::Entity::find();

        if let Some(from_id) = from_account_id {
            query = query.filter(fund_transfer_record::Column::FromAccountId.eq(from_id));
        }
        if let Some(to_id) = to_account_id {
            query = query.filter(fund_transfer_record::Column::ToAccountId.eq(to_id));
        }
        if let Some(s) = status {
            query = query.filter(fund_transfer_record::Column::Status.eq(s));
        }

        let records = query
            .order_by(fund_transfer_record::Column::TransferDate, Order::Desc)
            .paginate(&*self.db, page_size)
            .fetch_page(page.saturating_sub(1))
            .await?;

        Ok(records)
    }

    /// V15 P1 17.6-D2：现金流预测
    /// 基于未核销应收发票（到期日收款流入）与未付应付发票（到期日付款流出），；预测未来 N 天（默认 30 天）每日现金流缺口，支持融资决策。；参数：days：预测天数（1-90，默认 30）；返回：按日期聚合的现金流预测点列表（含期初余额累计）
    pub async fn cash_flow_forecast(
        &self,
        days: Option<i32>,
    ) -> Result<Vec<CashFlowForecastPoint>, AppError> {
        let days = days.unwrap_or(30).clamp(1, 90);
        let today = Local::now().date_naive();
        let horizon = today + Duration::days(days as i64);

        // 期初余额：当前所有 active 账户的可用余额合计
        let accounts = fund_management::Entity::find()
            .filter(fund_management::Column::Status.eq(master_data::ACTIVE))
            .all(&*self.db)
            .await?;
        let opening_balance: Decimal = accounts.iter().map(|a| a.available_balance).sum();

        // 应收流入：未核销应收发票（未取消，未付金额>0，到期日在 [today, horizon]）
        let ar_invoices = crate::models::ar_invoice::Entity::find()
            .filter(crate::models::ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(crate::models::ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(crate::models::ar_invoice::Column::DueDate.gte(today))
            .filter(crate::models::ar_invoice::Column::DueDate.lte(horizon))
            .all(&*self.db)
            .await?;

        // 应付流出：未付应付发票（未取消，未付金额>0，到期日在 [today, horizon]）
        let ap_invoices = crate::models::ap_invoice::Entity::find()
            .filter(crate::models::ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(crate::models::ap_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(crate::models::ap_invoice::Column::DueDate.gte(today))
            .filter(crate::models::ap_invoice::Column::DueDate.lte(horizon))
            .all(&*self.db)
            .await?;

        // 按日期聚合
        let mut daily_map: std::collections::HashMap<NaiveDate, (Decimal, Decimal)> =
            std::collections::HashMap::new();
        for inv in &ar_invoices {
            let entry = daily_map
                .entry(inv.due_date)
                .or_insert_with(|| (Decimal::ZERO, Decimal::ZERO));
            entry.0 += inv.unpaid_amount;
        }
        for inv in &ap_invoices {
            let entry = daily_map
                .entry(inv.due_date)
                .or_insert_with(|| (Decimal::ZERO, Decimal::ZERO));
            entry.1 += inv.unpaid_amount;
        }

        // 按日期升序构造预测点，累计余额
        let mut dates: Vec<NaiveDate> = daily_map.keys().copied().collect();
        dates.sort();
        let mut points = Vec::with_capacity(dates.len());
        let mut running = opening_balance;
        for d in dates {
            let (inflow, outflow) = daily_map[&d];
            let net = inflow - outflow;
            running += net;
            points.push(CashFlowForecastPoint {
                date: d,
                inflow,
                outflow,
                net_flow: net,
                projected_balance: running,
            });
        }
        Ok(points)
    }

    /// V15 P1 17.6-D3：按账户类型差异化查询（返回指定账户类型的所有活跃账户，并对不同类型给出风控提示：bank：需银企对账；alipay/wechat：需第三方对账；cash：需手工盘点）
    pub async fn list_accounts_by_type(
        &self,
        account_type: &str,
    ) -> Result<Vec<AccountWithTypeHint>, AppError> {
        let accounts = fund_management::Entity::find()
            .filter(fund_management::Column::AccountType.eq(account_type))
            .filter(fund_management::Column::Status.eq(master_data::ACTIVE))
            .order_by(fund_management::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;
        let reconciliation_required = requires_reconciliation(account_type);
        let control_hint = match account_type {
            account_type::BANK => "银行账户需定期银企对账".to_string(),
            account_type::ALIPAY => "支付宝账户需第三方对账".to_string(),
            account_type::WECHAT => "微信账户需第三方对账".to_string(),
            account_type::CASH => "现金账户需月末手工盘点".to_string(),
            _ => "未知账户类型".to_string(),
        };
        Ok(accounts
            .into_iter()
            .map(|a| AccountWithTypeHint {
                account: a,
                reconciliation_required,
                control_hint: control_hint.clone(),
            })
            .collect())
    }

    /// V15 P1 17.6-D4：银企对账
    /// 对比系统资金账户余额与银行对账单余额，输出差异列表。；差异类型：timing（在途）、missing（系统缺失）、error（系统错误）。；参数：account_id：资金账户 ID（必须为 bank/alipay/wechat 类型）；bank_statement_balance：银行/第三方对账单余额；statement_date：对账单日期
    pub async fn bank_reconciliation(
        &self,
        account_id: i32,
        bank_statement_balance: Decimal,
        statement_date: NaiveDate,
    ) -> Result<BankReconciliationResult, AppError> {
        let account = self.get_account_by_id(account_id).await?;

        if !requires_reconciliation(&account.account_type) {
            return Err(AppError::validation(format!(
                "账户类型 {} 不支持银企对账（仅 bank/alipay/wechat 支持）",
                account.account_type
            )));
        }

        let system_balance = account.balance;
        let difference = bank_statement_balance - system_balance;

        // 查询在途转账：对账单日期当天及之前发起、但状态非 COMPLETED 的转账
        let pending_transfers = fund_transfer_record::Entity::find()
            .filter(fund_transfer_record::Column::FromAccountId.eq(account_id))
            .filter(fund_transfer_record::Column::TransferDate.lte(statement_date))
            .filter(fund_transfer_record::Column::Status.ne("COMPLETED"))
            .all(&*self.db)
            .await?;
        let pending_out: Decimal = pending_transfers.iter().map(|t| t.amount).sum();

        let pending_in_transfers = fund_transfer_record::Entity::find()
            .filter(fund_transfer_record::Column::ToAccountId.eq(account_id))
            .filter(fund_transfer_record::Column::TransferDate.lte(statement_date))
            .filter(fund_transfer_record::Column::Status.ne("COMPLETED"))
            .all(&*self.db)
            .await?;
        let pending_in: Decimal = pending_in_transfers.iter().map(|t| t.amount).sum();

        let timing_diff = pending_in - pending_out;
        let adjusted_difference = difference - timing_diff;

        // 差异分类
        let diff_type = if adjusted_difference.abs() < Decimal::new(1, 2) {
            // |adjusted_difference| < 0.01 视为对平
            "balanced".to_string()
        } else if adjusted_difference > Decimal::ZERO {
            "system_missing".to_string() // 银行有，系统无
        } else {
            "system_excess".to_string() // 系统有，银行无
        };

        Ok(BankReconciliationResult {
            account_id,
            account_no: account.account_no,
            account_name: account.account_name,
            statement_date,
            system_balance,
            bank_statement_balance,
            difference,
            timing_difference: timing_diff,
            adjusted_difference,
            diff_type,
            pending_out_count: pending_transfers.len() as i64,
            pending_in_count: pending_in_transfers.len() as i64,
        })
    }

    /// V15 P1 17.6-D6：资金日报
    /// 汇总指定日期各账户的期初余额、转入/转出、期末余额。
    /// 期初余额 = 当前余额 − 当日净变动（通过反推法计算）。
    pub async fn get_daily_report(
        &self,
        report_date: NaiveDate,
    ) -> Result<DailyReportSummary, AppError> {
        let next_date = report_date + Duration::days(1);

        let accounts = fund_management::Entity::find()
            .filter(fund_management::Column::Status.eq(master_data::ACTIVE))
            .order_by(fund_management::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut account_summaries = Vec::with_capacity(accounts.len());
        let mut total_opening = Decimal::ZERO;
        let mut total_closing = Decimal::ZERO;
        let mut total_inflow = Decimal::ZERO;
        let mut total_outflow = Decimal::ZERO;

        for acc in &accounts {
            // 当日转入（to_account_id = 本账户，status = COMPLETED，transfer_date = 当日）
            let inflow_transfers = fund_transfer_record::Entity::find()
                .filter(fund_transfer_record::Column::ToAccountId.eq(acc.id))
                .filter(fund_transfer_record::Column::TransferDate.gte(report_date))
                .filter(fund_transfer_record::Column::TransferDate.lt(next_date))
                .filter(fund_transfer_record::Column::Status.eq("COMPLETED"))
                .all(&*self.db)
                .await?;
            let day_inflow: Decimal = inflow_transfers.iter().map(|t| t.amount).sum();
            let inflow_count = inflow_transfers.len() as i64;

            // 当日转出（from_account_id = 本账户，status = COMPLETED，transfer_date = 当日）
            let outflow_transfers = fund_transfer_record::Entity::find()
                .filter(fund_transfer_record::Column::FromAccountId.eq(acc.id))
                .filter(fund_transfer_record::Column::TransferDate.gte(report_date))
                .filter(fund_transfer_record::Column::TransferDate.lt(next_date))
                .filter(fund_transfer_record::Column::Status.eq("COMPLETED"))
                .all(&*self.db)
                .await?;
            let day_outflow: Decimal = outflow_transfers.iter().map(|t| t.amount).sum();
            let outflow_count = outflow_transfers.len() as i64;

            let net_change = day_inflow - day_outflow;
            let closing_balance = acc.balance;
            let opening_balance = closing_balance - net_change;

            total_opening += opening_balance;
            total_closing += closing_balance;
            total_inflow += day_inflow;
            total_outflow += day_outflow;

            account_summaries.push(AccountDailySummary {
                account_id: acc.id,
                account_no: acc.account_no.clone(),
                account_name: acc.account_name.clone(),
                account_type: acc.account_type.clone(),
                opening_balance,
                closing_balance,
                total_inflow: day_inflow,
                total_outflow: day_outflow,
                net_change,
                inflow_count,
                outflow_count,
            });
        }

        Ok(DailyReportSummary {
            report_date,
            accounts: account_summaries,
            total_opening_balance: total_opening,
            total_closing_balance: total_closing,
            total_inflow,
            total_outflow,
            total_net_change: total_inflow - total_outflow,
        })
    }

    /// V15 P1 17.6-D6：资金月报
    /// 汇总指定月份各账户的期初余额、总转入/转出、期末余额、日均余额。
    /// 期初余额 = 当前余额 − 月内净变动（反推法）。
    pub async fn get_monthly_report(
        &self,
        year: i32,
        month: u32,
    ) -> Result<MonthlyReportSummary, AppError> {
        let month_start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::validation("无效的年月"))?;
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let month_end = NaiveDate::from_ymd_opt(next_year, next_month, 1)
            .ok_or_else(|| AppError::validation("无效的年月"))?;

        let accounts = fund_management::Entity::find()
            .filter(fund_management::Column::Status.eq(master_data::ACTIVE))
            .order_by(fund_management::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut account_summaries = Vec::with_capacity(accounts.len());
        let mut total_opening = Decimal::ZERO;
        let mut total_closing = Decimal::ZERO;
        let mut total_inflow = Decimal::ZERO;
        let mut total_outflow = Decimal::ZERO;
        let mut total_transfer_count: i64 = 0;
        let mut total_transfer_amount = Decimal::ZERO;

        for acc in &accounts {
            // 月内转入
            let inflow_transfers = fund_transfer_record::Entity::find()
                .filter(fund_transfer_record::Column::ToAccountId.eq(acc.id))
                .filter(fund_transfer_record::Column::TransferDate.gte(month_start))
                .filter(fund_transfer_record::Column::TransferDate.lt(month_end))
                .filter(fund_transfer_record::Column::Status.eq("COMPLETED"))
                .all(&*self.db)
                .await?;
            let month_inflow: Decimal = inflow_transfers.iter().map(|t| t.amount).sum();

            // 月内转出
            let outflow_transfers = fund_transfer_record::Entity::find()
                .filter(fund_transfer_record::Column::FromAccountId.eq(acc.id))
                .filter(fund_transfer_record::Column::TransferDate.gte(month_start))
                .filter(fund_transfer_record::Column::TransferDate.lt(month_end))
                .filter(fund_transfer_record::Column::Status.eq("COMPLETED"))
                .all(&*self.db)
                .await?;
            let month_outflow: Decimal = outflow_transfers.iter().map(|t| t.amount).sum();

            let transfer_count = (inflow_transfers.len() + outflow_transfers.len()) as i64;
            let total_amount = month_inflow + month_outflow;

            let net_change = month_inflow - month_outflow;
            let closing_balance = acc.balance;
            let opening_balance = closing_balance - net_change;

            // 日均余额 = (期初 + 期末) / 2（简化估算）
            let daily_avg_balance = (opening_balance + closing_balance) / Decimal::from(2);

            total_opening += opening_balance;
            total_closing += closing_balance;
            total_inflow += month_inflow;
            total_outflow += month_outflow;
            total_transfer_count += transfer_count;
            total_transfer_amount += total_amount;

            account_summaries.push(AccountMonthlySummary {
                account_id: acc.id,
                account_no: acc.account_no.clone(),
                account_name: acc.account_name.clone(),
                account_type: acc.account_type.clone(),
                opening_balance,
                closing_balance,
                total_inflow: month_inflow,
                total_outflow: month_outflow,
                net_change,
                daily_avg_balance,
                transfer_count,
                total_transfer_amount: total_amount,
            });
        }

        Ok(MonthlyReportSummary {
            year,
            month,
            accounts: account_summaries,
            total_opening_balance: total_opening,
            total_closing_balance: total_closing,
            total_inflow,
            total_outflow,
            total_net_change: total_inflow - total_outflow,
            total_transfer_count,
            total_transfer_amount,
        })
    }
}

/// V15 P1 17.6-D2：现金流预测数据点
#[derive(Debug, Clone, serde::Serialize)]
pub struct CashFlowForecastPoint {
    /// 日期
    pub date: NaiveDate,
    /// 当日流入（应收到期）
    pub inflow: Decimal,
    /// 当日流出（应付到期）
    pub outflow: Decimal,
    /// 当日净流 = 流入 - 流出
    pub net_flow: Decimal,
    /// 累计预计余额（含期初余额）
    pub projected_balance: Decimal,
}

/// V15 P1 17.6-D3：账户 + 类型风控提示
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountWithTypeHint {
    /// 账户模型
    pub account: fund_management::Model,
    /// 是否需要银企对账
    pub reconciliation_required: bool,
    /// 风控提示
    pub control_hint: String,
}

/// V15 P1 17.6-D4：银企对账结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct BankReconciliationResult {
    /// 账户 ID
    pub account_id: i32,
    /// 账户编号
    pub account_no: String,
    /// 账户名称
    pub account_name: String,
    /// 对账单日期
    pub statement_date: NaiveDate,
    /// 系统余额
    pub system_balance: Decimal,
    /// 银行对账单余额
    pub bank_statement_balance: Decimal,
    /// 原始差异 = 银行余额 - 系统余额
    pub difference: Decimal,
    /// 在途差异 = 在途流入 - 在途流出
    pub timing_difference: Decimal,
    /// 调整后差异 = 原始差异 - 在途差异
    pub adjusted_difference: Decimal,
    /// 差异分类：balanced / system_missing / system_excess
    pub diff_type: String,
    /// 在途转出笔数
    pub pending_out_count: i64,
    /// 在途转入笔数
    pub pending_in_count: i64,
}

/// V15 P1 17.6-D6：资金日报 — 每个账户日维度摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountDailySummary {
    pub account_id: i32,
    pub account_no: String,
    pub account_name: String,
    pub account_type: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub net_change: Decimal,
    pub inflow_count: i64,
    pub outflow_count: i64,
}

/// V15 P1 17.6-D6：资金日报汇总
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyReportSummary {
    pub report_date: NaiveDate,
    pub accounts: Vec<AccountDailySummary>,
    pub total_opening_balance: Decimal,
    pub total_closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub total_net_change: Decimal,
}

/// V15 P1 17.6-D6：资金月报 — 每个月度维度摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountMonthlySummary {
    pub account_id: i32,
    pub account_no: String,
    pub account_name: String,
    pub account_type: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub net_change: Decimal,
    pub daily_avg_balance: Decimal,
    pub transfer_count: i64,
    pub total_transfer_amount: Decimal,
}

/// V15 P1 17.6-D6：资金月报汇总
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonthlyReportSummary {
    pub year: i32,
    pub month: u32,
    pub accounts: Vec<AccountMonthlySummary>,
    pub total_opening_balance: Decimal,
    pub total_closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub total_net_change: Decimal,
    pub total_transfer_count: i64,
    pub total_transfer_amount: Decimal,
}
