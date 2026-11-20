use gloo_net::http::Request;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use studio_core::GameStage;
use wasm_bindgen::prelude::*;

const API: &str = "http://127.0.0.1:3000";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GameDto {
    id: String,
    name: String,
    genre: Option<String>,
    platforms: Vec<String>,
    stage: String,
    owner_id: String,
    store_url: Option<String>,
    version: i64,
    updated_at: String,
}

#[derive(Serialize)]
struct CreateGameBody<'a> {
    name: &'a str,
    genre: Option<&'a str>,
    platforms: &'a [String],
    stage: &'a str,
    store_url: Option<&'a str>,
}

#[derive(Serialize)]
struct PatchStageBody<'a> {
    stage: &'a str,
}

fn stage_label(stage: &str) -> &'static str {
    match stage {
        "idea" => "Ý tưởng",
        "prototype" => "Prototype",
        "vertical_slice" => "Vertical slice",
        "soft_launch" => "Soft launch",
        "live" => "Live",
        "stopped" => "Ngừng",
        _ => "Không rõ",
    }
}

fn parse_platforms(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn blank_to_option(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

async fn error_message(response: gloo_net::http::Response) -> String {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(message) = value.get("error").and_then(|item| item.as_str()) {
            return format!("{message} ({status})");
        }
    }
    if text.is_empty() {
        format!("Lỗi API ({status})")
    } else {
        format!("{text} ({status})")
    }
}

async fn fetch_games() -> Result<Vec<GameDto>, String> {
    let response = Request::get(&format!("{API}/api/games"))
        .send()
        .await
        .map_err(|err| format!("Không gọi được API: {err}"))?;
    if !response.ok() {
        return Err(error_message(response).await);
    }
    response
        .json()
        .await
        .map_err(|err| format!("API trả về dữ liệu không đọc được: {err}"))
}

async fn create_game(body: &CreateGameBody<'_>) -> Result<GameDto, String> {
    let response = Request::post(&format!("{API}/api/games"))
        .json(body)
        .map_err(|err| format!("Không tạo được yêu cầu: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Không gọi được API: {err}"))?;
    if !response.ok() {
        return Err(error_message(response).await);
    }
    response
        .json()
        .await
        .map_err(|err| format!("API trả về dữ liệu không đọc được: {err}"))
}

async fn patch_stage(id: &str, stage: &str) -> Result<GameDto, String> {
    let response = Request::patch(&format!("{API}/api/games/{id}"))
        .json(&PatchStageBody { stage })
        .map_err(|err| format!("Không tạo được yêu cầu: {err}"))?
        .send()
        .await
        .map_err(|err| format!("Không gọi được API: {err}"))?;
    if !response.ok() {
        return Err(error_message(response).await);
    }
    response
        .json()
        .await
        .map_err(|err| format!("API trả về dữ liệu không đọc được: {err}"))
}

fn spawn_reload(
    set_games: WriteSignal<Vec<GameDto>>,
    set_error: WriteSignal<Option<String>>,
    set_loading: WriteSignal<bool>,
    selected_id: RwSignal<Option<String>>,
) {
    spawn_local(async move {
        set_loading.set(true);
        match fetch_games().await {
            Ok(list) => {
                set_games.set(list.clone());
                set_error.set(None);
                selected_id.update(|current| {
                    if let Some(id) = current.clone() {
                        if !list.iter().any(|game| game.id == id) {
                            *current = None;
                        }
                    }
                });
            }
            Err(err) => set_error.set(Some(err)),
        }
        set_loading.set(false);
    });
}

fn stage_options() -> impl IntoView {
    GameStage::ALL
        .into_iter()
        .map(|stage| {
            let value = stage.as_str();
            view! { <option value=value>{stage_label(value)}</option> }
        })
        .collect_view()
}

#[component]
pub fn App() -> impl IntoView {
    let games = RwSignal::new(Vec::<GameDto>::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let selected_id = RwSignal::new(None::<String>);
    let draft_stage = RwSignal::new(String::from("idea"));
    let name = RwSignal::new(String::new());
    let genre = RwSignal::new(String::new());
    let platforms = RwSignal::new(String::new());
    let create_stage = RwSignal::new(String::from("idea"));
    let store_url = RwSignal::new(String::new());
    let stage_filter = RwSignal::new(String::from("all"));

    spawn_reload(
        games.write_only(),
        error.write_only(),
        loading.write_only(),
        selected_id,
    );

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name_value = name.get().trim().to_string();
        if name_value.is_empty() {
            error.set(Some("Tên không được để trống".to_string()));
            return;
        }
        let genre_value = genre.get();
        let store_value = store_url.get();
        let platform_values = parse_platforms(&platforms.get());
        let stage_value = create_stage.get();
        let set_error = error.write_only();
        let set_games = games.write_only();
        let set_loading = loading.write_only();
        let selected = selected_id;
        let set_name = name.write_only();
        let set_genre = genre.write_only();
        let set_platforms = platforms.write_only();
        let set_store = store_url.write_only();
        spawn_local(async move {
            let body = CreateGameBody {
                name: &name_value,
                genre: blank_to_option(&genre_value),
                platforms: &platform_values,
                stage: &stage_value,
                store_url: blank_to_option(&store_value),
            };
            match create_game(&body).await {
                Ok(game) => {
                    draft_stage.set(game.stage.clone());
                    selected.set(Some(game.id));
                    set_name.set(String::new());
                    set_genre.set(String::new());
                    set_platforms.set(String::new());
                    set_store.set(String::new());
                    spawn_reload(set_games, set_error, set_loading, selected);
                }
                Err(err) => set_error.set(Some(err)),
            }
        });
    };

    view! {
        <main>
            <h1>"Danh mục game"</h1>
            {move || error.get().map(|message| view! { <p class="banner error" id="api-error">{message}</p> })}
            <section>
                <h2>"Tạo game"</h2>
                <form id="create-game" on:submit=on_create>
                    <label>
                        "Tên"
                        <input id="game-name" prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev)) />
                    </label>
                    <label>
                        "Thể loại"
                        <input id="game-genre" prop:value=move || genre.get() on:input=move |ev| genre.set(event_target_value(&ev)) />
                    </label>
                    <label>
                        "Nền tảng"
                        <input id="game-platforms" placeholder="pc, ios" prop:value=move || platforms.get() on:input=move |ev| platforms.set(event_target_value(&ev)) />
                    </label>
                    <label>
                        "Giai đoạn"
                        <select id="game-stage" prop:value=move || create_stage.get() on:change=move |ev| create_stage.set(event_target_value(&ev))>
                            {stage_options()}
                        </select>
                    </label>
                    <label>
                        "Link store"
                        <input id="game-store" prop:value=move || store_url.get() on:input=move |ev| store_url.set(event_target_value(&ev)) />
                    </label>
                    <button type="submit">"Tạo game"</button>
                </form>
            </section>
            <section>
                <label>
                    "Lọc giai đoạn"
                    <select id="stage-filter" prop:value=move || stage_filter.get() on:change=move |ev| stage_filter.set(event_target_value(&ev))>
                        <option value="all">"Tất cả"</option>
                        {stage_options()}
                    </select>
                </label>
            </section>
            {move || {
                let show = games.get().is_empty() && (loading.get() || error.get().is_none());
                if show {
                    let message = if loading.get() {
                        "Đang tải..."
                    } else {
                        "Studio chưa có game."
                    };
                    Either::Left(view! { <p class="empty" id="catalog-status">{message}</p> })
                } else {
                    Either::Right(())
                }
            }}
            {move || {
                let filter = stage_filter.get();
                let rows: Vec<GameDto> = games
                    .get()
                    .into_iter()
                    .filter(|game| filter == "all" || game.stage == filter)
                    .collect();
                let current = selected_id.get();
                if games.get().is_empty() {
                    Either::Left(())
                } else if rows.is_empty() {
                    Either::Right(Either::Left(view! {
                        <p class="empty">"Không có game ở giai đoạn này."</p>
                    }))
                } else {
                    Either::Right(Either::Right(view! {
                        <div class="table-wrap">
                            <table id="game-table">
                                <thead>
                                    <tr>
                                        <th>"Tên"</th>
                                        <th>"Giai đoạn"</th>
                                        <th>"Nền tảng"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows.into_iter().map(|game| {
                                        let id = game.id.clone();
                                        let row_id = id.clone();
                                        let label = stage_label(&game.stage);
                                        let platform_text = if game.platforms.is_empty() {
                                            "—".to_string()
                                        } else {
                                            game.platforms.join(", ")
                                        };
                                        let name_text = game.name.clone();
                                        let stage_value = game.stage.clone();
                                        let selected = current.as_deref() == Some(id.as_str());
                                        view! {
                                            <tr
                                                class:selected=selected
                                                on:click=move |_| {
                                                    selected_id.set(Some(row_id.clone()));
                                                    draft_stage.set(stage_value.clone());
                                                }
                                            >
                                                <td>{name_text}</td>
                                                <td>{label}</td>
                                                <td>{platform_text}</td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }))
                }
            }}
            {move || {
                let current = selected_id.get();
                let game = current.as_ref().and_then(|id| {
                    games.get().into_iter().find(|game| &game.id == id)
                });
                match game {
                    None => Either::Left(()),
                    Some(game) => {
                        let id = game.id.clone();
                        let heading = game.name.clone();
                        let genre_text = game.genre.clone().unwrap_or_else(|| "—".to_string());
                        let platform_text = if game.platforms.is_empty() {
                            "—".to_string()
                        } else {
                            game.platforms.join(", ")
                        };
                        let stage_text = stage_label(&game.stage).to_string();
                        let owner = game.owner_id.clone();
                        let version = game.version;
                        let updated = game.updated_at.clone();
                        let store_text = game.store_url.clone().unwrap_or_else(|| "—".to_string());
                        let store_href = game.store_url.clone().filter(|url| {
                            let lower = url.to_ascii_lowercase();
                            lower.starts_with("https://") || lower.starts_with("http://")
                        });
                        Either::Right(view! {
                            <section id="game-detail">
                                <button class="ghost" type="button" on:click=move |_| selected_id.set(None)>
                                    "Quay lại danh mục"
                                </button>
                                <h2>{heading}</h2>
                                <dl>
                                    <dt>"Thể loại"</dt>
                                    <dd>{genre_text}</dd>
                                    <dt>"Nền tảng"</dt>
                                    <dd>{platform_text}</dd>
                                    <dt>"Giai đoạn"</dt>
                                    <dd>{stage_text}</dd>
                                    <dt>"Chủ sở hữu"</dt>
                                    <dd>{owner}</dd>
                                    <dt>"Cửa hàng"</dt>
                                    <dd>{
                                        match store_href {
                                            Some(url) => {
                                                let href = url.clone();
                                                Either::Left(view! {
                                                    <a href=href target="_blank" rel="noreferrer">{url}</a>
                                                })
                                            }
                                            None => Either::Right(store_text),
                                        }
                                    }</dd>
                                    <dt>"Phiên bản"</dt>
                                    <dd>{version}</dd>
                                    <dt>"Cập nhật"</dt>
                                    <dd>{updated}</dd>
                                </dl>
                                <div class="stage-edit">
                                    <label>
                                        "Đổi giai đoạn"
                                        <select id="detail-stage" prop:value=move || draft_stage.get() on:change=move |ev| draft_stage.set(event_target_value(&ev))>
                                            {stage_options()}
                                        </select>
                                    </label>
                                    <button id="save-stage" type="button" on:click=move |_| {
                                        let id = id.clone();
                                        let stage = draft_stage.get();
                                        let set_error = error.write_only();
                                        let set_games = games.write_only();
                                        let set_loading = loading.write_only();
                                        let selected = selected_id;
                                        spawn_local(async move {
                                            match patch_stage(&id, &stage).await {
                                                Ok(_) => spawn_reload(set_games, set_error, set_loading, selected),
                                                Err(err) => set_error.set(Some(err)),
                                            }
                                        });
                                    }>"Lưu giai đoạn"</button>
                                </div>
                                <div class="placeholders">
                                    <p>"Tài liệu"</p>
                                    <p>"(trống tuần này: không dữ liệu, không hành vi)"</p>
                                    <p>"Task"</p>
                                    <p>"(trống tuần này: không dữ liệu, không hành vi)"</p>
                                </div>
                            </section>
                        })
                    }
                }
            }}
        </main>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });
}
