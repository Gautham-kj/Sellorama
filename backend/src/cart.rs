use crate::user::{check_session_validity, GeneralResponse};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Form, Json,
};
// use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;