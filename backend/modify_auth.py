import re

with open('src/middleware/auth.rs', 'r') as f:
    content = f.read()

check_block = '''    if state.cookie_secret.len() < 32 {
        tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全处理请求");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
'''
content = content.replace(check_block, '')

with open('src/middleware/auth.rs', 'w') as f:
    f.write(content)

with open('src/handlers/auth_handler.rs', 'r') as f:
    content = f.read()

login_check = '''            if state.cookie_secret.len() < 32 {
                tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全生成 Cookie");
                return Err(AppError::InternalError("系统配置错误: 加密密钥不足 32 字节".to_string()));
            }
'''
content = content.replace(login_check, '')

logout_check = '''    if state.cookie_secret.len() < 32 {
        tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全撤销 Cookie");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ApiResponse::error("系统配置错误: 加密密钥不足 32 字节"))));
    }
'''
content = content.replace(logout_check, '')

with open('src/handlers/auth_handler.rs', 'w') as f:
    f.write(content)
