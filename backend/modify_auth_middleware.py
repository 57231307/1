import re

with open('src/middleware/auth.rs', 'r') as f:
    content = f.read()

# Check length of state.cookie_secret.as_bytes().
# Wait, the user suggests `if key.len() < 32 { return Err(StatusCode::INTERNAL_SERVER_ERROR); }`
# Wait, axum_extra::extract::cookie::Key doesn't have a `.len()` method because it's a fixed size 64-byte key internally (master key).
# The user probably means `state.cookie_secret.len() < 32`.

old_key_extract = '''    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let token_from_cookie = cookie_jar.private(&key).get("jwt").map(|c| c.value().to_string());'''

new_key_extract = '''    if state.cookie_secret.len() < 32 {
        tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全处理请求");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let token_from_cookie = cookie_jar.private(&key).get("jwt").map(|c| c.value().to_string());'''

content = content.replace(old_key_extract, new_key_extract)

with open('src/middleware/auth.rs', 'w') as f:
    f.write(content)

# We should also add this check to handlers/auth_handler.rs for consistency (in login and logout).
with open('src/handlers/auth_handler.rs', 'r') as f:
    content = f.read()

old_login_key = '''            let key = Key::derive_from(state.cookie_secret.as_bytes());
            let mut jar = CookieJar::new();'''

new_login_key = '''            if state.cookie_secret.len() < 32 {
                tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全生成 Cookie");
                return Err(AppError::InternalError("系统配置错误: 加密密钥不足 32 字节".to_string()));
            }
            let key = Key::derive_from(state.cookie_secret.as_bytes());
            let mut jar = CookieJar::new();'''

content = content.replace(old_login_key, new_login_key)

old_logout_key = '''    let mut jar = axum_extra::extract::cookie::CookieJar::new();
    let key = axum_extra::extract::cookie::Key::derive_from(state.cookie_secret.as_bytes());'''

new_logout_key = '''    if state.cookie_secret.len() < 32 {
        tracing::error!("严重安全错误: cookie_secret 长度不足 32 字节，无法安全撤销 Cookie");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, axum::Json(ApiResponse::error("系统配置错误: 加密密钥不足 32 字节"))));
    }
    let mut jar = axum_extra::extract::cookie::CookieJar::new();
    let key = axum_extra::extract::cookie::Key::derive_from(state.cookie_secret.as_bytes());'''

content = content.replace(old_logout_key, new_logout_key)

with open('src/handlers/auth_handler.rs', 'w') as f:
    f.write(content)
