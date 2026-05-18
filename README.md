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
- **Đăng nhập bằng Google** qua *browser-OAuth + polling* flow — chỉ cần bấm nút,
  ứng dụng mở trình duyệt hệ thống, bạn đăng nhập Google bình thường, app tự nhận
  đăng nhập (không phải gõ mã tay). Refresh token lưu trong OS keyring (Windows
  Credential Manager / macOS Keychain / Linux secret-service), không phải file phẳng.
- **Bắt đầu / dừng phiên** bằng nút hoặc phím tắt toàn cục (cấu hình được).
- **Chụp ảnh màn hình định kỳ** (mặc định 180s), nén JPEG ~50% chất lượng +
  resize ≤ 1280px để vừa đủ thấy đang làm gì mà không tốn băng thông.
- **Theo dõi Idle / Active** bằng cách phát hiện hoạt động bàn phím + chuột
  (mẫu mỗi 30s, ngưỡng idle mặc định 120s).
- **Snapshot ứng dụng đang chạy** + cửa sổ foreground (mặc định 60s).
- **Hoạt động offline**: tất cả dữ liệu được lưu vào SQLite cục bộ; có mạng
  sẽ tự đồng bộ lên server (vòng đồng bộ 20s, có nút "Đồng bộ ngay").
- **Tự khởi động cùng hệ thống** (tuỳ chọn, dùng plugin autostart của Tauri).
- **Tự cập nhật** an toàn qua Tauri Updater + chữ ký Ed25519 (xem mục
  *Tự động cập nhật* bên dưới).
- **UI dark, gọn**: sidebar có icon + active highlight, status pill có dot
  pulse khi active, đồng hồ phiên monospace, KPI cards highlight theo
  trạng thái, lịch sử có filter (tất cả / đã sync / chờ sync), và banner
  cập nhật riêng cho download / install / error.

### Admin web (chung server)
- Đăng nhập Google. Chỉ email trong allow-list (env `ADMIN_EMAILS`) mới vào được.
- Tổng quan: số người dùng, số phiên, số ảnh chụp, phiên đang chạy.
- Danh sách người dùng + chi tiết từng phiên.
- Xem ảnh chụp màn hình.
- Phê duyệt thiết bị desktop tự động khi user đăng nhập (qua login flow), với
  giới hạn số thiết bị mỗi user (env `DESKTOP_DEVICE_LIMIT`, mặc định 5).
- **Quản lý thành viên**: thêm/gỡ email khỏi allow-list runtime tại `/admin/members`
  (bổ sung cho danh sách env `MEMBER_EMAILS`).
- **Trả lời phản hồi** từ thành viên (đổi trạng thái: đang mở / đang xử lý / đã giải quyết / không xử lý).
- **Giao diện dark mode** thống nhất, có active nav highlight, status badge
  màu theo trạng thái, empty state cho mọi danh sách, layout responsive cho màn hình nhỏ.

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
     # HS256 ký JWT desktop. BẮT BUỘC — backend từ chối khởi động nếu rỗng/<32 ký tự.
     JWT_SECRET=$(openssl rand -hex 32)
     # Tuỳ chọn — giới hạn số thiết bị mỗi member, mặc định 5, 0 = vô hạn.
     DESKTOP_DEVICE_LIMIT=5
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
2. **Đăng nhập bằng Google** — bấm nút, ứng dụng tự mở trình duyệt hệ thống và
   trỏ tới `${PUBLIC_URL}/auth/desktop/authorize?flow_id=…`.
   - **Fast path**: nếu bạn đã đăng nhập web admin/member trên trình duyệt từ
     trước → server bỏ qua Google, chỉ cần 1 click confirm.
   - **Slow path**: bạn đăng nhập Google như bình thường, browser redirect về
     `/admin/oauth/callback` → server hoàn tất login flow, trang hiện "Đăng
     nhập thành công, có thể đóng tab".
3. Ứng dụng poll server mỗi 2 giây, nhận token (access + refresh) ngay khi
   bạn vừa duyệt xong, lưu refresh token vào OS keyring và sẵn sàng dùng.

Lưu ý: token không được lưu trên đĩa dưới dạng file phẳng. Truy cập refresh
token cần quyền user hiện tại trên máy.

### Phím tắt mặc định
- `Ctrl/Cmd + Alt + S` — bắt đầu phiên.
- `Ctrl/Cmd + Alt + E` — kết thúc phiên.
- Đổi trong **Cài đặt → Phím tắt**.

---

## 🧪 Phát triển local & kiểm tra chất lượng

CI (`backend-ci.yml`) chạy `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
và `cargo build --release` cho backend; release pipeline cũng yêu cầu desktop
build sạch. Để chạy y hệt CI tại máy:

```bash
# Backend
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release

# Desktop (Tauri Rust)
cd desktop/src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Desktop (Svelte/TypeScript)
cd desktop
npm install
npm run icons:build      # tạo icon (yêu cầu để `tauri build` thấy icon.ico)
npm run check            # svelte-check (TypeScript + a11y)
npm run build            # vite production bundle
```

Toàn bộ codebase hiện tại build & lint sạch warnings — bất kỳ warning nào xuất
hiện trong PR đều cần được xử lý trước khi merge.

---

## 🎨 Hệ thiết kế UI

Admin web (Askama) và desktop app (Svelte) dùng cùng một bộ design token
trong [backend/static/admin.css](backend/static/admin.css) và
[desktop/src/app.css](desktop/src/app.css):

| Token | Giá trị | Mục đích |
| --- | --- | --- |
| `--bg` `#0f1115` / `--surface` `#161a22` | nền & card | dark mode mặc định |
| `--primary` `#d4a017` | vàng kim cô | nút chính, active state, badge admin |
| `--ok` `--warn` `--danger` `--info` | màu trạng thái | banner, badge, KPI accent |
| `--radius` 8px, `--radius-lg` 12px | bo góc | card, button, input |

Pattern UI dùng chung:
- **Status badge** trong feedback / membership: class `badge status-<state>`
  (open / in_progress / resolved / wontfix / pending / approved / rejected).
- **Empty state**: `.empty` với icon + mô tả khi danh sách rỗng.
- **Tabs / pills**: `.tabs > .tab.active` cho filter bar (vd. /admin/feedback).
- **Status pill** desktop (`.status-pill.active|idle|offline|online`): có
  dot pulse khi đang hoạt động để feedback rõ trạng thái real-time.
- **Active nav** trên topbar admin: highlight được thêm client-side bằng
  inline script trong `base.html` so khớp `pathname` với `href` — không
  cần thêm field nào vào Askama struct.

---

## 🏗️ Kiến trúc thư mục

```
vongkimco/
├── backend/                   # Rust + Axum + SQLite
│   ├── src/
│   │   ├── main.rs            # bootstrap + tracing + serve
│   │   ├── routes.rs          # router + session + CORS
│   │   ├── auth.rs            # OAuth helpers + device token + extractors
│   │   ├── state.rs           # Config (env vars) + AppState
│   │   ├── error.rs           # AppError → HTTP response mapping
│   │   ├── models.rs          # sqlx structs (User, Session, …)
│   │   ├── db.rs              # SQLite pool + migrations
│   │   └── handlers/
│   │       ├── admin.rs        # admin dashboard + users + screenshots
│   │       ├── desktop_auth.rs # device-link flow
│   │       ├── device_api.rs   # /api/v1/* for desktop client
│   │       ├── feedback.rs     # /feedback + admin replies
│   │       ├── health.rs       # /api/v1/health + /api/v1/server-info
│   │       ├── membership.rs   # /pending + admin approve/reject
│   │       └── oauth.rs        # Google OAuth login/callback
│   ├── templates/             # Askama (admin UI + feedback + pending)
│   ├── migrations/            # SQLite migrations (init, device_link, members_feedback, membership_requests)
│   ├── static/                # admin.css
│   └── Dockerfile
├── desktop/                   # Tauri 2 + Svelte + Rust
│   ├── src/                   # Svelte UI
│   │   ├── App.svelte         # shell + sidebar nav
│   │   ├── app.css            # design tokens
│   │   ├── lib/               # stores, updater client, banner
│   │   └── routes/            # ServerSelect, Login, Home, History, Settings
│   ├── src-tauri/             # Rust core
│   │   ├── src/
│   │   │   ├── main.rs        # thin entry into lib::run()
│   │   │   ├── lib.rs         # plugin setup + invoke handlers
│   │   │   ├── commands.rs    # #[tauri::command]
│   │   │   ├── state.rs       # in-memory state + persistence
│   │   │   ├── settings.rs    # Settings struct + defaults
│   │   │   ├── db.rs          # local SQLite (rusqlite)
│   │   │   ├── monitor.rs     # idle/active + app list + hotkeys + screenshots
│   │   │   ├── screenshot.rs  # xcap + image (JPEG compression)
│   │   │   └── sync.rs        # background sync to backend
│   │   ├── capabilities/      # Tauri 2 permissions
│   │   ├── icons/             # generated app icons (gitignored)
│   │   └── tauri.conf.json
│   └── scripts/               # icon generator + update-manifest builder
├── docker-compose.yml         # Coolify-ready
├── .env.example
└── .github/workflows/
    ├── backend-ci.yml         # fmt --check, clippy -D warnings, build, Docker
    └── desktop-release.yml    # cross-platform Tauri release + signed updater manifest
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
- **Desktop auth pipeline (production-tested pattern):**
  - **Access token**: JWT HS256 ký bằng `JWT_SECRET`, TTL 1 giờ. Claims:
    `{sub, did, tier, iat, exp, jti}`. Mỗi request `DeviceAuth` extractor
    verify chữ ký, kiểm tra device chưa bị revoke, và recheck membership
    runtime — admin revoke có hiệu lực ngay request tiếp theo.
  - **Refresh token**: 32-byte random, SHA-256 lưu DB (server không bao giờ
    thấy raw refresh token sau khi cấp). TTL 60 ngày. **Rotation strict**:
    mỗi lần refresh, token cũ đánh dấu `rotated_at` và bị từ chối ngay.
  - **Device fingerprint binding**: refresh chỉ valid khi
    `device_fingerprint` request khớp với device row. Refresh token rò rỉ
    sang máy khác không xài được.
  - **One-shot token delivery**: server `UPDATE login_flows SET access_token
    = NULL, refresh_token = NULL` ngay sau poll lần đầu thấy `completed` —
    DB leak sau đó cũng không replay được token.
  - **Browser hệ thống** chứ không embed WebView: user thấy URL Google
    thật, tránh phishing in-app; không cần đóng gói Chromium.
  - **Refresh token storage**: OS keyring (Windows Credential Manager,
    macOS Keychain, Linux Secret Service) — chưa bao giờ chạm đĩa dưới
    dạng file phẳng.
- Cookie session (admin web) chỉ chứa session ID ngẫu nhiên do
  `tower-sessions` cấp; dữ liệu session lưu server-side trong SQLite, nên
  không cần khoá ký cookie. Cookie `SameSite=Lax`, `Secure` khi `PUBLIC_URL`
  là HTTPS.
- Ảnh chụp lưu trong volume riêng, chỉ admin xác thực mới đọc qua
  `/admin/screenshots/:id/image`.
- Mọi component đều mã nguồn mở (MIT) — soi được toàn bộ pipeline.

---

## 📡 REST API (cho desktop hoặc tích hợp)

| Method | Path | Auth | Mục đích |
| --- | --- | --- | --- |
| `GET`  | `/api/v1/health` | — | Health check |
| `GET`  | `/api/v1/server-info` | — | Tên server + API version |
| `POST` | `/api/v1/auth/desktop/start` | — | Khởi tạo login flow, trả `flow_id` + `auth_url` (URL mở trình duyệt) |
| `GET`  | `/auth/desktop/authorize?flow_id=…` | (cookie) | Trang trung gian: fast-path web session hoặc redirect Google OAuth |
| `GET`  | `/auth/desktop/done` | — | Trang HTML "Đăng nhập thành công" hiển thị sau khi login |
| `GET`  | `/api/v1/auth/desktop/poll/:flow_id` | — | Poll status. `completed` → trả `access_token + refresh_token + user + subscription` **đúng 1 lần** rồi server tự clear |
| `POST` | `/api/v1/auth/refresh` | — | Body `{refresh_token, device_fingerprint}` → cấp access mới, rotate refresh. 401 = client phải xoá keyring |
| `GET`  | `/api/v1/auth/verify` | Bearer | Trả `{valid, user, subscription}` để client xác nhận access token còn dùng được |
| `GET`  | `/api/v1/whoami` | Bearer | Trả user của token (legacy alias của `/auth/verify`) |
| `POST` | `/api/v1/sessions` | Bearer | Tạo/cập nhật phiên (idempotent theo `client_session_id`) |
| `POST` | `/api/v1/activity` | Bearer | Batch upload activity samples |
| `POST` | `/api/v1/app-snapshots` | Bearer | Batch upload app snapshots |
| `POST` | `/api/v1/screenshots` | Bearer | Multipart upload 1 ảnh JPEG |

`Bearer` = JWT HS256 access token (claims `sub`, `did`, `tier`, `iat`, `exp`, `jti`).
TTL 1 giờ — desktop client tự refresh khi còn <5 phút.

---

## 📜 Giấy phép

MIT — xem [LICENSE](./LICENSE). Đây là dự án mã nguồn mở, đóng góp được hoan
nghênh tại pull request.
