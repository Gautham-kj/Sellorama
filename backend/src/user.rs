use crate::AppState;
use crate::Duration;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Form, Json,
};
use base64::engine::{general_purpose, Engine};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{prelude::FromRow, types::chrono, Pool, Postgres};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Debug, FromRow, ToSchema)]
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

#[derive(Deserialize, Serialize, ToSchema, FromRow)]
pub struct Session {
    pub session_id: Uuid,
}

#[derive(Deserialize, Serialize, ToSchema, FromRow, Debug)]
pub struct UserWithSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct CreateUserForm {
    username: String,
    email_id: String,
    password: String,
}
#[derive(FromRow, Serialize, Debug)]
pub struct UserId {
    user_id: Uuid,
}

#[derive(ToSchema, Serialize)]
pub struct GeneralResponse {
    pub detail: String,
}

#[derive(ToSchema, Serialize)]
pub struct UserResponse {
    detail: User,
}

#[derive(ToSchema, Serialize)]
pub struct SessionResponse {
    detail: Session,
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
    let (username, email_id, db_pool, password) = (
        &form_data.username,
        &form_data.email_id,
        &state.db_pool,
        create_hashed_password(form_data.password),
    );
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
    let (username, hashed_password) = (
        form_data.username,
        create_hashed_password(form_data.password),
    );
    invalidate_dangling_sessions(&state.db_pool)
        .await
        .expect("Error Deleting invalid sessions");
    let query = r#"
            WITH INS AS (
                SELECT "user_id" FROM "user"
                WHERE "username" = $1 
            )
            SELECT "user_id" FROM "password"
            WHERE "user_id" in (SELECT * FROM INS) AND "hashed_pass" = $2
        "#;

    match sqlx::query_as::<_, UserId>(query)
        .bind(username)
        .bind(hashed_password)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(user) => {
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

fn create_hashed_password(password: String) -> String {
    let hashed_password = general_purpose::STANDARD.encode(password);
    hashed_password
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
