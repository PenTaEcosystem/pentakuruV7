# Hỗ trợ hệ điều hành

Rust hỗ trợ nhiều nền tảng, nhưng API hệ điều hành và thư viện native cần được build/kiểm tra riêng. GitHub Actions build trên Windows, Ubuntu 24.04 x64 và macOS (kiến trúc của runner macos-latest). Xem mục Artifacts trong lượt chạy thành công ở tab Actions.

| Thành phần | Windows | Linux/macOS |
|---|---|---|
| Giao diện lõi, ghi chú, tìm file, chat AI | Có | Có mã triển khai; CI kiểm tra và build |
| Đồng hồ: HTTP/BLE bridge và chat | Có | Có mã dùng chung; cần kiểm tra thực tế với thiết bị |
| Sally nhúng, khung Music/Clip | WebView2 | Chưa port khung nhúng |
| Office Writer/Impress/Calc qua UNO | Có | Bridge hiện tìm soffice.exe/python.exe; chưa port |
| Cài DeepTutor một nút | Python/Node riêng trên Windows | Chưa port bộ cài runtime |
| Chụp màn hình/thao tác AI/tự khởi động | API Windows | Chưa triển khai; không coi build thành công là có các chức năng này |

Linux: gói tar.gz chứa binary, icon, giấy phép và hướng dẫn. Cần desktop X11/Wayland, OpenGL, ALSA, xdg-open; hộp chọn file dùng zenity. Build Ubuntu cần pkg-config, libasound2-dev, libdbus-1-dev và các thư viện X11/Wayland/OpenGL trong workflow.

macOS: gói .dmg và .tar.gz chứa PentaKuru.app. Bundle được ký ad-hoc để kiểm tra cấu trúc, chưa có chứng chỉ Developer ID hoặc notarization của Apple. Dữ liệu tiếp tục dùng thư mục Penteye Notes tương thích với bản trước. Gói ghi rõ kiến trúc để chọn đúng máy.

Windows: gói ZIP chứa EXE; bộ cài Inno Setup được build riêng bằng scripts/package-installer.ps1.

Build thành công chỉ xác nhận biên dịch, kiểm tra tự động và đóng gói. Chưa có kiểm tra sử dụng thực tế Linux/macOS và chưa cam kết mọi tiện ích ngang với Windows.
