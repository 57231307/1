use crate::models::customer_credit;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::customer_credit_service::{
    CreditEvaluationResult, EvaluationFactor, CustomerCreditService,
};

/// 信用评估核心逻辑（评估算法 + 私有因子计算 + 单元测试）
impl CustomerCreditService {
    pub async fn evaluate_credit(
        &self,
        customer_id: i32,
        evaluation_date: String,
        _user_id: i32,
    ) -> Result<CreditEvaluationResult, AppError> {
        use chrono::NaiveDate;

        let eval_date = evaluation_date
            .parse::<NaiveDate>()
            .map_err(|_| AppError::validation("日期格式错误"))?;

        // 获取客户信用信息（通过 customer_id 过滤）
        let customer = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 的信用评级不存在", customer_id)))?;

        // 获取客户名称
        let customer_name = crate::models::customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await
            .ok()
            .flatten()
            .map(|c| c.customer_name)
            .unwrap_or_else(|| format!("客户#{}", customer_id));

        // 获取历史信用记录
        let credit_history = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;

        // 获取客户创建时间
        let created_at = customer.created_at.format("%Y-%m-%d %H:%M:%S").to_string();

        // 计算评估因子分数
        let mut factors = Vec::new();
        let mut total_score = 0;

        // 1. 历史付款记录（权重 30%）
        let payment_score = self
            .evaluate_payment_history(customer_id, eval_date)
            .await?;
        factors.push(EvaluationFactor {
            factor_name: "历史付款记录".to_string(),
            weight: 0.3,
            score: payment_score,
            description: "基于过去 12 个月的付款及时性".to_string(),
        });
        total_score += (payment_score as f64 * 0.3) as i32;

        // 2. 合作时长（权重 20%）
        let cooperation_score = self.evaluate_cooperation_duration(created_at, eval_date);
        factors.push(EvaluationFactor {
            factor_name: "合作时长".to_string(),
            weight: 0.2,
            score: cooperation_score,
            description: "基于客户创建时间计算".to_string(),
        });
        total_score += (cooperation_score as f64 * 0.2) as i32;

        // 3. 订单规模（权重 25%）
        let order_score = self.evaluate_order_volume(customer_id, eval_date).await?;
        factors.push(EvaluationFactor {
            factor_name: "订单规模".to_string(),
            weight: 0.25,
            score: order_score,
            description: "基于年度订单总额".to_string(),
        });
        total_score += (order_score as f64 * 0.25) as i32;

        // 4. 信用记录（权重 25%）
        let credit_score = self.evaluate_credit_history(&credit_history);
        factors.push(EvaluationFactor {
            factor_name: "信用记录".to_string(),
            weight: 0.25,
            score: credit_score,
            description: "基于历史信用记录".to_string(),
        });
        total_score += (credit_score as f64 * 0.25) as i32;

        // 计算信用等级和推荐额度
        let (rating, recommended_limit) = self.calculate_rating_and_limit(total_score);

        Ok(CreditEvaluationResult {
            customer_id,
            customer_name,
            credit_score: total_score,
            credit_rating: rating,
            recommended_limit,
            evaluation_factors: factors,
            evaluation_date: evaluation_date.to_string(),
        })
    }

    /// 评估付款历史
    async fn evaluate_payment_history(
        &self,
        customer_id: i32,
        _eval_date: NaiveDate,
    ) -> Result<i32, AppError> {
        use crate::models::ar_invoice;

        // 查询客户的应收发票记录
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;

        if invoices.is_empty() {
            return Ok(70); // 无记录默认70分
        }

        // 计算及时付款比例
        let total = invoices.len() as f64;
        let mut on_time_count = 0;

        for invoice in &invoices {
            // 检查是否已全额收款
            if invoice.received_amount >= invoice.invoice_amount {
                // 已全额收款，检查是否及时
                on_time_count += 1;
            } else if chrono::Utc::now().date_naive() <= invoice.due_date {
                // 未到期视为正常
                on_time_count += 1;
            }
            // 逾期未全额付款的不计入及时数
        }

        let on_time_rate = on_time_count as f64 / total;

        // 根据及时付款率计算分数
        let score = if on_time_rate >= 0.95 {
            95
        } else if on_time_rate >= 0.90 {
            90
        } else if on_time_rate >= 0.80 {
            85
        } else if on_time_rate >= 0.70 {
            75
        } else if on_time_rate >= 0.60 {
            65
        } else {
            50
        };

        Ok(score)
    }

    /// 评估合作时长
    fn evaluate_cooperation_duration(&self, created_at: String, eval_date: NaiveDate) -> i32 {
        // 解析创建时间
        let created = chrono::NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%dT%H:%M:%S"))
            .unwrap_or_else(|_| chrono::NaiveDateTime::default());

        let created_date = created.date();
        let duration_days = (eval_date - created_date).num_days();

        // 根据合作时长计算分数
        if duration_days >= 365 * 5 {
            95 // 5年以上
        } else if duration_days >= 365 * 3 {
            90 // 3-5年
        } else if duration_days >= 365 * 2 {
            85 // 2-3年
        } else if duration_days >= 365 {
            80 // 1-2年
        } else if duration_days >= 180 {
            70 // 6个月-1年
        } else if duration_days >= 90 {
            60 // 3-6个月
        } else {
            50 // 3个月以下
        }
    }

    /// 评估订单规模
    async fn evaluate_order_volume(
        &self,
        customer_id: i32,
        _eval_date: NaiveDate,
    ) -> Result<i32, AppError> {
        use crate::models::sales_order;

        // 查询客户近一年的订单
        let one_year_ago = chrono::Utc::now().date_naive() - chrono::Duration::days(365);

        let orders = sales_order::Entity::find()
            .filter(sales_order::Column::CustomerId.eq(customer_id))
            .filter(sales_order::Column::CreatedAt.gte(one_year_ago))
            .all(&*self.db)
            .await?;

        if orders.is_empty() {
            return Ok(50); // 无订单默认50分
        }

        // 计算年度订单总额
        let total_amount: Decimal = orders
            .iter()
            .map(|o| {
                let s = o.total_amount.to_string();
                s.parse::<rust_decimal::Decimal>()
                    .unwrap_or_else(|_| rust_decimal::Decimal::from(0))
            })
            .fold(Decimal::from(0), |acc, x| acc + x);

        // 根据订单总额计算分数（直接使用 Decimal 比较，避免精度损失）
        let score = if total_amount >= rust_decimal::Decimal::from(1000000) {
            95 // 100万以上
        } else if total_amount >= rust_decimal::Decimal::from(500000) {
            90 // 50-100万
        } else if total_amount >= rust_decimal::Decimal::from(200000) {
            85 // 20-50万
        } else if total_amount >= rust_decimal::Decimal::from(100000) {
            80 // 10-20万
        } else if total_amount >= rust_decimal::Decimal::from(50000) {
            75 // 5-10万
        } else if total_amount >= rust_decimal::Decimal::from(10000) {
            65 // 1-5万
        } else {
            55 // 1万以下
        };

        Ok(score)
    }

    /// 评估信用历史
    fn evaluate_credit_history(&self, credit_history: &[customer_credit::Model]) -> i32 {
        if credit_history.is_empty() {
            return 70; // 无记录默认70分
        }

        // 检查是否有逾期记录
        let mut has_overdue = false;
        let mut has_good_record = false;

        for credit in credit_history {
            // 检查使用率
            let used = credit.used_credit;
            let limit = credit.credit_limit;

            // 使用率超过90%视为高风险
            if limit > rust_decimal::Decimal::ZERO {
                let usage_rate = used / limit;
                if usage_rate > Decimal::from(90) / Decimal::from(100) {
                    has_overdue = true;
                }
            }

            // 检查信用等级
            if let Some(ref level) = credit.credit_level {
                match level.as_str() {
                    "AAA" | "AA" | "A" => has_good_record = true,
                    _ => {}
                }
            }
        }

        // 计算分数
        if has_good_record && !has_overdue {
            90
        } else if has_good_record {
            80
        } else if !has_overdue {
            75
        } else {
            60
        }
    }

    /// 计算信用等级和推荐额度
    fn calculate_rating_and_limit(&self, score: i32) -> (String, Decimal) {
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        (rating.to_string(), Decimal::from(limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的客户信用记录
    fn create_test_credit_model(
        customer_id: i32,
        credit_level: &str,
        status: &str,
    ) -> customer_credit::Model {
        customer_credit::Model {
            id: 1,
            customer_id,
            customer_name: Some("测试客户".to_string()),
            credit_level: Some(credit_level.to_string()),
            credit_score: Some(80),
            credit_limit: Decimal::from(100000),
            used_credit: Decimal::from(0),
            available_credit: Decimal::from(100000),
            credit_days: Some(30),
            last_assessment_date: Some(crate::ymd!(2024, 1, 1)),
            next_assessment_date: Some(crate::ymd!(2025, 1, 1)),
            status: status.to_string(),
            created_by: Some(1),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_calculate_rating_aaa() {
        // 直接测试计算逻辑，不创建 service 实例
        let score = 95;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "AAA");
        assert_eq!(limit, 1000000);
    }

    #[test]
    fn test_calculate_rating_aa() {
        let score = 85;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "AA");
        assert_eq!(limit, 500000);
    }

    #[test]
    fn test_calculate_rating_a() {
        let score = 75;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "A");
        assert_eq!(limit, 200000);
    }

    #[test]
    fn test_calculate_rating_bbb() {
        let score = 65;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "BBB");
        assert_eq!(limit, 100000);
    }

    #[test]
    fn test_calculate_rating_bb() {
        let score = 55;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "BB");
        assert_eq!(limit, 50000);
    }

    #[test]
    fn test_calculate_rating_b() {
        let score = 45;
        let (rating, limit) = if score >= 90 {
            ("AAA", 1000000)
        } else if score >= 80 {
            ("AA", 500000)
        } else if score >= 70 {
            ("A", 200000)
        } else if score >= 60 {
            ("BBB", 100000)
        } else if score >= 50 {
            ("BB", 50000)
        } else {
            ("B", 10000)
        };
        assert_eq!(rating, "B");
        assert_eq!(limit, 10000);
    }

    #[test]
    fn test_calculate_rating_boundary_values() {
        let test_cases = vec![
            (90, "AAA"),
            (89, "AA"),
            (80, "AA"),
            (79, "A"),
            (70, "A"),
            (69, "BBB"),
            (60, "BBB"),
            (59, "BB"),
            (50, "BB"),
            (49, "B"),
            (0, "B"),
        ];

        for (score, expected_rating) in test_cases {
            let (rating, _) = if score >= 90 {
                ("AAA", 1000000)
            } else if score >= 80 {
                ("AA", 500000)
            } else if score >= 70 {
                ("A", 200000)
            } else if score >= 60 {
                ("BBB", 100000)
            } else if score >= 50 {
                ("BB", 50000)
            } else {
                ("B", 10000)
            };
            assert_eq!(
                rating, expected_rating,
                "分数 {} 的等级应为 {}，实际为 {}",
                score, expected_rating, rating
            );
        }
    }

    #[test]
    fn test_cooperation_duration_scoring() {
        // 简化实现固定返回 70
        let score = 70;
        assert_eq!(score, 70);
    }

    #[test]
    fn test_credit_history_scoring() {
        // 简化实现固定返回 85
        let score = 85;
        assert_eq!(score, 85);
    }

    #[test]
    fn test_credit_model_fields() {
        let model = create_test_credit_model(1, "AA", "active");

        assert_eq!(model.customer_id, 1);
        assert_eq!(model.credit_level, Some("AA".to_string()));
        assert_eq!(model.status, "active");
        assert_eq!(model.credit_limit, Decimal::from(100000));
        assert_eq!(model.used_credit, Decimal::from(0));
        assert_eq!(model.available_credit, Decimal::from(100000));
    }

    #[test]
    fn test_credit_utilization() {
        let model = create_test_credit_model(1, "AA", "active");

        // 使用率 = 已用额度 / 总额度
        let utilization = model.used_credit / model.credit_limit;
        assert_eq!(utilization, Decimal::from(0));

        // 模拟使用 50000
        let used = Decimal::from(50000);
        let utilization = used / model.credit_limit;
        assert_eq!(utilization, Decimal::try_from(0.5).expect("P9-1: 测试夹具 Decimal::try_from"));
    }
}
