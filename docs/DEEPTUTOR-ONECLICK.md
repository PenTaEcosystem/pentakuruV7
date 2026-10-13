# DeepTutor: cài bằng một nút và cách sử dụng

Mở Settings → DeepTutor → **Cài đặt / mở DeepTutor**. Người dùng không cần cài Python, Node.js, Docker hoặc chạy lệnh. Có thể bật DeepTutor trong danh sách tiện ích; thao tác này cũng bắt đầu cài/mở. Chương trình hiện từng giai đoạn và có nút Dừng.

## Luồng cài đặt

1. Kiểm tra thư mục runtime riêng.
2. Chuẩn bị Python 3.13.12 từ gói NuGet chính thức của Python, kiểm tra SHA256 và chữ ký Python Software Foundation.
3. Chuẩn bị Node.js 24.21.0 LTS dạng ZIP chính thức, kiểm tra SHA256 và chữ ký OpenJS Foundation.
4. Tạo môi trường Python riêng, cài DeepTutor 1.6.14 với bộ ràng buộc 166 thư viện đã kiểm tra.
5. Khởi động backend/giao diện, kiểm tra cả hai sẵn sàng rồi mở cửa sổ DeepTutor.

Python/Node nằm dưới `Penteye Notes/DeepTutor/engines`; không sửa PATH hệ thống, đăng ký Python mới hay thay đổi Node/Python của chương trình khác. Các gói đã tải được giữ để thử lại/mở nhanh. Tắt tiện ích dừng những tiến trình PentaKuru tạo, giữ runtime và dữ liệu học.

Cài và mở giao diện vẫn được khi chưa có API key. Nếu PentaKuru đã chọn API/model và bật dùng chung, cấu hình được đưa vào profile PentaKuru AI của DeepTutor. Sau khi đổi API/model, bấm **Kết nối lại / đồng bộ AI**. Cấu hình được chốt khi bắt đầu công việc, nên thay đổi trong lúc cài cần đồng bộ lại sau khi hoàn tất.

## Khi bắt đầu học

- Cấu hình API/model trong PentaKuru, hoặc dùng cấu hình riêng trong DeepTutor.
- Mở DeepTutor; dùng Chat, Reading và những chức năng học/bài tập của giao diện chính thức.
- Nạp tài liệu và chọn kho kiến thức cần dùng. Với truy xuất tài liệu/RAG, chọn model embedding tương thích trong Settings của DeepTutor; không dùng model chat thay cho embedding.
- Đặt câu hỏi, yêu cầu giải thích nội dung hoặc tạo bài tập. AI gọi nhà cung cấp/model đã chọn; bản cài không chứa model AI hay API key miễn phí.

Nguồn tính năng: [DeepTutor HKUDS v1.6.14](https://github.com/HKUDS/DeepTutor/tree/v1.6.14).

## Tường lửa và mạng

Giao diện/backend chỉ bind `127.0.0.1`, cổng 3897/3896. PentaKuru kiểm tra sức khỏe qua loopback, không qua HTTP proxy. Ngoại lệ NO_PROXY cho localhost được thêm riêng cho tiến trình DeepTutor và giữ các mục đã có. Không tạo quy tắc inbound, không tắt tường lửa hoặc bỏ kiểm tra chứng chỉ TLS.

Lần cài đầu cần HTTPS đến `www.nuget.org`, `globalcdn.nuget.org`, `nodejs.org`, `pypi.org`, `files.pythonhosted.org`. Khi dùng AI cloud cần truy cập miền của provider đã chọn. Nếu mạng cơ quan chặn các miền này, quản trị mạng cần cấp phép hoặc cấu hình proxy hợp lệ; ứng dụng không vượt chính sách mạng. Model chạy cục bộ dùng địa chỉ của model đã cấu hình.

Nếu cổng bị chương trình khác sử dụng, PentaKuru báo rõ thay vì dừng chương trình đó. Lỗi tải/cài có thể thử lại; chi tiết ở `DeepTutor/install.log`, `bootstrap-error.txt`, `startup.log`. Không đưa những thư mục này hay catalog chứa API key vào gói phát hành.

## Icon

EXE, bộ cài, shortcut và icon mặc định của các cửa sổ PentaKuru dùng ngũ giác đen viền trắng. Biểu tượng từng chức năng trên thanh 0–5 icon vẫn riêng biệt để dễ nhận diện.

Nguồn runtime: [Python NuGet](https://docs.python.org/3.13/using/windows.html#the-nuget-org-packages), [Node.js 24.21.0 LTS](https://nodejs.org/en/blog/release/v24.21.0).
