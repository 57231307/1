#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use super::*;

    /// 辅助函数：从 IntoResponse 提取 body JSON
    async fn extract_body_json(response: Response) -> serde_json::Value {
        let body_bytes = to_bytes(response.into_body(), 65536)
            .await
            .expect("读取响应体失败");
        serde_json::from_slice(&body_bytes).expect("响应体不是合法 JSON")
    }

    /// 漏洞 #11 测试：生产环境响应（APP_ENV=production）**不含** `error_type` 字段
    #[tokio::test]
    async fn test_production_response_omits_error_type() {
        // 强制设置生产环境
        unsafe { std::env::set_var("APP_ENV", "production"); }
        let err = AppError::DatabaseError("connection refused".to_string());
        let response = err.into_response();
        let body_json = extract_body_json(response).await;
        assert!(
            body_json.get("error_type").is_none(),
            "生产环境响应不应包含 error_type 字段，实际 body: {}",
            body_json
        );
        // 验证 code + message 仍存在（脱敏后保留基本信息）
        assert!(body_json.get("code").is_some(), "生产环境响应应包含 code");
        assert!(
            body_json.get("message").is_some(),
            "生产环境响应应包含 message"
        );
        unsafe { std::env::remove_var("APP_ENV"); }
    }

    /// 漏洞 #11 测试：生产环境响应（APP_ENV=production）**不含** `detail` 字段
    #[tokio::test]
    async fn test_production_response_omits_detail() {
        unsafe { std::env::set_var("APP_ENV", "production"); }
        let err = AppError::ValidationError("字段 email 格式错误".to_string());
        let response = err.into_response();
        let body_json = extract_body_json(response).await;
        assert!(
            body_json.get("detail").is_none(),
            "生产环境响应不应包含 detail 字段，实际 body: {}",
            body_json
        );
        unsafe { std::env::remove_var("APP_ENV"); }
    }

    /// 漏洞 #4 / #8 修复测试：开发环境响应**也不包含** `error_type` 和 `detail` 字段
    #[tokio::test]
    async fn test_development_response_omits_error_type_and_detail() {
        // 确保不是 production
        unsafe { std::env::remove_var("APP_ENV"); }
        let err = AppError::NotFound("用户 ID=42".to_string());
        let response = err.into_response();
        let body_json = extract_body_json(response).await;
        assert!(
            body_json.get("error_type").is_none(),
            "开发环境响应也不应包含 error_type 字段，实际 body: {}",
            body_json
        );
        assert!(
            body_json.get("detail").is_none(),
            "开发环境响应也不应包含 detail 字段，实际 body: {}",
            body_json
        );
        // 验证 message 已是脱敏文案（"用户 ID=42" 不会泄露）
        let message = body_json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !message.contains("ID=42"),
            "开发环境 message 也不应泄露原始 msg，实际 message: {}",
            message
        );
    }

    /// 漏洞 #4 修复测试：DatabaseError 响应脱敏
    #[tokio::test]
    async fn test_database_error_response_is_sanitized() {
        unsafe { std::env::remove_var("APP_ENV"); }
        let sensitive = "duplicate key value violates unique constraint \"users_email_key\"";
        let err = AppError::DatabaseError(sensitive.to_string());
        let response = err.into_response();
        let body_json = extract_body_json(response).await;
        let message = body_json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !message.contains("users_email_key") && !message.contains("duplicate"),
            "DatabaseError 响应不应泄露约束名/SQL 片段，实际 message: {}",
            message
        );
    }

    /// 漏洞 #12 反向测试：to_response() 在生产环境下返回脱敏 message
    #[tokio::test]
    async fn test_to_response_uses_public_message_in_production() {
        unsafe { std::env::set_var("APP_ENV", "production"); }
        let err = AppError::DatabaseError("internal SQL: SELECT * FROM secrets".to_string());
        let response = err.to_response();
        // 脱敏后不应包含原始 SQL 片段
        assert!(
            !response.message.contains("secrets"),
            "生产环境 message 不应泄露内部细节，实际 message: {}",
            response.message
        );
        // 脱敏后应包含通用文案
        assert!(
            response.message.contains("数据库错误") || response.message.contains("服务器"),
            "生产环境 message 应为脱敏文案，实际 message: {}",
            response.message
        );
        unsafe { std::env::remove_var("APP_ENV"); }
    }

    /// 漏洞 #12 反向测试：to_response() 在非生产环境下也使用脱敏 message
    #[tokio::test]
    async fn test_to_response_uses_public_message_in_development() {
        unsafe { std::env::remove_var("APP_ENV"); }
        let err = AppError::DatabaseError("connection timeout with secrets table".to_string());
        let response = err.to_response();
        // 开发环境也不再泄露原始 msg
        assert!(
            !response.message.contains("secrets")
                && !response.message.contains("connection timeout"),
            "开发环境 message 也不应泄露原始 msg，实际 message: {}",
            response.message
        );
        // 脱敏后应包含通用文案
        assert!(
            response.message.contains("数据库错误") || response.message.contains("服务器"),
            "开发环境 message 应为脱敏文案，实际 message: {}",
            response.message
        );
    }
}