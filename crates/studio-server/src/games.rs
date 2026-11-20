use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use studio_core::GameStage;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::AppState;

const LIST_GAMES: &str =
    "SELECT id, name, genre, platforms, stage, owner_id, store_url, version, updated_at
         FROM games
         WHERE studio_id = $1 AND deleted_at IS NULL
         ORDER BY updated_at DESC, id DESC";

const FETCH_GAME: &str =
    "SELECT id, name, genre, platforms, stage, owner_id, store_url, version, updated_at
         FROM games
         WHERE id = $1 AND studio_id = $2 AND deleted_at IS NULL";

const INSERT_GAME: &str =
    "INSERT INTO games (id, studio_id, name, genre, platforms, stage, owner_id, store_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, name, genre, platforms, stage, owner_id, store_url, version, updated_at";

const UPDATE_GAME: &str = "UPDATE games
         SET name = $3,
             genre = $4,
             platforms = $5,
             stage = $6,
             store_url = $7,
             version = version + 1,
             updated_at = now()
         WHERE id = $1 AND studio_id = $2 AND deleted_at IS NULL
         RETURNING id, name, genre, platforms, stage, owner_id, store_url, version, updated_at";

#[derive(Debug)]
pub enum ApiError {
    BadRequest(&'static str),
    NotFound,
    Internal(&'static str),
    Db(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound => (StatusCode::NOT_FOUND, "Không tìm thấy game"),
            Self::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
            Self::Db(err) => {
                eprintln!("database error: {err}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Lỗi cơ sở dữ liệu")
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: Uuid,
    pub name: String,
    pub genre: Option<String>,
    pub platforms: Vec<String>,
    pub stage: GameStage,
    pub owner_id: Uuid,
    pub store_url: Option<String>,
    pub version: i64,
    pub updated_at: String,
}

#[derive(Debug, FromRow)]
struct GameRow {
    id: Uuid,
    name: String,
    genre: Option<String>,
    platforms: Vec<String>,
    stage: String,
    owner_id: Uuid,
    store_url: Option<String>,
    version: i64,
    updated_at: OffsetDateTime,
}

impl GameRow {
    fn into_response(self) -> Result<GameResponse, ApiError> {
        let stage = GameStage::parse(&self.stage).ok_or(ApiError::Internal(
            "Giai đoạn trong cơ sở dữ liệu không hợp lệ",
        ))?;
        let updated_at = self
            .updated_at
            .format(&Rfc3339)
            .map_err(|_| ApiError::Internal("Không định dạng được updated_at"))?;
        Ok(GameResponse {
            id: self.id,
            name: self.name,
            genre: self.genre,
            platforms: self.platforms,
            stage,
            owner_id: self.owner_id,
            store_url: self.store_url,
            version: self.version,
            updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CreateGameBody {
    name: String,
    genre: Option<String>,
    platforms: Option<Vec<String>>,
    stage: Option<String>,
    store_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PatchGameBody {
    name: Option<String>,
    genre: Option<String>,
    platforms: Option<Vec<String>>,
    stage: Option<String>,
    store_url: Option<String>,
}

fn decode_json<T: for<'de> Deserialize<'de>>(body: &Bytes) -> Result<T, ApiError> {
    serde_json::from_slice(body).map_err(|_| ApiError::BadRequest("JSON không hợp lệ"))
}

fn normalize_name(value: &str) -> Result<String, ApiError> {
    let name = value.trim();
    if name.is_empty() {
        Err(ApiError::BadRequest("Tên không được để trống"))
    } else {
        Ok(name.to_string())
    }
}

fn blank_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_platforms(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn require_stage(value: &str) -> Result<GameStage, ApiError> {
    GameStage::parse(value).ok_or(ApiError::BadRequest("Giai đoạn không hợp lệ"))
}

async fn fetch_game(state: &AppState, id: Uuid) -> Result<GameRow, ApiError> {
    sqlx::query_as::<_, GameRow>(FETCH_GAME)
        .bind(id)
        .bind(state.studio_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)
}

pub async fn list_games(
    State(state): State<AppState>,
) -> Result<Json<Vec<GameResponse>>, ApiError> {
    let rows = sqlx::query_as::<_, GameRow>(LIST_GAMES)
        .bind(state.studio_id)
        .fetch_all(&state.pool)
        .await?;
    rows.into_iter()
        .map(GameRow::into_response)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

pub async fn get_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameResponse>, ApiError> {
    Ok(Json(fetch_game(&state, id).await?.into_response()?))
}

pub async fn create_game(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<(StatusCode, Json<GameResponse>), ApiError> {
    let body: CreateGameBody = decode_json(&body)?;
    let name = normalize_name(&body.name)?;
    let stage = match body.stage.as_deref() {
        None => GameStage::Idea,
        Some(value) => require_stage(value)?,
    };
    let genre = body.genre.and_then(blank_to_none);
    let store_url = body.store_url.and_then(blank_to_none);
    let platforms = normalize_platforms(body.platforms.unwrap_or_default());
    let id = Uuid::now_v7();

    let row = sqlx::query_as::<_, GameRow>(INSERT_GAME)
        .bind(id)
        .bind(state.studio_id)
        .bind(name)
        .bind(genre)
        .bind(platforms)
        .bind(stage.as_str())
        .bind(state.user_id)
        .bind(store_url)
        .fetch_one(&state.pool)
        .await?;

    Ok((StatusCode::CREATED, Json(row.into_response()?)))
}

pub async fn patch_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Bytes,
) -> Result<Json<GameResponse>, ApiError> {
    let body: PatchGameBody = decode_json(&body)?;
    let current = fetch_game(&state, id).await?;
    let name = match body.name {
        Some(value) => normalize_name(&value)?,
        None => current.name,
    };
    let genre = match body.genre {
        Some(value) => blank_to_none(value),
        None => current.genre,
    };
    let platforms = match body.platforms {
        Some(values) => normalize_platforms(values),
        None => current.platforms,
    };
    let stage = match body.stage.as_deref() {
        Some(value) => require_stage(value)?,
        None => require_stage(&current.stage)?,
    };
    let store_url = match body.store_url {
        Some(value) => blank_to_none(value),
        None => current.store_url,
    };

    let row = sqlx::query_as::<_, GameRow>(UPDATE_GAME)
        .bind(id)
        .bind(state.studio_id)
        .bind(name)
        .bind(genre)
        .bind(&platforms)
        .bind(stage.as_str())
        .bind(store_url)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into_response()?))
}
