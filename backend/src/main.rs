use dotenv::dotenv;

use axum::{
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Duration;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use utoipa::ToSchema;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub mod item;
pub mod user;

use item::{create_item, delete_item, edit_item, Item, ItemForm, ItemId, ItemResponse};
use user::{
    get_user_by_id, logout, signup, user_login, CreateUserForm, GeneralResponse, Session,
    SessionResponse, User, UserLogin, UserResponse, UserWithSession,
};

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
}

#[derive(Serialize, ToSchema)]
struct Ping {
    response: String,
}

#[derive(OpenApi)]
#[openapi(
    info(description = "API documentation for Sellorama",
title = "Sellorama"),
    paths(
        user::signup,
        user::get_user_by_id,
        user::user_login,
        user::logout,
        item::create_item,
        item::delete_item,
        item::edit_item
    ),
    components(
        schemas(
            Ping,
            User,
            CreateUserForm,
            UserLogin,
            Session,
            UserWithSession,
            GeneralResponse,
            SessionResponse,
            UserResponse,
            Item,
            ItemId,
            ItemForm,
            ItemResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "session_id",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("session_id"))),
            )
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Error building a connection pool");

    match sqlx::migrate!("./migrations").run(&pool).await {
        Err(e) => println!("{e}"),
        Ok(_) => (),
    };

    let dbpool = AppState {
        db_pool: pool.clone(),
    };

    let user_router = Router::new()
        .route("/login", post(user_login))
        .route("/signup", post(signup))
        .route("/logout", post(logout))
        .route("/:username", get(get_user_by_id))
        .with_state(dbpool.clone());

    let item_router = Router::new()
        .with_state(dbpool.clone())
        .route("/create", post(create_item))
        .route("/:item_id", delete(delete_item))
        .route("/:item_id", put(edit_item))
        .with_state(dbpool.clone());

    let comment_router = Router::new().with_state(dbpool.clone());

    let app = Router::new()
        .route("/", get(ping))
        .nest("/user", user_router)
        .nest("/item", item_router)
        .nest("/comment", comment_router)
        .merge(SwaggerUi::new("/docs").url("/apidoc", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .merge(RapiDoc::new("/apidoc").path("/rapidoc"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, body=[Ping])
    )
)]
async fn ping() -> Json<Ping> {
    println!("Server was pinged!");
    let ping = Ping {
        response: "Pong".to_string(),
    };
    let response = Json(ping);
    response
}
