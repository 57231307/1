use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};





fn sanitize_query(query: Option<&str>) -> String {
    const SENSITIVE_PARAMS: [&str; 6] =
        ["password", "token", "secret", "key", "auth", "access_token"];

    query
        .map(|q| {
            q.split('&')
                .map(|pair| {
                    if let Some((key, _)) = pair.split_once('=') {
                        let key_lower = key.to_lowercase();
                        for sensitive in &SENSITIVE_PARAMS {
                            if key_lower.contains(sensitive) {
                                return format!("{}=***", key);
                            }
                        }
                    }
                    pair.to_string()
                })
                .collect::<Vec<_>>()
                .join("&")
        })
        .unwrap_or_default()
}
