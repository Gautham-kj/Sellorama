use crate::user::{check_session_validity, extract_session_header, GeneralResponse};
use crate::AppState;
use axum::{
    extract::{Path, State},
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

#[derive(Deserialize, ToSchema)]
pub struct ItemForm {
    title: String,
    content: String,
    price: f32,
}

#[derive(Deserialize, ToSchema)]
pub struct RateForm {
    rating: i32,
    content: String,
    item_id: Uuid,
}

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct ItemId {
    item_id: Uuid,
}

#[derive(Deserialize, Serialize, FromRow, ToSchema)]
pub struct Item {
    item_id: Uuid,
    user_id: Uuid,
    title: String,
    content: String,
    rating:Option<f32>,
    price: f32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemResponse {
    detail: Item,
    sameuser: bool,
}

#[utoipa::path(
        post,
        path="/item/create",
        responses (
            (status = 201, body = GeneralResponse),
            (status = 401, body = GeneralResponse),
            (status = 100, body = GeneralResponse)
        ),
        security(
            ("session_id"=[])
        )
    )]
pub async fn create_item(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<ItemForm>,
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
        Some(userwithsession) => {
            let query = r#"
                    INSERT INTO "item" ("user_id","title","content","price") 
                    VALUES ($1,$2,$3,$4) returning "item_id";
                "#;
            match sqlx::query_as::<_, ItemId>(query)
                .bind(userwithsession.user_id)
                .bind(&form_data.title)
                .bind(&form_data.content)
                .bind(&form_data.price)
                .fetch_optional(&state.db_pool)
                .await
                .unwrap()
            {
                Some(_response) => (
                    StatusCode::CREATED,
                    Json(json!(GeneralResponse {
                        detail: "Item Created".to_string()
                    })),
                ),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(json!(GeneralResponse {
                        detail: "Could not make Item".to_string()
                    })),
                ),
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "session expired or does not exist".to_string()
            })),
        ),
    }
}

#[utoipa::path(
        delete,
        path = "/item/{id}",
        security(
            ("session_id"=[])
        ),
        responses(
            (status = 200 , body = GeneralResponse),
            (status = 401 , body = GeneralResponse),
            (status = 500 , body = GeneralResponse),
        )
    )]
pub async fn delete_item(
    headers: HeaderMap,
    state: State<AppState>,
    Path(item_id): Path<Uuid>,
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
        Some(response) => {
            println!("{:?}", response);
            match sqlx::query_as::<_,ItemId>(r#"DELETE FROM "item" WHERE "item_id" = $1 AND "user_id" = $2 RETURNING "item_id" "#)
                    .bind(item_id)
                    .bind(response.user_id)
                    .fetch_optional(&state.db_pool)
                    .await
                {
                    Ok(response) => { match response {
                        Some(_item) => (
                            StatusCode::OK,
                            Json(json!(GeneralResponse {
                                detail: "Item Deleted".to_string()
                            })),
                        ),
                        None => (
                            StatusCode::UNAUTHORIZED,
                            Json(json!(GeneralResponse {
                                detail: "Invalid credentials".to_string()
                            }))),
                    }},
                    Err(_e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Error deleting item".to_string()
                        })),
                    ),
                }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Inavlid credentials".to_string()
            })),
        ),
    }
}

// pub async fn get_items_of_user(
//     state: State<AppState>,
//     Path(user_id):Path<Uuid>
// ) -> impl IntoResponse{

// }
#[utoipa::path(
        put,
        path = "/item/{id}",
        security(
            ("session_id"=[])
        ),
        responses(
            (status = 200 , body = GeneralResponse),
            (status = 401 , body = GeneralResponse),
            (status = 500 , body = GeneralResponse),
        )
    )]
pub async fn edit_item(
    headers: HeaderMap,
    state: State<AppState>,
    Path(item_id): Path<Uuid>,
    Form(form_data): Form<ItemForm>,
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
        Some(response) => {
            println!("{:?}", response);
            let query = r#"
                UPDATE "item" SET
                "title" = $1,
                "content" = $2,
                "price" = $3
                WHERE "item_id" = $4 AND "user_id" = $5 RETURNING "item_id" "#;
            match sqlx::query_as::<_, ItemId>(query)
                .bind(&form_data.title)
                .bind(&form_data.content)
                .bind(&form_data.price)
                .bind(item_id)
                .bind(response.user_id)
                .fetch_optional(&state.db_pool)
                .await
            {
                Ok(response) => match response {
                    Some(_item) => (
                        StatusCode::OK,
                        Json(json!(GeneralResponse {
                            detail: "Item Updated".to_string()
                        })),
                    ),
                    None => (
                        StatusCode::UNAUTHORIZED,
                        Json(json!(GeneralResponse {
                            detail: "Invalid credentials".to_string()
                        })),
                    ),
                },
                Err(e) => {
                    println!("{:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Error deleting item".to_string()
                        })),
                    )
                }
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Inavlid credentials".to_string()
            })),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/item/{id}",
    security(
        ("session_id"=[])
    ),
    responses(
        (status = 200 , body = GeneralResponse),
        (status = 401 , body = GeneralResponse),
        (status = 500 , body = GeneralResponse),
    )
)]
pub async fn get_item(
    headers: HeaderMap,
    state: State<AppState>,
    Path(item_id): Path<Uuid>,
) -> impl IntoResponse {
    let session_id;
    match extract_session_header(headers).await {
        Some(session) => session_id = session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(GeneralResponse {
                    detail: "Invalid Credentials lol".to_string()
                })),
            )
        }
    }
    match check_session_validity(&state.db_pool, session_id).await {
        Some(uresponse) => {
            let query = r#"
            SELECT t1."item_id",t1."user_id",t1."title",t1."content",t1."date_created",t1."price",t2."rating" 
            FROM (SELECT * FROM "item" WHERE "item_id"=$1) AS t1 
            LEFT JOIN
            (SELECT "item_id",AVG("rating")::FLOAT4 "rating" FROM "comment" GROUP BY "item_id" ) AS t2 ON t1."item_id" = t2."item_id"; "#;
            match sqlx::query_as::<_, Item>(query)
                .bind(item_id)
                .fetch_one(&state.db_pool)
                .await
            {
                Ok(response) => (
                    StatusCode::OK,
                    Json(json!(ItemResponse {
                        detail: Item {
                            item_id: response.item_id,
                            user_id: response.user_id,
                            title: response.title,
                            content: response.content,
                            price: response.price,
                            rating: response.rating
                        },
                        sameuser: if response.user_id == uresponse.user_id {
                            true
                        } else {
                            false
                        }
                    })),
                ),
                Err(_e) => (
                    StatusCode::NOT_FOUND,
                    Json(json!(GeneralResponse {
                        detail: "Item Not Found".to_string()
                    })),
                ),
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Inavlid credentials".to_string()
            })),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/item/rate",
    responses (
        (status = 201, body = GeneralResponse),
        (status = 401, body = GeneralResponse),
        (status = 100, body = GeneralResponse)
    ),
    security(
        ("session_id"=[])
    )
)]
pub async fn rate_item(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<RateForm>,
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
        Some(user_response) => {
            match sqlx::query!(
                r#"INSERT INTO 
                "comment" ("user_id","item_id","rating","content") 
                VALUES ($1,$2,$3,$4) "#,
                user_response.user_id,
                form_data.item_id,
                form_data.rating,
                form_data.content
            )
            .execute(&state.db_pool)
            .await
            {
                Ok(_result) => (
                    StatusCode::CREATED,
                    Json(json!(GeneralResponse {
                        detail: "Comment Created".to_string()
                    })),
                ),
                Err(_e) => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!(GeneralResponse {
                        detail: "Error creating comment".to_string()
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
