use dotenv::dotenv;

use axum::{
    routing::{get, post},
    Json, Router,
};
use chrono::Duration;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use utoipa::ToSchema;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};

pub mod user;
use crate::user::user as User;

pub mod item;
use crate::item::item as Item;

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
        // ping,
        User::signup,
        User::get_user_by_id,
        User::user_login,
        User::logout,
        Item::create_item
    ),
    components(
        schemas(
            Ping,
            User::User,
            User::CreateUserForm,
            User::UserLogin,
            User::Session,
            User::UserWithSession,
            User::GeneralResponse,
            User::SessionResponse,
            User::UserResponse,
            Item::Item,
            Item::ItemForm,
            Item::ItemResponse,
        )
    )
)]
struct ApiDoc;

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
        .route("/login", post(User::user_login))
        .route("/signup", post(User::signup))
        .route("/logout", post(User::logout))
        .route("/:username", get(User::get_user_by_id))
        .with_state(dbpool.clone());

    let item_router = Router::new().with_state(dbpool.clone())
        .route("/create",post(Item::create_item))
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
