import re

with open('src/handlers/auth_handler.rs', 'r') as f:
    content = f.read()

old_parse = 'removal_cookie.to_string().parse().unwrap()'
new_parse = '''removal_cookie.to_string().parse().map_err(|e| {
            tracing::error!("清理 Cookie 失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ApiResponse::error("Cookie 删除失败"))
            )
        })?'''

content = content.replace(old_parse, new_parse)

# Wait, let's make sure there isn't another unwrap on set_cookie in login.
old_login_parse = 'cookie.to_string().parse().unwrap()'
new_login_parse = '''cookie.to_string().parse().map_err(|e| {
                tracing::error!("写入 Cookie 失败: {}", e);
                AppError::InternalError("Cookie 写入失败".to_string())
            })?'''

content = content.replace(old_login_parse, new_login_parse)

with open('src/handlers/auth_handler.rs', 'w') as f:
    f.write(content)
