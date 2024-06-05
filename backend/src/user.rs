use crate::errors::MyError;
use crate::AppState;
use crate::Duration;
use argon2::PasswordHash;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::extract::Query;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Form, Json,
};

use chrono::{NaiveDateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{prelude::FromRow, types::chrono, Pool, Postgres};
use utoipa::IntoParams;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    user_id: Uuid,
    username: String,
    email_id: String,
    date_created: chrono::NaiveDateTime,
    post_count: i32,
}
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UserLogin {
    username: String,
    password: String,
}

#[derive(FromRow)]
pub struct UserCreds {
    user_id: Uuid,
    hashed_pass: String,
}
#[derive(Deserialize, Serialize, ToSchema, FromRow)]
pub struct Session {
    pub session_id: Uuid,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct MyOrderDetails {
    order_id: Uuid,
    order_date: NaiveDateTime,
    item_id: Uuid,
    dispatched: bool,
}

#[derive(Deserialize, Serialize,ToSchema,IntoParams)]
pub struct MyOrderQuery {
    page_no: Option<u32>,
    take: Option<u32>,
    dispatched: Option<bool>,
}
#[derive(Deserialize, Serialize, ToSchema, FromRow, Debug)]
pub struct UserWithSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserForm {
    username: String,
    email_id: String,
    password: String,
}
#[derive(FromRow, Serialize)]
pub struct UserId {
    user_id: Uuid,
}

#[derive(ToSchema, Serialize)]
pub struct GeneralResponse {
    pub detail: String,
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct UserResponse {
    detail: User,
}

#[derive(ToSchema, Serialize, Deserialize)]
pub struct SessionResponse {
    pub detail: Session,
}

#[derive(FromRow, ToSchema, Serialize, Deserialize)]
pub struct Address {
    address_line_1: String,
    address_line_2: Option<String>,
    city: String,
    country: String,
    pincode: String,
}

#[derive(FromRow, ToSchema, Serialize)]
pub struct AddressId {
    address_id: Uuid,
}

#[utoipa::path(
        post,
        path = "/user/signup",
        responses(
            (status = 201, body=SessionResponse),
            (status = 404, body=GeneralResponse)
        )
    )]
pub async fn signup(
    state: State<AppState>,
    Form(form_data): Form<CreateUserForm>,
) -> impl IntoResponse {
    if form_data.password.len() < 6 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!(GeneralResponse {
                detail: "Password must be atleast 6 characters long".to_string()
            })),
        );
    }
    let (username, email_id, db_pool, password) = (
        &form_data.username,
        &form_data.email_id,
        &state.db_pool,
        create_hashed_password(form_data.password),
    );
    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();
    if !email_regex.is_match(email_id) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!(GeneralResponse {
                detail: "Invalid Email Id".to_string()
            })),
        );
    }
    let check_query = r#"
        SELECT * FROM "user" WHERE "username" = $1 OR "email_id" = $2;
        "#;
    let insert_query = r#"
        WITH INS AS (
            INSERT INTO "user" ("username","email_id") VALUES ($1,$2)
            RETURNING "user_id","date_created"
        )
        INSERT INTO "password" ("user_id","hashed_pass")
        (SELECT "user_id",$3 FROM INS) 
        RETURNING "user_id";  
        "#;
    match sqlx::query_as::<_, User>(check_query)
        .bind(&username)
        .bind(&email_id)
        .bind(&password)
        .fetch_optional(db_pool)
        .await
        .expect("Database error")
    {
        Some(_t) => (
            StatusCode::CONFLICT,
            Json(json!(GeneralResponse {
                detail: "user or email_id exists".to_string()
            })),
        ),
        None => match sqlx::query_as::<_, UserId>(insert_query)
            .bind(&username)
            .bind(&email_id)
            .bind(password)
            .fetch_optional(db_pool)
            .await
            .expect("Server Error")
        {
            Some(user) => {
                match create_session(
                    db_pool,
                    user.user_id,
                    Utc::now().naive_utc() + Duration::days(1),
                )
                .await
                {
                    Some(session) => {
                        println!("Session {} created", session.session_id);
                        (
                            StatusCode::CREATED,
                            Json(json!(SessionResponse { detail: session })),
                        )
                    }
                    None => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Error Creating Session".to_string()
                        })),
                    ),
                }
            }
            None => (
                StatusCode::NOT_FOUND,
                Json(json!(GeneralResponse {
                    detail: "Internal Server Error".to_string()
                })),
            ),
        },
    }
}

#[utoipa::path(
        get,
        path = "/user/{user_id}",
        responses(
            (status = 201, body=UserResponse),
            (status = 404, body = GeneralResponse )
        )
    )]
pub async fn get_user_by_id(
    state: State<AppState>,
    Path(username): Path<Uuid>,
) -> impl IntoResponse {
    let query = r#"
        SELECT * FROM "user" WHERE "user_id"=$1;
        "#;

    match sqlx::query_as::<_, User>(query)
        .bind(username)
        .fetch_optional(&state.db_pool)
        .await
        .expect("Server Error")
    {
        Some(user) => (StatusCode::OK, Json(json!(UserResponse { detail: user }))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!(GeneralResponse {
                detail: "User Not Found".to_string()
            })),
        ),
    }
}

#[utoipa::path(
        post,
        path = "/user/login",
        responses(
            (status = 201, body=SessionResponse),
            (status = 401, body=GeneralResponse)
        )
    )]
pub async fn user_login(
    state: State<AppState>,
    Form(form_data): Form<UserLogin>,
) -> impl IntoResponse {
    let (username, password) = (form_data.username, form_data.password);
    invalidate_dangling_sessions(&state.db_pool)
        .await
        .expect("Error Deleting invalid sessions");
    let query = r#"
            WITH INS AS (
                SELECT "user_id" FROM "user"
                WHERE "username" = $1 
            )
            SELECT "user_id","hashed_pass" FROM "password"
            WHERE "user_id" in (SELECT * FROM INS)
        "#;

    match sqlx::query_as::<_, UserCreds>(query)
        .bind(username)
        .bind(&password)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(user) => match validate_password(&password, user.hashed_pass) {
            Ok(_) => {
                match create_session(
                    &state.db_pool,
                    user.user_id,
                    Utc::now().naive_utc() + Duration::days(1),
                )
                .await
                {
                    Some(session) => {
                        println!("Session {} created", session.session_id);
                        (
                            StatusCode::CREATED,
                            Json(json!(SessionResponse { detail: session })),
                        )
                    }
                    None => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(GeneralResponse {
                            detail: "Error Creating Session".to_string()
                        })),
                    ),
                }
            }
            Err(_e) => (
                StatusCode::UNAUTHORIZED,
                Json(json!(GeneralResponse {
                    detail: "Wrong username or password".to_string()
                })),
            ),
        },
        Err(_e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!(GeneralResponse {
                detail: "Invalid Username or password".to_string()
            })),
        ),
    }
}

#[utoipa::path(
        post,
        path = "/user/logout",
        responses(
            (status = 201, body=SessionResponse),
            (status = 401, body=GeneralResponse)
        )
    )]
pub async fn logout(state: State<AppState>, Form(form_data): Form<Session>) -> impl IntoResponse {
    match invalidate_session(&state.db_pool, form_data.session_id).await {
        Ok(t) => {
            println!("Session {t} deleted");
            (
                StatusCode::OK,
                Json(json!(GeneralResponse {
                    detail: "User logged out".to_string()
                })),
            )
        }
        Err(e) => {
            println!("Error: {e}");
            (
                StatusCode::BAD_REQUEST,
                Json(json!(GeneralResponse {
                    detail: "Error while logging out".to_string()
                })),
            )
        }
    }
}

#[utoipa::path(
    post,
    path = "/user/address",
    security(
        ("session_id"=[])
    ),
    responses(
        (status =200 ,body = GeneralResponse),
        (status =500 ,body = GeneralResponse),
        (status =401 ,body = GeneralResponse),
    )
)]
/// Create User Address
///
/// Endpoint to create a new address for the user
pub async fn create_user_address(
    headers: HeaderMap,
    state: State<AppState>,
    Form(form_data): Form<Address>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = r#"
                INSERT INTO "address" ("user_id","address_line_1", "address_line_2", "city", "country", "pincode")
                VALUES ($1, $2, $3, $4, $5, $6)
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

#[utoipa::path(
    get,
    path = "/user/myorders",
    params(
        MyOrderQuery
    ),
    security(
        ("session_id" = [])
    ),
    responses(
        (status = 200, body = Vec<MyOrderDetails>),
        (status = 401, body = GeneralResponse),
        (status = 500, body = GeneralResponse)
    )
)]
/// Get User Orders
///
/// Endpoint to get all the orders placed by the user
pub async fn get_user_orders(
    headers: HeaderMap,
    state: State<AppState>,
    Query(form_data): Query<MyOrderQuery>,
) -> Result<impl IntoResponse, MyError> {
    let session_id = extract_session_header(headers).await?;
    match check_session_validity(&state.db_pool, session_id).await {
        Some(user) => {
            let query = paginate_orders(form_data);
            match sqlx::query_as::<_, MyOrderDetails>(query.as_str())
                .bind(user.user_id)
                .fetch_all(&state.db_pool)
                .await
                .map_err(|_| MyError::InternalServerError)?
            {
                response => Ok((StatusCode::OK, Json(json!(response)))),
            }
        }
        None => Err(MyError::UnauthorizedError),
    }
}

fn create_hashed_password(password: String) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    password_hash
}

fn validate_password(password: &String, hashed_password: String) -> Result<(), ()> {
    let password_ref = PasswordHash::new(&hashed_password.as_str()).unwrap();
    match Argon2::default().verify_password(password.as_bytes(), &password_ref) {
        Ok(_t) => Ok(()),
        Err(_e) => Err(()),
    }
}

async fn create_session(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    expiry: NaiveDateTime,
) -> Option<Session> {
    let query = r#"
            INSERT INTO "session" ("user_id","expiry") VALUES ($1,$2) RETURNING "session_id";
        "#;

    match sqlx::query_as::<_, Session>(query)
        .bind(user_id)
        .bind(expiry)
        .fetch_optional(pool)
        .await
        .expect("Error accessing database")
    {
        Some(response) => Some(Session {
            session_id: response.session_id,
        }),
        None => None,
    }
}

#[allow(dead_code)]
async fn invalidate_sessions_user(pool: &Pool<Postgres>, user_id: Uuid) -> Result<String, String> {
    match sqlx::query!(
        r#"DELETE FROM "session" WHERE "user_id" = $1 AND expiry < CURRENT_TIMESTAMP"#,
        user_id
    )
    .execute(pool)
    .await
    {
        Ok(_) => Ok(format!("all sessions for {} deleted", user_id)),
        Err(_) => Err(String::from("Error Deleting session")),
    }
}

async fn invalidate_dangling_sessions(pool: &Pool<Postgres>) -> Result<String, String> {
    match sqlx::query!(r#"DELETE FROM "session" where  "expiry" < CURRENT_TIMESTAMP"#)
        .execute(pool)
        .await
    {
        Ok(_) => Ok(format!("Dangling sessions removed")),
        Err(e) => Err(format!("Error :{}", e)),
    }
}

pub async fn invalidate_session(pool: &Pool<Postgres>, session_id: Uuid) -> Result<Uuid, String> {
    match sqlx::query!(
        r#"DELETE FROM "session" WHERE "session_id" = $1"#,
        session_id
    )
    .execute(pool)
    .await
    {
        Ok(_) => Ok(session_id),
        Err(_) => Err(String::from("Error Deleting session")),
    }
}

pub async fn check_session_validity(
    pool: &Pool<Postgres>,
    session_id: Uuid,
) -> Option<UserWithSession> {
    let query = r#"
            SELECT "user_id","session_id" FROM "session" 
            WHERE 
            "session_id" = $1 AND "expiry" > CURRENT_TIMESTAMP;
        "#;
    match sqlx::query_as::<_, UserWithSession>(query)
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .expect("Error accessing database")
    {
        Some(response) => Some(UserWithSession {
            session_id: response.session_id,
            user_id: response.user_id,
        }),
        None => None,
    }
}

pub async fn extract_session_header(headers: HeaderMap) -> Result<uuid::Uuid, MyError> {
    let session;
    match headers.get("session_id") {
        Some(session_id) => session = session_id,
        None => return Err(MyError::UnauthorizedError),
    }
    let session_id = uuid::Uuid::parse_str(session.to_str().unwrap()).unwrap();
    Ok(session_id)
}

fn paginate_orders(pagination: MyOrderQuery) -> String {
    struct PaginationParams {
        take: u32,
        offset: u32,
        dispatched: bool,
    }
    // Default values
    let mut query_params = PaginationParams {
        take: 10,
        offset: 0,
        dispatched: false,
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
        Some(dispatched) => query_params.dispatched = dispatched,
        None => query_params.dispatched = false,
    }
    format!(
        r#"SELECT t1."order_id",t1."item_id",t1."quantity",t2."order_date",t2."address_id",t2."dispatched" FROM 
        (SELECT * from "order_items" ) as t1 
        INNER JOIN
        (SELECT * FROM "order" WHERE "user_id" = $1 ) as t2
        ON t1."order_id" = t2."order_id"
        WHERE "dispatched" = {} ORDER BY t2."order_date" DESC LIMIT {} OFFSET {};"#,
        query_params.dispatched, query_params.take, query_params.offset
    )
}
