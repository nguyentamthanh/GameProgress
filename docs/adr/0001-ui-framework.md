# ADR 0001: UI framework

Ngày: 2026-11-20

Trạng thái: đã chấp nhận cho web giai đoạn 1

## Bối cảnh

Mục tiêu là một crate UI dùng cho web và sau này cho desktop. Dioxus chia sẻ component mà không cần IPC. Leptos 0.8 có hệ sinh thái web chín hơn, nhưng desktop sau này sẽ đi qua IPC của Tauri 2. Spike này chỉ so sánh web WASM. Tauri 2.12.7 cố ý không nằm trong phép so sánh tuần này.

## Lựa chọn

- Dioxus 0.7.10 (ổn định). Bản 0.8.0-alpha.3 có tồn tại và bị loại khỏi spike này.
- Leptos 0.8.23 (ổn định). Bản 0.9.0-rc1 có tồn tại và bị loại.

## Quyết định

Giai đoạn 1 chỉ có web, nên chọn Leptos 0.8.23 cho `studio-web`.

WASM gzip trong trình duyệt là 50625 byte với Leptos, so với 174270 byte với Dioxus. Thời gian virtual list của hai bên gần nhau. Desktop qua Tauri vẫn hoãn và không nằm trong giai đoạn 1.

## Số đo

Cả hai bản release đều build được với `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` (rustc 1.100.0, target `wasm32-unknown-unknown`). `wasm-opt` không có trên PATH và không được cài. `wasm-bindgen` 0.2.130 cảnh báo custom section `target_features` bị `strip = true` gỡ mất; cả hai trang vẫn chạy.

Kích thước là số byte của file. Gzip là số byte đầu ra của `gzip -9 -c`.

| Hạng mục | Dioxus 0.7.10 | Leptos 0.8.23 |
| --- | ---: | ---: |
| WASM cargo, trước bindgen (byte) | 1373342 | 1238137 |
| WASM cargo, gzip -9 (byte) | 378220 | 293460 |
| WASM trình duyệt `*_bg.wasm` (byte) | 450838 | 129131 |
| WASM trình duyệt, gzip -9 (byte) | 174270 | 50625 |
| Render cửa sổ ảo lần đầu (ms) | 2.200 | 2.800 |
| Render đủ 10.000 dòng lần đầu (ms) | 110.600 | 192.400 |

File:

- `spikes/dioxus-task-list/target/wasm32-unknown-unknown/release/dioxus_task_list.wasm`
- `spikes/dioxus-task-list/dist/dioxus_task_list_bg.wasm`
- `spikes/leptos-task-list/target/wasm32-unknown-unknown/release/leptos_task_list.wasm`
- `spikes/leptos-task-list/dist/leptos_task_list_bg.wasm`

Thời gian lấy bằng `performance.now()`, từ lúc bắt đầu dựng các dòng đến microtask ngay sau khi DOM của lần dựng đó được ghi, trước paint. Lượt đầu là cửa sổ ảo 40 dòng, lượt sau là 10.000 dòng, rồi trang trở lại cửa sổ ảo. Số trên là một lần tải bản release trong trình duyệt.

Đã cuộn vùng danh sách. Dioxus đổi từ cửa sổ 0–40 sang 100–140, vẫn 40 dòng. Leptos đổi từ 0–40 sang 300–340, vẫn 40 dòng. Nút hiện đủ 10.000 dòng của cả hai tạo đúng 10.000 phần tử trên DOM.

## Hệ quả

- Web giai đoạn 1 dùng Leptos 0.8.23 (CSR) trong `studio-web` và gọi API trực tiếp.
- Desktop qua Tauri vẫn hoãn. Chưa có crate UI dùng chung cho desktop.
