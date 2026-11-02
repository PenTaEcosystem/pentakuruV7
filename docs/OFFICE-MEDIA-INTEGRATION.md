# Office, Penta Music và Penta Clip

Ngày triển khai: 13/10/2026. Bản Windows x64, WebView2. Các tiện ích mới mặc định tắt; bật trong Alt+Q → Settings. Không có phím tắt Office/Music/Clip mới.

## Office

Mở mục **Office · Writer / Impress / Calc**, bật Office và chọn các ứng dụng cần dùng. Writer và Impress được chọn sẵn, Calc tùy chọn. Bấm **Mở Office** hoặc mục Office trong menu chuột phải PentaKuru.

- Tài liệu mở trong cửa sổ LibreOffice thật. PentaKuru chỉ hiện bảng điều khiển nhỏ, tab active và yêu cầu AI.
- **Tài liệu** mở danh sách có tìm kiếm, lọc Word/PowerPoint/Excel, lọc nhãn và Unknown. Thư mục nguồn mỗi dòng một đường dẫn; bấm Quét lại sau khi đổi.
- Quét file mặc định không gán nhãn. Bấm **Hệ thống** để AI phân loại các thư mục theo lô bằng cấu hình AI chung. Menu chuột phải file → Đổi nhãn để sửa thủ công; nhãn thủ công không bị ghi đè.
- Bấm file thêm active, bấm tab đưa đúng cửa sổ LibreOffice lên. Nút × bỏ active, tài liệu vẫn mở để người dùng xử lý thay đổi chưa lưu trong LibreOffice.
- Các phạm vi: Vùng đang chọn, File đang xem, Các file giao AI. Với phạm vi nhiều file, checkbox chỉ định file được sửa; danh sách được chốt trước khi chạy.
- **Sửa chính tả / Nâng cấp bài viết** dùng model AI PentaKuru. **Căn giữa ảnh / Đổi kiểu chữ / Lưu / Hoàn tác** chạy UNO trực tiếp, không cần gọi model.
- Writer hỗ trợ đoạn văn, bảng và text frame. Impress hỗ trợ text shape và ảnh trên slide. Sửa nội dung bằng các phần khác nhau của chuỗi, giữ định dạng phần chữ không đổi.
- Lưu tạo bản sao file gốc trong `Penteye Notes/Office/backups` trước khi ghi. AI sửa trên phiên đang mở, chưa tự lưu cho đến khi bấm Lưu hoặc dùng lệnh lưu của LibreOffice.
- Dừng ngăn các file và bước ghi tiếp theo; không tự đảo ngược thao tác đã xong. Hoàn tác dùng nút riêng.

LibreOffice đã có trên máy thì tái dùng `program/python.exe` và UNO. Nếu chưa có, bật Office sẽ tải bộ MSI chính thức 26.8.1, kiểm tra chữ ký và giải nén runtime riêng; nút **Cài LibreOffice** cho phép thử lại khi gặp lỗi. Không cài khi tiện ích tắt. Có thể nhập thư mục `program` của bản LibreOffice khác trong Settings.

Calc hiện mở/chuyển file; các thao tác AI trực tiếp trong bản này dành cho Writer và Impress. Draw/Base/Math, mẫu định dạng cá nhân, nhập tài liệu mã hóa/password và sửa ảnh bên trong nhóm chọn phức tạp là phạm vi tiếp theo.

Hoàn tác Writer dùng nhóm undo của LibreOffice. Impress dùng lịch sử PentaKuru trong worker đang chạy vì UNO không luôn tạo native undo cho text shape. Lịch sử Impress không tồn tại qua việc thoát PentaKuru; lưu hoặc hoàn tác trước khi thoát. Khi đóng PentaKuru, tài liệu LibreOffice vẫn có thể còn mở để bảo toàn việc đang làm.

## Icon và menu chuột phải

Trong Settings → Cài đặt Ẩn / Hiện & Chế độ hiển thị, chọn các icon muốn đặt trên thanh công cụ. Mặc định 4 icon; có thể chọn 0–5. Bộ đếm hiển thị 0/5, 1/5…5/5, có ô tìm icon và nút Tắt tất cả icon. Mỗi chức năng có một icon riêng; Penta Music là nốt nhạc, Penta Video là màn hình phát video. Khi đủ 5, bỏ một icon để chọn icon khác. Bật tiện ích trước khi chọn icon của nó.

Các chức năng đã bật vẫn có trong menu chuột phải trên mô hình hoặc thanh icon, kể cả khi không được chọn hiển thị. Ẩn icon không tắt tiện ích.

## Penta Music và Penta Video

PentaKuru mở trực tiếp trang Music và Clip trong WebView riêng, tối ưu CSS cho khung nhỏ. Không tạo player, playlist, đăng nhập hoặc hệ thống tài khoản riêng. Tìm nhạc, đăng nhập, playlist và phát nhạc đều do chính trang Music cung cấp; video và subscriptions dùng trang Clip. Không còn mục JWT trong Settings hay logic nhập token từ PentaKuru. Phiên web được giữ cho lần mở sau.

- Music: `https://music.pentasys.id.vn/`, khung mặc định 480×420.
- Video: `https://clip.pentasys.id.vn/feed/subscriptions`, khung mặc định 480×420.
- Khung co giãn được, điều chỉnh khoảng cách và bố cục cho màn hình nhỏ; giữ nguyên nút đăng nhập và điều khiển của trang.
- Tắt tiện ích hoặc đóng khung hủy WebView để dừng phát. Đăng xuất trên chính trang dịch vụ để xóa phiên đăng nhập.

WebView Music/Clip không được cấp capabilities của Sally và chỉ điều hướng trong origin dịch vụ tương ứng. API key AI không được chuyển sang Music/Clip.

## Kiểm chứng

- Rust: bộ kiểm thử, strict Clippy, giới hạn icon và menu chuột phải.
- Office: DOCX/PPTX thử nghiệm thật, mở có giao diện, sửa, style, căn ảnh, hoàn tác, lưu/mở lại và từ chối revision cũ. Không dùng tài liệu cá nhân.
- Music/Clip: kiểm tra DOM trong WebView2 thật, audio engine, tìm kiếm và đăng nhập gốc, bố cục khung nhỏ, mở/đóng. Không đăng nhập tài khoản thật trong kiểm thử.

## Nguồn chính thức

- LibreOffice UNO: https://api.libreoffice.org/
- Định dạng Office: https://help.libreoffice.org/latest/en-US/text/shared/guide/ms_user.html
- Penta Music: https://music.pentasys.id.vn/
- Penta Clip: https://clip.pentasys.id.vn/feed/subscriptions

## Office gọn hơn

Thanh công cụ dùng icon vector có nhãn trợ năng và tooltip. Thư viện và danh sách active dùng chữ 12px, không có khung từng file; tên dài rút gọn và xem đầy đủ khi rê chuột. Bộ lọc loại file/nhãn và phạm vi AI nằm trong menu icon. Một khung chat chứa lịch sử yêu cầu/kết quả, ô nhập và icon thao tác. Các lệnh hoàn tác/lưu/căn ảnh/đổi font giữ bản nháp đang gõ. Chỉnh sửa vẫn dùng LibreOffice trực tiếp và chỉ những file thuộc phạm vi được chọn.
