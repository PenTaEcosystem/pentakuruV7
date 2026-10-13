# Start và âm thanh video

Luồng Start cũ luôn mở microphone. Lựa chọn tắt mic chỉ được gửi sau
Start, và backend chỉ làm im mẫu âm thanh mà vẫn giữ thiết bị mở.
Trên tai nghe Bluetooth Classic, mở mic có thể chuyển từ Stereo sang
Hands-Free và gián đoạn tiếng video. Máy kiểm tra có các endpoint
ATH-CKS50TW Stereo và ATH-CKS50TW Hands-Free AG Audio.

Bản sửa mặc định chỉ thu tiếng hệ thống/ứng dụng. Lựa chọn thu thêm mic
được gửi cùng lệnh Start; nếu tắt mic thì không tạo luồng microphone.
Tắt mic giữa phiên đóng luồng, xóa mẫu mic còn đệm và bỏ kiểm tra mic
mất kết nối. Bật mic lại chỉ thành công khi luồng thực sự chạy được.
Bước thiết lập trên Windows cũng không mở mic để kiểm tra quyền nữa.

Để vừa nghe video vừa xem bản dịch: thoát bản cũ, mở bản Audio-Fix,
Alt + T, chọn nguồn video và Start, để ô thu microphone bỏ chọn.
Không thay đổi âm lượng, mute hoặc thiết bị phát mặc định của Windows.

Kiểm tra ngày 2026-10-07:

- 96 kiểm thử tự động qua; thêm 1 kiểm thử phần cứng được chạy riêng.
- Kiểm thử phần cứng mở loopback mặc định mà không mở microphone,
  xác nhận thiết bị phát mặc định không đổi; không lưu âm thanh, gọi API
  hoặc dùng khóa Gemini.
- Kiểm thử pipeline xác nhận xóa mẫu mic không làm mất tiếng hệ thống.
- TypeScript/Vite build, Cargo fmt và Clippy qua.
- Bản EXE Audio-Fix đã qua kiểm tra native: giao diện React/IPC hoạt động,
  mic mặc định tắt, mở → ẩn → mở lại → tắt thành công trong cùng tiến trình.

Việc dịch trực tiếp với Gemini và nghe xác nhận bằng tai vẫn cần được
kiểm tra trong phiên sử dụng của người dùng; các kiểm thử trên không
khẳng định chất lượng bản dịch hay đo chất lượng âm thanh Bluetooth.

Tài liệu Windows về các chế độ Bluetooth:
https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/bluetooth-classic-audio

## Vòng lặp giọng đọc bản dịch

Device loopback cũ thu cả tiếng do Sally phát ra. Bản No-Loop dùng
process loopback để loại trừ tiến trình Penta/Sally cùng các tiến trình
con khi thu toàn hệ thống với thiết bị mặc định. Khi thu một ứng dụng,
backend kiểm tra cây tiến trình đó không chứa Penta trước khi cho đọc.

Đọc bản dịch chỉ được phát khi nguồn thu đã được xác nhận loại trừ
ứng dụng và mic đang tắt. Nếu Windows không hỗ trợ bộ lọc này hoặc người
dùng chọn loopback của một thiết bị cụ thể, bản dịch chữ vẫn chạy nhưng
giọng đọc bị chặn. Bật mic cũng xóa hàng đợi giọng đọc và tạm ngừng đọc
để tránh âm loa vọng vào mic. Đổi nguồn sang nguồn không được cách ly
cũng xóa hàng đợi đọc và chặn phát các đoạn tiếp theo.

Tài liệu chính thức về include/exclude process tree:
https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/

Kiểm tra No-Loop ngày 2026-10-07: 98 kiểm thử tự động qua; 2 kiểm thử
phần cứng chạy riêng cũng qua. Một kiểm thử phát âm nhỏ 997 Hz trong
một giây: nguồn include-process đối chứng thu được tín hiệu, nguồn
exclude-process không thu lại tín hiệu đó. Kiểm thử còn lại xác nhận
không mở mic và không đổi thiết bị phát. Clippy và fmt qua. Không gọi
Gemini hoặc lưu âm thanh trong các kiểm thử này.
Bản EXE No-Loop cũng qua kiểm tra giao diện native và chu kỳ mở/ẩn/mở lại/tắt.
