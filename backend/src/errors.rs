use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use utoipa::{ToResponse, ToSchema};
pub enum MyError {
    NotFound,
    InternalServerError,
    ConflictError,
    UnauthorizedError,
    UnproccessableEntityError,
    BadRequest,
    CustomError((u16, String)),
}

#[derive(Serialize, ToSchema, ToResponse)]
pub struct ErrorResponse {
    detail: String,
}

impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        let (statuscode, body) = match self {
            MyError::NotFound => (StatusCode::NOT_FOUND, "Not Found".to_string()),
            MyError::BadRequest => (StatusCode::BAD_REQUEST, "Bad Request".to_string()),
            MyError::ConflictError => (StatusCode::CONFLICT, "Conflict Error".to_string()),
            MyError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            ),
            MyError::UnauthorizedError => (
                StatusCode::UNAUTHORIZED,
                "User Is Not Authorized".to_string(),
            ),
            MyError::UnproccessableEntityError => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Unproccessable Entity".to_string(),
            ),
            MyError::CustomError(code) => (StatusCode::from_u16(code.0).unwrap(), code.1),
        };
        (statuscode, Json(json!(ErrorResponse { detail: body }))).into_response()
    }
}
