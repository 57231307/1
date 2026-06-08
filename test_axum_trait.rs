use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};

pub struct MyExtractor;

impl<S> FromRequestParts<S> for MyExtractor
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(MyExtractor)
    }
}
