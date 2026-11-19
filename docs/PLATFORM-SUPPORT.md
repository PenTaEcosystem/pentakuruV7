# Hỗ trợ hệ điều hành & Phiên bản Full / Lite (v0.2.0)

PentaKuru v0.2.0 được tách thành **2 phiên bản** cho cả Windows, macOS (Apple Silicon `arm64` & Intel `x86_64`), và Linux:
- **PentaKuru Full** (`cargo build --release --locked`): Đầy đủ mọi tính năng, bao gồm **Office (LibreOffice Writer / Impress / Calc)**.
- **PentaKuru Lite** (`cargo build --release --locked --no-default-features --features lite`): Gọn nhẹ, **không bao gồm LibreOffice / Office**, giữ nguyên toàn bộ các tính năng còn lại (Ghi chú & Kanban, Tìm kiếm nhanh, Chat AI, Edge TTS theo chunk, Sửa chính tả, Sally, Penta Music, Penta Clip, Đồng bộ đồng hồ).
- **UI-TARS** đã được loại bỏ hoàn toàn ở cả 2 phiên bản.

| Thành phần | Windows | macOS (arm64 / x86_64) | Linux (x86_64) |
|---|---|---|---|
| Giao diện lõi 3D, Ghi chú & Kanban, Tìm file, Chat AI | Có | Có | Có |
| Giọng đọc AI Edge TTS theo chunk & Thông báo xong tác vụ | Có (loopback + MediaPlayer) | Có (native `afplay`) | Có (`ffplay`/`mpv`/`mpg123`) |
| Nhắc nhở qua Chat AI (tự dừng khi nhắn tin AI) | Có | Có (nhịp làm việc qua `ioreg`) | Có |
| Penta Music Mini Player (MCP) | Có (MediaPlayer) | Có (`curl` + `afplay`) | Mở trình duyệt / MCP |
| Penta Music / Clip Web & Sally | WebView2 nhúng | Mở trình duyệt hệ thống / Sally | Mở trình duyệt hệ thống |
| Office Writer/Impress/Calc (chỉ bản Full) | Có (LibreOffice Windows) | Có (`/Applications/LibreOffice.app`) | Có (`/usr/bin/soffice`) |
| Đồng hồ Amazfit HTTP bridge | Có | Có | Có |

## Hướng dẫn macOS
- Tải `.dmg` đúng chip máy (**`arm64`** cho Apple Silicon M1/M2/M3/M4, **`x86_64`** cho Mac Intel) và chọn **Full** hoặc **Lite**.
- Kéo `PentaKuru Full.app` hoặc `PentaKuru Lite.app` vào thư mục `Applications`.
- Bundle được ký ad-hoc (chưa Apple notarized). Nếu macOS cảnh báo lần đầu mở, nhấp chuột phải → **Open**, hoặc chạy trong Terminal:
  ```sh
  xattr -cr "/Applications/PentaKuru Full.app"
  ```
- Bản Full tự động tìm LibreOffice tại `/Applications/LibreOffice.app` (cần cài đặt LibreOffice nếu dùng tính năng Office).

## Hướng dẫn Windows & Linux
- **Windows**: Giải nén `PentaKuru-Full-Windows-x64.zip` hoặc `PentaKuru-Lite-Windows-x64.zip` và chạy trực tiếp file `.exe`.
- **Linux**: Giải nén `PentaKuru-Full-Linux-x86_64.tar.gz` hoặc `PentaKuru-Lite-Linux-x86_64.tar.gz` (cần desktop X11/Wayland, OpenGL, ALSA, `xdg-open`, và `ffplay`/`mpv`/`mpg123` để phát Edge TTS).

CI cố định Rust `1.96.0` qua `rust-toolchain.toml`, chạy `cargo fmt --check`, `cargo test --locked`, và `cargo clippy --locked --all-targets -- -D warnings` cho cả **Full** và **Lite** trên Windows, Ubuntu 24.04, macOS Apple Silicon (`macos-latest`), và macOS Intel (`macos-15-intel`).
