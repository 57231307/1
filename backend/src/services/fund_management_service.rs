use crate::models::fund_management;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
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

        let total = query.clone().count(&*self.db).await?;

        let accounts = query
            .order_by(fund_management::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

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
            status: Set("active".to_string()),
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
            .ok_or_else(|| AppError::NotFound(format!("资金账户不存在：{}", id)))?;
        Ok(account)
    }

    /// 账户存款
    pub async fn deposit(
        &self,
        account_id: i32,
        amount: Decimal,
        user_id: i32,
        _remark: Option<String>,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在向账户 {} 存款 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if account.status != "active" {
            return Err(AppError::ValidationError("账户状态非活跃".to_string()));
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
        info!(
            "用户 {} 正在从账户 {} 取款 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if account.status != "active" {
            return Err(AppError::ValidationError("账户状态非活跃".to_string()));
        }

        if amount > account.available_balance {
            return Err(AppError::ValidationError("可用余额不足".to_string()));
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
        info!(
            "用户 {} 正在冻结账户 {} 资金 {:.2}，原因：{}",
            user_id, account_id, amount, reason
        );

        let account = self.get_account_by_id(account_id).await?;

        if amount > account.available_balance {
            return Err(AppError::ValidationError("可用余额不足".to_string()));
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
        info!(
            "用户 {} 正在解冻账户 {} 资金 {:.2}",
            user_id, account_id, amount
        );

        let account = self.get_account_by_id(account_id).await?;

        if amount > account.frozen_balance {
            return Err(AppError::ValidationError("冻结余额不足".to_string()));
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
            return Err(AppError::ValidationError(
                "账户余额不为零，无法删除".to_string(),
            ));
        }

        fund_management::Entity::delete_many()
            .filter(fund_management::Column::Id.eq(account_id))
            .exec(&*self.db)
            .await?;

        info!("账户 {} 删除成功", account_id);
        Ok(())
    }

    pub async fn transfer_fund(&self, req: crate::models::dto::fund_dto::TransferFundRequest, user_id: i32) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        use sea_orm::TransactionTrait;
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // 1. 扣减转出账户
        let from_acc = crate::models::fund_management::Entity::find_by_id(req.from_account_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("From account not found".into()))?;
        let total_deduct = req.amount + req.fee.unwrap_or_default();
        if from_acc.available_balance < total_deduct {
            return Err(AppError::ValidationError("Insufficient balance".into()));
        }

        let from_balance = from_acc.balance;
        let from_available_balance = from_acc.available_balance;
        let mut from_active: crate::models::fund_management::ActiveModel = from_acc.into();
        
        from_active.balance = sea_orm::Set(from_balance - total_deduct);
        from_active.available_balance = sea_orm::Set(from_available_balance - total_deduct);
        let from_acc = from_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 2. 增加转入账户
        let to_acc = crate::models::fund_management::Entity::find_by_id(req.to_account_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("To account not found".into()))?;

        let to_balance = to_acc.balance;
        let to_available_balance = to_acc.available_balance;
        let mut to_active: crate::models::fund_management::ActiveModel = to_acc.into();
        
        to_active.balance = sea_orm::Set(to_balance + req.amount);
        to_active.available_balance = sea_orm::Set(to_available_balance + req.amount);
        to_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 3. 记录 Transfer
        let transfer_no = format!("TR{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let record = crate::models::fund_transfer_record::ActiveModel {
            transfer_no: sea_orm::Set(transfer_no),
            from_account_id: sea_orm::Set(req.from_account_id),
            to_account_id: sea_orm::Set(req.to_account_id),
            transfer_date: sea_orm::Set(chrono::Local::now().naive_local().date()),
            amount: sea_orm::Set(req.amount),
            status: sea_orm::Set("COMPLETED".to_string()),
            remarks: sea_orm::Set(req.reason),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        }.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(record)
    }
}
