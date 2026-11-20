use axum::http::{HeaderValue, Method, header};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

mod db;
mod games;

use games::{create_game, get_game, list_games, patch_game};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    studio_id: Uuid,
    user_id: Uuid,
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://127.0.0.1:8080"),
            HeaderValue::from_static("http://localhost:8080"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/games", get(list_games).post(create_game))
        .route("/api/games/{id}", get(get_game).patch(patch_game))
        .layer(cors_layer())
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let pool = db::connect().await.expect("connect to postgres");
    db::ensure_schema(&pool).await.expect("ensure schema");
    let state = db::ensure_dev_actor(&pool).await.expect("dev actor");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind 127.0.0.1:3000");
    println!("studio-server http://127.0.0.1:3000");
    axum::serve(listener, app(state))
        .await
        .expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_body_serializes_to_ok() {
        let Json(body) = health().await;
        assert_eq!(body, json!({ "status": "ok" }));
    }

    async fn test_state() -> AppState {
        let pool = db::connect().await.expect("postgres");
        db::ensure_schema(&pool).await.expect("schema");
        db::ensure_dev_actor(&pool).await.expect("dev actor")
    }

    async fn call(state: &AppState, request: Request<Body>) -> (StatusCode, Value) {
        let response = app(state.clone()).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, json)
    }

    fn json_request(method: &str, uri: &str, body: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        builder
            .body(Body::from(body.unwrap_or("").to_string()))
            .unwrap()
    }

    async fn delete_test_games(pool: &PgPool) {
        sqlx::query("DELETE FROM games WHERE name LIKE 'test-%'")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn games_crud_and_validation() {
        let state = test_state().await;
        delete_test_games(&state.pool).await;

        let (status, body) = call(
            &state,
            json_request("POST", "/api/games", Some(r#"{"name":"  "}"#)),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Tên không được để trống");

        let (status, body) = call(
            &state,
            json_request(
                "POST",
                "/api/games",
                Some(r#"{"name":"test-bad","stage":"shipped"}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Giai đoạn không hợp lệ");

        let (status, created) = call(
            &state,
            json_request(
                "POST",
                "/api/games",
                Some(
                    r#"{"name":" test-alpha ","genre":" puzzle ","platforms":[" pc ","","ios"],"store_url":" https://example.com/a "}"#,
                ),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created["name"], "test-alpha");
        assert_eq!(created["genre"], "puzzle");
        assert_eq!(created["platforms"], json!(["pc", "ios"]));
        assert_eq!(created["stage"], "idea");
        assert_eq!(created["owner_id"], state.user_id.to_string());
        assert_eq!(created["store_url"], "https://example.com/a");
        assert_eq!(created["version"], 1);
        let id = created["id"].as_str().unwrap().to_string();

        let (status, later) = call(
            &state,
            json_request(
                "POST",
                "/api/games",
                Some(r#"{"name":"test-beta","stage":"prototype","platforms":[]}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let later_id = later["id"].as_str().unwrap();

        let (status, patched) = call(
            &state,
            json_request(
                "PATCH",
                &format!("/api/games/{id}"),
                Some(r#"{"stage":"live"}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(patched["stage"], "live");
        assert_eq!(patched["version"], 2);
        assert_eq!(patched["name"], "test-alpha");

        let (status, listed) = call(&state, json_request("GET", "/api/games", None)).await;
        assert_eq!(status, StatusCode::OK);
        let listed = listed.as_array().unwrap();
        let ours: Vec<_> = listed
            .iter()
            .filter(|game| game["name"].as_str().unwrap_or("").starts_with("test-"))
            .collect();
        assert!(ours.len() >= 2);
        assert_eq!(ours[0]["id"].as_str(), Some(id.as_str()));
        assert!(
            ours.iter()
                .any(|game| game["id"].as_str() == Some(later_id))
        );

        let (status, fetched) = call(
            &state,
            json_request("GET", &format!("/api/games/{id}"), None),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fetched["stage"], "live");

        let (status, body) = call(
            &state,
            json_request(
                "PATCH",
                &format!("/api/games/{id}"),
                Some(r#"{"name":" ","stage":"nope"}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Tên không được để trống");

        let (status, body) = call(
            &state,
            json_request(
                "PATCH",
                &format!("/api/games/{id}"),
                Some(r#"{"stage":"nope"}"#),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Giai đoạn không hợp lệ");

        let (status, still) = call(
            &state,
            json_request("GET", &format!("/api/games/{id}"), None),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(still["version"], 2);
        assert_eq!(still["stage"], "live");

        sqlx::query("UPDATE games SET deleted_at = now() WHERE id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .execute(&state.pool)
            .await
            .unwrap();

        let (status, body) = call(
            &state,
            json_request("GET", &format!("/api/games/{id}"), None),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "Không tìm thấy game");

        let (status, listed) = call(&state, json_request("GET", "/api/games", None)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            listed
                .as_array()
                .unwrap()
                .iter()
                .all(|game| { game["id"].as_str() != Some(id.as_str()) })
        );

        let missing = Uuid::now_v7();
        let (status, _) = call(
            &state,
            json_request("GET", &format!("/api/games/{missing}"), None),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        delete_test_games(&state.pool).await;
    }

    #[tokio::test]
    async fn cors_allows_local_web_origin() {
        let state = test_state().await;
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/games")
                    .header("origin", "http://localhost:8080")
                    .header("access-control-request-method", "PATCH")
                    .header("access-control-request-headers", "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok()),
            Some("http://localhost:8080")
        );
        let allow_methods = response
            .headers()
            .get("access-control-allow-methods")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        assert!(allow_methods.contains("PATCH"));
        assert!(allow_methods.contains("POST"));
    }
}
