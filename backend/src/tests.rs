#[cfg(test)]
mod tests {
    use crate::{dotenv, objects, AppState, PgPoolOptions};

    use axum::Router;
    use reqwest::multipart;

    async fn create_app() -> (Router, String) {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let api_url = std::env::var("API_URL").unwrap_or_else(|_| "localhost:9000".to_string());
        // Getting S3 env variables
        let s3_access_key = std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY must be set");
        let s3_secret_access_key =
            std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_KEY must be set");
        let s3_endpoint_url =
            std::env::var("AWS_ENDPOINT_URL").expect("AWS_ENDPOINT_URL must be set");
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

        let appstate = AppState {
            db_pool: pool.clone(),
            s3_client: s3_client,
            image_bucket: image_bucket,
        };
        let app = crate::app(appstate);
        (app, api_url)
    }

    async fn start_app_instance() -> String {
        let (app, url) = create_app().await;
        let listener = tokio::net::TcpListener::bind(url.clone()).await.unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        url
    }

    async fn get_session_id(url: String) -> uuid::Uuid {
        let mut params = std::collections::HashMap::new();
        params.insert("username", "test_user");
        params.insert("password", "test_pass");
        params.insert("email_id", "test@testing.com");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/user/login", url);
        let res = client
            .post(endpoint_url)
            .form(&params)
            .send()
            .await
            .map_err(|_| assert!(false))
            .unwrap();

        let session: crate::SessionResponse =
            serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();
        session.detail.session_id
    }
    #[tokio::test]

    async fn test_1_signup_with_valid_creds() {
        let url = start_app_instance().await;
        let mut params = std::collections::HashMap::new();
        params.insert("username", "test_user");
        params.insert("password", "test_pass");
        params.insert("email_id", "test@testing.com");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/user/signup", url);
        let res = client
            .post(endpoint_url)
            .form(&params)
            .send()
            .await
            .map_err(|err| println!("{}", err))
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    }
    #[tokio::test]

    async fn test_2_signup_with_invalid_creds() {
        let url = start_app_instance().await;
        let mut params = std::collections::HashMap::new();
        params.insert("username", "test_user");
        params.insert("password", "test_pass");
        params.insert("email_id", "test@testing.com");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/user/signup", url);
        let res = client
            .post(endpoint_url)
            .form(&params)
            .send()
            .await
            .map_err(|err| println!("{}", err))
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_3_login_with_valid_creds() {
        let url = start_app_instance().await;
        let mut params = std::collections::HashMap::new();
        params.insert("username", "test_user");
        params.insert("password", "test_pass");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/user/login", url);
        let res = client
            .post(endpoint_url)
            .form(&params)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_4_login_with_invalid_creds() {
        let url = start_app_instance().await;
        let mut params = std::collections::HashMap::new();
        params.insert("username", "test_user");
        params.insert("password", "test_notpass");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/user/login", url);
        let res = client
            .post(endpoint_url)
            .form(&params)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_5_create_post_without_image() {
        let url = start_app_instance().await;
        let session_id = get_session_id(url.clone()).await;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("session_id", session_id.to_string().parse().unwrap());
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let form = multipart::Form::new()
            .text("title", "Item 1")
            .text("content", "Details about item")
            .text("price", "99.99");
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/item/create", url);
        let res = client
            .post(endpoint_url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .map_err(|_| assert!(false))
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_6_create_post_with_image() {
        let url = start_app_instance().await;
        let session_id = get_session_id(url.clone()).await;
        //headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("session_id", session_id.to_string().parse().unwrap());
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let image_fd = std::fs::read("./test_assets/sellorama_test.jpg").expect("file not found");
        let part = multipart::Part::bytes(image_fd).file_name("image.jpg");
        let form = multipart::Form::new()
            .text("title", "Item with image")
            .text("content", "Details about item with image")
            .text("price", "1099.99")
            .part("item_media", part);
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/item/create", url);
        let res = client
            .post(endpoint_url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .map_err(|_| assert!(false))
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_7_get_items() {
        let url = start_app_instance().await;
        let session_id = get_session_id(url.clone()).await;
        //headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("session_id", session_id.to_string().parse().unwrap());
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::new();
        let endpoint_url = format!("http://{}/item", url);
        let res = client.get(endpoint_url).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}
