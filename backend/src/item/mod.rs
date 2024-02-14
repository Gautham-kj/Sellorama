pub mod item {
    use crate::user::user::{check_session_validity, GeneralResponse};
    use crate::AppState;
    use axum::{
        extract::{Path, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        Form, Json,
    };
    use chrono::{NaiveDateTime, Utc};
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use sqlx::{prelude::FromRow, types::chrono, Pool, Postgres};
    use utoipa::ToSchema;
    use uuid::{uuid, Uuid};

    #[derive(Serialize, Deserialize, ToSchema)]
    pub struct ItemForm {
        session_id: Uuid,
        title: String,
        content: String,
    }

    #[derive(Serialize, Deserialize, FromRow, ToSchema)]
    pub struct Item {
        item_id: Uuid,
    }

    #[derive(Serialize, Deserialize, ToSchema)]
    pub struct ItemResponse {
        detail: Item,
    }
    #[utoipa::path(
        post,
        path="/item/create",
        responses (
            (status = 201, body = ItemResponse),
            (status = 100, body = Item)
        )
    )]
    pub async fn create_item(
        state: State<AppState>,
        Form(form_data): Form<ItemForm>,
    ) -> impl IntoResponse {
        match check_session_validity(&state.db_pool, form_data.session_id).await {
            Some(userwithsession) => {
                let query = r#"
                    INSERT INTO "item" ("user_id","title","content") 
                    VALUES ($1,$2,$3) returning "item_id";
                "#;
                match sqlx::query_as::<_, Item>(query)
                    .bind(userwithsession.user_id)
                    .bind(&form_data.title)
                    .bind(&form_data.content)
                    .fetch_optional(&state.db_pool)
                    .await
                    .unwrap()
                {
                    Some(response) => (
                        StatusCode::CREATED,
                        Json(json!(ItemResponse {
                            detail: Item {
                                item_id: response.item_id
                            }
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
        state: State<AppState>,
        Path(item_id): Path<Uuid>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        let session;
        match headers.get("session_id") {
            Some(session_id) => session = session_id,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!(GeneralResponse {
                        detail: "Invalid credentials".to_string()
                    })),
                )
            }
        };
        let session_id = Uuid::parse_str(session.to_str().unwrap()).unwrap();
        match check_session_validity(&state.db_pool, session_id).await {
            Some(response) => {
                println!("{:?}",response);
                match sqlx::query_as::<_,Item>(r#"DELETE FROM "item" WHERE "item_id" = $1 AND "user_id" = $2 RETURNING "item_id" "#)
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

    // pub async fn get_item_by_id(
    //     state: State<AppState>,
    //     Path(item_id): Path<Uuid>,
    // )-> impl IntoResponse{

    // }
}
