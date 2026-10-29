# dema4

Ứng dụng `sm-dema4` (Giải pháp số Sao Mai), 2 chức năng:

- **Đếm trang A4**: phân loại khổ giấy A0–A7, quy đổi ra số trang A4 theo hệ số, cảnh báo trang sát ngưỡng chuyển khổ và trang có DPI bất thường.
- **Tách văn bản**: tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã Code 128), bỏ trang trắng, gợi ý trang scan bị xoay; duyệt và sửa nhãn trên giao diện trước khi tách. Không cần WSL / Python.

`dema4_rs/` — bản Rust (giao diện egui + chế độ dòng lệnh), build qua GitHub Actions.
Bản gốc viết bằng Python (PyQt5 + PyMuPDF) được giữ cục bộ trong thư mục `temp/`, không đưa lên repo.

## Build bản Rust

Cần Rust (rustup). Trên Windows cần thêm Visual Studio Build Tools (C++) và LLVM.

```
cd dema4_rs
cargo build --release
```

File chạy: `dema4_rs/target/release/sm-dema4` (Windows: `sm-dema4.exe`) — một file duy nhất, ONNX Runtime liên kết tĩnh, mô hình hướng trang nhúng sẵn.

GitHub Actions tự build cho macOS (arm64) và Windows (x64) mỗi khi push lên `main`.
Tạo tag `vX.Y.Z` để sinh Release kèm file chạy.

## Tách văn bản

Menu **Chức năng › Tách văn bản**. Các bước (cũng có trong **Trợ giúp › Hướng dẫn sử dụng**, F1):

1. **Chọn** thư mục hồ sơ PDF → **Phân tích**. Lưới hiện ngay; nhãn tự động (Index, Trắng, Nghi trắng, Nghi xoay) đến dần trong lúc chạy.
2. Tab **Hồ sơ**: kiểm tra nhãn, sửa bằng phím **S** (Split — văn bản mới bắt đầu từ trang này), **I** (Index), **B** (Trắng), **C** (bỏ nhãn). Chọn trang như Windows Explorer (Ctrl / Shift / kéo khung); Ctrl+Z / Ctrl+Y riêng từng nhóm lọc.
3. Duyệt hết **Nghi trắng** và **Nghi xoay** (menu lọc) — bắt buộc trước khi tách.
4. Tab **Văn bản**: xem trước từng văn bản (Lưới · 1 trang · 2 trang); xoay hoặc **Bỏ khỏi văn bản** nếu cần.
5. **Tách** → `<thư mục gốc>_vanban/…/<hồ sơ>/<hồ sơ>_part1.pdf …`, giữ nguyên cấu trúc thư mục. Hồ sơ không có nhãn và không xoay được chép nguyên từng byte (giữ chữ ký số).

Nhãn, góc xoay, cache phân tích và log lưu trong thư mục ẩn `<thư mục gốc>/.sm-dema4/`; file PDF gốc không bị sửa. Mở lại thư mục giữ nguyên trạng thái lần trước.

### Định dạng file

- `.sm-dema4/index_and_blank.txt` — 1 dòng/trang có nhãn, trang đếm từ 1 (giữ định dạng cũ để script Python dùng tiếp):
  ```
  HOP 462/4766/4766.pdf;10;index
  HOP 462/4766/4766.pdf;53;blank
  HOP 462/4766/4766.pdf;70;split
  ```
- `.sm-dema4/xoay_trang.txt` — góc xoay thêm theo chiều kim đồng hồ khi tách; `;xac_nhan` = trang Nghi xoay đã được người duyệt xác nhận (0° = đề xuất sai, giữ nguyên):
  ```
  HOP 462/4766/4766.pdf;24;180
  HOP 462/4766/4766.pdf;31;0;xac_nhan
  ```
- `.sm-dema4/ketqua_tach.txt` — log tách (tab): STT · file đầu vào · dải trang · file đầu ra · trang trắng đã bỏ · số trang; dòng `#` cuối = file cũ không xoá được.

### Dòng lệnh

```
sm-dema4 --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục cha>]
```

Không có file nhãn → tự phân tích (Cấu hình mặc định), ghi `index_and_blank.txt` rồi tách.

## Cấu hình (`sm-dema4.cfg`)

Menu **Cấu hình** (2 tab Đếm trang A4 / Tách PDF) ghi file `sm-dema4.cfg` cạnh file chạy (thư mục đó không ghi được → `%APPDATA%\sm-dema4\`, macOS `~/Library/Application Support/sm-dema4/`). Thiếu khoá / sai → giá trị mặc định.

| Khoá | Mặc định | Ý nghĩa |
|---|---|---|
| `he_so_a0` … `he_so_a7` | 16 · 8 · 4 · 2 · 1 · 0,5 · 0,25 · 0,125 | hệ số quy A4 |
| `ranh_gioi` | 1015;717.5;507;358.5;253.5;179;126.5 | ranh giới khổ theo cạnh dài (mm); khác mặc định thì ghi vào `_dem.txt` |
| `a0_lon` · `a0_lon_he_so` · `a0_nguong_mm` | `dien_tich` · 16 · 1399 | trang A0 dài hơn ngưỡng: theo diện tích ÷ A4 hoặc hệ số cố định |
| `quet_trang` · `quet_ma` · `quet_xoay` | 1 · 1 · 1 | loại nhận dạng tự động khi Phân tích |
| `nguong_trang` · `nguong_nghi` | 1.00 · 3.20 | ngưỡng Trắng / Nghi trắng (% diện tích có nét mực đậm) |
| `loai_ma` · `ma_index` | `code128` · `NAMSAO-TACH` | mã tờ index (lệch ≤ 3 ký tự; trống = mọi mã) |
| `tin_cay_xoay` | 0.80 | độ tin cậy gợi ý Nghi xoay (0,50–0,95) |
| `thu_muc_ra` · `mau_ten` · `luu_nhan` | trống · `{hoso}_part{n}` · `tu_dong` | thư mục cha kết quả; mẫu tên (`{hoso}` `{n}` `{n3}`); lưu nhãn tự động / `bang_tay` |
| `rong_cot_trai` · `co_anh_hs` · `co_anh_vb` · `ty_le_vb` · `che_do_vb` · `an_dong_thu_muc` | 260 · 2 · 2 · 100 · `luoi` · 0 | trạng thái giao diện |

## Chế độ dòng lệnh (Đếm)

```
sm-dema4 <thư mục>             # đếm, ghi <thư mục>/<tên>_dem.txt và _dem_chitiet.txt
sm-dema4 <thư mục> --dpi       # kiểm tra DPI ảnh scan (chậm hơn)
sm-dema4 <thư mục> --kyso      # kiểm tra file đã ký số (AcroForm) hay chưa
sm-dema4 <thư mục> --khong-ghi # chỉ in kết quả, không ghi file, không gửi thống kê
```

Hệ số và ranh giới khổ đọc từ `sm-dema4.cfg` (không có file → mặc định).

## Gửi thống kê

Sau mỗi lần đếm / tách xong, app gửi thống kê (tên máy, địa chỉ MAC, thư mục, số liệu — không gửi nội dung PDF) và báo công khai trong popup "Thông tin … đã được ghi lại".

- **Telegram**: token và id nhóm nhúng sẵn (xem `dema4_rs/src/telegram.rs`). Ghi đè bằng `telegram.txt` cạnh file chạy:
  ```
  token=<bot token>
  chat_id=<id nhóm>
  ```
  Lần tách gửi thêm 2 dòng thống kê ngưỡng trắng và file CSV histogram (dùng để chỉnh ngưỡng mặc định: `dema4_rs/scripts/tong-hop-nguong.py`).
- **Google Sheet** "Monitor_dema4" (tab `nguong`): ghi bằng service account; khoá nhúng lúc build từ `dema4_rs/bi_mat/sheet_key.json` (máy) hoặc secret `SHEET_KEY_JSON` (CI) — **không có trong repo**. Sheet phải chia sẻ quyền chỉnh sửa cho email service account.

## Giấy phép thành phần

MuPDF (AGPL) · zxing-cpp (Apache 2.0) · ONNX Runtime (MIT) · mô hình PP-LCNet_x1_0_doc_ori của PaddleOCR (Apache 2.0, xem `dema4_rs/assets/NOTICE-doc_ori.txt`) · egui (MIT / Apache 2.0).
