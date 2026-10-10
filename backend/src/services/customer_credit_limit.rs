use crate::models::customer;
use crate::models::customer_credit;
// 批次 208 P2-5 修复（v12 复审）：硬编码 "active"/"inactive" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
// 批次 357 v13 复审 baseline 清零：移除 unused import std::sync::Arc
use tracing::info;

use super::customer_credit_service::{
    CreditLimitAdjustmentRequest, CreditRatingRequest, CustomerCreditService,
};

/// 信用评级与额度操作扩展方法
impl CustomerCreditService {
    /// 创建/更新客户信用评级
    /// 批次 414 技术债务修复：credit_limit 改为 Option<Decimal>，；创建场景：None 时默认为 0（新建必须有初始额度，但允许从 0 开始）；更新场景：None 表示保持原值，Some(v) 表示显式设置新额度（含 Some(0)）
    pub async fn set_credit_rating(
        &self,
        req: CreditRatingRequest,
        user_id: i32,
    ) -> Result<customer_credit::Model, AppError> {
        info!(
            "用户 {} 正在设置客户 {} 的信用评级",
            user_id, req.customer_id
        );

        // 检查客户是否已有信用评级
        let existing = self.get_by_customer_id(req.customer_id).await?;

        let credit = match existing {
            Some(credit) => {
                // 更新现有评级
                let used_credit = credit.used_credit;
                let old_limit = credit.credit_limit;
                // 批次 414：None 保持原值，Some(v) 显式设置
                let new_limit = req.credit_limit.unwrap_or(old_limit);
                let mut credit_active: customer_credit::ActiveModel = credit.into();
                credit_active.credit_level = Set(req.credit_level.or(Some("B".to_string())));
                credit_active.credit_score = Set(req.credit_score.or(Some(60)));
                credit_active.available_credit = Set(new_limit - used_credit);
                credit_active.credit_limit = Set(new_limit);
                credit_active.credit_days = Set(req.credit_days.or(Some(30)));
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &*self.db,
                    "auto_audit",
                    credit_active,
                    // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                    Some(user_id),
                )
                .await?
            }
            None => {
                // FK 前置校验：customer_credit_ratings.customer_id 有外键指向
                // customers(id)；客户行不存在时以 404 业务错误返回，而不是把驱动
                // 约束违例拍平成 500（并发窗口内客户被删的残余场景由
                // map_credit_constraint_error 兜底降级）。
                if customer::Entity::find_by_id(req.customer_id)
                    .one(&*self.db)
                    .await?
                    .is_none()
                {
                    return Err(AppError::not_found("引用客户不存在，无法建立信用评级"));
                }
                // 创建新评级：None 默认为 0
                let limit = req.credit_limit.unwrap_or_default();
                let active_credit = customer_credit::ActiveModel {
                    customer_id: Set(req.customer_id),
                    credit_level: Set(req.credit_level.or(Some("B".to_string()))),
                    credit_score: Set(req.credit_score.or(Some(60))),
                    used_credit: Set(Decimal::ZERO),
                    available_credit: Set(limit),
                    credit_limit: Set(limit),
                    credit_days: Set(req.credit_days.or(Some(30))),
                    status: Set(master_data::ACTIVE.to_string()),
                    ..Default::default()
                };
                active_credit
                    .insert(&*self.db)
                    .await
                    .map_err(AppError::from)
                    .map_err(Self::map_credit_constraint_error)?
            }
        };

        info!(
            "客户 {} 信用评级设置成功，等级：{:?}",
            req.customer_id, credit.credit_level
        );
        Ok(credit)
    }

    /// 占用信用额度
    pub async fn occupy_credit(
        &self,
        customer_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在占用客户 {} 的信用额度 {}",
            user_id, customer_id, amount
        );

        // 使用事务确保原子性
        let txn = (*self.db).begin().await?;

        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if credit.status != master_data::ACTIVE {
            txn.rollback().await?;
            // 状态门：信用评级记录当前非活跃，前置状态未满足，归业务族
            return Err(AppError::business_displayable("客户信用状态非活跃"));
        }

        if amount > credit.available_credit {
            txn.rollback().await?;
            // 额度门：请求超出系统记录的可用额度，属业务族；文案含可用余额数字，
            // 按 error.rs 安全边界不得外显，保持脱敏 business。
            return Err(AppError::business(format!(
                "可用额度不足：请求 {}，可用 {}",
                amount, credit.available_credit
            )));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.clone().into();
        credit_active.used_credit = Set(credit.used_credit + amount);
        credit_active.available_credit = Set(credit.available_credit - amount);
        credit_active.save(&txn).await?;

        txn.commit().await?;

        info!(
            "客户 {} 信用额度占用成功，已占用：{}",
            customer_id,
            credit.used_credit + amount
        );
        Ok(())
    }

    /// 释放信用额度
    pub async fn release_credit(
        &self,
        customer_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在释放客户 {} 的信用额度 {}",
            user_id, customer_id, amount
        );

        // 使用事务确保原子性
        let txn = (*self.db).begin().await?;

        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if amount > credit.used_credit {
            txn.rollback().await?;
            // 额度门：释放量受系统记录的已占用额度约束，属业务族；文案不含数字可外显
            return Err(AppError::business_displayable(
                "释放额度超过已占用额度".to_string(),
            ));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.clone().into();
        credit_active.used_credit = Set(credit.used_credit - amount);
        credit_active.available_credit = Set(credit.available_credit + amount);
        credit_active.save(&txn).await?;

        txn.commit().await?;

        info!(
            "客户 {} 信用额度释放成功，已占用：{}",
            customer_id,
            credit.used_credit - amount
        );
        Ok(())
    }

    /// 调整信用额度
    pub async fn adjust_credit_limit(
        &self,
        req: CreditLimitAdjustmentRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在调整客户 {} 的信用额度，类型：{}",
            user_id, req.customer_id, req.adjustment_type
        );

        // 开启事务并锁定行
        let txn = (*self.db).begin().await?;

        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(req.customer_id))
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("客户 {} 的信用评级不存在", req.customer_id))
            })?;

        let new_limit = match req.adjustment_type.as_str() {
            "increase" => credit.credit_limit + req.amount,
            "decrease" => {
                let decreased = credit.credit_limit - req.amount;
                // 确保降低后的额度不低于已使用额度
                if decreased < credit.used_credit {
                    txn.rollback().await?;
                    // 额度门：调整后额度受系统记录的已使用额度约束，属业务族；文案不含数字可外显
                    return Err(AppError::business_displayable(
                        "降低后的额度不能低于已使用额度".to_string(),
                    ));
                }
                decreased
            }
            _ => {
                txn.rollback().await?;
                return Err(AppError::validation_displayable("无效的额度调整类型"));
            }
        };

        let new_available = new_limit - credit.used_credit;

        // 更新信用额度
        let mut credit_active: customer_credit::ActiveModel = credit.into();
        credit_active.credit_limit = Set(new_limit);
        credit_active.available_credit = Set(new_available);
        credit_active.save(&txn).await?;

        txn.commit().await?;

        info!(
            "客户 {} 信用额度调整成功，新额度：{}",
            req.customer_id, new_limit
        );
        Ok(())
    }

    /// 检查信用额度是否可用
    pub async fn check_credit_available(
        &self,
        customer_id: i32,
        order_amount: Decimal,
    ) -> Result<bool, AppError> {
        let credit = match self.get_by_customer_id(customer_id).await? {
            Some(c) => c,
            None => return Ok(true),
        };

        if credit.status != master_data::ACTIVE {
            return Ok(false);
        }

        Ok(order_amount <= credit.available_credit)
    }

    /// P2 3-20 修复：事务内检查信用额度是否可用，避免 TOCTOU
    /// 原 check_credit_available 用 self.db 查询，与订单提交事务隔离，；并发场景下可能在查询后、提交前信用额度被调整，导致超卖。；新增 _txn 变体，在订单提交事务内查询信用额度。
    pub async fn check_credit_available_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
        order_amount: Decimal,
    ) -> Result<bool, AppError> {
        let credit = match self.get_by_customer_id_txn(txn, customer_id).await? {
            Some(c) => c,
            None => return Ok(true),
        };

        if credit.status != master_data::ACTIVE {
            return Ok(false);
        }

        Ok(order_amount <= credit.available_credit)
    }

    /// 检查信用预警
    pub async fn check_credit_warning(&self, customer_id: i32) -> Result<Option<String>, AppError> {
        let credit = match self.get_by_customer_id(customer_id).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        if credit.credit_limit <= rust_decimal::Decimal::ZERO {
            return Ok(None);
        }

        let usage_rate = credit.used_credit / credit.credit_limit;
        let warning_threshold = Decimal::from(80) / Decimal::from(100);

        if usage_rate >= warning_threshold {
            let usage_percent = (usage_rate * Decimal::from(100))
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0);
            Ok(Some(format!(
                "客户 {} 信用使用率已达 {:.1}%，超过 80% 预警阈值。总额度：{}，已用：{}，可用：{}",
                customer_id,
                usage_percent,
                credit.credit_limit,
                credit.used_credit,
                credit.available_credit
            )))
        } else {
            Ok(None)
        }
    }

    /// 停用客户信用
    /// 批次 86 v2 复审 P2-8 修复：find + 状态门 + update 移入单一事务 + lock_exclusive 串行化；原实现 find（get_by_customer_id 内部 self.db）+ update 在 self.db 上分别执行，无 txn 无 lock，；存在 TOCTOU（并发占用/释放额度会基于过期状态通过检查后停用）
    pub async fn deactivate(&self, customer_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在停用客户 {} 的信用", user_id, customer_id);

        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发状态变更（基于 customer_id 直接查询主键索引）
        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if credit.used_credit > rust_decimal::Decimal::ZERO {
            // 状态门：记录仍有占用额度这一业务前置未满足，不可停用，归业务族；文案不含数字可外显
            return Err(AppError::business_displayable(
                "客户仍有占用额度，无法停用".to_string(),
            ));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.into();
        credit_active.status = Set(master_data::INACTIVE.to_string());
        // 注意：customer_credit 模型没有 updated_by 字段

        credit_active.save(&txn).await?;
        txn.commit().await?;

        info!("客户 {} 信用停用成功", customer_id);
        Ok(())
    }

    /// DB 约束兜底映射：`set_credit_rating` 插入路径在「先查后插」竞态窗口内可能
    /// 撞上游离的 FK(23503)/UNIQUE(23505) 违例，From<DbErr> 会归类为
    /// DatabaseError(DB_RELATION/DB_DUPLICATE)（对外 500）；此处把这两类降级为
    /// 400 业务错误，文案脱敏不带记录 ID，其余错误原样返回（不猜成约束违例，
    /// 判据常量与 utils::error::classify_db_constraint 同源）。
    fn map_credit_constraint_error(err: AppError) -> AppError {
        match &err {
            AppError::DatabaseError(m) if m == err_msg::DB_RELATION => {
                AppError::business_displayable("引用客户不存在，无法建立信用评级")
            }
            AppError::DatabaseError(m) if m == err_msg::DB_DUPLICATE => {
                AppError::business_displayable("该客户信用评级已存在，请刷新后重试")
            }
            _ => err,
        }
    }
}
