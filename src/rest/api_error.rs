use std::fmt::{Debug, Display, Formatter};

use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct ApiError {
    /// HTTP status code
    code: u16,
    /// Error title
    title: String,
    /// Error message
    message: String,
}

#[allow(dead_code)]
impl ApiError {
    pub fn new(code: StatusCode, message: &str) -> Self {
        ApiError {
            code: code.as_u16(),
            title: code
                .canonical_reason()
                .unwrap_or("Unbekannter Fehler")
                .to_string(),
            message: message.to_string(),
        }
    }

    pub fn new_with_title(code: StatusCode, title: &str, message: &str) -> Self {
        ApiError {
            code: code.as_u16(),
            title: title.to_string(),
            message: message.to_string(),
        }
    }

    pub fn unimplemented() -> Self {
        ApiError::new(StatusCode::NOT_IMPLEMENTED, "Not implemented")
    }

    pub fn bad_request(message: &str) -> Self {
        ApiError::new(StatusCode::BAD_REQUEST, message)
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, &value.into().to_string())
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Debug for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(self);
        if let Ok(json) = json {
            f.write_str(&json)
        } else {
            let mut debug_struct = f.debug_struct("ApiError");
            debug_struct.field("code", &self.code);
            debug_struct.field("message", &self.message);
            debug_struct.finish()
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code()).json(self)
    }
}
