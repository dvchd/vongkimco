# Vòng Kim Cô

> *"Chiếc vòng kim cô — siết khi tâm trí trượt khỏi việc."*

**Vòng Kim Cô** là một ứng dụng mã nguồn mở (MIT) giúp kiểm soát mức độ nghiêm
túc của nhân sự khi làm việc online. Ứng dụng gồm 3 phần được viết hoàn toàn
bằng Rust:

| Thành phần | Stack | Mục đích |
| --- | --- | --- |
| **Backend** | Rust + Axum + SQLite | API + nhận dữ liệu hoạt động + lưu ảnh chụp |
| **Admin frontend** | Rust + Askama (MVC, nằm chung backend) | Trang quản trị, xem phiên + ảnh chụp |
| **Desktop App** | Tauri 2 + Rust + Svelte | Chạy nền, thu thập hoạt động, đồng bộ về server |

Triển khai chính thức: **https://vongkimco.hoctuthien.com**

---

## ✨ Tính năng chính

### Desktop App (đa nền tảng — Windows / macOS / Linux)
- **Chọn server backend** ngay từ lần chạy đầu tiên (mặc định
  `https://vongkimco.hoctuthien.com`, có thể custom domain).
- **Đăng nhập bằng Google** qua "device link" flow — mở browser, đăng nhập, paste mã.
- **Bắt đầu / dừng phiên** bằng nút hoặc phím tắt toàn cục (cấu hình được).
- **Chụp ảnh màn hình định kỳ**, nén JPEG ~50% chất lượng + resize ≤ 1280px để
  vừa đủ thấy đang làm gì mà không tốn băng thông.
- **Theo dõi Idle / Active** bằng cách phát hiện hoạt động bàn phím + chuột.
- **Snapshot ứng dụng đang chạy** + cửa sổ foreground.
- **Hoạt động offline**: tất cả dữ liệu được lưu vào SQLite cục bộ, có mạng
  sẽ tự đồng bộ lên server.

### Admin web (chung server)
- Đăng nhập Google. Chỉ email trong allow-list (env `ADMIN_EMAILS`) mới vào được.
- Tổng quan: số người dùng, số phiên, số ảnh chụp, phiên đang chạy.
- Danh sách người dùng + chi tiết từng phiên.
- Xem ảnh chụp màn hình.
- Phê duyệt liên kết thiết bị desktop.
- **Quản lý thành viên**: thêm/gỡ email khỏi allow-list runtime tại `/admin/members`
  (bổ sung cho danh sách env `MEMBER_EMAILS`).
- **Trả lời phản hồi** từ thành viên (đổi trạng thái: đang mở / đang xử lý / đã giải quyết / không xử lý).

### Member web (`/feedback`)
- Đăng nhập Google. Bắt buộc email phải nằm trong allow-list member (env
  `MEMBER_EMAILS` ∪ allowed_members trong DB) — nếu không sẽ thấy trang
  "chờ phê duyệt".
- Đăng bình luận, báo lỗi, hoặc đề xuất tính năng — **chỉ văn bản**, tối đa
  4000 ký tự, rate-limit 8 bài/giờ/người để chống spam.
- Xem trạng thái xử lý và phản hồi của admin theo thời gian thực.
- Cần đính kèm ảnh? Liên hệ Facebook / email maintainer (hiển thị trên trang) —
  cách này tránh upload hình NSFW lên server.

---

## 🚀 Triển khai backend qua Coolify

1. **Tạo Google OAuth Credentials** ở
   <https://console.cloud.google.com/apis/credentials>
   - Loại: **Web application**
   - Authorized redirect URI: `https://vongkimco.hoctuthien.com/admin/oauth/callback`
   - Lưu lại `client_id` và `client_secret`.

2. **Trên Coolify**:
   - Tạo **New Resource → Docker Compose**, trỏ tới repo này.
   - Compose file: `docker-compose.yml`
   - Thiết lập domain: `vongkimco.hoctuthien.com` (Coolify tự cấp TLS qua Let's Encrypt).
   - Thêm biến môi trường (xem `.env.example`):
     ```env
     PUBLIC_URL=https://vongkimco.hoctuthien.com
     ADMIN_EMAILS=dvcuong.hust@gmail.com
     # Có thể bỏ trống — admin tự thêm member qua /admin/members sau khi deploy
     MEMBER_EMAILS=
     GOOGLE_CLIENT_ID=...
     GOOGLE_CLIENT_SECRET=...
     SESSION_SECRET=$(openssl rand -hex 32)
     ```
   - Volume `vongkimco_data` sẽ lưu SQLite + thư mục ảnh chụp giữa các lần redeploy.

3. **Deploy**. Coolify build image, chạy migrations tự động (SQLx migrate khi
   khởi động), expose backend ra `:8080`, reverse-proxy về domain.

### Chạy local (không qua Coolify)

```bash
cd backend
cp .env.example .env
# Sửa GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET / SESSION_SECRET / ADMIN_EMAILS
cargo run --release
# → http://localhost:8080/admin
```

Hoặc qua Docker:
```bash
docker compose up --build
```

---

## 🖥️ Build Desktop App

### Cài đặt yêu cầu
- **Node.js 20+**
- **Rust stable**
- Tauri prerequisites theo OS: <https://v2.tauri.app/start/prerequisites/>

### Build local
```bash
cd desktop
npm install
npm run icons:build          # sinh icon placeholder
npm run tauri dev            # chạy dev
npm run tauri build          # build bundle release cho OS hiện tại
```

Bundle sinh ra ở `desktop/src-tauri/target/release/bundle/`.

### Phát hành tự động qua GitHub Actions
Push tag dạng `desktop-v0.1.0`:
```bash
git tag desktop-v0.1.0
git push origin desktop-v0.1.0
```
Workflow `.github/workflows/desktop-release.yml` sẽ:
- Build cho `windows-msvc`, `macos-intel`, `macos-arm`, `linux-gnu` song song.
- Sinh các bundle: `.msi` / `.exe` (Win), `.dmg` (macOS), `.AppImage` + `.deb` (Linux).
- Ký bundle bằng `TAURI_SIGNING_PRIVATE_KEY` (mỗi file kèm `.sig`).
- Tạo GitHub Release đính kèm tất cả artifact (gồm cả `.sig`).

---

## 🔄 Tự động cập nhật (Tauri Updater)

Desktop tự kiểm tra bản mới khi khởi động và có nút "Kiểm tra cập nhật" trong
**Cài đặt → Cập nhật ứng dụng**. Nếu có bản mới, banner trên cùng cửa sổ hiện
"Cài đặt và khởi động lại"; bundle tải về được verify chữ ký Ed25519 bằng public
key nhúng trong app trước khi ghi đè.

### Pipeline hoạt động
1. CI build desktop bundle cho 4 nền tảng song song, ký bằng
   `TAURI_SIGNING_PRIVATE_KEY` → mỗi `.msi / .app.tar.gz / .AppImage` đi kèm một
   file `.sig` cùng tên.
2. Sau khi toàn bộ matrix xong, **release job** tải tất cả artifact về, chạy
   [`build-update-manifest.cjs`](desktop/scripts/build-update-manifest.cjs) để
   gộp metadata + URL download GitHub + nội dung từng `.sig` thành một file
   `latest.json` duy nhất.
3. Release job upload toàn bộ bundle + `.sig` + `latest.json` lên GitHub
   Release (`desktop-v*`).
4. Desktop client gọi thẳng tới
   `https://github.com/<owner>/<repo>/releases/latest/download/latest.json` —
   GitHub luôn redirect URL này về asset `latest.json` của release mới nhất, cứ
   thế CDN cache, không tốn API quota, không phụ thuộc backend hay rate-limit.
5. Tauri đọc manifest → so version → tải bundle phù hợp platform/arch → verify
   chữ ký → ghi đè + relaunch.

> **Tại sao không proxy qua backend?** Vì không cần. Pubkey hardcoded trong
> binary đã đủ để chặn payload giả mạo; GitHub CDN nhanh hơn và rảnh tay
> backend. Chỉ khi cần staged rollout / metrics / kênh beta-stable mới đáng
> dựng proxy.

### Sinh keypair (chỉ làm 1 lần)
```bash
cd desktop
npx @tauri-apps/cli signer generate -w ~/.tauri/vongkimco.key
# Nhập password. CLI in ra public key (~/.tauri/vongkimco.key.pub)
cat ~/.tauri/vongkimco.key.pub
```

### Cấu hình GitHub Actions secrets
Repo → **Settings → Secrets and variables → Actions → New repository secret**:

| Secret | Nội dung |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Nội dung file `~/.tauri/vongkimco.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password đã đặt khi `signer generate` |
| `TAURI_SIGNING_PUBLIC_KEY` | Nội dung file `~/.tauri/vongkimco.key.pub` |

Workflow tự inject pubkey + endpoint
(`https://github.com/${GITHUB_REPOSITORY}/releases/latest/download/latest.json`)
vào `tauri.conf.json` trước khi build, đồng thời ký bundle bằng private key.

### Release flow đầy đủ
```bash
# bump version trong cả 3 file:
#   desktop/package.json
#   desktop/src-tauri/Cargo.toml
#   desktop/src-tauri/tauri.conf.json
git commit -am "release: desktop v0.2.0"
git tag desktop-v0.2.0
git push && git push --tags
```
CI build + ký + sinh manifest + upload. Vài phút sau, mọi client desktop sẽ
thấy banner thông báo bản mới.

### Khi nào cập nhật manual
Nếu chỉ build local (không qua CI) và cần test updater, sau `npm run tauri build`
chạy thêm:
```bash
GITHUB_REPOSITORY=<owner>/<repo> GITHUB_REF_NAME=desktop-v0.2.0 \
  node scripts/build-update-manifest.cjs \
  src-tauri/target/release/bundle/ 0.2.0 \
  > latest.json
```
rồi upload thủ công các file `*.msi/.app.tar.gz/.AppImage/.sig/latest.json` lên
GitHub Release.

---

## 🔌 Khi mở Desktop App lần đầu

1. **Chọn server** — mặc định `https://vongkimco.hoctuthien.com`. Có thể đổi
   sang server backend khác (ví dụ instance riêng của công ty).
2. **Đăng nhập Google** — ứng dụng hiển thị mã `XXXX-XXXX` và mở trình duyệt.
3. **Vào trang `/device/activate`** trên server, đăng nhập Google, paste mã,
   bấm phê duyệt.
4. Ứng dụng tự nhận token, lưu xuống đĩa, sẵn sàng bắt đầu phiên.

### Phím tắt mặc định
- `Ctrl/Cmd + Alt + S` — bắt đầu phiên.
- `Ctrl/Cmd + Alt + E` — kết thúc phiên.
- Đổi trong **Cài đặt → Phím tắt**.

---

## 🏗️ Kiến trúc thư mục

```
vongkimco/
├── backend/                   # Rust + Axum + SQLite
│   ├── src/
│   │   ├── main.rs
│   │   ├── routes.rs          # router + session + CORS
│   │   ├── auth.rs            # Google OAuth + device token
│   │   ├── handlers/          # admin / oauth / device API / desktop auth
│   │   ├── models.rs          # sqlx structs
│   │   └── db.rs              # pool + migrations
│   ├── templates/             # Askama (admin UI)
│   ├── migrations/            # SQLite migrations
│   ├── static/                # admin.css
│   └── Dockerfile
├── desktop/                   # Tauri 2 + Svelte + Rust
│   ├── src/                   # Svelte UI
│   ├── src-tauri/             # Rust core
│   │   ├── src/
│   │   │   ├── lib.rs         # plugin setup + invoke handlers
│   │   │   ├── commands.rs    # #[tauri::command]
│   │   │   ├── state.rs       # in-memory state + persistence
│   │   │   ├── settings.rs
│   │   │   ├── db.rs          # local SQLite (rusqlite)
│   │   │   ├── monitor.rs     # idle/active + app list + hotkeys
│   │   │   ├── screenshot.rs  # xcap + image (JPEG compression)
│   │   │   └── sync.rs        # background sync to backend
│   │   └── tauri.conf.json
│   └── scripts/               # icon generator
├── docker-compose.yml         # Coolify-ready
├── .env.example
└── .github/workflows/
    ├── backend-ci.yml         # Rust fmt/clippy/build + Docker
    └── desktop-release.yml    # cross-platform Tauri release
```

---

## 👥 Phân quyền: Admin · Member · Guest

Mỗi user sau khi đăng nhập Google rơi vào 1 trong 3 vai trò:

| Vai trò | Điều kiện | Có thể làm |
| --- | --- | --- |
| **Admin** | Email nằm trong env `ADMIN_EMAILS` | Tất cả: trang quản trị, quản lý members, duyệt yêu cầu xin làm thành viên, xem/reply feedback |
| **Member** | Admin **hoặc** env `MEMBER_EMAILS` **hoặc** bảng DB `allowed_members` (admin thêm hoặc duyệt yêu cầu) | Liên kết desktop + gửi & xem feedback |
| **Guest** | Đăng nhập nhưng chưa được duyệt | **Vẫn được gửi feedback / báo lỗi**; có thể nộp yêu cầu xin làm thành viên tại `/pending`; không pair được desktop |

### Ba nguồn allow-list member, hợp lại theo phép hợp
1. **Env `MEMBER_EMAILS`** — set trước khi deploy, phân cách bằng dấu phẩy:

   ```env
   MEMBER_EMAILS=hoctuthien11@gmail.com,dhammacode@gmail.com
   ```

2. **Bảng DB `allowed_members`** — admin thêm/gỡ tay tại `/admin/members`.
3. **Yêu cầu được duyệt** — guest nộp request tại `/pending`, admin click
   **Duyệt** → tự insert vào `allowed_members` + flip `is_member = 1`.

Khi admin gỡ/từ chối, hệ thống refresh `is_member` của user ngay; token desktop
trên thiết bị họ sẽ bị reject trên request kế tiếp.

### Flow guest → member
1. Guest đăng nhập Google → landing page `/feedback` với banner
   *"Bạn đang là khách. [Xin làm thành viên]"*.
2. Click vào /pending → form 1 ô textarea "Lời nhắn" (tuỳ chọn, max 1000 ký tự).
3. Admin vào `/admin/members` thấy section *"Yêu cầu xin làm thành viên"* trên
   đầu trang → click **Duyệt** hoặc **Từ chối**.
4. Nếu duyệt: lần load page tiếp theo của guest hết banner, có thể activate
   desktop. Nếu từ chối: guest vào /pending vẫn nộp lại được yêu cầu.

---

## 💬 Bình luận, báo lỗi, đề xuất

**Bất kỳ user nào đăng nhập Google đều dùng được** — kể cả guest chưa được duyệt
làm thành viên. Đây là kênh hỗ trợ mở để cộng đồng phản ánh vấn đề.

Tại `/feedback`:

- Tạo bài viết 3 loại: **Bình luận / Báo lỗi / Đề xuất tính năng**.
- Bài viết **chỉ chấp nhận văn bản** (max 4000 ký tự, rate-limit 8 bài/giờ/user).
  Đây là quyết định bảo mật có chủ ý — tránh ai đó push hình NSFW lên server.
- Theo dõi trạng thái xử lý: `Đang mở → Đang xử lý → Đã giải quyết / Không xử lý`.
- Xem reply của admin và reply lại.

Admin vào `/admin/feedback` thấy mọi post của mọi user, có thể filter theo trạng
thái, reply trực tiếp và đổi status.

### Khi cần gửi ảnh

Trang feedback và `/pending` đều ghi rõ: nếu cần đính kèm screenshot hoặc file,
vui lòng liên hệ maintainer qua **Facebook** (`fb.com/dvcuong.hust`) hoặc
**email** (`dvcuong.hust@gmail.com`) — 2 link này configurable qua env
(`MAINTAINER_FACEBOOK`, `MAINTAINER_EMAIL`).

---

## 🔐 Bảo mật

- Backend chỉ chấp nhận login Google. Quyền admin lấy theo email allow-list
  ngay khi đăng nhập.
- Desktop token: chuỗi 32-byte ngẫu nhiên, SHA-256 lưu DB (server không bao
  giờ thấy raw token sau khi cấp).
- Cookie session ký bằng SESSION_SECRET (HMAC-SHA512, SameSite=Lax,
  Secure khi PUBLIC_URL là HTTPS).
- Ảnh chụp lưu trong volume riêng, chỉ admin xác thực mới đọc qua
  `/admin/screenshots/:id/image`.
- Mọi component đều mã nguồn mở (MIT) — soi được toàn bộ pipeline.

---

## 📡 REST API (cho desktop hoặc tích hợp)

| Method | Path | Auth | Mục đích |
| --- | --- | --- | --- |
| `GET`  | `/api/v1/health` | — | Health check |
| `GET`  | `/api/v1/server-info` | — | Tên server + API version |
| `POST` | `/api/v1/device/link/start` | — | Khởi tạo device-link, trả `device_code` + `user_code` + `verification_url` |
| `POST` | `/api/v1/device/link/poll` | — | Poll, nếu approved → trả bearer token |
| `GET`  | `/api/v1/whoami` | Bearer | Trả user của token |
| `POST` | `/api/v1/sessions` | Bearer | Tạo/cập nhật phiên (idempotent theo `client_session_id`) |
| `POST` | `/api/v1/activity` | Bearer | Batch upload activity samples |
| `POST` | `/api/v1/app-snapshots` | Bearer | Batch upload app snapshots |
| `POST` | `/api/v1/screenshots` | Bearer | Multipart upload 1 ảnh JPEG |

---

## 📜 Giấy phép

MIT — xem [LICENSE](./LICENSE). Đây là dự án mã nguồn mở, đóng góp được hoan
nghênh tại pull request.
