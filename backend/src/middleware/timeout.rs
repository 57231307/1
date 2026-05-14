use std::time::Duration;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tower::Layer;
use tracing::{info, warn};

#[derive(Clone)]
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
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    tokio::select! {
        response = next.run(request) => {
            info!(
                method = %method,
                path = %path,
                "请求完成"
            );
            response
        }
        _ = tokio::time::sleep(state.default_timeout) => {
            warn!(
                method = %method,
                path = %path,
                timeout_secs = %state.default_timeout.as_secs(),
                "请求超时"
            );
            Response::builder()
                .status(StatusCode::REQUEST_TIMEOUT)
                .body(Body::empty())
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                })
        }
    }
}

pub fn create_timeout_layer<S>() -> impl Layer<S> {
    axum::middleware::from_fn_with_state::<_, TimeoutConfig, S>(
        TimeoutConfig::default(),
        timeout_middleware,
    )
}
