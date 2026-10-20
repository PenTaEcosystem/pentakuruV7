# Bộ cài PentaKuru

Chạy `PentaKuru-Setup.exe`: giới thiệu → giấy phép → thư mục → chọn tiện ích → biểu tượng → xác nhận → tiến độ → hoàn tất. Giao diện chính tiếng Việt, màu tối, có Quay lại và Hủy. Cài theo tài khoản Windows hiện tại, mặc định `%LOCALAPPDATA%/Programs/PentaKuru`; phần ứng dụng không cần quyền quản trị. Nếu thiếu Microsoft Visual C++ Runtime x64, Windows sẽ yêu cầu quyền quản trị để cài thành phần dùng chung này.

PentaKuru và AI là phần lõi. Có thể chọn Ghi chú, Tìm file, Chính tả, Sally, Writer, Impress, Calc, Music, Clip, DeepTutor và UI-TARS. Tùy chọn được ghi vào `installation-options.json` và ứng dụng áp dụng một lần cho mỗi lượt cài; thay đổi sau trong Settings được giữ nguyên. Cài lại vào cùng thư mục sẽ áp dụng bộ lựa chọn mới, giữ cấu hình tài khoản/API key hiện có.

EXE chứa ứng dụng, hướng dẫn và Microsoft Visual C++ Runtime x64 đã được xác minh chữ ký. Setup kiểm tra phiên bản trong cả hai nhánh registry x64/32-bit và sự hiện diện của `vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140.dll`. Bản ứng dụng hiện tại dùng mức runtime đã kiểm tra `14.44.35211.0` trở lên; mức này phải được xét lại nếu đổi MSVC/toolchain. Runtime phù hợp được bỏ qua; bản cũ được cập nhật; DLL thiếu khi phiên bản đã đủ sẽ được đề nghị sửa. Payload được kiểm tra SHA-256 trước khi chạy Microsoft Setup. Không cần Internet cho bước runtime này. Nếu người dùng hủy quyền quản trị hoặc runtime cài lỗi, Setup dừng trước khi ghi ứng dụng. Nếu cần khởi động lại, Setup không tự mở PentaKuru và hiển thị yêu cầu khởi động lại. Gỡ PentaKuru không gỡ runtime dùng chung. Log Microsoft installer được giữ trong `%LOCALAPPDATA%/PentaKuru/SetupLogs`.

EXE chứa ứng dụng, hướng dẫn và bộ cài LibreOffice chính thức. Office được chuẩn bị trong Setup khi chọn Writer/Impress/Calc, không cần tải mạng; bỏ chọn thì bỏ qua. LibreOffice có sẵn được tái sử dụng. Setup kiểm tra và cài Microsoft WebView2 từ bộ cài Microsoft đi kèm nếu máy thiếu. DeepTutor được chuẩn bị bằng Python/Node và toàn bộ thư viện đi kèm khi được chọn; API key/model cấu hình trong Settings hoặc dùng cấu hình đã có của PentaKuru. Music/Clip đăng nhập riêng trên trang dịch vụ. Calc hiện chỉ mở/chuyển bảng tính.

Gỡ qua Windows Installed apps hoặc shortcut Gỡ PentaKuru. Chỉ xóa file bộ cài sở hữu; ghi chú, bản sao tài liệu, runtime và phiên đăng nhập được giữ. Có thể mở Settings bật/tắt tiện ích mà không cần cài lại.

## Build

Yêu cầu Inno Setup 6.7.3+ chính thức: https://jrsoftware.org/isdl.php

1. `./scripts/package.ps1`
2. `./scripts/build-setup.ps1 -Compiler "C:/Program Files (x86)/Inno Setup 6/ISCC.exe"`
3. Kết quả `dist/PentaKuru-Setup.exe`.

Bộ cài chưa ký chứng thư phát hành của Pentasys.

Kiểm thử bộ cài dùng AppId riêng (ISCC /DPentaKuruTest) và thư mục thử riêng, giữ nguyên đăng ký của bản PentaKuru thật.

Script build tải `https://aka.ms/vc14/vc_redist.x64.exe` vào `dist/prerequisites`, xác minh chữ ký Microsoft, ghi phiên bản và SHA-256 cho Inno Setup. Có thể chạy `prepare-vc-runtime.ps1 -Refresh` để cập nhật payload trước khi build lại. File EXE portable vẫn cần runtime; dùng Setup để tự kiểm tra và cài.

Script build cũng chạy `prepare-office-runtime.ps1` để tải và xác minh MSI LibreOffice 26.8.1 trước khi nhúng vào Setup. Có thể chuẩn bị từ file đã tải bằng `./scripts/prepare-office-runtime.ps1 -InstallerPath "C:/downloads/LibreOffice_26.8.1_Win_x86-64.msi"`. Máy build cần Internet nếu chưa có payload; máy nhận bộ cài không cần Internet cho Office. Bộ cài lớn hơn do chứa toàn bộ MSI; đây là bản phân phối đã biên dịch, không cần biên dịch mã nguồn trên máy người dùng.

`PentaKuru-Setup.exe /VCRUNTIMEREPORT="C:/path/runtime-report.json"` chỉ ghi báo cáo phiên bản/nhu cầu runtime rồi thoát, không cài đặt.

Kiểm thử bootstrap: build với `-TestBuild`, rồi chạy `scripts/check-vc-runtime-installer.ps1`. Các trạng thái runtime và mã thoát Microsoft được mô phỏng trong bản test; bản sản xuất không nhận tham số mô phỏng. Tests xác minh bỏ qua, cài, sửa, phiên bản mới hơn, hủy, lỗi và khởi động lại. Chưa thay thế cho kiểm thử cài thực tế trên máy Windows sạch thiếu runtime.

Tài liệu Microsoft: https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files

## LibreOffice trong Setup

Khi chọn Writer, Impress hoặc Calc, Setup xác minh SHA-256 của MSI đi kèm rồi kiểm tra LibreOffice có đủ soffice.exe và python.exe. Nếu chưa có, Setup xác minh chữ ký The Document Foundation rồi giải nén MSI đi kèm vào `Penteye Notes/Office/runtime`. Windows có thể hiện UAC; nếu bị từ chối hoặc cài lỗi, Setup dừng trước khi báo hoàn tất và cho thử lại. Không chọn Office thì không giải nén/chạy MSI. DeepTutor được chuẩn bị trong Setup nếu được chọn; không cần cài Python/Node hoặc tải thư viện trên máy nhận.

Log lỗi nằm tại `Penteye Notes/Office/install.log` và `msiexec.log` trong thư mục cài. Windows 10/11 dùng cửa sổ Office nền tối đặc và khung tiêu đề tối của Windows.

## Sửa lỗi máy mới trong v0.1.1

- Setup kiểm tra WebView2 ở HKLM và HKCU, cài gói Evergreen x64 Microsoft đã xác minh chữ ký/SHA-256 nếu thiếu. Bước này chạy cả khi không chọn Sally vì Music, Clip và DeepTutor cũng cần WebView2. Lỗi cài WebView2 chặn hoàn tất Setup.
- Chọn DeepTutor sẽ giải nén Python 3.13.12, Node 24.21.0 và wheel bundle DeepTutor 1.6.14. Setup tạo môi trường riêng, cài bằng `--no-index`, xác minh SHA-256 từng payload và thử import trước khi báo hoàn tất. Nếu VC Runtime yêu cầu khởi động lại, bước chuẩn bị được thực hiện khi mở DeepTutor sau khi restart. API/model vẫn cần cấu hình để dùng AI.
- DeepTutor và Chính tả được ưu tiên trong tối đa 5 icon sau khi áp dụng lựa chọn Setup; các tiện ích khác vẫn có trong menu chuột phải. Lựa chọn icon người dùng sửa sau đó được giữ.
- Chính tả dùng đúng cửa sổ soạn thảo trước khi bấm icon, không lấy nhầm clipboard cũ khi Ctrl+C thất bại, kiểm tra kết quả SendInput và mở kết quả để sao chép khi Windows chặn dán. Ứng dụng không tự nâng quyền để điều khiển trình soạn thảo đang chạy với quyền quản trị.
- Bộ Setup có đủ runtime để cài offline; AI cloud, Music và Clip vẫn cần Internet khi sử dụng. Bộ EXE portable tải runtime DeepTutor khi bật nếu không có payload đi kèm.

Build chạy thêm `prepare-webview-runtime.ps1` và `prepare-deeptutor-runtime.ps1`. Các payload tải chính thức nằm trong `dist/prerequisites`, không đưa credentials hay dữ liệu học vào release. Khi tự build phải có mạng để chuẩn bị payload lần đầu.

Tham khảo Microsoft: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution

## Kiểm thử phát hành v0.1.1

- `cargo test --locked` và `cargo clippy --locked --all-targets -- -D warnings`.
- `cargo test --locked --test native_spell -- --ignored --test-threads=1`: tạo ô EDIT Win32 tạm, lấy vùng chọn, sửa offline, dán rồi đọc lại nội dung thật; kiểm tra vùng chọn rỗng không dùng lại clipboard. Cần desktop Windows tương tác, không thao tác tài liệu có sẵn.
- Build `ISCC /DPentaKuruTest scripts/pentakuru-setup.iss`, chạy `check-installer.ps1`, `check-vc-runtime-installer.ps1`, `check-webview-installer.ps1`. Các ca prerequisite dùng mô phỏng, không gỡ/cài runtime chung của máy.
- Build `ISCC /DPentaKuruTest /DPentaKuruRealRuntimeCheck scripts/pentakuru-setup.iss`, chạy `check-offline-tutor-setup.ps1` để thực thi callback Setup thật trong thư mục/AppId riêng, chặn index mạng của pip, cài DeepTutor và kiểm tra backend/frontend/catalog rồi gỡ bản thử. Máy kiểm tra đã có VC++/WebView2; chưa thay thế kiểm thử trên VM Windows sạch.
