# ADR 0003: Môi trường chạy

Trạng thái: đã chấp nhận cho tuần này.

## Bối cảnh

Tuần này chỉ cần một Postgres local để chạy migration và phát triển. Chưa có số người dùng thật và chưa có ước lượng kích thước asset, nên chưa chọn nơi chạy production.

## Quyết định

- Runtime duy nhất trong tuần này là Docker Compose trên máy local:
  - Postgres 17
  - user, database và password: `studio`
  - cổng 5432
  - migration: `migrations/0001_studio_user_game.sql`
- Nhà cung cấp: để trống. Chưa chọn Fly.io, Railway, VPS, Neon, Supabase hay R2. Chọn lại khi có số người dùng thật và ước lượng kích thước asset.
- Deploy production, TLS và backup nằm ngoài phạm vi tuần này.

## Hệ quả

- Môi trường làm việc tuần này là Postgres 17 local qua Docker Compose, với user, database, password và cổng ở trên, rồi áp migration `migrations/0001_studio_user_game.sql`.
- Không deploy production, không cấu hình TLS và không lập backup trong tuần này.
- Mục nhà cung cấp để trống cho đến khi có số người dùng thật và ước lượng kích thước asset.
