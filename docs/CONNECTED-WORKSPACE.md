# Ghi chú, AI và công việc dùng chung

## Kết quả phân tích

Trước thay đổi, Notes lưu Markdown cục bộ, Chat AI chỉ nhận lịch sử và tệp đính kèm, Music mở một WebView, còn DeepTutor có workspace và Task Board riêng. Dùng chung API key chỉ thống nhất nhà cung cấp AI; chưa chia sẻ ngữ cảnh công việc.

Luồng đề xuất: ghi ý tưởng → bật AI cho note cần dùng → trao đổi với AI có ngữ cảnh → tạo thẻ công việc → theo dõi trong Kanban của DeepTutor → đưa kết quả về note. Ghi chú giữ nội dung, thẻ giữ tiến độ, AI hỗ trợ trên những dữ liệu người dùng chủ động chia sẻ.

## Đã triển khai

- Cửa sổ note mặc định 480 × 560, có thể thay đổi kích thước. Thư viện thu gọn mặc định; mở từ thanh trên.
- Thanh công cụ dùng icon AI, hội thoại, Kanban, H1/H2/B/I và con mắt xem định dạng; mỗi icon có chú thích khi rê chuột và tên cho trình đọc màn hình. Định dạng lưu Markdown và có xem trước.
- Nút AI có chữ và trạng thái sáng. Mặc định tắt, được lưu riêng cho từng note trong metadata.
- Bật AI cho note sẽ tự gửi nội dung tới API/model đang cấu hình để tóm tắt sau khi ngừng gõ 5 giây. Ý chính được lưu trong metadata cạnh note, tồn tại qua các lần mở ứng dụng. Sửa note sẽ làm bản cũ hết hiệu lực và tự tạo bản mới; kết quả đến muộn không được ghi đè note đã đổi hoặc đã tắt AI. Mỗi lần chỉ chạy một tác vụ tóm tắt; lỗi được báo và thử lại sau 60 giây. Note vượt 24.000 ký tự được yêu cầu tách, không âm thầm bỏ phần cuối.
- Chat dùng các bản tóm tắt còn khớp với note, tối đa 1.200 ký tự mỗi bản và 16.000 ký tự tổng; không tự đưa toàn bộ nội dung gốc vào chat. Tab Profile AI cho kiểm tra dữ liệu này. Khi chưa có bản tóm tắt mới, AI thấy trạng thái chờ và được yêu cầu không suy đoán nội dung.
- Không tự đọc attachments. Tắt AI ngừng bổ sung note vào ngữ cảnh và ngừng cập nhật tóm tắt mới; dữ liệu đã gửi, bản tóm tắt lưu trên máy và câu trả lời cũ không bị xóa. API đang chạy có thể đã nhận dữ liệu trước lúc tắt; kết quả sau đó không được lưu. Chi phí API theo nhà cung cấp đang cấu hình.
- Kanban dùng API của DeepTutor 1.6.14 đang chạy do PentaKuru quản lý, cùng dữ liệu với bảng gốc: `GET /api/task-board`, `POST /api/task-board/cards`, `PATCH /api/task-board/cards/{id}`. Chỉ dùng loopback và không qua proxy.
- Từ note: Tạo thẻ Kanban lưu liên kết bằng ID; Gửi sang thẻ cập nhật tiêu đề/nội dung của thẻ đã liên kết.
- Từ Kanban: đổi trạng thái Cần làm/Đang làm/Hoàn tất; Mở note tìm note đã liên kết hoặc tạo note từ thẻ; Lấy nội dung thẻ yêu cầu xác nhận trước khi thay nội dung note.
- Bảng làm mới mỗi 5 giây khi đang hiển thị. Đây là bảng đọc trạng thái chung và đồng bộ nội dung theo thao tác, chưa phải tự động hợp nhất nội dung hai chiều.
- Thẻ có giới hạn 160 ký tự tiêu đề và 2.000 ký tự nội dung theo API DeepTutor. Note dài được yêu cầu rút gọn/tách, không âm thầm cắt mất nội dung.
- UI-TARS tiếp tục được bật/tắt trong tiện ích và dùng từ AI; bỏ icon riêng khỏi dock, bộ chọn icon và menu tiện ích. Cấu hình dock cũ được chuẩn hóa.
- Trong Chat AI có thể gõ `mở nhạc`, `mở music`, `@music` hoặc `/music` để mở Penta Music đã bật, không cần gửi yêu cầu lên model. Chưa tự chọn hoặc phát bài theo yêu cầu tự nhiên.

## Cách dùng thử

1. Tạo note về mục tiêu và việc đang làm; chọn nội dung và bấm H1/H2/B/I, sau đó Xem định dạng.
2. Bật icon AI trên note, ngừng gõ 5 giây và chờ trạng thái Đã ghi nhớ ý chính. Mở phần này hoặc Profile AI để kiểm tra tóm tắt. Icon hội thoại mở ô chat; AI dùng những ý chính đã lưu khi trả lời.
3. Mở DeepTutor, chờ sẵn sàng. Tạo thẻ từ note, mở Kanban và đổi trạng thái; kiểm tra bảng trong DeepTutor cũng đổi.
4. Sửa nội dung thẻ bên DeepTutor, làm mới Kanban rồi chọn Lấy nội dung thẻ và xác nhận. Nội dung note không bị thay chỉ vì bảng làm mới.
5. Mở note liên kết rồi bấm Gửi sang thẻ để cập nhật theo chiều ngược lại. Thao tác này thay tiêu đề/nội dung thẻ; trạng thái vẫn giữ.
6. Tắt AI ở note và kiểm tra Profile AI bỏ note đó. Bắt đầu cuộc trò chuyện mới nếu muốn bỏ cả lịch sử đã có.

## Hướng nối tiếp

Music cần một giao diện điều khiển có hợp đồng rõ ràng: tìm bài, phát/tạm dừng và báo trạng thái. Sau đó AI có thể đề nghị nhạc dựa trên note được chia sẻ và thực hiện hành động được người dùng yêu cầu. Hiện chưa có hợp đồng điều khiển này trong tích hợp PentaKuru; việc mở WebView không chứng minh bài đã phát.

Để đồng bộ note tự động, cần lưu phiên bản đã đồng bộ, phát hiện sửa đồng thời và giao diện giải quyết xung đột. Tránh dùng hai bản nội dung độc lập rồi âm thầm ghi đè. Nếu mở liên kết DeepTutor từ máy chủ ngoài hoặc chế độ nhiều tài khoản, cần thêm chọn tài khoản, xác thực và workspace; tích hợp hiện tại chỉ kết nối phiên cục bộ do PentaKuru khởi chạy.
