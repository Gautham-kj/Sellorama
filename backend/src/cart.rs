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

#[derive(FromRow, ToSchema, Deserialize, Serialize)]
pub struct CartItem {
    item_id: Uuid,
    quantity: i32,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct Cart {
    items: Vec<CartItem>,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct CartResponse {
    detail: Cart,
}

#[utoipa::path(
    get,
    path = "/cart",
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 200 , body = CartResponse),
        (status = 401, body = GeneralResponse),
        (status = 500, body = GeneralResponse)
    )
)]
pub async fn get_cart(state: State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let session_id;
    match extract_session_header(headers).await {
        Some(session) => session_id = session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(GeneralResponse {
                    detail: "Invalid Credentials".to_string()
                })),
            )
        }
    }
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"SELECT "item_id","quantity" FROM "cart" WHERE "cart_id" = $1"#;

            match sqlx::query_as::<_, CartItem>(query)
                .bind(user.user_id)
                .fetch_all(&state.db_pool)
                .await
            {
                Ok(cart) => (
                    StatusCode::OK,
                    Json(json!(CartResponse {
                        detail: Cart { items: cart }
                    })),
                ),
                Err(_e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!(GeneralResponse {
                        detail: "Internal Server Error".to_string()
                    })),
                ),
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "User not authorized".to_string()
            })),
        ),
    }
}

#[utoipa::path(post,
    path = "/cart/item",
    security(
        ("session_id"=[])
    ),
    responses(
        (status =200 ,body = GeneralResponse),
        (status =500 ,body = GeneralResponse),
        (status =401 ,body = GeneralResponse),
    )
)]
pub async fn add_item(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<CartItem>,
) -> impl IntoResponse {
    let session_id;
    match extract_session_header(headers).await {
        Some(session) => session_id = session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(GeneralResponse {
                    detail: "Invalid Credentials".to_string()
                })),
            )
        }
    }
    match check_session_validity(&state.db_pool, session_id).await {
        Some(userresponse) => {
            match sqlx::query!(
                r#"INSERT INTO "cart" ("cart_id","item_id","quantity") 
                VALUES ($1,$2,$3) 
                ON CONFLICT("cart_id","item_id")
                DO UPDATE SET "quantity" = "cart"."quantity"+ EXCLUDED."quantity"  "#,
                userresponse.user_id,
                form_data.item_id,
                form_data.quantity
            )
            .execute(&state.db_pool)
            .await
            {
                Ok(_response) => (
                    StatusCode::OK,
                    Json(json!(GeneralResponse {
                        detail: "Cart updated".to_string()
                    })),
                ),
                Err(_e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!(GeneralResponse {
                        detail: "Server Error".to_string()
                    })),
                ),
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Invalid Credentials".to_string()
            })),
        ),
    }
}
#[utoipa::path(
    post,
    path = "/cart/update",
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 401, body = GeneralResponse),
        (status = 200, body = GeneralResponse),
        (status = 500, body = GeneralResponse)
    )
)]
pub async fn update_cart_item(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<CartItem>,
) -> impl IntoResponse {
    let session_id;
    match extract_session_header(headers).await {
        Some(session) => session_id = session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(GeneralResponse {
                    detail: "Invalid Credentials".to_string()
                })),
            )
        }
    }
    match check_session_validity(&state.db_pool, session_id).await {
        Some(userresponse) => {
            if form_data.quantity > 0 {
                match sqlx::query!(
                    r#"UPDATE "cart" SET "quantity" = $3 WHERE "cart_id" = $1 AND "item_id" = $2"#,
                    userresponse.user_id,
                    form_data.item_id,
                    form_data.quantity
                )
                .execute(&state.db_pool)
                .await
                {
                    Ok(_response) => (
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Cart updated".to_string()
                        })),
                    ),
                    Err(_e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Server Error".to_string()
                        })),
                    ),
                }
            } else {
                match sqlx::query!(
                    r#"DELETE FROM "cart" WHERE "cart_id" = $1 AND "item_id" = $2"#,
                    userresponse.user_id,
                    form_data.item_id,
                )
                .execute(&state.db_pool)
                .await
                {
                    Ok(_response) => (
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Cart updated".to_string()
                        })),
                    ),
                    Err(_e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Server Error".to_string()
                        })),
                    ),
                }
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Invalid Credentials".to_string()
            })),
        ),
    }
}
