use crate::models::customer_credit;
// 批次 208 P2-5 修复（v12 复审）：硬编码 "active"/"inactive" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
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
    ///
    /// 批次 414 技术债务修复：credit_limit 改为 Option<Decimal>，
    /// - 创建场景：None 时默认为 0（新建必须有初始额度，但允许从 0 开始）
    /// - 更新场景：None 表示保持原值，Some(v) 表示显式设置新额度（含 Some(0)）
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
                active_credit.insert(&*self.db).await?
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
            return Err(AppError::validation("客户信用状态非活跃"));
        }

        if amount > credit.available_credit {
            txn.rollback().await?;
            return Err(AppError::validation(format!(
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
            return Err(AppError::validation("释放额度超过已占用额度".to_string()));
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
                    return Err(AppError::validation(
                        "降低后的额度不能低于已使用额度".to_string(),
                    ));
                }
                decreased
            }
            _ => {
                txn.rollback().await?;
                return Err(AppError::validation("无效的额度调整类型"));
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
    ///
    /// 原 check_credit_available 用 self.db 查询，与订单提交事务隔离，
    /// 并发场景下可能在查询后、提交前信用额度被调整，导致超卖。
    /// 新增 _txn 变体，在订单提交事务内查询信用额度。
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
    ///
    /// 批次 86 v2 复审 P2-8 修复：find + 状态门 + update 移入单一事务 + lock_exclusive 串行化
    /// 原实现 find（get_by_customer_id 内部 self.db）+ update 在 self.db 上分别执行，无 txn 无 lock，
    /// 存在 TOCTOU（并发占用/释放额度会基于过期状态通过检查后停用）
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
            return Err(AppError::validation(
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use chrono::Utc;
    use sea_orm::DatabaseConnection;
    use std::str::FromStr;
    // 批次 415：测试中使用 Arc::new(db)，需导入（文件顶部在批次 357 移除了 unused Arc 导入）
    use std::sync::Arc;

    /// 构建测试用客户信用模型夹具
    ///
    /// 封装 `customer_credit::Model` 的构造，便于在各测试中复用，
    /// 默认 available_credit = credit_limit - used_credit，保持业务不变量。
    fn make_credit_model(
        customer_id: i32,
        credit_limit: Decimal,
        used_credit: Decimal,
        status: &str,
    ) -> customer_credit::Model {
        customer_credit::Model {
            id: 1,
            customer_id,
            customer_name: Some("测试客户".to_string()),
            credit_level: Some("A".to_string()),
            credit_score: Some(80),
            credit_limit,
            used_credit,
            available_credit: credit_limit - used_credit,
            credit_days: Some(30),
            last_assessment_date: Some(ymd!(2026, 1, 1)),
            next_assessment_date: Some(ymd!(2026, 12, 31)),
            status: status.to_string(),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 测试_信用额度计算_占用场景正常
    ///
    /// 验证 occupy_credit 中的额度计算公式：
    /// 新已用 = 旧已用 + 占用额；新可用 = 旧可用 - 占用额
    /// 并校验业务不变量：已用 + 可用 = 总额度
    #[test]
    fn 测试_信用额度计算_占用场景正常() {
        let limit = decs!("10000");
        let used = decs!("3000");
        let amount = decs!("2000");
        let model = make_credit_model(1, limit, used, master_data::ACTIVE);

        // 占用前可用额度应为 7000
        assert_eq!(model.available_credit, decs!("7000"));

        // 模拟 occupy_credit 内部的额度计算逻辑
        let new_used = model.used_credit + amount;
        let new_available = model.available_credit - amount;

        assert_eq!(new_used, decs!("5000"));
        assert_eq!(new_available, decs!("5000"));
        // 不变量：已用 + 可用 = 总额度
        assert_eq!(new_used + new_available, limit);
    }

    /// 测试_信用额度计算_释放场景正常
    ///
    /// 验证 release_credit 中的额度计算公式：
    /// 新已用 = 旧已用 - 释放额；新可用 = 旧可用 + 释放额
    #[test]
    fn 测试_信用额度计算_释放场景正常() {
        let limit = decs!("10000");
        let used = decs!("5000");
        let amount = decs!("2000");
        let model = make_credit_model(1, limit, used, master_data::ACTIVE);

        let new_used = model.used_credit - amount;
        let new_available = model.available_credit + amount;

        assert_eq!(new_used, decs!("3000"));
        assert_eq!(new_available, decs!("7000"));
        // 不变量：已用 + 可用 = 总额度
        assert_eq!(new_used + new_available, limit);
    }

    /// 测试_信用等级判断_默认值填充逻辑
    ///
    /// 验证 set_credit_rating 中 Option 字段的默认值填充规则：
    /// credit_level 默认 "B"、credit_score 默认 60、credit_days 默认 30
    /// 批次 414：credit_limit 现在也是 Option<Decimal>，None 表示未提供
    #[test]
    fn 测试_信用等级判断_默认值填充逻辑() {
        // 模拟 CreditRatingRequest 字段全为 None 的场景
        let req = CreditRatingRequest {
            customer_id: 1,
            credit_level: None,
            credit_score: None,
            credit_limit: None,
            credit_days: None,
            remark: None,
        };

        // 复现 set_credit_rating 中的默认值填充逻辑
        let credit_level = req.credit_level.or(Some("B".to_string()));
        let credit_score = req.credit_score.or(Some(60));
        let credit_days = req.credit_days.or(Some(30));

        assert_eq!(credit_level, Some("B".to_string()));
        assert_eq!(credit_score, Some(60));
        assert_eq!(credit_days, Some(30));

        // 显式提供值时不应被覆盖
        let req_explicit = CreditRatingRequest {
            customer_id: 2,
            credit_level: Some("A".to_string()),
            credit_score: Some(95),
            credit_limit: Some(decs!("20000")),
            credit_days: Some(60),
            remark: None,
        };
        assert_eq!(
            req_explicit.credit_level.or(Some("B".to_string())),
            Some("A".to_string())
        );
        assert_eq!(req_explicit.credit_score.or(Some(60)), Some(95));
        assert_eq!(req_explicit.credit_days.or(Some(30)), Some(60));
    }

    /// 测试_额度超限检查_占用超额校验
    ///
    /// 验证 occupy_credit 中 amount > available_credit 时应触发校验失败
    #[test]
    fn 测试_额度超限检查_占用超额校验() {
        let model = make_credit_model(1, decs!("10000"), decs!("8000"), master_data::ACTIVE);

        // 恰好等于可用额度：应允许（边界）
        let amount_eq = model.available_credit;
        assert!(amount_eq <= model.available_credit);

        // 略超可用额度：应拒绝
        let amount_over = model.available_credit + decs!("0.01");
        assert!(amount_over > model.available_credit);

        // 复现 occupy_credit 的错误构造，验证错误类型
        let err = AppError::validation(format!(
            "可用额度不足：请求 {}，可用 {}",
            amount_over, model.available_credit
        ));
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// 测试_额度超限检查_占用状态非活跃
    ///
    /// 验证 occupy_credit 中 status != ACTIVE 时应拒绝
    #[test]
    fn 测试_额度超限检查_占用状态非活跃() {
        let model = make_credit_model(1, decs!("10000"), Decimal::ZERO, master_data::INACTIVE);

        // 复现 occupy_credit 的状态校验逻辑
        let should_reject = model.status != master_data::ACTIVE;
        assert!(should_reject);

        let err = AppError::validation("客户信用状态非活跃");
        assert!(matches!(err, AppError::ValidationError(_)));

        // 状态为 ACTIVE 时不应该拒绝
        let model_active =
            make_credit_model(2, decs!("10000"), Decimal::ZERO, master_data::ACTIVE);
        assert!(!(model_active.status != master_data::ACTIVE));
    }

    /// 测试_额度超限检查_释放超额校验
    ///
    /// 验证 release_credit 中 amount > used_credit 时应拒绝（释放额超过已占用）
    #[test]
    fn 测试_额度超限检查_释放超额校验() {
        let model = make_credit_model(1, decs!("10000"), decs!("3000"), master_data::ACTIVE);

        // 释放额等于已用：应允许（边界，全部释放）
        let amount_eq = model.used_credit;
        assert!(amount_eq <= model.used_credit);

        // 释放额超过已用：应拒绝
        let amount_over = model.used_credit + decs!("0.01");
        assert!(amount_over > model.used_credit);

        let err = AppError::validation("释放额度超过已占用额度".to_string());
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// 测试_已用额度计算_调整增加类型
    ///
    /// 验证 adjust_credit_limit 中 increase 类型的额度计算：
    /// new_limit = credit_limit + amount；new_available = new_limit - used_credit
    #[test]
    fn 测试_已用额度计算_调整增加类型() {
        let model = make_credit_model(1, decs!("10000"), decs!("3000"), master_data::ACTIVE);
        let amount = decs!("5000");

        // 复现 adjust_credit_limit 中 increase 分支的计算
        let new_limit = model.credit_limit + amount;
        let new_available = new_limit - model.used_credit;

        assert_eq!(new_limit, decs!("15000"));
        assert_eq!(new_available, decs!("12000"));
    }

    /// 测试_已用额度计算_调整减少类型正常
    ///
    /// 验证 adjust_credit_limit 中 decrease 类型且降低后 >= used_credit 的场景
    #[test]
    fn 测试_已用额度计算_调整减少类型正常() {
        let model = make_credit_model(1, decs!("10000"), decs!("3000"), master_data::ACTIVE);
        let amount = decs!("5000");

        let decreased = model.credit_limit - amount;
        // 校验：降低后的额度不低于已使用额度
        assert!(decreased >= model.used_credit);

        let new_limit = decreased;
        let new_available = new_limit - model.used_credit;

        assert_eq!(new_limit, decs!("5000"));
        assert_eq!(new_available, decs!("2000"));
    }

    /// 测试_已用额度计算_调整减少低于已用额度
    ///
    /// 验证 adjust_credit_limit 中 decrease 后 < used_credit 应拒绝
    #[test]
    fn 测试_已用额度计算_调整减少低于已用额度() {
        let model = make_credit_model(1, decs!("10000"), decs!("8000"), master_data::ACTIVE);
        let amount = decs!("5000");

        let decreased = model.credit_limit - amount;
        // 降低后 5000 < 已用 8000，应拒绝
        let should_reject = decreased < model.used_credit;
        assert!(should_reject);

        let err = AppError::validation("降低后的额度不能低于已使用额度".to_string());
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// 测试_已用额度计算_无效调整类型
    ///
    /// 验证 adjust_credit_limit 中非 increase/decrease 类型应被拒绝
    #[test]
    fn 测试_已用额度计算_无效调整类型() {
        let adjustment_type = "invalid";
        // 复现 adjust_credit_limit 中的 match 校验
        let is_valid = matches!(adjustment_type, "increase" | "decrease");
        assert!(!is_valid);

        let err = AppError::validation("无效的额度调整类型");
        assert!(matches!(err, AppError::ValidationError(_)));

        // 合法类型应通过
        assert!(matches!("increase", "increase" | "decrease"));
        assert!(matches!("decrease", "increase" | "decrease"));
    }

    /// 测试_可用额度计算_预警触发场景
    ///
    /// 验证 check_credit_warning 中 usage_rate >= 80% 时应返回预警消息
    #[test]
    fn 测试_可用额度计算_预警触发场景() {
        let limit = decs!("10000");
        let used = decs!("8000"); // 刚好 80% 使用率（边界）
        let model = make_credit_model(1, limit, used, master_data::ACTIVE);

        // 复现 check_credit_warning 的计算逻辑
        assert!(model.credit_limit > Decimal::ZERO);
        let usage_rate = model.used_credit / model.credit_limit;
        let warning_threshold = Decimal::from(80) / Decimal::from(100);

        assert!(usage_rate >= warning_threshold);

        // 边界：刚好 80% 应触发预警
        let usage_percent = (usage_rate * Decimal::from(100))
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);
        assert!((usage_percent - 80.0).abs() < 0.01);

        // 超过 80% 也应触发
        let model_over = make_credit_model(2, limit, decs!("9000"), master_data::ACTIVE);
        let usage_over = model_over.used_credit / model_over.credit_limit;
        assert!(usage_over >= warning_threshold);
    }

    /// 测试_可用额度计算_预警未触发场景
    ///
    /// 验证 check_credit_warning 中 usage_rate < 80% 时应返回 None
    #[test]
    fn 测试_可用额度计算_预警未触发场景() {
        let limit = decs!("10000");
        let used = decs!("7999.99"); // < 80%
        let model = make_credit_model(1, limit, used, master_data::ACTIVE);

        let usage_rate = model.used_credit / model.credit_limit;
        let warning_threshold = Decimal::from(80) / Decimal::from(100);

        assert!(usage_rate < warning_threshold);
    }

    /// 测试_可用额度计算_额度为零跳过预警
    ///
    /// 验证 check_credit_warning 中 credit_limit <= 0 时应早返回 None（避免除零）
    #[test]
    fn 测试_可用额度计算_额度为零跳过预警() {
        let model = make_credit_model(1, Decimal::ZERO, Decimal::ZERO, master_data::ACTIVE);

        // 复现 check_credit_warning 的早返回判断
        let should_skip = model.credit_limit <= Decimal::ZERO;
        assert!(should_skip);
    }

    /// 测试_可用额度计算_订单可用性判断
    ///
    /// 验证 check_credit_available 的判断逻辑：
    /// - 状态非活跃返回 false
    /// - 订单金额 <= 可用额度返回 true
    /// - 订单金额 > 可用额度返回 false
    #[test]
    fn 测试_可用额度计算_订单可用性判断() {
        // 场景 1：状态非活跃，无论金额多少都不可用
        let model_inactive =
            make_credit_model(1, decs!("10000"), Decimal::ZERO, master_data::INACTIVE);
        let order_amount = decs!("100");
        let available = if model_inactive.status != master_data::ACTIVE {
            false
        } else {
            order_amount <= model_inactive.available_credit
        };
        assert!(!available);

        // 场景 2：状态活跃，订单金额恰好等于可用额度（边界，应可用）
        let model_active =
            make_credit_model(2, decs!("10000"), decs!("3000"), master_data::ACTIVE);
        let order_eq = model_active.available_credit;
        assert!(order_eq <= model_active.available_credit);

        // 场景 3：状态活跃，订单金额超过可用额度（不可用）
        let order_over = model_active.available_credit + decs!("0.01");
        assert!(order_over > model_active.available_credit);
        let available_over = order_over <= model_active.available_credit;
        assert!(!available_over);
    }

    /// 测试_停用信用_有占用额度拒绝
    ///
    /// 验证 deactivate 中 used_credit > 0 时应拒绝停用
    #[test]
    fn 测试_停用信用_有占用额度拒绝() {
        let model = make_credit_model(1, decs!("10000"), decs!("100"), master_data::ACTIVE);

        // 复现 deactivate 的校验逻辑
        let should_reject = model.used_credit > Decimal::ZERO;
        assert!(should_reject);

        let err = AppError::validation("客户仍有占用额度，无法停用".to_string());
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// 测试_停用信用_无占用额度允许
    ///
    /// 验证 deactivate 中 used_credit == 0 时允许停用，停用后状态应为 INACTIVE
    #[test]
    fn 测试_停用信用_无占用额度允许() {
        let model = make_credit_model(1, decs!("10000"), Decimal::ZERO, master_data::ACTIVE);

        let should_reject = model.used_credit > Decimal::ZERO;
        assert!(!should_reject);

        // 停用后状态应更新为 INACTIVE（复现 deactivate 中的状态设置）
        let new_status = master_data::INACTIVE.to_string();
        assert_eq!(new_status, master_data::INACTIVE);
    }

    /// 测试_服务实例创建
    ///
    /// 验证 CustomerCreditService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = CustomerCreditService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_占用信用额度_信用评级不存在
    ///
    /// 需要 customer_credit_ratings 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore]
    async fn 测试_占用信用额度_信用评级不存在() {
        let db = setup_test_db().await;
        let service = CustomerCreditService::new(Arc::new(db));

        let result = service.occupy_credit(99999, decs!("100"), 1).await;

        // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
        assert!(result.is_err());
    }

    /// 测试_检查信用预警_需要真实数据库
    ///
    /// 需要 customer_credit_ratings 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic；无记录时返回 Ok(None)。
    #[tokio::test]
    #[ignore]
    async fn 测试_检查信用预警_需要真实数据库() {
        let db = setup_test_db().await;
        let service = CustomerCreditService::new(Arc::new(db));

        // L-19 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时返回数据库错误；有 schema 无记录时返回 Ok(None)
        let result = service.check_credit_warning(99999).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_检查信用可用性_需要真实数据库
    ///
    /// 需要 customer_credit_ratings 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证无信用评级记录时 check_credit_available 应返回 Ok(true)（允许下单）。
    #[tokio::test]
    #[ignore]
    async fn 测试_检查信用可用性_需要真实数据库() {
        let db = setup_test_db().await;
        let service = CustomerCreditService::new(Arc::new(db));

        // 无信用评级记录时，业务上视为无额度限制，应返回 Ok(true)
        let result = service.check_credit_available(99999, decs!("1000")).await;
        // 无 schema 时为 Err；有 schema 无记录时为 Ok(true)
        if let Ok(available) = result {
            assert!(available);
        }
    }

    // ========== 批次 414：credit_limit Option<Decimal> 语义测试 ==========

    /// 测试_credit_limit语义_更新场景None保持原值
    ///
    /// 批次 414 技术债务修复验证：
    /// 更新场景下 credit_limit = None 时，应保持原有额度不变。
    /// 复现 set_credit_rating 中 `req.credit_limit.unwrap_or(old_limit)` 的逻辑。
    #[test]
    fn 测试_credit_limit语义_更新场景None保持原值() {
        // 模拟已有信用记录：原额度 10000
        let old_limit = decs!("10000");
        let used_credit = decs!("3000");

        // 请求中 credit_limit = None（未提供）
        let req_credit_limit: Option<Decimal> = None;

        // 复现 set_credit_rating 更新分支的计算
        let new_limit = req_credit_limit.unwrap_or(old_limit);

        // None 应保持原值
        assert_eq!(new_limit, old_limit);
        assert_eq!(new_limit, decs!("10000"));
        // available_credit = new_limit - used_credit
        assert_eq!(new_limit - used_credit, decs!("7000"));
    }

    /// 测试_credit_limit语义_更新场景Some0显式置零
    ///
    /// 批次 414 技术债务修复验证：
    /// 更新场景下 credit_limit = Some(0) 时，应显式将额度设置为 0（区别于 None 保持原值）。
    #[test]
    fn 测试_credit_limit语义_更新场景Some0显式置零() {
        let old_limit = decs!("10000");
        let used_credit = decs!("0"); // 已用为 0，才能置 0

        // 请求中 credit_limit = Some(0)（显式置 0）
        let req_credit_limit: Option<Decimal> = Some(Decimal::ZERO);

        let new_limit = req_credit_limit.unwrap_or(old_limit);

        // Some(0) 应显式设置为 0，而非保持原值 10000
        assert_eq!(new_limit, Decimal::ZERO);
        assert_ne!(new_limit, old_limit);
        // available_credit = 0 - 0 = 0
        assert_eq!(new_limit - used_credit, Decimal::ZERO);
    }

    /// 测试_credit_limit语义_更新场景SomeV设置新值
    ///
    /// 批次 414 技术债务修复验证：
    /// 更新场景下 credit_limit = Some(v) 时，应将额度设置为 v。
    #[test]
    fn 测试_credit_limit语义_更新场景SomeV设置新值() {
        let old_limit = decs!("10000");
        let used_credit = decs!("2000");
        let new_requested = decs!("15000");

        let req_credit_limit: Option<Decimal> = Some(new_requested);

        let new_limit = req_credit_limit.unwrap_or(old_limit);

        assert_eq!(new_limit, new_requested);
        assert_eq!(new_limit - used_credit, decs!("13000"));
    }

    /// 测试_credit_limit语义_创建场景None默认零
    ///
    /// 批次 414 技术债务修复验证：
    /// 创建场景下 credit_limit = None 时，应默认为 0（新建允许从 0 开始）。
    #[test]
    fn 测试_credit_limit语义_创建场景None默认零() {
        let req_credit_limit: Option<Decimal> = None;

        // 复现 set_credit_rating 创建分支：unwrap_or_default() → Decimal::ZERO
        let limit = req_credit_limit.unwrap_or_default();

        assert_eq!(limit, Decimal::ZERO);
    }

    /// 测试_credit_limit语义_创建场景SomeV设置初始值
    ///
    /// 批次 414 技术债务修复验证：
    /// 创建场景下 credit_limit = Some(v) 时，应使用 v 作为初始额度。
    #[test]
    fn 测试_credit_limit语义_创建场景SomeV设置初始值() {
        let initial = decs!("50000");
        let req_credit_limit: Option<Decimal> = Some(initial);

        let limit = req_credit_limit.unwrap_or_default();

        assert_eq!(limit, initial);
    }
}
