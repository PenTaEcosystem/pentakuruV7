# Bộ cài PentaKuru

Chạy `PentaKuru-Setup.exe`: giới thiệu → giấy phép → thư mục → chọn tiện ích → biểu tượng → xác nhận → tiến độ → hoàn tất. Giao diện chính tiếng Việt, màu tối, có Quay lại và Hủy. Cài theo tài khoản Windows hiện tại, mặc định `%LOCALAPPDATA%/Programs/PentaKuru`; không cần quyền quản trị.

PentaKuru và AI là phần lõi. Có thể chọn Ghi chú, Tìm file, Chính tả, Sally, Writer, Impress, Calc, Music, Clip, DeepTutor và UI-TARS. Tùy chọn được ghi vào `installation-options.json` và ứng dụng áp dụng một lần cho mỗi lượt cài; thay đổi sau trong Settings được giữ nguyên. Cài lại vào cùng thư mục sẽ áp dụng bộ lựa chọn mới, giữ cấu hình tài khoản/API key hiện có.

EXE chứa ứng dụng và hướng dẫn. Runtime Office/DeepTutor chưa được nhúng vào bộ cài: ứng dụng chuẩn bị chúng khi tiện ích được bật, cần Internet. LibreOffice có sẵn được tái sử dụng. DeepTutor tự chuẩn bị Python/Node riêng bằng nút Cài đặt / mở; API key/model cấu hình trong Settings hoặc dùng cấu hình đã có của PentaKuru. Music/Clip đăng nhập riêng trên trang dịch vụ. Calc hiện chỉ mở/chuyển bảng tính.

Gỡ qua Windows Installed apps hoặc shortcut Gỡ PentaKuru. Chỉ xóa file bộ cài sở hữu; ghi chú, bản sao tài liệu, runtime và phiên đăng nhập được giữ. Có thể mở Settings bật/tắt tiện ích mà không cần cài lại.

## Build

Yêu cầu Inno Setup 6.7.3+ chính thức: https://jrsoftware.org/isdl.php

1. `cargo build --release --locked`
2. `ISCC.exe scripts/pentakuru-setup.iss`
3. Kết quả `dist/PentaKuru-Setup.exe`.

Bộ cài chưa ký chứng thư phát hành của Pentasys.

Kiểm thử bộ cài dùng AppId riêng (ISCC /DPentaKuruTest) và thư mục thử riêng, giữ nguyên đăng ký của bản PentaKuru thật.
