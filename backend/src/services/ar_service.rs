#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。
use chrono::Utc;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::utils::error::AppError;

/// 应收账款服务
pub struct ArService {
    db: Arc<DatabaseConnection>,
}

impl ArService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ========== 收款管理 ==========

    /// 获取收款列表
    pub async fn list_payments(
        &self,
        _page: u64,
        _page_size: u64,
        _status: Option<String>,
        _customer_id: Option<i32>,
        _payment_no: Option<String>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        // 简化实现：返回空列表
        Ok((vec![], 0))
    }

    /// 获取收款详情
    pub async fn get_payment(&self, payment_id: i32) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": payment_id,
            "payment_no": format!("PAY{}", payment_id),
            "customer_id": 0,
            "amount": "0",
            "payment_method": "bank_transfer",
            "payment_date": "2024-01-01",
            "status": "pending",
            "bank_account": null,
            "remark": null,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    /// 创建收款
    pub async fn create_payment(
        &self,
        customer_id: i32,
        amount: rust_decimal::Decimal,
        payment_method: String,
        payment_date: chrono::NaiveDate,
        bank_account: Option<String>,
        remark: Option<String>,
        _invoice_ids: Option<Vec<i32>>,
        _user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let payment_no = format!("PAY{}", Utc::now().format("%Y%m%d%H%M%S"));

        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": 0,
            "payment_no": payment_no,
            "customer_id": customer_id,
            "amount": amount.to_string(),
            "payment_method": payment_method,
            "payment_date": payment_date,
            "status": "pending",
            "bank_account": bank_account,
            "remark": remark,
        }))
    }

    /// 更新收款
    pub async fn update_payment(
        &self,
        payment_id: i32,
        _payload: serde_json::Value,
        _user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": payment_id,
            "status": "updated",
        }))
    }

    /// 确认收款
    pub async fn confirm_payment(
        &self,
        payment_id: i32,
        _user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": payment_id,
            "status": "confirmed",
        }))
    }

    // ========== 核销管理 ==========

    /// 获取核销列表
    pub async fn list_verifications(
        &self,
        _page: u64,
        _page_size: u64,
        _invoice_id: Option<i32>,
        _payment_id: Option<i32>,
        _status: Option<String>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        // 简化实现：返回空列表
        Ok((vec![], 0))
    }

    /// 获取核销详情
    pub async fn get_verification(
        &self,
        verification_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": verification_id,
            "invoice_id": 0,
            "payment_id": 0,
            "amount": "0",
            "status": "verified",
            "verified_by": 0,
            "verified_at": Utc::now(),
            "remark": null,
        }))
    }

    /// 自动核销
    pub async fn auto_verify(&self, _user_id: i32) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "verified_count": 0,
            "verified_amount": "0",
        }))
    }

    /// 手动核销
    pub async fn manual_verify(
        &self,
        invoice_id: i32,
        payment_id: i32,
        amount: rust_decimal::Decimal,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": 0,
            "invoice_id": invoice_id,
            "payment_id": payment_id,
            "amount": amount.to_string(),
            "status": "verified",
            "verified_by": user_id,
            "verified_at": Utc::now(),
            "remark": remark,
        }))
    }

    /// 取消核销
    pub async fn cancel_verification(
        &self,
        verification_id: i32,
        _user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "id": verification_id,
            "status": "cancelled",
        }))
    }

    /// 获取未核销发票
    pub async fn get_unverified_invoices(
        &self,
        _query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回空列表
        Ok(serde_json::json!([]))
    }

    /// 获取未核销收款
    pub async fn get_unverified_payments(
        &self,
        _query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回空列表
        Ok(serde_json::json!([]))
    }

    // ========== 报表管理 ==========

    /// 获取统计报表
    pub async fn get_statistics_report(
        &self,
        _start_date: Option<chrono::NaiveDate>,
        _end_date: Option<chrono::NaiveDate>,
        _customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回模拟数据
        Ok(serde_json::json!({
            "total_invoices": 0,
            "total_amount": "0",
            "paid_amount": "0",
            "unpaid_amount": "0",
            "overdue_count": 0,
            "overdue_amount": "0",
            "collection_rate": 0.0,
        }))
    }

    /// 获取日报表
    pub async fn get_daily_report(
        &self,
        _start_date: Option<chrono::NaiveDate>,
        _end_date: Option<chrono::NaiveDate>,
        _customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回空列表
        Ok(serde_json::json!([]))
    }

    /// 获取月报表
    pub async fn get_monthly_report(
        &self,
        _start_date: Option<chrono::NaiveDate>,
        _end_date: Option<chrono::NaiveDate>,
        _customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回空列表
        Ok(serde_json::json!([]))
    }

    /// 获取账龄报表
    pub async fn get_aging_report(
        &self,
        _customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回空列表
        Ok(serde_json::json!([]))
    }
}
