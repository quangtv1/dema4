# dema4-saomai

Ứng dụng hỗ trợ nghiệp vụ số hoá tài liệu (Giải pháp số Sao Mai):

- **Đếm trang A4** — đếm trang mọi PDF trong thư mục, phân loại khổ A0–A7, quy đổi ra số trang A4.
- **Tách văn bản** — tách hồ sơ PDF scan thành từng văn bản theo tờ index (mã vạch), bỏ trang trắng, gợi ý trang xoay.
- **Sinh mục lục hồ sơ** — từ file biên mục Excel/CSV và mẫu DOCX, sinh mục lục cho từng hồ sơ.
- **Bóc tách dữ liệu** — đọc văn bản PDF bằng mô hình AI (máy chủ nội bộ hoặc Gemini / Claude / GPT / OpenRouter) lấy số ký hiệu, ngày, trích yếu, tác giả, người ký…; ghi Excel và Google Sheet.
- **Chuẩn hoá tên**, **Ký văn bản**, **Đóng gói AIP** — đang phát triển.

Một file chạy duy nhất, không cần cài đặt. Tải ở [Releases](https://github.com/quangtv1/dema4/releases): `dema4_saomai.exe` (Windows x64) · `dema4_saomai` (macOS Apple Silicon). Trong app: **Trợ giúp › Kiểm tra cập nhật**.

Lần đầu mở (file chưa ký số với Apple / Microsoft):

- **macOS**: **System Settings › Privacy & Security › Open Anyway**, hoặc `xattr -d com.apple.quarantine ~/Downloads/dema4_saomai && chmod +x ~/Downloads/dema4_saomai`.
- **Windows** (SmartScreen): **More info › Run anyway**.
- **Windows, mở không lên cửa sổ** (máy card Intel driver cũ): từ v2.2.2 app không còn dò Vulkan nên không còn crash ở `igvk64.dll`. Vẫn không lên → cập nhật driver card đồ hoạ, hoặc thử chạy trong PowerShell: `$env:WGPU_BACKEND="gl"; .\dema4_saomai.exe`.

**Nâng cấp lên v2.3.0**: dữ liệu làm việc chuyển sang thư mục dữ liệu của người dùng (xem [Dữ liệu làm việc](#dữ-liệu-làm-việc-và-làm-tiếp-trên-máy-khác)); bản mới **không mang dữ liệu bản cũ sang** — làm xong / xuất Excel phần đang dở trước khi nâng cấp, sau đó dán lại khoá API và chỉnh lại Cấu hình. Số lượt chạy lấy từ trạm: máy chưa từng kích hoạt, và máy Windows lần chạy đầu sau nâng cấp, **cần có mạng**. Thư mục `.sm-dema4` cạnh file chạy của bản cũ không còn dùng, xoá tay được (quay lại bản cũ thì bản cũ vẫn đọc thư mục đó, không thấy phần làm trên bản mới).

**Hướng dẫn sử dụng chi tiết** (có hình minh hoạ): [docs/huong-dan/huong-dan-su-dung-dema4.docx](docs/huong-dan/huong-dan-su-dung-dema4.docx) · [bản PDF](docs/huong-dan/huong-dan-su-dung-dema4.pdf).

## Trang chủ

Mở app vào Trang chủ: bấm ô chức năng để bắt đầu (rê chuột xem tóm tắt). Về Trang chủ: menu **Chức năng › Trang chủ** hoặc bấm logo Sao Mai góc dưới phải màn giới thiệu.

**Làm tiếp gần đây**: 5 thư mục / file vừa làm (Đếm, Tách, Mục lục, Bóc tách) kèm tình trạng lúc rời màn — bấm 1 dòng là mở lại đúng chức năng, đúng thư mục (Bóc tách: cả loại tài liệu; Mục lục: cả sheet, dòng tiêu đề, mẫu). Thư mục đã đổi chỗ → **Chọn lại…**; × bỏ 1 dòng, **Xoá lịch sử** bỏ hết. Cùng danh sách ở **Chức năng › Gần đây**.

![Trang chủ](docs/anh/trang-chu.png)

## Đếm trang A4

Chọn thư mục (gồm cả thư mục con) → **Đếm**. Lọc theo khổ (tab A0–A7), bấm tên file để mở. Tuỳ chọn **Check ký số**, **Check DPI**. Xong tự lưu `<thư mục>_dem.txt` và `_dem_chitiet.txt`; popup có **Mở kết quả** · **Copy dữ liệu** (dán vào Excel).

![Đếm trang A4](docs/anh/dem-trang-a4.png)

## Tách văn bản

1. **Chọn thư mục → Phân tích**: nhãn tự động (Index, Trắng, Nghi trắng, Nghi xoay) hiện dần theo từng hồ sơ; có thể Dừng và phân tích tiếp lần sau.
2. **Tab Hồ sơ**: sửa nhãn bằng phím **S** (Split) · **I** (Index) · **B** (Trắng) · **C** (bỏ nhãn) · **90 / 180** (xoay); bấm đúp để phóng to.
3. **Duyệt Nghi trắng / Nghi xoay** (bắt buộc trước khi tách).
4. **Tab Văn bản**: xem trước từng văn bản sẽ sinh ra.
5. **Tách** → `<thư mục gốc>_vanban/…/<hồ sơ>/<hồ sơ>.1.pdf …`, giữ cấu trúc thư mục con; file PDF gốc không bị sửa.

![Tab Hồ sơ](docs/anh/tach-ho-so.png)

![Tab Văn bản](docs/anh/tach-van-ban.png)

Nhãn, cache và log nằm trong [thư mục dữ liệu](#dữ-liệu-làm-việc-và-làm-tiếp-trên-máy-khác), không ghi vào thư mục đầu vào.

## Sinh mục lục hồ sơ

1. **File biên mục**: Excel (.xlsx .xlsb .xls) hoặc CSV, mỗi văn bản một dòng, các dòng cùng hồ sơ liền nhau; chọn sheet và **Dòng tiêu đề** (nút ⌄ xem 10 dòng đầu).
2. **Mẫu mục lục**: 4 mẫu có sẵn hoặc mẫu DOCX riêng (biến dạng `{ten_bien}`).
3. **Tab Mẫu**: biến tự khớp cột cùng tên; bấm ô **Lấy từ** để chọn cột khác hoặc gõ giá trị cố định. **Nhóm theo** = cột tách hồ sơ.
4. **Tab Kết quả**: xem trước mục lục từng hồ sơ.
5. **Sinh mục lục** → mỗi hồ sơ một file DOCX (tên mặc định `MLHS_{so_ho_so}`).

Ví dụ **Mẫu 04 – Vĩnh Phúc** với file biên mục có đủ cột (Số hồ sơ, Tên hồ sơ, STT, Số ký hiệu, Ngày tháng, Trích yếu, Tác giả, Tờ số, Ghi chú): 11/11 biến tự ghép, tab Kết quả ra mục lục từng hồ sơ.

![Mẫu 04 – Vĩnh Phúc: tab Mẫu đã ghép đủ biến](docs/anh/muc-luc-mau-04-mau.png)

![Mẫu 04 – Vĩnh Phúc: tab Kết quả](docs/anh/muc-luc-mau-04-ket-qua.png)

## Bóc tách dữ liệu

1. **Nguồn văn bản**: chọn thư mục PDF (gồm thư mục con).
2. **Mô hình đọc**: mô hình đã khai ở **Cấu hình › Bóc tách** — máy chủ nội bộ (vLLM / Ollama / LM Studio, địa chỉ chuẩn OpenAI `http://<máy>:<cổng>/v1`), Gemini (Vertex AI express mode hoặc AI Studio), Claude, GPT, OpenRouter. Dịch vụ đám mây có sẵn địa chỉ và danh sách model: chỉ cần dán khoá API (Gemini mặc định `gemini-3.7-flash` qua Vertex AI); địa chỉ sửa được khi đi qua proxy riêng (bắt buộc `https://`).
3. **Loại tài liệu**: Văn bản hành chính, Hồ sơ đất đai, Hợp đồng hoặc loại riêng. Tab **Trường dữ liệu** xem các trường và câu lệnh gửi mô hình, sửa tên cột / mô tả / chuẩn hoá; **Thử đọc** file đang xem (không trừ lượt chạy).
4. **Bắt đầu**: tab **Kết quả bóc** hiện từng file (PDF bên trái, các trường bên phải), sửa tay rồi **Lưu**; lọc `lỗi` / `5/6` rồi **Chạy lại (n)** chỉ đọc lại các file đang hiện. Tắt app giữa chừng (app hỏi trước khi tắt) → mở lại qua **Làm tiếp gần đây**: kết quả đã đọc còn nguyên, nút **Tiếp tục (n)** chỉ đọc n file còn lại.
5. Kết quả: `<thư mục>_boc_tach.xlsx` cạnh thư mục nguồn — mỗi file 1 dòng, nạp thẳng vào **Sinh mục lục** làm file biên mục; bật **Google Sheet** để có tab bản sao kết quả (tab trống = tên thư mục nguồn): có thay đổi thì tự ghi lại cả tab, tối đa 1 lần mỗi phút (khoá service account hoặc Apps Script — script mẫu `dema4_rs/apps-script-mau/boc_tach.gs`; bản trước v2.1.2 cần dán lại script mới). Kết quả (cả phần sửa tay) còn được gửi về trạm Sao Mai để sao lưu, nâng chất lượng bóc tách — mỗi máy 1 thư mục theo mã máy.

Mỗi trang cần đọc được gửi tới mô hình đọc đã chọn: máy chủ nội bộ thì ảnh không ra khỏi mạng đơn vị; dịch vụ đám mây thì ảnh gửi tới máy chủ của hãng (không dùng cho tài liệu mật). Khoá API lưu mã hoá trên máy.

![Bóc tách: tab Trường dữ liệu sau Thử đọc](docs/anh/boc-tach-truong.png)

![Bóc tách: tab Kết quả bóc](docs/anh/boc-tach-ket-qua.png)

## Cấu hình

Menu **Cấu hình** (4 tab Đếm trang A4 / Tách PDF / Sinh mục lục / Bóc tách), lưu ở file `.dema4_saomai.cfg` trong thư mục dữ liệu.

![Cấu hình Đếm trang A4](docs/anh/cau-hinh-dem.png)

![Cấu hình Tách PDF](docs/anh/cau-hinh-tach.png)

![Cấu hình Sinh mục lục](docs/anh/cau-hinh-muc-luc.png)

![Cấu hình Bóc tách](docs/anh/cau-hinh-boc-tach.png)

## Dữ liệu làm việc và làm tiếp trên máy khác

Mọi dữ liệu làm việc (cấu hình, khoá API mã hoá, license, nhãn / cache / log Tách, kết quả Bóc tách, lịch sử gần đây) nằm ở **một chỗ cố định theo người dùng**, không phụ thuộc chỗ đặt file chạy — dời / chép file chạy sang thư mục khác vẫn làm tiếp được:

- Windows: `%LOCALAPPDATA%\sm-dema4`
- macOS: `~/Library/Application Support/sm-dema4`

Mỗi thư mục đầu vào có 1 thư mục con trong `viec/`. Cache phân tích nhận PDF theo nội dung, nên thư mục chép qua USB / Drive sang máy khác không phải phân tích lại.

**Làm tiếp trên máy khác** (màn Tách hoặc Bóc tách, đã mở thư mục): **Chức năng › Xuất phiên làm việc…** đóng gói nhãn, góc xoay, cache phân tích và kết quả bóc tách của thư mục đó thành `<tên>.sm-dema4.zip` (mặc định vào thư mục Tải về; không có khoá API, license, cấu hình). Ở máy kia mở **cùng thư mục** (Drive, USB…) rồi **Chức năng › Nhập phiên làm việc…**: hộp xác nhận cho biết % PDF khớp, loại tài liệu mới; mỗi hồ sơ giữ **bản sửa sau cùng** (máy này hoặc gói), kết quả bóc tách đọc được không bị bản lỗi đè; báo cáo liệt kê hồ sơ hai máy cùng sửa. Nên chia hồ sơ cho từng người, không đổi ngưỡng / Phân tích lại giữa chừng, và để đồng hồ máy đúng giờ.

## Dòng lệnh

```
dema4_saomai <thư mục> [--dpi] [--kyso] [--khong-ghi]                        # Đếm
dema4_saomai --tach <thư mục pdf> [--nhan <index_and_blank.txt>] [--ra <thư mục cha>]
```

## Trợ giúp

**Trợ giúp › Hướng dẫn sử dụng** (F1) mở đúng tab của màn đang dùng; **Thông tin** có phiên bản và website.

![Hướng dẫn sử dụng](docs/anh/huong-dan.png)

Sau mỗi lần chạy xong, app gửi số liệu thống kê (không gửi nội dung tài liệu) về Sao Mai và báo trong popup kết quả. Riêng **Bóc tách** gửi ảnh trang PDF tới mô hình đọc do người dùng chọn để lấy dữ liệu; thống kê gửi về Sao Mai chỉ gồm số file / trang / trường và tên mô hình.

## Build từ mã nguồn

Cần Rust; Windows cần thêm Visual Studio Build Tools (C++) và LLVM.

```
cd dema4_rs
cargo build --release
```

GitHub Actions tự build macOS (arm64) và Windows (x64); tag `vX.Y.Z` tạo Release kèm file chạy.

## Giấy phép thành phần

MuPDF (AGPL) · zxing-cpp (Apache 2.0) · ONNX Runtime (MIT) · PP-LCNet_x1_0_doc_ori của PaddleOCR (Apache 2.0, `dema4_rs/assets/NOTICE-doc_ori.txt`) · egui (MIT / Apache 2.0) · rust_xlsxwriter, aes-gcm (MIT / Apache 2.0) · jpeg-encoder (MIT / Apache 2.0; *this software is based in part on the work of the Independent JPEG Group*).
