# Sally đầy đủ bên trong PentaKuru

Alt + T và nút dịch trên dock mở giao diện React/Tauri gốc của Sally trong
cùng tiến trình PentaKuru. Không tìm, đồng bộ `.env` hay chạy `bin/Sally.exe`.
Mô hình 3D, ghi chú, AI, sửa chính tả và tìm kiếm vẫn dùng eframe.
Tauri chạy trên một luồng giao diện Windows riêng (`any_thread`).

Giữ đầy đủ các lệnh Sally: thu âm theo ứng dụng, microphone, dịch trực tiếp,
đọc bản dịch, đổi nguồn âm thanh, tạm dừng, lịch sử, nghe lại, chỉnh biên bản,
xuất Markdown, AI làm sạch/tóm tắt và phục hồi phiên dang dở.

Sally được tạo khi mở lần đầu. Nút × ẩn cửa sổ và giữ phiên đang chạy;
Alt + T mở lại cửa sổ đó. Bấm kết thúc trong Sally để dừng cuộc họp.

Start mặc định chỉ thu tiếng video/ứng dụng để dịch; tiếng phát vẫn dùng
thiết bị nghe hiện tại. Trong hộp chọn nguồn, bật “Thu thêm tiếng từ micrô
của tôi” nếu cần thu giọng nói. Với tai nghe Bluetooth, nên dùng mic máy
tính hoặc USB để tránh chuyển sang Hands-Free làm gián đoạn tiếng video.
Tắt mic sẽ đóng luồng microphone thật sự, không chỉ làm im mẫu âm thanh.
Trạng thái mic được truyền cùng lệnh Start để không mở mic chớp nhoáng.
Tắt tiện ích Sally trong cài đặt Penta sẽ kết thúc phiên rồi ẩn cửa sổ.
Thoát Penta gửi lệnh dừng và chờ tối đa 10 giây để lưu dữ liệu.

Thư mục mặc định: `Penteye Notes/Sally` cạnh exe. Có thể chọn thư mục khác
trong cài đặt Sally; lựa chọn đó được dùng ở lần mở tiếp theo. Khi chưa có
khóa riêng, lần đầu mở có thể dùng khóa Gemini `AIza…` từ cài đặt Penta.
Các khóa của nhà cung cấp khác không được gửi cho Gemini. Các cài đặt riêng
của Sally được giữ, không bị ghi đè mỗi lần bấm Alt + T.

## Biên dịch

Yêu cầu Node.js/npm, Rust 1.95+, Visual Studio C++ Build Tools và WebView2.

```powershell
Set-Location vendor/Sally
npm.cmd ci
npm.cmd run build
Set-Location ../..
cargo build --release --locked
```

Feature `sally-full` bật mặc định trên Windows. `--no-default-features`
biên dịch bản Penta không có phần nhúng Sally. Giao diện egui Sally cũ chỉ
được giữ cho các preview trước đây; Alt + T sử dụng giao diện React gốc.

```powershell
cargo test --locked
cargo test -p sally --lib --features embedded,custom-protocol --locked
# Local loopback hardware check; opens no microphone, saves no audio, calls no API.
cargo test -p sally --lib --locked video_capture_keeps_playback_device_and_never_opens_microphone -- --ignored
target/release/pentakuru.exe --sally-integration-check
```

Kiểm tra tích hợp chạy cả eframe và Tauri, xác nhận DOM giao diện gốc,
IPC get_boot_info, cùng process ID và chu kỳ mở → đóng → mở lại → tắt.
Dữ liệu kiểm tra ở `.build/sally-integration-data`; không thu âm, không gọi
Gemini, không đọc ghi chú hay khóa cá nhân. Báo cáo ở
`.build/sally-integration-check.json`.

Đóng gói bằng `scripts/package-sally.ps1`. Bản đầy đủ cần WebView2 Runtime;
React/CSS/JS được nhúng vào exe, không cần Node.js khi chạy.
