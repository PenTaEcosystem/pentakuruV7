# KẾ HOẠCH HỆ THỐNG GIẢI QUYẾT PHÂN TÁN DỰ ÁN VÀ TÀI LIỆU
## Dành Cho Máy Tính Giáo Viên Và Lập Trình Viên

---

## 1. BỐI CẢNH & PHÂN TÍCH THỰC TRẠNG

### 1.1. Thực trạng máy tính Giáo viên (Educators)
- **Tập tin phân tán khắp nơi**: Tệp giáo án, bài giảng, đề thi, slide thường lưu rải rác trên **Desktop**, **Downloads**, thư mục Zalo/Telegram nhận file, USB và nhiều ổ đĩa khác nhau (`C:\`, `D:\`, `E:\`).
- **Hỗn loạn phiên bản**: Rất nhiều tệp trùng lặp hoặc đặt tên không nhất quán, ví dụ:
  - `Giao_an_Toan_10.docx`
  - `Giao_an_Toan_10_chuan.docx`
  - `Giao_an_Toan_10_final_sua_lan_2.docx`
  - `De_kiem_tra_1_tiet_moi_nhat.pdf`
- **Định dạng đa dạng nhưng thiếu liên kết**:
  - Soạn giảng: `.docx`, `.doc`, `.odt`
  - Trình chiếu: `.pptx`, `.ppt`, `.pdf`
  - Đánh giá & quản lý: `.xlsx`, `.csv` (bảng điểm, danh sách học sinh)
  - Đa phương tiện: `.mp4`, `.mp3`, `.png`, `.jpg` (ảnh bài tập, video thí nghiệm)
- **Hệ quả**: Mất hàng chục phút mỗi lần chuẩn bị hồ sơ thanh tra hoặc tìm lại đề thi cũ; nguy cơ nhầm lẫn đề kiểm tra giữa các năm học.

---

### 1.2. Thực trạng máy tính Lập trình viên (Developers)
- **Dự án nằm rải rác không kiểm soát**:
  - Các dự án nằm rải rác trong `Desktop/test`, `Downloads/repo-master`, `D:/workspace`, `C:/Users/.../source/repos`, `E:/client-projects`.
- **Rác build và thư viện chiếm dung lượng khổng lồ**:
  - `node_modules/` (Node.js/React/Vue/Next.js) – mỗi dự án từ 300MB đến 1.5GB.
  - `target/` (Rust Cargo) – mỗi dự án từ 500MB đến 10GB.
  - `.venv/`, `__pycache__/` (Python) – môi trường ảo trùng lặp khắp nơi.
  - `.gradle/`, `.m2/`, `bin/`, `obj/` (Java/C#/.NET).
  - Kết quả: Ổ cứng SSD báo đỏ (90-99%), dung lượng hao hụt vô ích từ 50GB – 250GB.
- **Rủi ro mất dữ liệu Git (Git Hygiene)**:
  - Nhiều dự án tạo ra để thử nghiệm (POC/Prototype) nhưng có chứa code quan trọng, đang ở trạng thái **uncommitted changes** hoặc **unpushed commits** mà quên bẵng đi, dẫn đến nguy cơ xóa nhầm khi dọn máy.

---

## 2. MỤC TIÊU & NGUYÊN TẮC THIẾT KẾ HỆ THỐNG

| Tiêu chí | Mô tả mục tiêu |
| :--- | :--- |
| **Tự động hóa cao** | Quét tự động toàn bộ ổ đĩa, nhận diện đúng loại dự án/tài liệu trong 3–5 giây. |
| **Tuyệt đối an toàn (Zero Data Loss)** | Không bao giờ xóa vĩnh viễn tệp gốc của người dùng. Mọi hành động di chuyển đều có **Nhật ký hoàn tác (Undo History)**. |
| **Không làm hỏng đường dẫn cũ** | Hỗ trợ Windows **Directory Junctions / Symlinks** để các phần mềm cũ, script, shortcut vẫn mở được bình thường. |
| **Siêu nhẹ & Chạy ngầm mượt mà** | Tích hợp trực tiếp vào **Penteye** (viết bằng Rust, chiếm < 30MB RAM, 60 FPS không tụt nhịp). |

---

## 3. KIẾN TRÚC GIẢI PHÁP CHI TIẾT

```
   ┌─────────────────────────────────────────────────────────────┐
   │                  PENTEYE SMART ORGANIZER                    │
   └─────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┴───────────────────────┐
        ▼                                               ▼
┌──────────────────────────────┐        ┌──────────────────────────────┐
│     MODULE CHO GIÁO VIÊN     │        │  MODULE CHO LẬP TRÌNH VIÊN   │
├──────────────────────────────┤        ├──────────────────────────────┤
│ 1. Quét tài liệu giảng dạy   │        │ 1. Quét Repo Git & Project   │
│ 2. Phân loại Khối/Môn/Kỳ     │        │ 2. Kiểm tra Git Clean/Dirty  │
│ 3. Gom phiên bản trùng lặp   │        │ 3. Dọn sạch rác build 50GB+  │
│ 4. Lưu vào Penteye Notes     │        │ 4. Quy hoạch Workspace chuẩn │
└──────────────────────────────┘        └──────────────────────────────┘
                                │
                                ▼
   ┌─────────────────────────────────────────────────────────────┐
   │  CÔNG CỤ THI HÀNH AN TOÀN (SAFE EXECUTION & UNDO ENGINE)   │
   └─────────────────────────────────────────────────────────────┘
```

---

### 3.1. Phân hệ dành riêng cho Giáo viên (Teaching Document Organizer)

#### A. Cấu trúc cây thư mục chuẩn đề xuất:
Hệ thống sẽ gom toàn bộ tài liệu về một thư mục mẹ thống nhất (ví dụ: `D:\Tai_Lieu_Giang_Day\` hoặc ngay trong thư mục `Penteye Notes`):
```text
Tai_Lieu_Giang_Day/
├── 2025-2026/
│   ├── Khoi_10/
│   │   ├── Mon_Toan/
│   │   │   ├── 01_Giao_An/         (Kế hoạch bài dạy .docx)
│   │   │   ├── 02_Bai_Giang_Slide/ (Bài trình chiếu .pptx)
│   │   │   ├── 03_De_Kiem_Tra/     (Đề thi, ma trận, đáp án .pdf/.docx)
│   │   │   └── 04_Tu_Lieu/         (Hình ảnh, video minh họa)
│   │   └── Mon_Tin_Hoc/
│   └── Khoi_11/
└── Ho_So_Chuyen_Mon/               (Sổ điểm, biên bản, kế hoạch tổ chuyên môn)
```

#### B. Cơ chế nhận diện tự động:
1. **Phân loại theo từ khóa tên tệp**:
   - `đề thi`, `kiểm tra`, `15 phút`, `1 tiết`, `học kỳ` $\rightarrow$ Thư mục `De_Kiem_Tra/`
   - `giáo án`, `kế hoạch bài dạy`, `KHBD`, `bài soạn` $\rightarrow$ Thư mục `Giao_An/`
   - `bài giảng`, `slide`, `trình chiếu`, `.pptx`, `.ppsx` $\rightarrow$ Thư mục `Bai_Giang_Slide/`
2. **Gom cụm phiên bản (Smart Version Deduplication)**:
   - Thuật toán so khớp chuỗi (Levenshtein Distance) để gom các file cùng họ:
     `GiaoAn_Bai1.docx`, `GiaoAn_Bai1_sua.docx`, `GiaoAn_Bai1_final.docx`.
   - Giữ lại tệp có thời gian sửa đổi mới nhất làm tệp chính, chuyển các tệp cũ vào thư mục con `_Lich_Su_Phien_Ban/` thay vì để tràn ngập màn hình.
3. **Tích hợp tìm kiếm nhanh Penteye (Alt + S)**:
   - Giáo viên chỉ cần bấm `Alt + S`, gõ `Toán 10 hàm số` $\rightarrow$ Hệ thống lập tức hiển thị file giáo án, slide và đề thi tương ứng để mở trong 1 giây.

---

### 3.2. Phân hệ dành riêng cho Lập trình viên (Developer Project Workspace)

#### A. Nhận diện dự án tự động (Project Detector):
Hệ thống quét ổ đĩa dựa trên các "dấu hiệu nhận biết" của từng ngôn ngữ/nền tảng:
- **Rust**: Có tệp `Cargo.toml`
- **Node.js / Web**: Có tệp `package.json`
- **Python**: Có `requirements.txt`, `pyproject.toml`, hoặc `Pipfile`
- **Java / Kotlin**: Có `pom.xml` hoặc `build.gradle`
- **C / C++**: Có `CMakeLists.txt` hoặc `Makefile`
- **Go**: Có `go.mod`
- **Mọi dự án Git**: Có thư mục ẩn `.git/`

#### B. Cấu trúc Workspace chuẩn hóa:
```text
D:\Workspace/ (hoặc ~/Projects/)
├── Web/
│   ├── my-frontend/
│   └── company-portal/
├── Rust/
│   ├── penteye/
│   └── desktop-tools/
├── Python_AI/
│   └── dataset-pipeline/
└── Archive_Old_Projects/      (Các dự án đã dừng trên 6 tháng)
```

#### C. Công cụ dọn rác Build thông minh (Smart Build Pruner):
Hệ thống hiển thị bảng thống kê dung lượng rác và cho phép 1-click giải phóng:
- **Xóa an toàn**:
  - `node_modules/` (có thể tải lại bất cứ lúc nào qua `npm install` hoặc `pnpm install`)
  - `target/` của Rust (tạo lại khi `cargo build`)
  - `__pycache__/`, `.pytest_cache/`
  - `build/`, `dist/`, `.next/`, `.nuxt/`
- **Quy tắc an toàn**: Chỉ đề xuất dọn dẹp các thư mục build của những dự án không mở trong 14 hoặc 30 ngày qua. Giải phóng ngay 50GB – 150GB bộ nhớ SSD.

#### D. Kiểm tra trạng thái an toàn Git (Git Safety Guard):
Trước khi gom hoặc lưu trữ dự án, Penteye tự động kiểm tra:
1. `git status --porcelain`: Còn file chưa commit không?
2. `git log @{u}..`: Còn commit nào chưa push lên GitHub/GitLab không?
$\rightarrow$ Cảnh báo màu vàng/đỏ cho lập trình viên để không bao giờ mất code dở dang.

---

## 4. LỘ TRÌNH TRIỂN KHAI 4 BƯỚC

### Giai đoạn 1: Khảo sát & Quét nhanh (Discovery & Scan)
- Thêm tính năng **"Quét ổ đĩa"** vào Penteye (chạy worker nền, không ảnh hưởng đến animation 3D).
- Xuất báo cáo trực quan ngay trong cửa sổ:
  - Số lượng dự án lập trình phát hiện được & dung lượng rác build có thể giải phóng.
  - Số lượng tài liệu giáo án/đề thi nằm rải rác ngoài Desktop & Downloads.

### Giai đoạn 2: Lập kế hoạch phân bổ (Staging & Preview)
- Người dùng xem trước bản đối chiếu **"Trước và Sau"**:
  - File/Thư mục ban đầu $\rightarrow$ Vị trí thư mục chuẩn đề xuất di chuyển.
- Cho phép người dùng tick chọn hoặc bỏ chọn từng mục theo ý muốn.

### Giai đoạn 3: Thực thi an toàn (Safe Organization)
- Di chuyển file vào thư mục tập trung.
- Tạo **Windows Directory Junction (mklink /J)** tại vị trí cũ nếu người dùng cần giữ đường dẫn cho các phần mềm khác.
- Lưu lại file rollback JSON (`.penteye-organize-undo.json`) để người dùng có thể **Hoàn tác 1-click** bất cứ lúc nào.

### Giai đoạn 4: Chế độ giám sát tự động (Continuous Watcher)
- Penteye chạy ngầm trên desktop với mức tiêu thụ CPU 0%.
- Khi phát hiện người dùng tải file mới vào `Downloads` hoặc lưu file ra `Desktop` quá 7 ngày:
  - Hiện thông báo nhỏ nhẹ nhàng bên cạnh mô hình 3D:
    *"Bạn có 5 file giáo án mới trong Downloads, bạn có muốn gom về thư mục Giáo Án không?"*

---

## 5. KẾT LUẬN & GIÁ TRỊ MANG LẠI

1. **Đối với Giáo viên**:
   - Máy tính luôn ngăn nắp, màn hình Desktop sạch sẽ, không còn tình trạng hàng trăm icon che kín hình nền.
   - Tìm kiếm bài giảng, đề thi chỉ mất 1 giây thông qua phím tắt `Alt + S`.
   - Có sẵn công cụ ghi chú phân mục và đính kèm tài liệu với ngày tháng rõ ràng (`Alt + N`).

2. **Đối với Lập trình viên**:
   - Thu hồi ngay 50GB – 200GB dung lượng SSD bị chiếm dụng bởi cache và thư mục build cũ.
   - Quản lý toàn bộ repository tập trung, không lo sót code chưa commit hay quên repo nằm ở thư mục tạm.
   - Tận dụng trợ lý AI ngay dưới mô hình (`Alt + A`) để giải đáp kỹ thuật, viết code và sửa chính tả mã nguồn (`Alt + Z`).
