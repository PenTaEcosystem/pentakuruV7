# PentaKuru

Windows, Linux và macOS: tải gói của hệ điều hành trong **Actions → Build desktop apps → Artifacts** sau lượt chạy thành công. [Phạm vi chức năng từng nền tảng](docs/PLATFORM-SUPPORT.md).

Bạn đồng hành trên desktop, viết bằng **Rust**. Bản tích hợp Sally đầy đủ dùng giấy phép GPL-3.0; thông báo MIT của phần PentaKuru gốc được giữ trong `LICENSE-PENTEYE-MIT`.

## Bộ cài EXE

Chạy `dist/PentaKuru-Setup.exe` để cài từng bước, chọn thư mục và tiện ích. Bộ cài màu tối, các bước chính tiếng Việt, tạo shortcut và hỗ trợ gỡ cài đặt. Tùy chọn sẽ được áp dụng vào PentaKuru; vẫn có thể thay đổi trong Settings. [Hướng dẫn bộ cài](docs/INSTALLER.md).

## Office · DeepTutor · Music · Clip

Mở `dist/PentaKuru-Office-Media.exe`, vào **Alt+Q → Settings** để bật từng tiện ích. Office tìm tài liệu, dán nhãn theo thư mục, giữ nhiều file active và chỉnh trực tiếp Writer/Impress qua UNO; có sửa chính tả, nâng cấp bài viết, kiểu chữ, căn ảnh, lưu và hoàn tác. Penta Music/Penta Video dùng trực tiếp trang web, giữ đăng nhập, playlist và tìm kiếm gốc; bố cục tối ưu khung nhỏ. Settings cho chọn 0–5 icon riêng, mặc định 4; chức năng không có icon vẫn ở menu chuột phải. DeepTutor HKUDS tải khi bật và dùng cấu hình AI chung khi được chọn.

Hướng dẫn sử dụng, phụ thuộc runtime, phạm vi hỗ trợ và kiểm chứng: [docs/OFFICE-MEDIA-INTEGRATION.md](docs/OFFICE-MEDIA-INTEGRATION.md). Các tiện ích mới mặc định tắt; không thêm phím tắt. Bản này ưu tiên Writer/Impress; Calc mở tài liệu. DeepTutor có nút cài/mở, tự chuẩn bị Python/Node riêng; embedding được cấu hình riêng. EXE, bộ cài và cửa sổ dùng icon ngũ giác đen viền trắng. [Hướng dẫn DeepTutor](docs/DEEPTUTOR-ONECLICK.md).

## Đồng hồ Bip Max: chat và lệnh Windows

Bản `PentaKuru-Office-Media.exe` đi với `PentaKuru-Watch-1.2.3.zab`: trên đồng hồ bỏ ngũ giác, dùng khung chat cuộn và bốn icon Ghi chú/Tìm kiếm/AI/Windows, hai icon bàn phím/mic và bàn phím tích hợp không phụ thuộc bàn phím hệ thống. Phản hồi cũ giữ nguyên đến khi phản hồi mới thành công. Windows gửi mục tiêu bằng chữ hoặc mic vào UI Agent PC và có nút Dừng. Ghép mã tại **Alt+Q → Đồng hồ Amazfit → Sao chép mã**, rồi dán vào cài đặt ứng dụng trong Zepp. Hướng dẫn cài đặt và build: [vendor/penta-bip/README.md](vendor/penta-bip/README.md).

## PentaKuru + Sally đầy đủ

Mở `dist/PentaKuru-Office-Media.exe`, rồi nhấn **Alt + T**. Giao diện React gốc và backend Rust của Sally nằm trong cùng ứng dụng, gồm dịch trực tiếp, chọn nguồn âm thanh, cài đặt, lịch sử, nghe lại và xử lý biên bản. Không cần chạy Sally.exe riêng. WebView2 Runtime phải có trên máy.

Nút × ẩn cửa sổ Sally; cuộc họp đang chạy vẫn tiếp tục. Bấm kết thúc trong Sally để dừng cuộc họp. Alt + T mở lại cửa sổ. Chi tiết biên dịch và lưu dữ liệu: [docs/SALLY-INTEGRATION.md](docs/SALLY-INTEGRATION.md).

Khối 3D gồm **12 mặt ngũ giác đều**, thân tô **đen**, cạnh trắng phát sáng nhẹ. Không có đường chia nhỏ kiểu Rubik. **Chỉ vùng ngoài khối có nền trong suốt** để thấy desktop phía sau; phần bên trong vẫn đen theo thiết kế. Mắt là viền tròn trắng không đồng tử, nhìn nhẹ theo chuột, chuyển động trong khối và chớp mắt mỗi khoảng 5–9 giây.

Khối quay đủ 360° theo giờ hệ thống cục bộ:

- Trục nhìn chính Z theo kim giây: một vòng mỗi 60 giây, chiều **3 giờ → 6 giờ → 9 giờ → 12 giờ**.
- Trục chiều sâu Y quay một vòng mỗi 120 giây để các mặt ngũ giác lần lượt đổi hướng. Tốc độ khác trục Z giúp hai chuyển động không triệt tiêu nhau quanh 3 giờ và 15 giờ.
- Trục chiều sâu X theo kim giờ: một vòng mỗi 12 giờ.

Góc lấy trực tiếp từ đồng hồ hệ điều hành, có phần giây lẻ, nên không tích lũy sai lệch theo FPS. Mỗi lần mở, ứng dụng tự quay ngay. Animation 3D luôn được bật mặc định theo thời gian thực.

---

## Tính năng thông minh & Phím tắt toàn hệ thống (Global Hotkeys)

Ứng dụng lắng nghe phím tắt toàn cục trên Windows, hoạt động ngay cả khi bạn đang làm việc trên Word, Excel, Visual Studio Code, Trình duyệt web hay màn hình Desktop:

| Phím tắt | Tính năng | Chi tiết thao tác |
| :--- | :--- | :--- |
| **Alt + Z** | **Sửa lỗi chính tả siêu tốc** | Bôi đen (tô đen) đoạn văn bản cần sửa rồi bấm `Alt + Z`. Hệ thống tự động: <br>• Chuẩn hóa vị trí dấu thanh tiếng Việt kiểu mới (`hòa` thay vì `hoà`).<br>• Tự động sửa lỗi telex dính phím, gõ sai (`thược` → `thực`, `ngiên cứu` → `nghiên cứu`).<br>• Tự động viết đúng thuật ngữ công nghệ (`javascript` → `JavaScript`, `chatgpt` → `ChatGPT`).<br>• Nếu đã cấu hình API Key, AI sẽ đọc ngữ cảnh câu để sửa lỗi nâng cao.<br>• Hộp thoại đối chiếu Before/After xuất hiện cho phép sao chép hoặc tự động dán đè thay thế (`SendInput` Ctrl+V). |
| **Alt + Q** | **Cài đặt & Đổi phím tắt nhanh** | Mở bảng Cài đặt hệ thống: <br>• Bấm nút phím tắt để gán lại tổ hợp phím mới tức thì (`RegisterHotKey` live update).<br>• Nhập API Key: Tự động nhận diện nhà cung cấp (Gemini, OpenAI, Claude, OpenRouter, Groq, DeepSeek, Ollama) và **tự động tải danh sách Model** về combobox lựa chọn. |
| **Alt + S** | **Tìm kiếm hệ thống (Search Dock)** | Mở thanh tìm kiếm nằm **ngay dưới chân mô hình 3D**: <br>• Quét tức thì các tài liệu, thư mục và đường dẫn tại Desktop, Documents, Downloads và Penteye Notes.<br>• Phân loại biểu tượng trực quan (File, Thư mục, Đường dẫn Web, Ghi chú).<br>• Nhập từ khóa để hiện kết quả; bấm kết quả hoặc dùng ↑/↓ và Enter để mở. Nút … mở vị trí tệp. |
| **Alt + N** | **Quản lý Ghi chú (Notes Window)** | Mở trung tâm ghi chú chuyên nghiệp: <br>• Phân chia theo Thư mục/Chuyên mục (Chung, Dự án & Lập trình, Giáo án & Bài giảng, Ý tưởng...).<br>• Hiển thị rõ ràng **ngày tháng năm & giờ phút tạo/sửa đổi**.<br>• **Hỗ trợ tải file / đính kèm tài liệu**: Nút "Tải file lên" và tính năng kéo thả trực tiếp (Drag & Drop).<br>• Tự động tạo thư mục `Penteye Notes` nằm cùng cấp kế bên file `.exe`. |
| **Alt + A** | **Trợ lý AI (AI Prompt Dock)** | Nút bên phải trên thanh công cụ là biểu tượng AI Sparkle (4 cánh). Bấm nút hoặc nhấn `Alt + A` sẽ mở khung nhập Prompt nằm **ngay dưới chân mô hình 3D**, hiển thị lịch sử trả lời có thể chọn văn bản để copy. |
| **Alt + T** | **Sally đầy đủ** | Mở giao diện Sally gốc được nhúng trong cùng tiến trình PentaKuru. |

---

## Điều khiển mô hình 3D trên Desktop

| Thao tác | Chức năng |
| :--- | :--- |
| **Kéo chuột trái trên khối** | Di chuyển vị trí mô hình tự do trên màn hình desktop |
| **Nút Ghi chú (Trái)** | Mở / đóng cửa sổ Quản lý Ghi chú (`Alt + N`) |
| **Nút AI (Phải)** | Bật / tắt thanh nhập AI Prompt Dock dưới mô hình (`Alt + A`) |
| **Nút Kính lúp (Giữa)** | Bật / tắt thanh Tìm kiếm Search Dock dưới mô hình (`Alt + S`) |
| **Chuột phải trên khối** | Menu ngữ cảnh: Ghi chú, Tìm kiếm, AI, Cài đặt, Thoát |
| **Kéo góc dưới phải / Lăn chuột** | Thu / phóng kích thước cửa sổ và khối đồng bộ |
| **Esc** | Đóng nhanh các thanh dock và popup |

---

## Kế hoạch sắp xếp tài liệu & dự án

Nhằm giải quyết triệt để vấn đề tài liệu giáo án của giáo viên và thư mục dự án của lập trình viên bị phân tán rải rác trên máy tính, dự án đã xây dựng bản kế hoạch chi tiết tại:
👉 **[`KE-HOACH-SAP-XEP-DU-AN.md`](KE-HOACH-SAP-XEP-DU-AN.md)**

Bản kế hoạch bao gồm:
1. **Phân tích nguyên nhân gốc rễ**: Tại sao Desktop và thư mục Downloads luôn trở thành "bãi rác" lưu trữ.
2. **Cấu trúc cây thư mục chuẩn hóa**:
   - Dành cho **Giáo viên**: Phân cấp theo Năm học / Học kỳ / Môn học / Khối lớp / Soạn giảng / Đề thi / Đính kèm.
   - Dành cho **Lập trình viên**: Cấu trúc `Projects/Active`, `Projects/Archive`, phân tách `Docs`, `Workspace`.
3. **Cơ chế dọn dẹp rác build lập trình**: Giải pháp quét và dọn tự động `node_modules`, `target`, `.gradle`, `venv`, `bin/obj` giải phóng hàng chục GB dung lượng.
4. **Tích hợp sâu vào PentaKuru**: Cách tận dụng tính năng Alt+S (Search Dock) và Alt+N (Notes + File Upload) để quản lý công việc hàng ngày một cách ngăn nắp nhất.

---

## Giao diện Studio

- Ghi chú có thanh tiêu đề riêng, thư viện bên trái, vùng soạn thảo lớn bên phải, thời điểm lưu và phần đính kèm thu gọn. Đóng bằng nút × hoặc Esc; kéo thanh tiêu đề để di chuyển.
- Font Noto Sans được nhúng vào ứng dụng để hiển thị đầy đủ tiếng Việt. Giấy phép font ở `assets/fonts/OFL.txt`.
- Tìm kiếm chỉ mở một thanh 540 × 64 logical pixels khi chưa nhập. Có từ khóa mới hiện tối đa 5 kết quả và tăng chiều cao; xóa từ khóa sẽ thu lại. Dùng ↑/↓, Enter và Esc.
- AI bắt đầu bằng thanh nhập nhỏ gọn. Enter gửi, Shift+Enter xuống dòng; nội dung nhập dài cuộn bên trong ô, không đẩy nút gửi ra ngoài. Chỉ mở vùng trả lời khi có hội thoại hoặc tác vụ. Nút + tạo hội thoại mới và thu về thanh nhập.
- Chuột phải trong khung chat để sao chép hội thoại, tạo hội thoại mới, mở cài đặt AI hoặc đóng. Chuột phải trên tin nhắn để sao chép tin nhắn; trong ô nhập để sao chép, dán hoặc xóa nội dung. Menu có đủ không gian ngay cả khi thanh chat đang thu gọn. Esc đóng menu trước, sau đó dừng tác vụ hoặc đóng chat.
- `@ui mục tiêu` bật điều khiển màn hình; biểu tượng máy ảnh cho biết chế độ này. Tiến trình nằm trong vùng cuộn, số bước và nút dừng luôn hiển thị. Thao tác màn hình chạy trên luồng nền; dừng tác vụ sẽ bỏ kết quả đang chờ và không tiếp tục bước kế tiếp. Một thao tác đã bắt đầu có thể hoàn tất trước khi dừng.
- `@ui` không giới hạn số bước: tiếp tục cho đến khi mô hình báo kết thúc sau khi kiểm tra kết quả, bạn bấm Dừng/Esc hoặc xảy ra lỗi. Tiến trình hiển thị số bước hiện tại, không có mốc dừng 15 bước.
- Menu ngữ cảnh chỉ xử lý chuột phải, để ô nhập, chọn văn bản và các nút vẫn nhận chuột trái. Khi tác vụ hoàn tất, dừng hoặc lỗi, chat nhận lại focus để tiếp tục nhập.
- Chụp màn hình và thao tác `@ui` không ẩn/hiện các cửa sổ PentaKuru ở từng bước. Cửa sổ được loại khỏi ảnh chụp bằng capture affinity; trạng thái cho chuột đi qua được khôi phục sau thao tác. Ngũ giác tiếp tục được vẽ trong lúc tác vụ chạy.
- Bộ đọc Markdown xử lý dấu `*`, `_` và backtick chưa đóng như văn bản thường, tránh lặp vô hạn; giữ nguyên dấu gạch dưới trong tên tệp và đường dẫn.
- Khi tác vụ AI thực sự đang chạy, viền khối 3D sáng nhẹ theo nhịp. Khi hoàn tất hoặc lỗi, ánh sáng giảm dần về mức bình thường. Hiệu ứng hoạt động cả khi tạm dừng xoay.

Bản Studio giữ thư mục `Penteye Notes` cạnh file exe để dùng lại ghi chú hiện có khi các bản chạy nằm cùng thư mục. Ảnh kiểm tra dùng dữ liệu mẫu trong `.build/ui-notes`, không lấy ghi chú cá nhân và không gọi API.

Các lệnh kiểm tra giao diện native:

```powershell
./target/release/pentakuru.exe --ui-preview notes --screenshot .build/notes.png
./target/release/pentakuru.exe --ui-preview notes-small --screenshot .build/notes-small.png
./target/release/pentakuru.exe --ui-preview search --screenshot .build/search.png
./target/release/pentakuru.exe --ui-preview search-results --screenshot .build/search-results.png
./target/release/pentakuru.exe --ui-preview ai --screenshot .build/ai.png
./target/release/pentakuru.exe --ui-preview ai-chat --screenshot .build/ai-chat.png
./target/release/pentakuru.exe --ui-preview ai-long --screenshot .build/ai-long.png
./target/release/pentakuru.exe --ui-preview ai-agent --window-size 320 360 --screenshot .build/ai-agent.png
./target/release/pentakuru.exe --ui-preview ai-menu-compact --screenshot .build/ai-menu.png
./target/release/pentakuru.exe --ui-preview ai-error --screenshot .build/ai-error.png
./target/release/pentakuru.exe --ui-preview thinking --screenshot .build/thinking.png
```

`--ui-preview` tự bật smoke test, dùng cùng mã vẽ của cửa sổ tương ứng trong viewport native kiểm tra, chụp frame OpenGL rồi đóng ứng dụng. Các nội dung trò chuyện trong preview là mẫu minh họa; preview thinking dùng tác vụ cục bộ giả lập để kiểm tra trạng thái xử lý, không gọi dịch vụ AI. Cần tạo `.build` nếu chạy các lệnh này ngoài repository.

## Chạy bản Windows

Mở `dist/PentaKuru-Office-Media.exe` hoặc giải nén `dist/PentaKuru-Office-Media-Windows-x64.zip`. Đóng các bản Penta và Sally cũ trước khi chạy bản mới, rồi bấm Alt + T để mở Sally đầy đủ. Không cần cài Rust hoặc Node.js để chạy. Yêu cầu GPU/driver hỗ trợ OpenGL và WebView2 Runtime. Mã nguồn tương ứng nằm trong `dist/PentaKuru-Office-Media-Source.zip`.

Khi khởi chạy lần đầu, ứng dụng sẽ tự động khởi tạo thư mục dữ liệu `Penteye Notes` ngay cạnh file thực thi.

## Nhịp vẽ và hiệu năng

Ứng dụng đặt mục tiêu **60 FPS**, bật VSync và có bộ giới hạn frame độc lập. Gọi API AI, sửa chính tả và quét chỉ mục tìm kiếm chạy ở luồng nền để giữ giao diện 3D phản hồi tốt. FPS và độ trễ thực tế phụ thuộc máy, driver và tác vụ đang chạy; cần kiểm chứng thay vì cam kết luôn đạt đúng 60 FPS.

`--motion-check --no-top` chụp hai frame native cách nhau khoảng 3 giây, ghi góc quay, độ dịch chuyển thực tế của các đỉnh chiếu lên màn hình và FPS vào `.build/motion-report.txt`. Thêm `--motion-panel notes`, `search`, `ai` hoặc `ui` để kiểm tra mô hình khi cửa sổ phụ đang mở; `ui` giả lập tác vụ điều khiển đang chạy. Các lệnh này dùng dữ liệu smoke test riêng, không gọi API. Test hình học quét cả 24 giờ, gồm đúng 03:00 và 15:00, để phát hiện trường hợp góc thay đổi nhưng khối nhìn như đứng yên.

`--window-state-check` kiểm tra hai cửa sổ native qua sáu chu kỳ chụp màn hình và bật/tắt trạng thái cho chuột đi qua, ghi kết quả vào `.build/window-state-check.txt`. Kiểm tra xác nhận cửa sổ còn hiện, được bật, giữ vị trí và khôi phục window style. Không gửi click hay phím tới desktop, không lưu ảnh màn hình và không gọi API.

## Biên dịch từ mã nguồn

Yêu cầu **Rust 1.95+**, Node.js/npm và Visual Studio Build Tools (C++):

```sh
cd vendor/Sally
npm ci
npm run build
cd ../..
cargo build --release --locked
```

Chạy script đóng gói:
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package.ps1
```

## Kiểm tra chất lượng mã nguồn

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Xem `LICENSE` và `THIRD-PARTY-NOTICES.md` để sử dụng, chỉnh sửa và phân phối bản hợp nhất.

## Đồng hồ PentaKuru qua Zepp

Mã đồng hồ nằm trong `vendor/penta-bip`, target hiện tại Amazfit Bip Max 432×514. Đồng hồ gửi qua BLE tới Zepp trên điện thoại, rồi điện thoại gọi máy chủ Penta cổng 3899 qua Wi-Fi/Tailscale. Trong Zepp có cài đặt URL PC; mic chỉ báo đang thu sau sự kiện START thành công, gửi sau STOP, kết quả có thể vuốt lên/xuống và được giữ lại. Nhận diện tiếng nói hiện dùng Gemini trong cài đặt AI của Penta. Xem `vendor/penta-bip/README.md` để cài, thử và build gói `.zab`.

## Đổi tên từ PentaEye

Tên sản phẩm và bộ cài là PentaKuru; binary Rust là `pentakuru.exe`. Các định danh lưu trữ cũ (`Penteye`, `Penteye Notes`, `com.penteye.studio` và profile DeepTutor) được giữ để đọc lại cấu hình, ghi chú và đăng nhập đã có. AppId bộ cài giữ nguyên để nâng cấp tại thư mục cũ. Thông báo MIT gốc được giữ nguyên trong `LICENSE-PENTEYE-MIT`. Không đưa API key, dữ liệu cá nhân hoặc runtime đã tải vào Git.
