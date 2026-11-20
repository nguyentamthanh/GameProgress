use dioxus::prelude::*;
use std::cell::Cell;
use wasm_bindgen::prelude::*;

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

fn app() -> Element {
    let phase = use_signal(|| 0u8);
    let virt_ms = use_signal(|| None::<f64>);
    let full_ms = use_signal(|| None::<f64>);
    let mut scroll_index = use_signal(|| 0usize);
    let mut show_all = use_signal(|| false);

    let p = phase();
    let all = show_all();
    let index = scroll_index();
    let full = p == 1 || all;
    let t0 = now_ms();

    if p < 2 && SCHEDULED_PHASE.with(|cell| cell.get() != p) {
        SCHEDULED_PHASE.with(|cell| cell.set(p));
        let mut phase = phase;
        let mut virt_ms = virt_ms;
        let mut full_ms = full_ms;
        after_microtask(move || {
            let dt = now_ms() - t0;
            web_sys::console::log_1(
                &format!("dioxus phase {p} render ms {dt:.3}").into(),
            );
            if p == 0 {
                virt_ms.set(Some(dt));
                phase.set(1);
            } else if p == 1 {
                full_ms.set(Some(dt));
                phase.set(2);
            }
        });
    }

    let (start, end) = if full {
        (0, TOTAL)
    } else {
        window_at(index)
    };
    let spacer_top = start * ROW_H;
    let spacer_bottom = (TOTAL - end) * ROW_H;
    let rendered = end - start;
    let mode = if full { "full" } else { "virtual" };
    let virt_label = fmt_ms(virt_ms());
    let full_label = fmt_ms(full_ms());
    let tasks = task_list();

    rsx! {
        main {
            h1 { "Dioxus 0.7.10 task list" }
            p { id: "virt-ms", "virtual render ms: {virt_label}" }
            p { id: "full-ms", "full render ms: {full_label}" }
            p { id: "mode", "mode: {mode} phase: {p}" }
            p { id: "window", "window: {start}-{end} rendered: {rendered}" }
            button {
                id: "toggle",
                onclick: move |_| show_all.set(!show_all()),
                if all { "Show virtual window" } else { "Render all 10000" }
            }
            div {
                id: "viewport",
                class: "viewport",
                onscroll: move |ev| {
                    let next = (ev.scroll_top().max(0.0) as usize) / ROW_H;
                    let next = next.min(TOTAL.saturating_sub(WINDOW));
                    if next != *scroll_index.peek() {
                        scroll_index.set(next);
                    }
                },
                div { id: "spacer-top", style: "height: {spacer_top}px" }
                for i in start..end {
                    div {
                        key: "{i}",
                        class: "row",
                        "data-row": "{i}",
                        span { "{tasks[i].title}" }
                        span { "{tasks[i].status}" }
                        span { "{tasks[i].assignee}" }
                    }
                }
                div { id: "spacer-bottom", style: "height: {spacer_bottom}px" }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    let _ = task_list();
    dioxus::launch(app);
}
