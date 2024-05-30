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
/// Get Cart
///
/// Endpoint to get all items in a user's cart
pub async fn get_cart(
    state: State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"SELECT "item_id","quantity" FROM "cart" WHERE "cart_id" = $1"#;

            match sqlx::query_as::<_, CartItem>(query)
                .bind(user.user_id)
                .fetch_all(&state.db_pool)
                .await
            {
                Ok(cart) => Ok((
                    StatusCode::OK,
                    Json(json!(CartResponse {
                        detail: Cart { items: cart }
                    })),
                )),
                Err(_e) => Err(MyError::InternalServerError),
            }
        }
        None => Err(MyError::UnauthorizedError),
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
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(userresponse) => {
            //create plsql function to check and return stock issues
            let query = r#"INSERT INTO "cart" ("cart_id","item_id","quantity") 
            SELECT $1,$2,$3 WHERE stock_validation($2,$3) IS TRUE AND item_ownership($2,$1) IS NOT TRUE
            ON CONFLICT("cart_id","item_id")
            DO UPDATE SET "quantity" = EXCLUDED."quantity" RETURNING "item_id","quantity""#;
            match sqlx::query_as::<_, CartItem>(query)
                .bind(userresponse.user_id)
                .bind(form_data.item_id)
                .bind(form_data.quantity)
                .fetch_optional(&state.db_pool)
                .await
            {
                Ok(response) => match response {
                    Some(_t) => Ok((
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Item Added to cart".to_string()
                        })),
                    )),
                    None => Err(MyError::ConflictError),
                },
                Err(_e) => Err(MyError::InternalServerError),
            }
        }
        None => Err(MyError::UnauthorizedError),
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
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(userresponse) => {
            if form_data.quantity > 0 {
                let query = r#"UPDATE "cart" SET "quantity" = $3 WHERE "cart_id" = $1 AND "item_id" = $2 AND item_ownership($2,$1) IS NOT TRUE AND stock_validation($2,$3) IS TRUE RETURNING "item_id","quantity""#;
                match sqlx::query_as::<_, CartItem>(query)
                    .bind(userresponse.user_id)
                    .bind(form_data.item_id)
                    .bind(form_data.quantity)
                    .fetch_optional(&state.db_pool)
                    .await
                {
                    Ok(response) => match response {
                        Some(_t) => Ok((
                            StatusCode::OK,
                            Json(json!(GeneralResponse {
                                detail: "Cart updated".to_string()
                            })),
                        )),
                        None => Ok((
                            StatusCode::OK,
                            Json(json!(GeneralResponse {
                                detail: "Cart Not Updated".to_string()
                            })),
                        )),
                    },
                    Err(_e) => Err(MyError::InternalServerError),
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
                    Ok(_response) => Ok((
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Cart updated".to_string()
                        })),
                    )),
                    Err(_e) => Err(MyError::InternalServerError),
                }
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}
#[utoipa::path(
    get,
    path = "/cart/subcheckout",
    security (("session_id" = [])),
    responses(
        (status = 200 , body = GeneralResponse),
        (status = 401 , body = GeneralResponse),
        (status = 500 , body = GeneralResponse),
        (status = 409 , body = CartResponse)
    )
)]
/// SubCheckout Cart
///
/// Checking whether items in the cart are still in stock.
pub async fn check_cart(
    headers: HeaderMap,
    state: State<AppState>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"DELETE FROM "cart" 
            where
            stock_validation("item_id","quantity") IS NOT TRUE 
            AND
            "cart_id" = $1 RETURNING "item_id","quantity"; 
                "#;
            match sqlx::query_as::<_, CartItem>(query)
                .bind(user.user_id)
                .fetch_all(&state.db_pool)
                .await
                .map_err(|_| MyError::InternalServerError)
            {
                Ok(items) => match items.len() {
                    0 => Ok((
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Items In Stock, Proceed to Checkout".to_string()
                        })),
                    )),
                    _ => Err(MyError::ConflictError),
                },
                Err(e) => Err(e),
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

// pub async fn create_order(headers: HeaderMap, state: State<AppState>) -> impl IntoResponse {}
