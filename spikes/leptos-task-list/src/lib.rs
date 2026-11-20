use leptos::prelude::*;
use std::cell::Cell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

const TOTAL: usize = 10_000;
const ROW_H: usize = 32;
const WINDOW: usize = 40;

struct Task {
    title: &'static str,
    status: &'static str,
    assignee: &'static str,
}

fn task_list() -> &'static [Task] {
    use std::sync::OnceLock;
    static TASKS: OnceLock<Vec<Task>> = OnceLock::new();
    TASKS.get_or_init(|| {
        const STATUS: [&str; 3] = ["todo", "doing", "done"];
        const WHO: [&str; 4] = ["An", "Binh", "Chi", "Dung"];
        (0..TOTAL)
            .map(|i| Task {
                title: Box::leak(format!("Task {i}").into_boxed_str()),
                status: STATUS[i % 3],
                assignee: WHO[i % 4],
            })
            .collect()
    })
}

fn now_ms() -> f64 {
    web_sys::window()
        .expect("window")
        .performance()
        .expect("performance")
        .now()
}

fn fmt_ms(value: Option<f64>) -> String {
    match value {
        Some(ms) => format!("{ms:.3}"),
        None => "pending".to_string(),
    }
}

fn window_at(start: usize) -> (usize, usize) {
    let start = start.min(TOTAL.saturating_sub(WINDOW));
    (start, (start + WINDOW).min(TOTAL))
}

thread_local! {
    static SCHEDULED_PHASE: Cell<u8> = const { Cell::new(255) };
}

#[wasm_bindgen]
extern "C" {
    fn queueMicrotask(callback: &JsValue);
}

fn after_microtask(f: impl FnOnce() + 'static) {
    let callback = wasm_bindgen::closure::Closure::once(f);
    queueMicrotask(callback.as_ref());
    callback.forget();
}

#[component]
pub fn App() -> impl IntoView {
    let _ = task_list();
    let (phase, set_phase) = signal(0u8);
    let (virt_ms, set_virt_ms) = signal(None::<f64>);
    let (full_ms, set_full_ms) = signal(None::<f64>);
    let (scroll_index, set_scroll_index) = signal(0usize);
    let (show_all, set_show_all) = signal(false);

    let on_scroll = move |ev: web_sys::Event| {
        let target = ev.target().expect("scroll target");
        let el = target.dyn_into::<web_sys::Element>().expect("element");
        let next = (el.scroll_top().max(0) as usize) / ROW_H;
        let next = next.min(TOTAL.saturating_sub(WINDOW));
        if next != scroll_index.get_untracked() {
            set_scroll_index.set(next);
        }
    };

    view! {
        <main>
            <h1>"Leptos 0.8.23 task list"</h1>
            <p id="virt-ms">{move || format!("virtual render ms: {}", fmt_ms(virt_ms.get()))}</p>
            <p id="full-ms">{move || format!("full render ms: {}", fmt_ms(full_ms.get()))}</p>
            <p id="mode">{move || {
                let p = phase.get();
                let full = p == 1 || show_all.get();
                format!("mode: {} phase: {p}", if full { "full" } else { "virtual" })
            }}</p>
            <p id="window">{move || {
                let p = phase.get();
                let full = p == 1 || show_all.get();
                if full {
                    format!("window: 0-{TOTAL} rendered: {TOTAL}")
                } else {
                    let (start, end) = window_at(scroll_index.get());
                    format!("window: {start}-{end} rendered: {}", end - start)
                }
            }}</p>
            <button id="toggle" on:click=move |_| set_show_all.update(|value| *value = !*value)>
                {move || if show_all.get() { "Show virtual window" } else { "Render all 10000" }}
            </button>
            <div id="viewport" class="viewport" on:scroll=on_scroll>
                {move || {
                    let p = phase.get();
                    let full = p == 1 || show_all.get();
                    let index = scroll_index.get();
                    let t0 = now_ms();
                    if p < 2 && SCHEDULED_PHASE.with(|cell| cell.get() != p) {
                        SCHEDULED_PHASE.with(|cell| cell.set(p));
                        let set_phase = set_phase;
                        let set_virt_ms = set_virt_ms;
                        let set_full_ms = set_full_ms;
                        after_microtask(move || {
                            let dt = now_ms() - t0;
                            web_sys::console::log_1(
                                &format!("leptos phase {p} render ms {dt:.3}").into(),
                            );
                            if p == 0 {
                                set_virt_ms.set(Some(dt));
                                set_phase.set(1);
                            } else if p == 1 {
                                set_full_ms.set(Some(dt));
                                set_phase.set(2);
                            }
                        });
                    }
                    let tasks = task_list();
                    let (start, end) = if full { (0, TOTAL) } else { window_at(index) };
                    let spacer_top = start * ROW_H;
                    let spacer_bottom = (TOTAL - end) * ROW_H;
                    view! {
                        <div>
                            <div id="spacer-top" style=format!("height: {spacer_top}px")></div>
                            {(start..end).map(|i| {
                                let title = tasks[i].title;
                                let status = tasks[i].status;
                                let assignee = tasks[i].assignee;
                                view! {
                                    <div class="row" data-row=i>
                                        <span>{title}</span>
                                        <span>{status}</span>
                                        <span>{assignee}</span>
                                    </div>
                                }
                            }).collect_view()}
                            <div id="spacer-bottom" style=format!("height: {spacer_bottom}px")></div>
                        </div>
                    }
                }}
            </div>
        </main>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let _ = task_list();
    mount_to_body(|| view! { <App/> });
}
