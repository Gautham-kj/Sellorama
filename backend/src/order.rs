use crate::errors::MyError;
use crate::user::{check_session_validity, extract_session_header, GeneralResponse};
use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Form, Json,
};
// use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(FromRow, ToSchema, Serialize,Deserialize)]
pub struct Address {
    address_line_1: String,
    address_line_2: Option<String>,
    city: String,
    country: String,
    pincode: String,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct Order {
    order_id: Uuid,
    address_id: Uuid,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct AddressId {
    address_id: Uuid,
}


#[utoipa::path(post,
    path = "/order/address",
    security(
        ("session_id"=[])
    ),
    responses(
        (status =200 ,body = GeneralResponse),
        (status =500 ,body = GeneralResponse),
        (status =401 ,body = GeneralResponse),
    )
)]
pub async fn create_order_address(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<Address>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"
                INSERT INTO "address" ("user_id","address_line_1", "address_line_2", "city", "country", "pincode")
                VALUES ($1, $2, $3, $4, $5)
                RETURNING "address_id";
            "#;
            match sqlx::query_as::<_, AddressId>(query)
                .bind(user.user_id)
                .bind(form_data.address_line_1)
                .bind(form_data.address_line_2)
                .bind(form_data.city)
                .bind(form_data.country)
                .bind(form_data.pincode)
                .fetch_one(&state.db_pool)
                .await
                .map_err(|_| MyError::InternalServerError)?
            {
                response => Ok((StatusCode::OK, Json(json!(response)))),
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

// pub async create_order(headers:HeaderMap,state: State<AppState>) ->  Result<impl IntoResponse, MyError> {

// }
