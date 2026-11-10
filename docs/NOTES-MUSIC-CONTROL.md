# Ghi chú và điều khiển Penta Music

- Thư viện chỉ đọc Markdown; không đọc JSON hay thư mục runtime (Office, Sally, integrations) thành note. Thư viện hiển thị dạng cây: thư mục con (`Cha/Con`) thu gọn/mở bằng mũi tên, tìm kiếm không phân biệt dấu. Các file gốc vẫn được giữ trên máy.
- Note mở ở **Xem định dạng**, hiển thị heading và chữ đậm/nghiêng. Chọn **Sửa Markdown** để sửa; H1/H2 đổi heading của dòng đang chọn.
- Bật **AI** trên note muốn chia sẻ. Alt+A dùng nội dung hiện tại của note đang chọn cùng các tóm tắt đã chia sẻ. **Hỏi AI về note** mở câu hỏi; **AI sắp xếp heading** yêu cầu bản đề xuất có cấu trúc.
- Mỗi phản hồi trong chat có nút **Thêm vào ghi chú đang chọn**. Nút thêm phản hồi dưới heading riêng, giữ nội dung cũ. Kiểm tra note đang chọn trước khi lưu. Nội dung note được coi là dữ liệu tham khảo, không phải lệnh điều khiển ứng dụng.

Bật **Penta Music** trong Alt+Q, rồi nhắn trong Alt+A:

| Câu nhắn | Hành động |
| --- | --- |
| Anh muốn nghe nhạc | Tiếp tục bài đang có hoặc phát bài đầu tiên trong danh sách |
| Tìm bài hát Nàng Thơ | Tìm trong giao diện Penta Music |
| Anh muốn nghe bài Nàng Thơ | Tìm và phát kết quả đầu tiên |
| Tạm dừng nhạc | Bấm điều khiển tạm dừng |

Lệnh nhạc chạy cục bộ qua WebView2, không cần API AI. Tìm kiếm/phát vẫn cần trang Penta Music tải được và có bài khả dụng. Dùng phiên đăng nhập của trang, không trích xuất tài khoản. Chỉ báo đang phát sau khi kiểm tra audio; có thời gian chờ và hỗ trợ Dừng/Esc cho thao tác đang đợi. Dừng tác vụ không tự tạm dừng nhạc đã bắt đầu phát; dùng câu lệnh tạm dừng.

Kiểm tra: `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `node scripts/test-music-commands.cjs`. Kiểm thử cầu nhạc mô phỏng DOM; chưa xác nhận âm thanh thực tế trên dịch vụ.
