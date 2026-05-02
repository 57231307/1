import re

with open('src/handlers/auth_handler.rs', 'r') as f:
    content = f.read()

# Make sure logout clears the cookie properly!
# It currently only processes the Authorization header.
# We need to make it clear the Cookie too, otherwise the user remains logged in if they used Cookie.

old_logout_sig = '''pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<LogoutResponse>>, (StatusCode, Json<ApiResponse<()>>)> {'''

new_logout_sig = '''pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<axum::response::Response, (StatusCode, Json<ApiResponse<()>>)> {'''

content = content.replace(old_logout_sig, new_logout_sig)

old_logout_return = '''    Ok(Json(ApiResponse::success(LogoutResponse { success: true })))
}'''

new_logout_return = '''    let mut jar = axum_extra::extract::cookie::CookieJar::new();
    let key = axum_extra::extract::cookie::Key::derive_from(state.cookie_secret.as_bytes());
    
    // Clear the cookie by setting it to empty with max_age=0
    let mut cookie = axum_extra::extract::cookie::Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict);
    let cookie = cookie.build();
    
    // Actually axum_extra CookieJar has a remove method. Wait, we need to send Set-Cookie to client.
    // In axum_extra, to remove a private cookie, we don't necessarily need private encryption for removal, but let's just send an expired cookie.
    
    let mut resp = axum::response::IntoResponse::into_response(
        axum::Json(ApiResponse::success(LogoutResponse { success: true }))
    );
    
    // Set max_age to 0 to delete
    let removal_cookie = axum_extra::extract::cookie::Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .max_age(axum_extra::extract::cookie::cookie::time::Duration::ZERO)
        .build();
        
    resp.headers_mut().append(
        axum::http::header::SET_COOKIE,
        removal_cookie.to_string().parse().unwrap()
    );

    Ok(resp)
}'''

content = content.replace(old_logout_return, new_logout_return)

with open('src/handlers/auth_handler.rs', 'w') as f:
    f.write(content)
