# Danh mục game

Wireframe chữ của màn hình danh sách game. Không phải mã giao diện.

## Ai mở được

- Producer và studio admin thấy mọi game trong studio.
- Member và viewer chỉ thấy game mình là owner hoặc được cấp quyền.

## Cột

- Tên game
- Giai đoạn: idea, prototype, vertical slice, soft launch, live, stopped
- Nền tảng
- Owner

## Lọc

Một bộ lọc theo giai đoạn. Mặc định là tất cả giai đoạn.

## Trạng thái trống

Studio chưa có game thì danh sách không có hàng. Hiện câu "Studio chưa có game."

## Phác thảo

```text
Danh mục game

Lọc giai đoạn: [ Tất cả | idea | prototype | vertical slice | soft launch | live | stopped ]

Tên game            Giai đoạn            Nền tảng            Owner
------------------------------------------------------------------------
<tên>               <giai đoạn>          <nền tảng>          <owner>

--- khi studio chưa có game ---

Danh mục game

Lọc giai đoạn: [ Tất cả ]

Studio chưa có game.
```
