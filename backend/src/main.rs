use dotenv::dotenv;

use axum::{
    routing::{get, post},
    Json, Router,
};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use serde::Serialize;
use utoipa::ToSchema;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod user;
use crate::user::user as User;

pub mod post;
// use crate::post::post as Post;

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
    paths(
        // ping,
        User::get_user_by_id,
        User::signup
    ),
    components(
        schemas(
            Ping,
            User::User,
            User::CreateUserForm,
            // UserSession,
            User::GeneralResponse<User::User>,
            User::GeneralResponse<String>
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
        Err(e) => panic!("{e}"),
        Ok(_) => (),
    };

    let dbpool = AppState { db_pool: pool.clone() };

    let user_router = Router::new()
        .route("/signup", post(User::signup))
        .route("/:username", get(User::get_user_by_id))
        .with_state(dbpool.clone());

    let post_router = Router::new().with_state(dbpool.clone());

    let comment_router = Router::new().with_state(dbpool.clone());

    let app = Router::new()
        .route("/", get(ping))
        .nest("/user", user_router)
        .nest("/post", post_router)
        .nest("/comment", comment_router)
        .with_state(dbpool)
        .merge(SwaggerUi::new("/docs").url("/apidoc", ApiDoc::openapi()));
    
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
