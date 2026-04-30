use rust_decimal::Decimal;
use tonic::{Request, Status};
use crate::middleware::auth_context::AuthContext;

/// 解析十进制数
pub fn parse_decimal(field_name: &str, value: &str) -> Result<Decimal, Status> {
    value.parse::<Decimal>()
        .map_err(|e| Status::invalid_argument(format!("{}格式错误：{}", field_name, e)))
}

/// 将空字符串转换为 None
pub fn empty_to_option(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 将 0 ID 转换为 None
pub fn id_to_option(id: i32) -> Option<i32> {
    if id == 0 {
        None
    } else {
        Some(id)
    }
}

/// 获取当前调用者的操作人 ID
pub fn operator_id<T>(req: &Request<T>) -> Result<i32, Status> {
    // 尝试从 extensions 获取 (通常由 auth middleware/interceptor 注入)
    if let Some(auth) = req.extensions().get::<AuthContext>() {
        return Ok(auth.user_id);
    }
    
    // 尝试从 metadata 读取 (主要用于测试或没有 interceptor 时的简单直连)
    if let Some(user_id_meta) = req.metadata().get("x-user-id") {
        if let Ok(user_id_str) = user_id_meta.to_str() {
            if let Ok(user_id) = user_id_str.parse::<i32>() {
                return Ok(user_id);
            }
        }
    }
    
    Err(Status::unauthenticated("未获取到当前用户上下文"))
}

/// 统一错误映射
pub fn handle_error(prefix: &str, err: impl std::fmt::Display) -> Status {
    Status::internal(format!("{}：{}", prefix, err))
}
