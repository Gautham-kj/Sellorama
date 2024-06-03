use dotenv::dotenv;

use axum::{
    http::Method,
    routing::{delete, get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
// use tokio::runtime::{Runtime,Builder};

use chrono::Duration;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use utoipa::ToSchema;

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

mod cart;
mod errors;
mod item;
mod objects;
mod order;
mod tests;
mod user;

use cart::{add_item, check_cart, get_cart, update_cart_item, Cart, CartItem, CartResponse};
use errors::ErrorResponse;
use item::{
    create_item, delete_item, edit_item, edit_stock, get_item, get_items, rate_item,
    search_suggestions, Item, ItemForm, ItemId, ItemResponse, ItemStock, PageResponse, RateForm,
    SearchQuery, SearchResult,
};
use order::{create_order, OrderDetails, OrderForm};
use user::{
    create_user_address, get_user_by_id, logout, signup, user_login, Address, AddressId,
    CreateUserForm, GeneralResponse, Session, SessionResponse, User, UserLogin, UserResponse,
    UserWithSession,
};

#[derive(Deserialize, PartialEq, ToSchema)]
pub enum Order {
    Inc,
    Dec,
}
#[derive(PartialEq, ToSchema)]
pub enum Filters {
    Rating(Order),
    DateOfCreation(Order),
    Alphabetical(Order),
    Price(Order),
}

impl<'de> Deserialize<'de> for Filters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = String::deserialize(deserializer)?;
        match input.as_str() {
            "Rating" => Ok(Filters::Rating(Order::Inc)),
            "Rating(Inc)" => Ok(Filters::Rating(Order::Inc)),
            "Rating(Dec)" => Ok(Filters::Rating(Order::Dec)),
            "DateOfCreation(Inc)" => Ok(Filters::DateOfCreation(Order::Inc)),
            "DateOfCreation(Dec)" => Ok(Filters::DateOfCreation(Order::Dec)),
            "Alphabetical" => Ok(Filters::Alphabetical(Order::Inc)),
            "Alphabetical(Inc)" => Ok(Filters::Alphabetical(Order::Inc)),
            "Alphabetical(Dec)" => Ok(Filters::Alphabetical(Order::Dec)),
            "Price" => Ok(Filters::Price(Order::Inc)),
            "Price(Inc)" => Ok(Filters::Price(Order::Inc)),
            "Price(Dec)" => Ok(Filters::Price(Order::Dec)),
            _ => Err(serde::de::Error::custom("Invalid value")),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    db_pool: Pool<Postgres>,
    s3_client: aws_sdk_s3::Client,
    image_bucket: String,
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
        user::create_user_address,
        item::create_item,
        item::edit_item,
        item::get_item,
        item::get_items,
        item::delete_item,
        item::rate_item,
        item::edit_stock,
        item::search_suggestions,
        cart::get_cart,
        cart::add_item,
        cart::update_cart_item,
        cart::check_cart
    ),
    components(
        schemas(
            Ping,
            User,
            CreateUserForm,
            AddressId,
            Address,
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
            ItemStock,
            PageResponse,
            SearchQuery,
            SearchResult,
            RateForm,
            Cart,
            CartItem,
            CartResponse,
            Filters,
            Order,
            OrderDetails,
            OrderForm,
            ErrorResponse
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

    // Getting env variables
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let api_url = std::env::var("API_URL").unwrap_or_else(|_| "localhost:9000".to_string());
    // Getting S3 env variables
    let s3_access_key = std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY must be set");
    let s3_secret_access_key =
        std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_KEY must be set");
    let s3_endpoint_url = std::env::var("AWS_ENDPOINT_URL").expect("AWS_ENDPOINT_URL must be set");
    let s3_region = std::env::var("AWS_REGION").expect("AWS_REGION must be set");
    let image_bucket = std::env::var("IMAGE_BUCKET").expect("IMAGE_BUCKET_NAME must be set");

    let s3_credentials = objects::S3Credentials::new(
        s3_access_key,
        s3_secret_access_key,
        None,
        None,
        s3_endpoint_url,
    );

    let s3_client = objects::get_s3_client(s3_region.to_owned(), s3_credentials)
        .await
        .unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Error building a connection pool");

    match sqlx::migrate!("./migrations").run(&pool).await {
        Err(e) => println!("{e}"),
        Ok(_) => (),
    };

    let appstate = AppState {
        db_pool: pool.clone(),
        s3_client: s3_client,
        image_bucket: image_bucket,
    };

    let listener = tokio::net::TcpListener::bind(api_url).await.unwrap();
    axum::serve(listener, app(appstate)).await.unwrap();
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
pub fn app(appstate: AppState) -> Router {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        // allow requests from any origin
        .allow_origin(Any);

    let user_router = Router::new()
        .route("/login", post(user_login))
        .route("/signup", post(signup))
        .route("/logout", post(logout))
        .route("/:username", get(get_user_by_id))
        .route("/address", post(create_user_address))
        .with_state(appstate.clone());

    let item_router = Router::new()
        .route("/create", post(create_item))
        .route(
            "/:item_id",
            delete(delete_item).put(edit_item).get(get_item),
        )
        .route("/", get(get_items))
        .route("/stock", post(edit_stock))
        .route("/search_suggestions", get(search_suggestions))
        .route("/rate", post(rate_item))
        .with_state(appstate.clone());

    let cart_router = Router::new()
        .route("/", get(get_cart))
        .route("/item", post(add_item))
        .route("/update", post(update_cart_item))
        .route("/subcheckout", get(check_cart))
        .with_state(appstate.clone());

    let order_router = Router::new()
        .route("/create", post(create_order))
        .with_state(appstate.clone());

    let app = Router::new()
        .route("/", get(ping))
        .nest("/cart", cart_router)
        .nest("/user", user_router)
        .nest("/item", item_router)
        .nest("/order", order_router)
        .merge(SwaggerUi::new("/docs").url("/apidoc", ApiDoc::openapi()))
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        .merge(RapiDoc::new("/apidoc").path("/rapidoc"))
        .layer(cors);
    app
}
