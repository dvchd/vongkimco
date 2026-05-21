# Changelog

Tất cả thay đổi đáng chú ý của ứng dụng desktop Vòng Kim Cô được ghi lại trong file này.

Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.7] - 2026-05-21

### Thêm mới
- Màn hình splash screen với logo vòng kim cô và hiệu ứng loading animation khi khởi động ứng dụng — thay thế hiện tượng nháy giao diện đăng nhập trước khi chuyển sang màn hình chính

## [0.1.6] - 2026-05-21

### Sửa lỗi
- Sửa lỗi "invalid updater binary format" trên Linux cài đặt bằng `.deb` — Tauri Updater chỉ hỗ trợ tự cập nhật cho AppImage, nên khi cài `.deb` sẽ hiển thị nút "Tải .deb" để tải thủ công thay vì tự cập nhật
- Thêm lệnh Tauri `is_appimage` — kiểm tra biến môi trường `APPIMAGE` để nhận diện định dạng cài đặt

## [0.1.5] - 2026-05-21

### Sửa lỗi
- Sửa lỗi phiên đăng nhập không được giữ lại khi mở lại ứng dụng — race condition giữa `boot()` (async restore session từ keyring) và `loadUser()` trên frontend; fix bằng cách Rust emit sự kiện `vkc://booted` sau khi boot xong, frontend đợi sự kiện này trước khi điều hướng
- Sửa lỗi kiểm tra cập nhật không hoạt động khi chưa đăng nhập — `UpdateBanner` giờ hiển thị trên cả màn hình đăng nhập và chọn server

### Thay đổi
- CI: `Swatinem/rust-cache@v2` dùng `continue-on-error: true` để tránh lỗi crash cache trên Windows làm thất bại toàn bộ pipeline

## [0.1.4] - 2026-05-21

### Thay đổi
- Cải thiện khu vực Server trong thanh bên — tăng kích thước vùng chạm (touch target), tách riêng nút đổi server

## [0.1.3] - 2026-05-21

### Thêm mới
- Hiển thị URL xác thực có thể sao chép trên trang đăng nhập — người dùng có thể mở thủ công trong trình duyệt nếu popup không hoạt động

## [0.1.2] - 2026-05-21

### Thêm mới
- Thêm điều hướng "Trang chủ" cho tất cả các trang admin backend
- Thêm `pull_policy: always` vào docker-compose

### Sửa lỗi
- Sửa lỗi đọc giá trị policy sau khi sleep trong các monitor loops
- Sửa lỗi `cargo fmt` gây thất bại CI lint Rust
- Thêm phiên bản backend vào footer của tất cả trang admin (base.html)

### Thay đổi
- Tối ưu UI/UX desktop — bỏ mục điều hướng server trùng lặp, cải thiện trang chủ
- Refactor `home.html` kế thừa `base.html` — loại bỏ header/footer/theme trùng lặp

## [0.1.1] - 2026-05-21

### Thêm mới
- Hiển thị phiên bản server backend trên trang Cài đặt (mục Server và mục Cập nhật ứng dụng)
- Lệnh Tauri mới `get_server_info` — lấy thông tin server đang kết nối từ `/api/v1/server-info`
- Endpoint `/api/v1/health` giờ trả thêm trường `version` của backend
- Footer trang chủ hiển thị phiên bản backend (ví dụ "Vòng Kim Cô v0.1.0")

### Sửa lỗi
- Sửa lỗi Backend CI: `secrets` context không dùng được trong `if` ở cấp step — chuyển sang kiểm tra qua `env` trong `run`
- Sửa lỗi `cargo fmt` do dòng code quá dài trong hàm `get_server_info`
- Trigger deploy Coolify dùng Bearer token thay vì webhook không xác thực

### Thay đổi
- Đơn giản hoá cấu hình Coolify deploy: gộp từ 3 secrets xuống 2 (`COOLIFY_WEBHOOK` + `COOLIFY_TOKEN`)
- Coolify deploy chỉ trigger sau khi Docker image đã push thành công lên ghcr.io (không còn deploy khi chưa build xong)

## [0.1.0] - 2026-05-20

### Thêm mới
- Ứ dụng desktop đa nền tảng (Windows, macOS Apple Silicon, Linux) xây dựng bằng Tauri 2 + Svelte + Rust
- Đăng nhập Google OAuth qua trình duyệt hệ thống
- Bắt đầu/kết thúc phiên bằng phím tắt toàn cục
- Chụp ảnh màn hình định kỳ, nén JPEG
- Theo dõi trạng thái idle/active qua sự kiện bàn phím và chuột
- Chụp snapshot ứng dụng đang chạy (foreground)
- Lưu trữ offline-first bằng SQLite, tự động đồng bộ lên backend khi có mạng
- Tự khởi động cùng hệ thống
- Giao diện tối với thanh điều hướng bên
- Tự cập nhật qua Tauri Updater với xác minh chữ ký Ed25519
- Kiểm tra cập nhật nền mỗi 4 giờ
- Thông báo cập nhật với thanh tiến trình, cài đặt, và hiển thị lỗi
- GitHub Actions CI (kiểm tra frontend + Rust, build Linux khi push main)
- GitHub Actions release với matrix 3 nền tảng (Linux, macOS ARM, Windows)
- Script đồng bộ phiên bản (`desktop/scripts/bump-version.cjs`)
- Tự động tạo manifest `latest.json` cho Tauri Updater
- Nút tải xuống nhận diện nền tảng trên trang chủ (phát hiện OS, gợi ý bản cài phù hợp)
- Proxy release backend (`/api/v1/desktop/latest`) với cache 5 phút
- Docker image được push lên ghcr.io bởi GitHub Actions (Coolify kéo image đã build sẵn)
- Backend CI: lint + clippy + Docker push lên GHCR
- Mã định danh ứng dụng: `com.hoctuthien.vongkimco`

### Thay đổi
- productName đặt thành `VongKimCo` (ASCII, không khoảng trắng/dấu) để tên file build sạch
- Tiêu đề cửa sổ và thông báo dùng tên tiếng Việt `Vòng Kim Cô`
- Bỏ mục tiêu MSI — chỉ giữ NSIS setup.exe cho Windows (tránh nhầm lẫn)
- Job build Linux chỉ chạy khi push main (không chạy cho PR) để tiết kiệm CI
