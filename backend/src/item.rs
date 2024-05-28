use std::collections::HashMap;

use crate::{
    objects::{get_presigned_url, put_object},
    user::{check_session_validity, extract_session_header, GeneralResponse},
    AppState, Filters, Order,
};
use axum::{
    extract::{Multipart, Path, Query, State},
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
    price: rust_decimal::Decimal,
    item_media: Option<Vec<Vec<u8>>>,
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
    price: rust_decimal::Decimal,
    stock: Option<i32>,
}

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct MediaResponse {
    media_id: Option<Uuid>,
    item_id: Uuid,
}

#[derive(Deserialize, Serialize, FromRow, ToSchema)]
pub struct ItemStock {
    item_id: Uuid,
    quantity: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemResponse {
    detail: Item,
    media: Option<Vec<String>>,
    sameuser: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PageResponse {
    items: Vec<ItemResponse>,
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
    request_body(content_type = "multipart/form-data", content = ItemForm),
    security(
        ("session_id"=[])
    )
)]
/// Create Item
///
/// Endpoint to create an Item
pub async fn create_item(
    headers: HeaderMap,
    state: State<AppState>,
    // Form(form_data): Form<ItemForm>,
    mut multipart: Multipart,
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
    //multipart form handling

    match check_session_validity(&state.db_pool, session_id).await {
        Some(userwithsession) => {
            let mut txn = state.db_pool.begin().await.unwrap();

            let mut form_data = ItemForm {
                title: "".to_string(),
                content: "".to_string(),
                price: rust_decimal::Decimal::new(0, 0),
                item_media: None,
            };

            let mut item_media: Vec<Vec<u8>> = vec![];

            while let Some(field) = multipart.next_field().await.unwrap() {
                let name = field.name().unwrap().to_owned();
                let data;
                match field.bytes().await {
                    Ok(bytes) => data = bytes.to_vec(),
                    Err(_e) => {
                        return (
                            StatusCode::UNPROCESSABLE_ENTITY,
                            Json(json!(GeneralResponse {
                                detail: "File too large".to_string()
                            })),
                        );
                    }
                }

                match name.as_str() {
                    "title" => form_data.title = String::from_utf8(data).unwrap(),
                    "content" => form_data.content = String::from_utf8(data).unwrap(),
                    "price" => form_data.price = String::from_utf8(data).unwrap().parse().unwrap(),
                    "item_media" => {
                        if data.len() > 0 {
                            item_media.push(data);
                        } else {
                            ()
                        }
                    }
                    _ => (),
                }
            }

            match item_media.len() {
                0 => form_data.item_media = None,
                _ => form_data.item_media = Some(item_media.clone()),
            }

            match sqlx::query_as::<_, ItemId>(
                r#"
                INSERT INTO "item" ("user_id","title","content","price")
                VALUES ($1,$2,$3,$4) returning "item_id""#,
            )
            .bind(&userwithsession.user_id)
            .bind(&form_data.title)
            .bind(&form_data.content)
            .bind(&form_data.price)
            .fetch_optional(&mut *txn)
            .await
            .unwrap()
            {
                Some(item_response) => match form_data.item_media {
                    Some(media) => {
                        let mut media_ids: Vec<Uuid> = vec![];
                        for _media_item in &media {
                            media_ids.push(Uuid::new_v4());
                        }
                        let media_query = r#"
                            INSERT INTO "item_media" ("media_id","item_id")
                            (SELECT * FROM UNNEST($1::uuid[],$2::uuid[])) RETURNING "item_id" ;
                        "#;
                        match sqlx::query_as::<_, ItemId>(media_query)
                            .bind(&media_ids)
                            .bind(vec![item_response.item_id; media_ids.len()])
                            .fetch_optional(&mut *txn)
                            .await
                            .unwrap()
                        {
                            Some(_response) => {
                                for (index, media_item) in media.iter().enumerate() {
                                    let file_key = format!("{}.jpg", media_ids[index]);
                                    let data_stream = aws_sdk_s3::primitives::ByteStream::from(
                                        media_item.clone(),
                                    );
                                    match put_object(
                                        &state.s3_client,
                                        &state.image_bucket,
                                        file_key,
                                        data_stream,
                                    )
                                    .await
                                    {
                                        Err(e) => {
                                            println!("{:?}",e);
                                            return (
                                                StatusCode::UNPROCESSABLE_ENTITY,
                                                Json(json!(GeneralResponse {
                                                    detail: "Error Creating Item".to_string()
                                                })),
                                            );
                                        }
                                        _ => (),
                                    }
                                }
                                txn.commit().await.unwrap();
                                (
                                    StatusCode::CREATED,
                                    Json(json!(GeneralResponse {
                                        detail: "Item Created".to_string()
                                    })),
                                )
                            }
                            None => {
                                txn.rollback().await.unwrap();
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(json!(GeneralResponse {
                                        detail: "Error Creating Item".to_string()
                                    })),
                                );
                            }
                        }
                    }
                    None => {
                        txn.commit().await.unwrap();
                        return (
                            StatusCode::CREATED,
                            Json(json!(GeneralResponse {
                                detail: "Item Created".to_string()
                            })),
                        );
                    }
                },
                None => {
                    txn.rollback().await.unwrap();
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!(GeneralResponse {
                            detail: "Could not make Item".to_string()
                        })),
                    );
                }
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
/// Delete Item
///
/// Endpoint to delete an Item
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
/// Edit Item
///
/// Endpoint to edit an Item
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
                Ok(response) => {
                    let media_urls = get_presigned_urls_for_items(
                        vec![response.item_id],
                        &state.db_pool,
                        &state.s3_client,
                        &state.image_bucket,
                    )
                    .await
                    .unwrap();
                    match media_urls.len() {
                        0 => (
                            StatusCode::OK,
                            Json(json!(ItemResponse {
                                detail: Item {
                                    item_id: response.item_id,
                                    user_id: response.user_id,
                                    title: response.title,
                                    content: response.content,
                                    price: response.price,
                                    rating: response.rating,
                                    stock: response.stock,
                                },
                                media: None,
                                sameuser: if response.user_id == uresponse.user_id {
                                    true
                                } else {
                                    false
                                }
                            })),
                        ),
                        _ => (
                            StatusCode::OK,
                            Json(json!(ItemResponse {
                                detail: Item {
                                    item_id: response.item_id.clone(),
                                    user_id: response.user_id,
                                    title: response.title,
                                    content: response.content,
                                    price: response.price,
                                    rating: response.rating,
                                    stock: response.stock,
                                },
                                media: Some(media_urls[&response.item_id].clone()),
                                sameuser: if response.user_id == uresponse.user_id {
                                    true
                                } else {
                                    false
                                }
                            })),
                        ),
                    }
                }
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
        Ok(result) => {
            let mut response: Vec<ItemResponse> = vec![];
            let items: Vec<Uuid> = result
                .iter()
                .map(|item| item.item_id)
                .collect::<Vec<Uuid>>();
            let media_urls = get_presigned_urls_for_items(
                items,
                &state.db_pool,
                &state.s3_client,
                &state.image_bucket,
            )
            .await
            .unwrap();

            for item in result {
                let media_item = media_urls[&item.item_id].clone();
                match media_item.len() {
                    0 => response.push(ItemResponse {
                        detail: item,
                        media: None,
                        sameuser: false,
                    }),
                    _ => response.push(ItemResponse {
                        detail: item,
                        media: Some(media_item),
                        sameuser: false,
                    }),
                }
            }
            (
                StatusCode::OK,
                Json(json!(PageResponse { items: response })),
            )
        }
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
///Rate an Item
///
/// Endpoint to rate an item
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
///Get Search Autocompletions
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

async fn get_presigned_urls_for_items(
    item_ids: Vec<Uuid>,
    db_pool: &sqlx::Pool<sqlx::Postgres>,
    s3_client: &aws_sdk_s3::Client,
    bucket: &String,
) -> Result<HashMap<Uuid, Vec<String>>, Box<dyn std::error::Error>> {
    let media_query = r#"
    SELECT t1."item_id",t2."media_id"
    FROM 
    (select * from UNNEST($1::uuid[]) as t("item_id")) as t1
    LEFT JOIN 
    (select * from "item_media") as t2 
    ON t1."item_id" = t2."item_id""#;
    let media_response = sqlx::query_as::<_, MediaResponse>(media_query)
        .bind(item_ids)
        .fetch_all(db_pool)
        .await?;
    let mut item_with_media: HashMap<Uuid, Vec<String>> = HashMap::new();
    for media_item in media_response {
        match media_item.media_id {
            Some(media_id) => match item_with_media.get_mut(&media_item.item_id) {
                Some(vector) => {
                    let url = get_presigned_url(
                        s3_client,
                        bucket.as_str(),
                        format!("{}.jpg", media_id).as_str(),
                        3600,
                    )
                    .await
                    .unwrap();
                    vector.push(url);
                }
                None => {
                    let url = get_presigned_url(
                        s3_client,
                        bucket.as_str(),
                        format!("{}.jpg", media_id).as_str(),
                        3600,
                    )
                    .await
                    .unwrap();
                    item_with_media.insert(media_item.item_id, vec![url]);
                }
            },
            None => {
                item_with_media.insert(media_item.item_id, vec![]);
            }
        }
    }
    Ok(item_with_media)
}
