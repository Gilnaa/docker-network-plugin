use async_trait::async_trait;
use axum::http::HeaderValue;
use axum::response::Response;
use axum::BoxError;
use axum::{
    body::HttpBody,
    extract::{rejection::*, FromRequest, RequestParts},
    response::IntoResponse,
};
use serde::{de::DeserializeOwned, Serialize};

/// Like `axum::Json` but it doesn't care if the incoming request has
/// a proper JSON Content-Type.
///
/// This is needed because Docker doesn't send it to plugins.
#[derive(Debug, Clone, Copy, Default)]
pub struct PermissiveJson<T>(pub T);

#[async_trait]
impl<T, B> FromRequest<B> for PermissiveJson<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = JsonRejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        if !req.headers().contains_key("Content-Type") {
            req.headers_mut()
                .append("Content-Type", HeaderValue::from_static("application/json"));
        }
        axum::Json::from_request(req)
            .await
            .map(|axum::Json(req)| Self(req))
    }
}

impl<T> From<T> for PermissiveJson<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> IntoResponse for PermissiveJson<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
