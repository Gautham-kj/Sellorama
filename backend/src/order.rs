use crate::errors::MyError;
use crate::user::{check_session_validity, extract_session_header, GeneralResponse};
use crate::AppState;
use crate::CartItem;
use axum::extract::Query;
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
use utoipa::{IntoParams, ToSchema};
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

#[derive(FromRow, ToSchema, Deserialize, Serialize, IntoParams)]
pub struct OrderQuery {
    ///page_no of the orders
    page_no: Option<u32>,
    ///take of the orders
    take: Option<u32>,
    ///dispatched status of the orders
    dispatched: Option<bool>,
    ///Order of the orders
    order: Option<bool>,
}

#[derive(FromRow)]
pub struct DispatchStatus {
    dispatched: bool,
}

#[derive(FromRow, ToSchema, Deserialize, Serialize)]
pub struct DispatchForm {
    order_id: Uuid,
    item_id: Uuid,
}
#[derive(FromRow, ToSchema, Serialize)]
pub struct Orders {
    detail: Vec<AllOrderDetails>,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct AllOrderDetails {
    ///order_id of the order
    order_id: Uuid,
    ///item_id of the item in the order
    item_id: Uuid,
    ///quantity of the item in the order
    quantity: i32,
    ///address_id of the user to deliver the order
    address_id: Uuid,
    ///order_date of the order
    order_date: chrono::NaiveDateTime,
    ///dispatched status of the order
    dispatched: bool,
}

#[derive(FromRow, ToSchema, Deserialize, Serialize)]
pub struct CartError {
    detail: Vec<CartItem>,
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
        (status = 409, body = CartError),
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
            match sqlx::query_as::<_, CartItem>(query)
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
                            .map_err(|e| {
                                println!("{e}");
                                return MyError::InternalServerError;
                            })? {
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
                                    Json(json!(CartError { detail: items })),
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
                            Json(json!(CartError { detail: items })),
                        ))
                    }
                },
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

#[utoipa::path(
    get,
    path = "/order/orders",
    params(
        OrderQuery
    ),
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 200, body = Orders),
        (status = 401, body = GeneralResponse),
        (status = 500, body = GeneralResponse)
    ),
)]
/// Get Orders
///
/// Endpoint to get all the orders placed for a user
pub async fn get_orders(
    headers: HeaderMap,
    state: State<AppState>,
    Query(form_data): Query<OrderQuery>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = paginate_orders(form_data);
            match sqlx::query_as::<_, AllOrderDetails>(query.as_str())
                .bind(user.user_id)
                .fetch_all(&state.db_pool)
                .await
                .map_err(|_| MyError::InternalServerError)?
            {
                orders => Ok(Json(Orders { detail: orders })),
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

#[utoipa::path(
    post,
    path = "/order/dispatch",
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 200, body = GeneralResponse),
        (status = 401, body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    ),
)]
pub async fn set_dispatch_by_item_id(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<DispatchForm>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"UPDATE "order_items" 
        SET "dispatched" = TRUE
        WHERE
        "item_id" = $2 AND "order_id" = $3 AND "dispatched" = FALSE AND item_ownership("item_id",$1)
        RETURNING "dispatched";"#;
            match sqlx::query_as::<_, DispatchStatus>(query)
                .bind(user.user_id)
                .bind(form_data.item_id)
                .bind(form_data.order_id)
                .fetch_optional(&state.db_pool)
                .await
                .map_err(|_| MyError::InternalServerError)?
            {
                Some(status) => {
                    if status.dispatched {
                        Ok(Json(GeneralResponse {
                            detail: "Order item dispatched successfully".to_string(),
                        }))
                    } else {
                        Err(MyError::UnauthorizedError)
                    }
                }
                None => Err(MyError::UnauthorizedError),
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

fn paginate_orders(pagination: OrderQuery) -> String {
    struct PaginationParams {
        take: u32,
        offset: u32,
        dispatched: String,
        order: String,
    }
    // Default values
    let mut query_params = PaginationParams {
        take: 10,
        offset: 0,
        dispatched: "".to_string(),
        order: "DESC".to_string(),
    };
    // Set values from pagination
    match pagination.take {
        Some(take) => query_params.take = take,
        None => query_params.take = 10,
    }
    match pagination.page_no {
        Some(page_no) => {
            query_params.offset = if page_no > 0 {
                (page_no - 1) * query_params.take
            } else {
                0
            }
        }
        None => query_params.offset = 0,
    }
    match pagination.dispatched {
        Some(dispatched) => {
            query_params.dispatched = format!(r#"WHERE "dispatched" = {}"#, dispatched)
        }
        None => query_params.dispatched = "".to_string(),
    }
    match pagination.order {
        Some(order) => match order {
            true => query_params.order = "ASC".to_string(),
            false => query_params.order = "DESC".to_string(),
        },
        None => query_params.order = "DESC".to_string(),
    }
    format!(
        r#"SELECT t1."order_id",t1."item_id",t1."quantity",t2."order_date",t2."address_id",t1."dispatched" FROM 
        (SELECT * from "order_items" WHERE item_ownership("item_id",$1) IS TRUE ) as t1 
        INNER JOIN
        (SELECT * FROM "order" ) as t2
        ON t1."order_id" = t2."order_id"
        {} ORDER BY t2."order_date" {} LIMIT {} OFFSET {};"#,
        query_params.dispatched, query_params.order, query_params.take, query_params.offset
    )
}
