use crate::{
    user::{check_session_validity, extract_session_header, GeneralResponse},
    AppState, Filters, Order,
};
use axum::{
    extract::{Path, Query, State},
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

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ItemsQuery {
    /// Number of items to fetch per page
    take: Option<u32>,
    /// Page number to fetch
    page_no: Option<u32>,
    /// The Filter should be of either Price(Order), Rating(Order), DateOfCreation(Order), Alphabetic(Order)
    ///
    /// The Order should be either Inc or Dec
    filter: Option<Filters>,
    /// Search String to filter items
    search_string: Option<String>,
}

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
    rating: Option<f32>,
    price: f32,
    stock: Option<i32>,
}

#[derive(Deserialize, Serialize, FromRow, ToSchema)]
pub struct ItemStock {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemResponse {
    detail: Item,
    sameuser: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PageResponse {
    items: Vec<Item>,
}

#[derive(Deserialize, ToSchema, Debug, IntoParams)]
pub struct SearchQuery {
    query: String,
}

#[derive(Serialize, ToSchema)]
pub struct SearchResult {
    keywords: Vec<Item>,
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
///Endpoint to create an Item
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
///Endpoint to delete an Item
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
///endpoint to edit an Item
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
///Get Item By Id
///
///Endpoint to retrieve details of an Item by id
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
                    detail: "Invalid Credentials".to_string()
                })),
            )
        }
    }
    match check_session_validity(&state.db_pool, session_id).await {
        Some(uresponse) => {
            let query = r#"SELECT t1.item_id, t1.user_id,t1.title,t1.content,t1.price,t1.rating,t2.stock 
            FROM 
            (SELECT * FROM "item" WHERE "item_id"= $1) AS t1 
            LEFT JOIN
            (SELECT "item_id","quantity" as stock FROM "stock" WHERE "item_id" = $1) AS t2 
            ON 
            t1."item_id" = t2."item_id""#;
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
                            rating: response.rating,
                            stock: response.stock
                        },
                        sameuser: if response.user_id == uresponse.user_id {
                            true
                        } else {
                            false
                        }
                    })),
                ),
                Err(e) => (
                    StatusCode::NOT_FOUND,
                    Json(json!(GeneralResponse {
                        detail: e.to_string()
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

///Get items by filter
///
/// Endpoint to get multiple items by page
#[utoipa::path(
    get,
    path = "/item",
    params(
        ItemsQuery
    ),
    responses (
        (status = 200, body = GeneralResponse),
        (status = 500, body = GeneralResponse)
    )
)]
pub async fn get_items(
    state: State<AppState>,
    Query(pagination): Query<ItemsQuery>,
) -> impl IntoResponse {
    let query = paginate_items(pagination);
    match sqlx::query_as::<_, Item>(query.as_str())
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(json!(PageResponse { items: result }))),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!(GeneralResponse {
                detail: e.to_string()
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
///Endpoint to rate an Item
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
            let query = r#"INSERT INTO 
            "comment" ("user_id","item_id","rating","content") 
            SELECT $1,$2,$3,$4 WHERE item_ownership($2,$1) IS FALSE RETURNING "item_id";
            "#;
            match sqlx::query_as::<_, ItemId>(query)
                .bind(user_response.user_id)
                .bind(form_data.item_id)
                .bind(form_data.rating)
                .bind(form_data.content)
                .fetch_optional(&state.db_pool)
                .await
            {
                Ok(result) => match result {
                    Some(_t) => (
                        StatusCode::CREATED,
                        Json(json!(GeneralResponse {
                            detail: "Comment Created".to_string()
                        })),
                    ),
                    None => (
                        StatusCode::CONFLICT,
                        Json(json!(GeneralResponse {
                            detail: "Cannot rate one's own item".to_string()
                        })),
                    ),
                },
                Err(_e) => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!(GeneralResponse {
                        detail: "Error creating comment".to_string() //
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
    path = "/item/stock",
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 200 , body = GeneralResponse),
        (status = 401 , body = GeneralResponse),
        (status = 404 , body = GeneralResponse),
        (status = 500 , body = GeneralResponse)
    )
)]
///Update Stock for an Item
pub async fn edit_stock(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<ItemStock>,
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
        // create plsql function to check if item actually belongs to user
        Some(user_response) => {
            let query = r#"INSERT INTO "stock" ("item_id","quantity") 
            SELECT $1,$2  WHERE item_ownership($1,$3) IS TRUE 
            ON CONFLICT("item_id")
            DO UPDATE SET "quantity" = EXCLUDED."quantity" RETURNING "item_id","quantity""#;
            match sqlx::query_as::<_, ItemStock>(query)
                .bind(&form_data.item_id)
                .bind(&form_data.quantity)
                .bind(user_response.user_id)
                .fetch_optional(&state.db_pool)
                .await
            {
                Ok(response) => match response {
                    Some(_t) => (
                        StatusCode::CREATED,
                        Json(json!(GeneralResponse {
                            detail: "Stock updated".to_string(),
                        })),
                    ),
                    None => (
                        StatusCode::UNAUTHORIZED,
                        Json(json!(GeneralResponse {
                            detail: "Invalid Credentials".to_string(),
                        })),
                    ),
                },
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!(GeneralResponse {
                        detail: e.to_string(),
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
    get,
    path = "/item/search_suggestions",
    params(
        SearchQuery
    ),
    responses(
        (status = 200, body = Vec<Item>)
    )
)]
///Endpoint for search autocompletions
pub async fn search_suggestions(
    state: State<AppState>,
    search_query: Query<SearchQuery>,
) -> impl IntoResponse {
    let query = r#"
    SELECT * FROM "item" WHERE to_tsvector("title"|| ' ' ||"content") @@ to_tsquery($1);
    "#;
    match sqlx::query_as::<_, Item>(query)
        .bind(&search_query.query)
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(response) => (
            StatusCode::OK,
            Json(json!(SearchResult { keywords: response })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!(GeneralResponse {
                detail: e.to_string()
            })),
        ),
    }
}

fn paginate_items(pagination: ItemsQuery) -> String {
    struct PaginationParams {
        take: u32,
        offset: u32,
    }
    let mut query_params = PaginationParams {
        take: 10,
        offset: 0,
    };
    // Setting default values for the pagination
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
    let mut query = r#"SELECT * FROM "item""#.to_owned();
    let pagination_query = format!("LIMIT {} OFFSET {}", query_params.take, query_params.offset);
    let search_token;
    match pagination.search_string {
        Some(token) => {
            search_token = format!(
                r#"WHERE to_tsvector("title"|| ' ' ||"content") @@ websearch_to_tsquery('english','{}')"#,
                token
            )
            .to_owned();
        }
        None => search_token = r#""#.to_owned(),
    }
    match pagination.filter {
        Some(filter_type) => match filter_type {
            Filters::Alphabetical(order) => match order {
                Order::Inc => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "title" ASC "#, pagination_query
                    )
                }
                Order::Dec => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "title" DESC "#, pagination_query
                    )
                }
            },
            Filters::DateOfCreation(order) => match order {
                Order::Inc => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "date_created" ASC "#, pagination_query
                    )
                }
                Order::Dec => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "date_created" DESC "#, pagination_query
                    )
                }
            },
            Filters::Rating(order) => match order {
                Order::Inc => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "rating" ASC "#, pagination_query
                    )
                }
                Order::Dec => {
                    query = format!(
                        "{} {} {} {}",
                        query,
                        search_token,
                        r#"ORDER BY "rating" DESC NULLS LAST"#,
                        pagination_query
                    )
                }
            },
            Filters::Price(order) => match order {
                Order::Inc => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "price" ASC "#, pagination_query
                    )
                }
                Order::Dec => {
                    query = format!(
                        "{} {} {} {}",
                        query, search_token, r#"ORDER BY "price" DESC "#, pagination_query
                    )
                }
            },
        },
        None => query = format!("{} {} {} ", query, search_token, pagination_query),
    }
    query = format!(
        r#"SELECT 
        t1.item_id, t1.user_id,t1.title,t1.content,t1.price,t1.rating,t2.stock 
         FROM ({}) AS t1 
         LEFT JOIN 
         ( SELECT "item_id","quantity" as stock from "stock") AS t2 
         ON t1."item_id" = t2."item_id" "#,
        query
    );
    return query;
}
