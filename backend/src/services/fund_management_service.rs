use crate::models::fund_management;
use crate::models::fund_transfer_record;
// 批次 210 P2-5 修复（v12 复审）：资金账户状态字符串替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use tracing::info;

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
        // 输入校验：转账金额必须大于零，防止 0 或负数转账破坏账户余额一致性
        if req.amount <= Decimal::ZERO {
            return Err(AppError::validation("转账金额必须大于零"));
        }
        // 手续费可为 0 但不能为负
        if let Some(fee) = req.fee {
            if fee < Decimal::ZERO {
                return Err(AppError::validation("手续费不能为负"));
            }
        }

        use sea_orm::TransactionTrait;
        let txn = self.db.begin().await?;

        // 1. 扣减转出账户
        let from_acc = crate::models::fund_management::Entity::find_by_id(req.from_account_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("From account not found"))?;
        let total_deduct = req.amount + req.fee.unwrap_or_default();
        if from_acc.available_balance < total_deduct {
            return Err(AppError::validation("Insufficient balance"));
        }
        let mut from_active: crate::models::fund_management::ActiveModel = from_acc.clone().into();

        let from_balance = from_acc.balance;
        let from_available_balance = from_acc.available_balance;

        from_active.balance = sea_orm::Set(from_balance - total_deduct);
        from_active.available_balance = sea_orm::Set(from_available_balance - total_deduct);
        let _from_acc = from_active.update(&txn).await?;

        // 2. 增加转入账户
        let to_acc = crate::models::fund_management::Entity::find_by_id(req.to_account_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("To account not found"))?;
        let mut to_active: crate::models::fund_management::ActiveModel = to_acc.clone().into();

        let to_balance = to_acc.balance;
        let to_available_balance = to_acc.available_balance;

        to_active.balance = sea_orm::Set(to_balance + req.amount);
        to_active.available_balance = sea_orm::Set(to_available_balance + req.amount);
        to_active.update(&txn).await?;

        // 3. 记录 Transfer
        let transfer_no = format!("TR{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let record = crate::models::fund_transfer_record::ActiveModel {
            transfer_no: sea_orm::Set(transfer_no),
            from_account_id: sea_orm::Set(Some(req.from_account_id)),
            to_account_id: sea_orm::Set(Some(req.to_account_id)),
            transfer_date: sea_orm::Set(chrono::Local::now().naive_local().date()),
            amount: sea_orm::Set(req.amount),
            transfer_type: sea_orm::Set("TRANSFER".to_string()),
            status: sea_orm::Set(Some("COMPLETED".to_string())),
            purpose: sea_orm::Set(req.reason),
            applied_by: sea_orm::Set(Some(user_id)),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(record)
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

    /// 查询转账记录详情
    pub async fn get_transfer_record(
        &self,
        id: i32,
    ) -> Result<fund_transfer_record::Model, AppError> {
        fund_transfer_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("资金转账记录"))
    }
}
