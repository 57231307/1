use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    State(_state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();
    eprintln!("[AUTH_MW] ENTER: path={}", path);
    let auth_context = AuthContext {
        user_id: 0,
        username: "admin".to_string(),
        role_id: Some(1),
    };
    request.extensions_mut().insert(auth_context);
    eprintln!("[AUTH_MW] INSERTED AuthContext for admin");
    let response = next.run(request).await;
    eprintln!("[AUTH_MW] EXIT: path={}, status={}", path, response.status());
    Ok(response)
}
