# PentaKuru Office — kế hoạch tích hợp

Ngày: 13/10/2026. Trạng thái: MVP Writer/Impress đã triển khai bằng UNO trên tài liệu thật. Hướng dẫn và giới hạn bản chạy xem `OFFICE-MEDIA-INTEGRATION.md`; các mục dưới là yêu cầu thiết kế, có một số phần mở rộng chưa nằm trong MVP.

Cập nhật: giao diện PentaKuru được rút thành bảng điều khiển khoảng 480 × 300 px ở trạng thái thu gọn với hai tab. Tài liệu thật mở bằng cửa sổ LibreOffice Writer/Impress/Calc; AI sửa qua UNO trên đúng document đang hiển thị. Không thay trình soạn thảo LibreOffice bằng một vùng text/ảnh mô phỏng trong PentaKuru.

## Mục tiêu

Một tiện ích Office trong PentaKuru để tìm tài liệu nhanh, dán nhãn, mở nhiều phiên làm việc và cho AI sửa trực tiếp tài liệu đang active. Writer và Impress là phạm vi ưu tiên; Calc vẫn có thể bật thêm trong Settings. Không thêm phím tắt toàn hệ thống.

## Settings và vòng đời

- Office mặc định tắt. Bật lần đầu mới tải/cài runtime cần thiết, kiểm tra phiên bản và kết nối.
- Writer (Word), Impress (PowerPoint) và Calc (Excel) có lựa chọn riêng trong Settings. Đây là quyền sử dụng từng bộ thao tác; không mặc định coi chúng là các bộ cài độc lập vì LibreOffice có thành phần dùng chung.
- Khi Office chưa bật, không quét tài liệu hay khởi chạy worker Office. Các chỉ mục tìm kiếm chung đã có vẫn có vòng đời riêng.
- Khi bật, người dùng chọn thư mục nguồn; đề xuất Documents, Desktop, Downloads. Quét đệ quy các nguồn đã chọn, bỏ thư mục build/cache và không đi theo liên kết tạo vòng lặp.
- Chỉ tải runtime Office khi bật; chỉ khởi chạy tiến trình khi cần tài liệu. Tắt xử lý phiên chưa lưu, dừng worker do PentaKuru sở hữu, giữ bản cài. Gỡ tiện ích xóa runtime/cache nhưng giữ tài liệu.
- UI-TARS dùng cùng bộ quản lý tiện ích, với runtime điều khiển và mô hình cục bộ là hai gói riêng. Dùng API không cần tải trọng số mô hình cục bộ.

## Thư viện tài liệu

- Hỗ trợ lập chỉ mục .doc/.docx, .ppt/.pptx, .xls/.xlsx; bổ sung .odt/.odp/.ods khi triển khai LibreOffice.
- Bộ lọc mặc định Word; thêm Tất cả, PowerPoint, Excel, nhãn và Unknown. Tìm theo tên, đường dẫn hoặc nhãn. Chỉ hiện tài liệu Office; không hiện ứng dụng, thư mục hoặc link như Search chung.
- Lập chỉ mục không di chuyển, đổi tên hoặc gộp nội dung file. Cùng tên nhưng khác đường dẫn vẫn là hai file riêng.
- Mỗi dòng: loại tài liệu, tên file, nhãn, trạng thái active. Đường dẫn và thời gian sửa trong phần chi tiết.
- Danh sách lớn dùng phân trang/virtualization; cập nhật chỉ mục theo thay đổi thay vì quét toàn bộ mỗi lần mở.

## Dán nhãn thông minh

1. Lấy tên thư mục cha và tổ tiên có ý nghĩa cùng tên file. Ví dụ Giáo án/Ngữ văn → Giáo án, Ngữ văn.
2. Chuẩn hóa bằng quy tắc cục bộ, gom theo thư mục trước khi gọi AI để xử lý hàng loạt. Cache theo đường dẫn thư mục và phiên bản quy tắc.
3. AI trả kết quả có cấu trúc: document_id, labels, confidence, reason. Kiểm tra ID và tập nhãn trước khi lưu.
4. Tên chỉ như New folder, tmp, 123, final không đủ ngữ cảnh → Unknown. Không ép AI đoán nhãn khi thiếu căn cứ.
5. Nếu thư mục có ý nghĩa thì tên file mơ hồ vẫn có thể kế thừa nhãn thư mục. Nếu thư mục chứa nhiều loại tài liệu, chỉ kế thừa nhãn rộng; phần chi tiết không chắc đưa Unknown.
6. Nhãn lưu trong cơ sở dữ liệu PentaKuru; người dùng sửa được. Nhãn do người dùng sửa không bị ghi đè khi phân loại lại. Không gọi AI khi mỗi lần vẽ giao diện.

## Active và chuyển file

- Bấm một file: mở phiên tài liệu, thêm tab active và chuyển focus sang tab đó. Bấm lại file đã active: chuyển sang phiên hiện có, không mở bản thứ hai.
- Mỗi phiên ánh xạ document_id → UNO document → controller/frame LibreOffice. Bấm tab active gọi activate trên frame tương ứng để đưa đúng cửa sổ tài liệu lên; không dùng tên cửa sổ để suy đoán file.
- Active là tập tài liệu có phiên làm việc đang mở; focused là tài liệu người dùng đang xem; AI target là phạm vi thao tác.
- Tab có tên file, trạng thái chưa lưu và nút đóng. Nhiều tab có vùng cuộn trong ứng dụng thật.
- AI mặc định sửa file đang xem. Có lựa chọn rõ ràng Tất cả file active; snapshot danh sách ID trước khi chạy, không tự thêm file mới mở vào tác vụ đang chạy.
- Không coi file chỉ xuất hiện trong kết quả tìm kiếm là tài liệu được phép sửa.
- Khi đóng file hoặc tắt tiện ích mà có thay đổi chưa lưu: Lưu / Bỏ thay đổi / Hủy. Đổi bộ lọc không đóng phiên active.

## Core AI

- Writer: sửa chính tả; nâng cấp bài viết giữ ý chính; đổi font, cỡ chữ và paragraph style; căn đoạn; chỉnh tỷ lệ, kích thước, neo và bố trí ảnh.
- Impress: sửa/nâng cấp nội dung trong text shape; đổi kiểu chữ; bố trí ảnh và đối tượng trên slide; xử lý slide hiện tại hoặc các slide được chỉ định.
- Calc: bật thêm khi cần; chỉnh ô, hàng/cột và hình ảnh; bảo toàn công thức khi tác vụ chỉ yêu cầu bố cục.
- Giao diện có các thao tác nhanh và một ô yêu cầu. Định dạng được hiển thị rõ cho người dùng: Word → Writer, PowerPoint → Impress, Excel → Calc.

Luồng thực thi: đọc tài liệu bằng UNO → xác định vùng/đối tượng và phiên bản → AI lập thao tác có cấu trúc → validate → thực thi lên document_id cụ thể → đọc lại/render để kiểm tra → ghi lịch sử và lưu.

Bridge chạy LibreOffice ở chế độ có giao diện, không headless; loadComponentFromURL với Hidden=false và ReadOnly=false khi có quyền ghi. Document có controller và frame thật. AI chỉnh cùng mô hình tài liệu với controller để nội dung, style và vị trí ảnh cập nhật ngay trong Writer/Impress. Không sửa một bản sao headless rồi ghi đè file đang mở. Nếu tài liệu mở trước trong LibreOffice khác profile/instance, bridge phải xác định instance và kết nối được trước khi nhận phiên; không tự mở thêm bản ghi thứ hai. Kết nối UNO qua IPC cục bộ chỉ dành cho worker do PentaKuru quản lý.

- Bộ thực thi nhận các thao tác được cho phép, không nhận shell code tùy ý từ AI. Nội dung tài liệu là dữ liệu, không phải chỉ dẫn điều khiển ứng dụng.
- Sửa nội dung bằng range/đối tượng, giữ cấu trúc và định dạng cần thiết; tránh lấy toàn bộ tài liệu thành text rồi ghi lại làm mất hình/bảng/style.
- Undo nhóm thao tác theo tài liệu và tác vụ; lưu bản sao trước khi ghi đè. Với nhiều tài liệu báo kết quả theo từng file, lỗi ở một file không được báo thành công cho toàn bộ.
- Nếu tài liệu bị sửa ngoài phiên AI, kiểm tra revision và tính lại thao tác. File đang khóa/read-only hiển thị trạng thái và không báo đã lưu.
- Mở/lưu .docx/.pptx/.xlsx bằng bộ lọc đúng định dạng. Kiểm tra vòng mở–sửa–lưu–mở lại cho nội dung, ảnh, style và bố cục; LibreOffice và Microsoft Office không bảo đảm hiển thị giống tuyệt đối.

## Giao diện gọn

Một bảng điều khiển nhỏ mở bằng mục trong Settings hoặc menu PentaKuru, không có phím tắt mới. Trạng thái thu gọn chỉ có tên tiện ích, nút Tài liệu/Settings, tab active, lựa chọn thao tác/phạm vi và ô yêu cầu. Thư viện và Settings mở thêm khi bấm. Chọn file xong thư viện thu lại; file mở trong LibreOffice, bảng điều khiển không chiếm vùng tài liệu. Tab active dùng nhãn ngắn, đường dẫn đầy đủ trong tooltip/chi tiết. Font, kích thước, vùng xử lý được chọn trong thao tác tương ứng.

Bản xem thử đi kèm dùng dữ liệu mẫu để khám phá luồng bảng điều khiển, không mở LibreOffice hay sửa file thật. Nội dung trực quan thuộc cửa sổ LibreOffice thật trong bản triển khai; bảng điều khiển chỉ thông báo phiên và thao tác. Không còn vùng tài liệu giả trong bản xem thử.

## Tích hợp với mã hiện tại

- src/main.rs: thêm enable_office và OfficeSettings với serde(default), trạng thái panel và phiên làm việc. Không thêm action/binding vào hotkeys.
- src/search.rs: tái dùng các tiện ích metadata khi phù hợp; index riêng cho Office. Search hiện có giới hạn độ sâu 3 và ngưỡng số lượng, không đủ cho yêu cầu gom tài liệu từ nguồn đã chọn.
- src/office/index.rs: metadata, tìm kiếm, bộ lọc, theo dõi file; lưu chỉ mục và nhãn.
- src/office/labels.rs: quy tắc thư mục, phân loại AI theo lô, Unknown, nhãn thủ công.
- src/office/session.rs: active/focused/revision/dirty state và vòng đời tài liệu.
- src/office/bridge.rs và worker UNO: kết nối runtime, mở file và điều khiển tài liệu theo ID. Python bridge là ứng viên thử nghiệm, phải đóng gói runtime tương thích và kiểm chứng trước.
- src/office/actions.rs: schema thao tác Writer/Impress/Calc, validation, undo và kết quả từng file.
- src/office_panel.rs: cửa sổ compact theo theme egui hiện tại.
- src/ai.rs: tái dùng provider/model, thêm đường xử lý thao tác Office; không chạy vòng click chuột cho các sửa đổi có UNO API.
- Bộ quản lý tiện ích chung: tải/cài/cập nhật/gỡ, checksum, dependencies, tiến trình sở hữu. Không gỡ LibreOffice do người dùng cài độc lập.

## Các mốc thực hiện

1. Chốt luồng với bản xem thử: Settings, lọc tài liệu, nhãn/Unknown, active tabs, phạm vi AI.
2. Bộ quản lý tiện ích và kiểm chứng bridge có giao diện: mở đồng thời một DOCX và PPTX trong LibreOffice thật, chuyển frame bằng tab PentaKuru, chỉnh trực tiếp trên document đang hiển thị, undo và lưu đúng định dạng. Không cần nhúng editor vào PentaKuru để đáp ứng luồng này.
3. Thư viện thật: index nguồn đã chọn, phân loại nhãn, tìm kiếm; bật/tắt đúng vòng đời.
4. Writer và Impress: sửa chính tả, cải thiện nội dung, kiểu chữ, hình ảnh theo phiên active. Có undo và báo cáo từng file.
5. Calc tùy chọn, mở rộng Draw/Base/Math theo từng adapter, tối ưu tài nguyên và đóng gói.

## Nghiệm thu

- Tắt Office: không tải/cài/khởi chạy worker Office.
- Writer và Impress có trong Settings; Calc có thể bật thêm. Không đăng ký phím tắt Office.
- Gom được các loại file yêu cầu trong nguồn đã chọn, phân biệt hai file cùng tên khác đường dẫn.
- Unknown lọc được; tên thư mục rõ tạo nhãn đúng; phân loại lại giữ nhãn thủ công.
- Bấm 3 file tạo 3 phiên; bấm lại không nhân đôi; đổi filter vẫn giữ phiên; đóng file chưa lưu xử lý đầy đủ.
- Sửa file đang xem không ảnh hưởng tab khác; tác vụ nhiều file chỉ đụng snapshot active được chọn.
- Người dùng thấy sửa nội dung, style và ảnh cập nhật ngay trong cửa sổ Writer/Impress đang mở; tab PentaKuru chuyển đúng frame, không mở lại bản thứ hai. PentaKuru thu gọn được khi thao tác tài liệu.
- Sau mở–sửa–lưu–mở lại, DOCX/PPTX giữ nội dung, ảnh và style trong bộ mẫu; bố cục được so sánh bằng render.
- Undo, file khóa, revision conflict, tải/cài thất bại và gián đoạn được báo đúng; không báo thành công giả.

## Tham khảo chính thức

- https://api.libreoffice.org/
- https://api.libreoffice.org/examples/DevelopersGuide/examples.html
- https://help.libreoffice.org/latest/en-US/text/shared/guide/ms_user.html
- https://help.libreoffice.org/latest/en-US/text/shared/guide/ms_import_export_limitations.html
- https://github.com/bytedance/UI-TARS-desktop
