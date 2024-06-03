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
pub struct OrderForm {
    ///address_id of the user to deliver the order
    address_id: Uuid,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct OrderDetails {
    ///order_id of the order
    order_id: Uuid,
    ///order_date of the order
    order_date: chrono::NaiveDateTime,
}

#[utoipa::path(
    post,
    path = "/order/create",
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 201 , body = OrderDetails),
        (status = 401, body = GeneralResponse),
        (status = 409, body = CartResponse),
        (status = 500, body = GeneralResponse)
    )
)]
/// Create Order
///
/// Endpoint to create an order from the items in the user's cart
pub async fn create_order(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<OrderForm>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let mut txn = state.db_pool.begin().await.unwrap();
            let query = r#"
                DELETE FROM "cart" 
                where
                stock_validation("item_id","quantity") IS NOT TRUE 
                AND
                "cart_id" = $1 RETURNING "item_id","quantity"; 
            "#;
            match sqlx::query_as::<_, crate::CartItem>(query)
                .bind(user.user_id)
                .fetch_all(&mut *txn)
                .await
                .map_err(|_| MyError::InternalServerError)?
            {
                items => match items.len() {
                    0 => {
                        let query = r#"
                        WITH "cart_items" AS (
                            DELETE FROM "cart" 
                            where
                            stock_validation("item_id","quantity") IS TRUE 
                            AND
                            "cart_id" = $1 RETURNING "item_id","quantity" 
                        ),
                        "order_details" as (
                            INSERT INTO "order"("user_id","address_id")
                            VALUES($1,$2) RETURNING "order_id","order_date"
                        ),
                        "stock_updation" as (
                            UPDATE "stock" set "quantity" = "stock"."quantity" - cart_items."quantity" FROM cart_items WHERE cart_items."item_id" = stock."item_id"
                        ),
                        result AS(
                        INSERT INTO "order_items"("order_id","item_id","quantity")
                        SELECT "order_id","item_id","quantity" FROM cart_items,order_details RETURNING "order_id"
                        )
                        SELECT result."order_id",order_details."order_date" from result,order_details;
                        "#;
                        match sqlx::query_as::<_, OrderDetails>(query)
                            .bind(user.user_id)
                            .bind(form_data.address_id)
                            .fetch_optional(&mut *txn)
                            .await
                            .map_err(|e|{println!("{e}");
                        return MyError::InternalServerError;} )?
                        {
                            Some(order) => {
                                txn.commit().await.unwrap();
                                Ok((StatusCode::CREATED, Json(json!(order))))
                            }
                            None => {
                                txn.rollback()
                                    .await
                                    .map_err(|_| MyError::InternalServerError)?;
                                Ok((
                                    StatusCode::CONFLICT,
                                    Json(json!(crate::Cart { items: items })),
                                ))
                            }
                        }
                    }
                    _ => {
                        txn.rollback()
                            .await
                            .map_err(|_| MyError::InternalServerError)?;
                        Ok((
                            StatusCode::CONFLICT,
                            Json(json!(crate::Cart { items: items })),
                        ))
                    }
                },
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

// pub async fn dispatch