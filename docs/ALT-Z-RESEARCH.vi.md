# Phương án sửa tiếng Việt bằng Alt + Z trong PentaKuru

Ngày tìm hiểu: 04/10/2026. Đây là đề xuất triển khai, chưa phải tính năng có trong file exe hiện tại. Chưa tải mô hình/dataset, chưa đăng ký phím tắt và chưa gửi nội dung người dùng tới dịch vụ nào.

Có thể bổ sung vào ứng dụng Rust hiện tại một lệnh sửa văn bản hoạt động giữa nhiều ứng dụng. Phạm vi là đoạn đang chọn trong ô nhập đang hoạt động. Không thể cam kết mọi ứng dụng đều hỗ trợ đọc/thay văn bản theo cùng một cách.

## Cách sử dụng đề xuất

1. Chọn đoạn cần sửa trong ứng dụng đang dùng.
2. Bấm Alt + Z. PentaKuru lấy đoạn đã chọn và xử lý trong một worker riêng, không chặn animation.
3. Mặc định sửa chính tả, dấu câu và cách viết thuật ngữ; giữ ý và giọng nói của người viết.
4. Các thay đổi rõ ràng có thể áp dụng trực tiếp theo tùy chọn. Chỗ mơ hồ và việc viết lại văn phong hiển thị bản đối chiếu để người dùng chọn.
5. Thay đúng vùng văn bản ban đầu khi cửa sổ, ô nhập và vùng chọn còn hợp lệ. Nếu người dùng đã chuyển ứng dụng hay sửa đoạn đó, chỉ hiển thị kết quả để copy.

Nếu chưa chọn đoạn, bản đầu yêu cầu chọn văn bản. Không tự gửi Ctrl + A vào mọi ứng dụng vì lệnh này có thể chọn cả tài liệu hay các đối tượng không phải văn bản.

Ví dụ minh họa mục tiêu, chưa phải kết quả chạy thử:

> “cho a sua loi chinh ta, giu api va fps”
>
> “Cho anh sửa lỗi chính tả, giữ API và FPS.”

Đoạn thiếu dấu có nhiều cách hiểu phải được xử lý theo ngữ cảnh hoặc đưa ra gợi ý; không thể đảm bảo luôn đoán đúng.

## Phần tích hợp Rust

Windows có API [RegisterHotKey](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) để đăng ký phím tắt toàn hệ thống. Dùng MOD_NOREPEAT để một lần giữ phím không tạo nhiều yêu cầu. Có thể dùng windows-sys đã có trong dự án hoặc [global-hotkey](https://github.com/tauri-apps/global-hotkey). Crate này hỗ trợ Windows, macOS và Linux X11; không hỗ trợ Wayland bằng chính backend đó.

Ưu tiên UI Automation để đọc vùng chọn trong ô nhập hỗ trợ TextPattern. [Tài liệu UI Automation của Microsoft](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/add-content-to-a-text-box-using-ui-automation) cho thấy không phải control nào cũng hỗ trợ ValuePattern để ghi văn bản, nên cần đường dự phòng copy/paste. [arboard](https://docs.rs/arboard/latest/arboard/) là thư viện Rust truy cập clipboard. Cần phát hiện copy thành công, không lấy nhầm clipboard cũ. Khôi phục clipboard chỉ khi không ghi đè nội dung mới của người dùng và có thể bảo toàn định dạng đã đọc.

Đường dự phòng trên Windows dùng [SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput). API bị giới hạn bởi UIPI khi tương tác với ứng dụng có mức quyền cao hơn. Cần đợi Alt/Z được nhả trước khi gửi tổ hợp copy/paste. Ô mật khẩu, ô chỉ đọc hoặc ô không lấy được văn bản không được sửa tự động. Bản đầu thay plain text, chưa cam kết giữ định dạng rich text trong Word.

macOS cần quyền [Accessibility](https://support.apple.com/en-my/guide/mac-help/mh43185/mac) để điều khiển ứng dụng khác. Linux Wayland cần backend riêng cho [GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html); có phím tắt không đồng nghĩa đã có quyền đọc và thay văn bản trong ứng dụng khác.

Alt + Z có thể xung đột với [NVIDIA Overlay](https://www.nvidia.com/en-us/geforce/news/nvidia-app-download-and-features/). Giữ phím theo yêu cầu nhưng có mục đổi phím và thông báo nếu đăng ký thất bại.

## Dữ liệu và bộ sửa

| Nguồn | Mục đích phù hợp | Giới hạn |
| --- | --- | --- |
| [VSEC](https://github.com/VSEC2021/VSEC) | Kiểm thử lỗi chính tả tiếng Việt theo câu; 9.341 câu, 11.202 lỗi theo tác giả | Là dataset, không phải dịch vụ sửa có thể gọi ngay; repository đã đọc chưa làm rõ giấy phép phân phối dữ liệu |
| [ViLexNorm](https://github.com/ngxtnhi/ViLexNorm) | Nghiên cứu chuẩn hóa cách viết trong hội thoại/mạng xã hội; 10.467 cặp câu | Tác giả công bố CC BY-NC-SA 4.0 và mục đích nghiên cứu; không mặc định đóng gói vào sản phẩm thương mại |
| [Google developer word list](https://developers.google.com/style/word-list), [Microsoft Writing Style Guide](https://learn.microsoft.com/style-guide) | Tham khảo cách viết thuật ngữ và văn phong công nghệ | Là tài liệu hướng dẫn, không phải bộ API sửa lỗi hoặc một chuẩn duy nhất cho mọi chuyên ngành |

Nên duy trì glossary riêng: cách viết chuẩn, biến thể thường gõ sai, lĩnh vực, nguồn tham khảo và các từ cần giữ nguyên. Ví dụ API, FPS, GPU, Rust, JavaScript, GitHub, macOS, Windows. Mã nguồn, lệnh shell, tên biến, URL, đường dẫn và số phiên bản được bảo vệ trước khi sửa. Không đổi mọi từ gần giống thuật ngữ bằng tìm/thay toàn cục.

Đề xuất kết hợp luật xác định được với mô hình hiểu ngữ cảnh. Dataset dùng để huấn luyện/đánh giá; glossary dùng để tra cứu trực tiếp. Bản đầu có thể dùng mô hình đa ngôn ngữ đã huấn luyện rồi đánh giá trên tiếng Việt, chưa cần tự huấn luyện từ đầu.

Có thể chạy mô hình local qua [Ollama API](https://docs.ollama.com/api/generate), gọi localhost từ Rust. Đây là phương án có thể triển khai, chưa xác minh chất lượng hay tốc độ của một mô hình cụ thể. Khi dùng mô hình cục bộ đã tải và endpoint local, nội dung sửa có thể xử lý trên máy; cần kiểm thử RAM/GPU và độ trễ thực tế. Dùng API trên mạng là lựa chọn khác, có gửi đoạn chọn ra dịch vụ và có thể phát sinh phí.

Tách hai chế độ: “Sửa chính tả” giữ nguyên ý và mức độ thân mật; “Chỉnh văn phong” cho phép viết lại theo hội thoại hoặc văn bản kỹ thuật. Không mặc định chuyển lời nói tự nhiên thành văn phong trang trọng.

## Các bước triển khai nên làm

1. Làm Windows trước: global hotkey, lấy vùng chọn, xem kết quả và thay lại có kiểm tra focus.
2. Thêm glossary và bộ sửa chạy trong worker riêng; hiển thị lỗi mạng/mô hình mà không thay đoạn gốc.
3. Kiểm thử câu có dấu, thiếu dấu, Telex, tiếng Việt trộn tiếng Anh, tên riêng, code, URL và câu không có lỗi. Đo cả lỗi sửa nhầm, giữ nguyên nghĩa và độ trễ.
4. Kiểm thử trong từng ứng dụng mục tiêu; công bố danh sách đã xác minh thay vì nói hỗ trợ mọi phần mềm.
5. Bổ sung adapter macOS/X11/Wayland sau khi xác minh quyền và hành vi thực tế.

Việc tìm hiểu này chưa thay đổi mã nguồn hoặc bản chạy của PentaKuru.
