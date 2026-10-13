# PentaKuru watch companion 1.2.3

Amazfit Bip Max **432×514, deviceSource 11206915**, Zepp OS API 3.0 trở lên. Giao diện ngũ giác được thay bằng khung chat cuộn toàn màn hình, bốn icon chức năng và thanh nhập/mic. Bàn phím mở bằng một trang riêng `page/keyboard` (không dùng lớp GROUP phủ trên chat), được đăng ký trong cả hai manifest. Gửi/Hủy trở về khung chat và giữ chế độ/câu trả lời trước. Bàn phím được tích hợp trong ứng dụng (chữ, số, dấu Việt, khoảng trắng, Xóa, Hủy, Gửi), không yêu cầu cài/bật bàn phím hệ thống Zepp. Hai nút **icon bàn phím** và **icon mic** dùng ảnh nền RGB không trong suốt, có vùng chạm lớn và nằm cao hơn đáy màn hình; ứng dụng ẩn thanh trạng thái hệ thống để dùng đầy đủ diện tích. Tiêu đề hiện 1.2.3 để phân biệt gói đã cài. Toàn bộ giao diện được thu nhỏ 20% và căn giữa: chữ, icon, bố cục, vùng chạm và bàn phím cùng tỉ lệ 0.8. Ảnh icon được xuất đúng kích thước 80%, tránh vùng ảnh lệch với vùng chạm.

## Cài đặt và kết nối

Đồng hồ → BLE → Zepp trên điện thoại → HTTP qua Wi-Fi/Tailscale → PentaKuru PC, cổng 3899. Điện thoại cần truy cập được PC và duy trì Zepp.

1. Đóng PentaKuru cũ, mở bản `PentaKuru-Office-Media.exe`. Giữ file ở thư mục chương trình hiện tại để dùng tiếp thư mục `Penteye Notes` bên cạnh exe.
2. Trong PC nhấn **Alt+Q → Đồng hồ Amazfit → Sao chép mã**. Trong cài đặt PentaKuru AI ở Zepp, nhập **URL PC** và dán vào **Mã ghép Windows** (chỉ một lần, ứng dụng tự lưu và dùng cho các lệnh tiếp theo). IP mặc định chỉ là ví dụ, thay theo PC của anh.
3. Cài gói `PentaKuru-Watch-1.2.3.zab` lên Bip Max qua chế độ nhà phát triển Zepp. Có thể dùng source: `npm ci`, `zeus login`, `zeus preview -t 432x514-amazfit-bip-max`, rồi quét QR bằng Zepp. Zeus cần đăng nhập; build không tự cài.
4. Trên PC cấu hình model AI có khả năng đọc ảnh hoặc UI-TARS riêng cho tác vụ Windows. Nếu dùng mic, cần **Gemini API key** để nhận diện giọng nói; nhập chữ không cần bước STT.
5. Mở ứng dụng trên đồng hồ, chờ **PC đã kết nối**. Chạm icon bàn phím để mở trang bàn phím và nhập chữ/số/dấu Việt; chạm mic để thu và chạm lại để gửi. Tự dừng ghi âm sau 15 giây.

## Bốn chức năng

| Icon | Chức năng |
| --- | --- |
| Ghi chú | Đọc ghi chú PC khi chọn; nhập/nói để lưu ghi chú mới. |
| Tìm kiếm | Nhập/nói từ khóa để tìm trong ghi chú. |
| AI | Hỏi AI bằng chữ hoặc giọng nói. |
| Windows | Gửi mục tiêu cho UI Agent PentaKuru, ví dụ “Mở Notepad”. PC thực hiện thao tác trên màn hình bằng cấu hình AI hiện tại. |

Nhập lệnh ngay trên đồng hồ rồi bấm Gửi: Zepp tự chuyển sang PentaKuru qua URL PC đã cấu hình. Bluetooth chỉ nối đồng hồ với điện thoại; mã ghép xác nhận quyền của Zepp với PC, không phải mật khẩu phải nhập mỗi lệnh. Các nút Ghi chú, Tìm kiếm và AI không yêu cầu mã ghép Windows.

Windows hiển thị tiến độ ở dòng trạng thái; icon mic chuyển thành **ô vuông Dừng** trong lúc thực thi. Nút Dừng gửi yêu cầu hủy và PC ngừng các bước tiếp theo; thao tác đã thực hiện không tự hoàn tác. Đóng ứng dụng đồng hồ không tự hủy lệnh PC. Chỉ nhận một tác vụ Windows mỗi lần; nếu PC đang chạy tác vụ khác sẽ báo bận.

**Câu hỏi và câu trả lời cũ giữ nguyên khi đổi chế độ, đang chờ, mất mạng hoặc có lỗi. Chỉ phản hồi mới thành công thay thế cặp cũ.** Tiến độ Windows không thay thế câu trả lời; chỉ khi hoàn tất mới cập nhật khung chat. Vuốt lên/xuống vùng chat để đọc đầy đủ. Lỗi hiển thị thêm ở cuối khung chat và dòng trạng thái.

Mã ghép Windows lưu trong cài đặt PC/Zepp, gửi trong thân POST và không đưa vào URL hay API trạng thái công khai. Không gửi lại POST thực thi khi lỗi mạng để tránh chạy lệnh hai lần. Trạng thái được hỏi lại sau lỗi kết nối; nếu PC khởi động lại, kiểm tra kết quả trên PC trước khi gửi lệnh mới.

## Khi không mở được mic

Bản 1.1.2 đồng bộ quyền `device:os.mic` từ cấu hình `app-ai.json` sang `app.json` mà Zeus thực sự sử dụng. Bản cũ chỉ có quyền này trong file cấu hình phụ, nên gói build thường không có nó. Cần cài gói mới để thay đổi quyền có hiệu lực; cho phép mic nếu đồng hồ hỏi.

Bộ ghi âm được mở khi chạm nút mic, sau khi trang đã sẵn sàng. Nếu lần mở đầu thất bại, chạm lần nữa sẽ thử lại. Với firmware có START/STOP, ứng dụng chờ các sự kiện này. Với firmware chỉ cung cấp setFormat/start/stop, ứng dụng xác nhận file thực sự có dữ liệu rồi mới báo đang thu; khi dừng thì chờ kích thước ổn định trước khi đọc. File Opus dạng length/final-range/packet được đóng gói lại thành Ogg Opus, giữ nguyên các gói âm thanh, để gửi PC.

Vuốt vùng kết quả để đọc mã lỗi và deviceSource: `MIC_CREATE` là bước hệ thống cấp bộ ghi âm, `MIC_API` là thiếu API, `MIC_SESSION` là bước khởi tạo xử lý ghi âm. Giữ nguyên lỗi chi tiết đi kèm để chẩn đoán. Chưa kiểm tra được mic trên Bip Max thật, nên chưa thể kết luận lỗi hệ thống/firmware nếu bản mới vẫn không thu được.

Penta PC mới có nút phóng to/khôi phục, thu xuống thanh tác vụ và tay kéo góc dưới phải trong cửa sổ ghi chú. Có thể nhấp đúp vùng trống trên thanh tiêu đề để phóng to/khôi phục.

## Kiểm tra và build

`npm ci`, `npm test`, `npm run build` (cần Zeus CLI). Gói xuất ở `dist/`. Các kiểm tra tự động xác nhận BLE, ghi âm, ghép mã, gửi lệnh, giữ câu trả lời, hoàn tất và hủy tác vụ. Build QuickJS thành công; chưa thay thế thử mic/BLE và Windows trên thiết bị thật.

Để tạo lại icon: từ thư mục source PentaKuru chạy `python scripts/render-watch-assets.py` (cần Pillow). Bản xem trước bố cục được dựng từ widget native bằng `WATCH_LAYOUT_OUTPUT` trong bài kiểm tra, không phải ảnh chụp đồng hồ thật.
