# Bộ cài PentaKuru

Chạy `PentaKuru-Setup.exe`: giới thiệu → giấy phép → thư mục → chọn tiện ích → biểu tượng → xác nhận → tiến độ → hoàn tất. Giao diện chính tiếng Việt, màu tối, có Quay lại và Hủy. Cài theo tài khoản Windows hiện tại, mặc định `%LOCALAPPDATA%/Programs/PentaKuru`; phần ứng dụng không cần quyền quản trị. Nếu thiếu Microsoft Visual C++ Runtime x64, Windows sẽ yêu cầu quyền quản trị để cài thành phần dùng chung này.

PentaKuru và AI là phần lõi. Có thể chọn Ghi chú, Tìm file, Chính tả, Sally, Writer, Impress, Calc, Music, Clip, DeepTutor và UI-TARS. Tùy chọn được ghi vào `installation-options.json` và ứng dụng áp dụng một lần cho mỗi lượt cài; thay đổi sau trong Settings được giữ nguyên. Cài lại vào cùng thư mục sẽ áp dụng bộ lựa chọn mới, giữ cấu hình tài khoản/API key hiện có.

EXE chứa ứng dụng, hướng dẫn và Microsoft Visual C++ Runtime x64 đã được xác minh chữ ký. Setup kiểm tra phiên bản trong cả hai nhánh registry x64/32-bit và sự hiện diện của `vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140.dll`. Bản ứng dụng hiện tại dùng mức runtime đã kiểm tra `14.44.35211.0` trở lên; mức này phải được xét lại nếu đổi MSVC/toolchain. Runtime phù hợp được bỏ qua; bản cũ được cập nhật; DLL thiếu khi phiên bản đã đủ sẽ được đề nghị sửa. Payload được kiểm tra SHA-256 trước khi chạy Microsoft Setup. Không cần Internet cho bước runtime này. Nếu người dùng hủy quyền quản trị hoặc runtime cài lỗi, Setup dừng trước khi ghi ứng dụng. Nếu cần khởi động lại, Setup không tự mở PentaKuru và hiển thị yêu cầu khởi động lại. Gỡ PentaKuru không gỡ runtime dùng chung. Log Microsoft installer được giữ trong `%LOCALAPPDATA%/PentaKuru/SetupLogs`.

EXE chứa ứng dụng và hướng dẫn. Runtime Office/DeepTutor chưa được nhúng vào bộ cài: ứng dụng chuẩn bị chúng khi tiện ích được bật, cần Internet. LibreOffice có sẵn được tái sử dụng. DeepTutor tự chuẩn bị Python/Node riêng bằng nút Cài đặt / mở; API key/model cấu hình trong Settings hoặc dùng cấu hình đã có của PentaKuru. Music/Clip đăng nhập riêng trên trang dịch vụ. Calc hiện chỉ mở/chuyển bảng tính.

Gỡ qua Windows Installed apps hoặc shortcut Gỡ PentaKuru. Chỉ xóa file bộ cài sở hữu; ghi chú, bản sao tài liệu, runtime và phiên đăng nhập được giữ. Có thể mở Settings bật/tắt tiện ích mà không cần cài lại.

## Build

Yêu cầu Inno Setup 6.7.3+ chính thức: https://jrsoftware.org/isdl.php

1. `./scripts/package.ps1`
2. `./scripts/build-setup.ps1 -Compiler "C:/Program Files (x86)/Inno Setup 6/ISCC.exe"`
3. Kết quả `dist/PentaKuru-Setup.exe`.

Bộ cài chưa ký chứng thư phát hành của Pentasys.

Kiểm thử bộ cài dùng AppId riêng (ISCC /DPentaKuruTest) và thư mục thử riêng, giữ nguyên đăng ký của bản PentaKuru thật.

Script build tải `https://aka.ms/vc14/vc_redist.x64.exe` vào `dist/prerequisites`, xác minh chữ ký Microsoft, ghi phiên bản và SHA-256 cho Inno Setup. Có thể chạy `prepare-vc-runtime.ps1 -Refresh` để cập nhật payload trước khi build lại. File EXE portable vẫn cần runtime; dùng Setup để tự kiểm tra và cài.

`PentaKuru-Setup.exe /VCRUNTIMEREPORT="C:/path/runtime-report.json"` chỉ ghi báo cáo phiên bản/nhu cầu runtime rồi thoát, không cài đặt.

Kiểm thử bootstrap: build với `-TestBuild`, rồi chạy `scripts/check-vc-runtime-installer.ps1`. Các trạng thái runtime và mã thoát Microsoft được mô phỏng trong bản test; bản sản xuất không nhận tham số mô phỏng. Tests xác minh bỏ qua, cài, sửa, phiên bản mới hơn, hủy, lỗi và khởi động lại. Chưa thay thế cho kiểm thử cài thực tế trên máy Windows sạch thiếu runtime.

Tài liệu Microsoft: https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files
