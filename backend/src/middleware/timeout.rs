use std::time::Duration;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{info, warn};

pub struct TimeoutConfig {
    pub default_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
        }
    }
}

pub async fn timeout_middleware(
    State(state): State<TimeoutConfig>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    
    tokio::select! {
        result = next.run(request) => {
            match result {
                Ok(response) => {
                    info!(
                        method = %method,
                        path = %path,
                        "请求成功"
                    );
                    Ok(response)
                }
                Err(status) => {
                    warn!(
                        method = %method,
                        path = %path,
                        status = %status,
                        "请求失败"
                    );
                    Err(status)
                }
            }
        }
        _ = tokio::time::sleep(state.default_timeout) => {
            warn!(
                method = %method,
                path = %path,
                timeout_secs = %state.default_timeout.as_secs(),
                "请求超时"
            );
            Err(StatusCode::REQUEST_TIMEOUT)
        }
    }
}

pub fn create_timeout_layer() -> axum::middleware::FromFnLayer<
    fn(axum::extract::State<TimeoutConfig>, Request<Body>, Next) -> _, 
    TimeoutConfig,
    ()
> {
    axum::middleware::from_fn_with_state(
        TimeoutConfig::default(),
        timeout_middleware,
    )
}
