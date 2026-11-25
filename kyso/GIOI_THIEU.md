# Sao Mai Ký số

**Sao Mai Ký số** là phần mềm ký số văn bản PDF trên máy tính Windows. Phần mềm dùng **USB Token chứng thư số chuyên
dùng Chính phủ** do **Ban Cơ yếu Chính phủ** cấp. Văn bản được ký ngay trên máy, không gửi lên máy chủ nào.

Một file chạy duy nhất (`SaoMai-KySo.exe`), không cần cài đặt.

> Phiên bản 1.5.0, bản thử nghiệm (beta). Tải ở mục **Releases** của kho này.

---

## Chức năng

| Chức năng | Làm được gì |
|---|---|
| **Ký số văn bản** | Ký một file hoặc cả thư mục PDF (kể cả thư mục con) bằng USB Token. Có **ký nháy** và **ký chính**. Mỗi chữ ký có dấu thời gian của Ban Cơ yếu và dữ liệu kiểm tra dài hạn. Mỗi file được kiểm lại ngay sau khi ký |
| **Vị trí & kiểu chữ ký** | Chữ ký hiện dạng chỉ thông tin, ảnh + thông tin, chỉ ảnh, hoặc ẩn. Đặt theo mẫu khổ giấy A1–A5, hoặc kéo thả trên bản xem trước. Thêm / xoá khung trên từng trang bằng chuột phải |
| **Tạo văn bản từ mẫu** | Điền dữ liệu Excel / CSV vào mẫu Word (`{ten_bien}`) để tạo hàng loạt văn bản PDF hoặc DOCX, xem trước từng văn bản |
| **Kiểm tra chữ ký số** | Xác minh chữ ký trong PDF theo **Nghị định 68/2024/NĐ-CP, Điều 30**: văn bản toàn vẹn, người ký, thời gian ký, hiệu lực chứng thư (kể cả thu hồi), chuỗi chứng thư về CA gốc Ban Cơ yếu. Phát hiện văn bản bị sửa sau khi ký. Xuất báo cáo ra PDF |
| **Thông báo Telegram** | Báo về nhóm Telegram của đơn vị khi có máy mới dùng phần mềm và sau mỗi lượt ký số / tạo văn bản |

---

## Yêu cầu máy

- Windows 10 hoặc 11, 64-bit.
- **WebView2 Runtime**: Windows 10 / 11 cập nhật thường đã có sẵn.
- **Driver USB Token** (bit4id, SafeNet… tuỳ loại token): tải ở trang của Ban Cơ yếu Chính phủ. Trong phần mềm có
  **Hỗ trợ › Cài đặt driver USB Token** để kiểm tra máy đã có driver chưa.
- **Microsoft Word**: chỉ cần khi Tạo văn bản ra PDF.
- Có mạng khi ký để lấy dấu thời gian. Mất mạng vẫn ký được theo giờ máy tính, nhưng văn bản không có dấu thời gian.

---

## Bắt đầu

1. Tải `SaoMai-KySo.exe` ở mục **Releases**, đặt vào thư mục bất kỳ.
2. Chạy file. Lần đầu, Windows có thể hiện cảnh báo SmartScreen ("Windows protected your PC") vì file chưa ký mã
   nguồn: bấm **More info › Run anyway**.
3. Cắm USB Token có chứng thư số.

Phím tắt: <kbd>Ctrl</kbd>+<kbd>K</kbd> Ký số · <kbd>Ctrl</kbd>+<kbd>N</kbd> Tạo văn bản · <kbd>Ctrl</kbd>+<kbd>E</kbd>
Kiểm tra chữ ký · <kbd>F1</kbd> Hướng dẫn.

---

## Cách sử dụng

### 1. Ký số văn bản (<kbd>Ctrl</kbd>+<kbd>K</kbd>)

1. Bấm **1 file** hoặc **Thư mục** để nạp văn bản PDF, hoặc kéo thả vào cửa sổ.
2. Chọn ảnh chữ ký / con dấu (`.png` nền trong là đẹp nhất) và kiểu **Hiển thị**. Khung chữ ký hiện trên bản xem
   trước bên phải:
   - kéo khung để dời chỗ, kéo cạnh / góc để đổi cỡ;
   - chuột phải vào trang để thêm khung, chuột phải vào khung để xoá;
   - **Cấu hình vị trí** để đặt khung chung cho mọi văn bản hoặc riêng từng văn bản.
3. Bấm **Ký số**, chọn chứng thư trong hộp **Xác nhận ký**, nhập mã PIN, bấm **Ký số**.
4. File đã ký nằm trong thư mục **DaKySo** cạnh file gốc, tên thêm `_signed`. Ký cả thư mục thì kết quả ở thư mục
   **`<tên thư mục>_DaKySo`** cùng cấp, giữ nguyên các thư mục con. File gốc giữ nguyên.

**Ký nháy rồi ký chính** (hai người):

1. Cắm token người nháy, chọn lượt **Nháy**, ký.
2. Nạp thư mục kết quả `DaKySo`, cắm token người ký chính, chọn lượt **Chính**, ký.

Khi đang ký có thể bấm **Tạm dừng**, sau đó **Ký tiếp** các văn bản còn lại.

### 2. Tạo văn bản từ mẫu (<kbd>Ctrl</kbd>+<kbd>N</kbd>)

1. Soạn mẫu Word `.docx`. Chỗ cần điền viết dạng `{ho_ten}`, `{so_quyet_dinh}`…
2. Chuẩn bị file dữ liệu `.xlsx` / `.xls` / `.csv`: mỗi dòng là một văn bản, dòng tiêu đề là tên cột.
3. Chọn mẫu và file dữ liệu. Phần mềm tự ghép biến với cột cùng tên; chỉnh tay chỗ chưa khớp.
4. Xem trước, chọn **PDF** hoặc **DOCX**, đặt mẫu tên file, bấm **Tạo văn bản**.
5. Muốn ký luôn: nạp thư mục kết quả ở màn Ký số.

Cần một văn bản lẻ: chọn **Nhập trực tiếp** và điền từng biến.

### 3. Kiểm tra chữ ký số (<kbd>Ctrl</kbd>+<kbd>E</kbd>)

1. Bấm **Chọn file** (hoặc kéo thả) một PDF đã ký.
2. Tab **Chữ ký**: từng chữ ký có kết luận (xanh: hợp lệ, vàng: cần lưu ý, đỏ: không hợp lệ). Bên dưới là 5 điều
   kiện, mỗi điều kiện ghi Đạt / Lưu ý / Không đạt kèm giải thích. **Chi tiết chứng thư**: nhà cung cấp CA, số
   serial, hiệu lực, dấu thời gian, tình trạng thu hồi, ký dài hạn, chuỗi chứng thư.
3. Tab **Văn bản**: vị trí từng chữ ký trên trang (khung cam là chữ ký đang chọn).
4. **Xuất báo cáo** để lưu kết quả ra PDF.

Ô **Kiểm tra thu hồi** (bật sẵn, cần mạng) hỏi máy chủ của Ban Cơ yếu xem chứng thư có bị thu hồi. Văn bản ký
bằng phần mềm này có sẵn dữ liệu kiểm tra dài hạn, nên kiểm được cả khi không có mạng.

| Kết luận | Nghĩa |
|---|---|
| Chữ ký hợp lệ | Đạt cả 5 điều kiện |
| Hợp lệ, chưa kiểm tra được thu hồi | Đạt, nhưng không hỏi được máy chủ CA (mất mạng) |
| Không có dấu thời gian | Giờ ký do máy người ký tự ghi, không chứng minh được |
| Người ký không thuộc CA Ban Cơ yếu | Chứng thư không do Ban Cơ yếu cấp (kể cả chữ ký số công cộng), hết hạn hoặc bị thu hồi |
| Văn bản bị sửa sau khi ký | Chữ ký đúng với bản đã ký, nhưng sau đó nội dung bị thay đổi |
| Chữ ký không hợp lệ | Nội dung không khớp chữ ký |

### 4. Cấu hình

Nút **Cấu hình** trên thanh menu:

- **Chữ ký theo khổ giấy**: mẫu A1–A5, phông, cỡ, màu chữ, lý do ký, nơi ký, ảnh chữ ký.
- **USB Token & chứng thư số**: danh sách chứng thư, PIN đã lưu.
- **Thư mục & tên file**: nơi lưu và hậu tố tên file.

Bấm **Lưu cấu hình** để áp dụng. Cấu hình nằm ở `%APPDATA%\SaoMai-KySo\config.json`.

---

## Mã PIN

- Nhập PIN ở hộp **Xác nhận ký**. Để trống thì nhập ở hộp của token.
- **Lưu PIN trên máy này** (tắt sẵn): PIN được mã hoá theo tài khoản Windows. Ai dùng được tài khoản đó và có
  token cắm vào máy đều ký được mà không cần biết PIN.
- PIN sai nhiều lần thì token bị khoá; khi đó liên hệ nơi cấp chứng thư.

---

## Thông báo Telegram

Bản phát hành của đơn vị gửi thông báo về nhóm Telegram quản trị:

- **Lần đầu chạy trên mỗi máy** (mỗi tài khoản Windows): tên máy, tài khoản Windows, hệ điều hành, phiên bản phần
  mềm, IP nội bộ, địa chỉ MAC.
- **Sau mỗi lượt ký số / tạo văn bản**: tên máy, tài khoản Windows, thời gian, tên file, kết quả (thành công, lỗi,
  bị dừng) và mã lỗi nếu có.

Không bao giờ gửi nội dung văn bản, file PDF, mã PIN, khoá bí mật hay dữ liệu chữ ký. Mất mạng thì việc ký vẫn chạy
bình thường; thông báo được gửi lại sau. Xem trạng thái ở **Hỗ trợ › Thông báo Telegram**.

---

## Lỗi thường gặp

| Hiện tượng | Cách xử lý |
|---|---|
| Không thấy chứng thư / không nhận token | Cắm lại token hoặc đổi cổng USB, bấm **Quét lại**. Kiểm tra driver ở **Hỗ trợ › Cài đặt driver USB Token** |
| Chứng thư ghi "không ký được" | Chỉ chứng thư của CA Ban Cơ yếu, còn hạn, có khoá trên token mới ký được |
| Một số file ký lỗi | File đang mở ở chương trình khác: đóng file rồi bấm **Ký tiếp**. PDF có mật khẩu: bỏ mật khẩu trước. PDF đã bị khoá không cho ký thêm thì không ký được |
| Văn bản "Chưa gán chữ ký" | Đã xoá hết khung chữ ký; văn bản được bỏ qua khi ký. Chuột phải trên trang › **Thêm khung** để gán lại |
| Tạo văn bản không ra PDF | Máy chưa có Microsoft Word; chọn DOCX hoặc cài Word |
| Windows chặn khi mở file | Bấm **More info › Run anyway** (file chưa ký mã nguồn) |

Nhật ký của phần mềm ở `%LOCALAPPDATA%\SaoMai-KySo\logs` (không chứa mã PIN hay nội dung văn bản); gửi kèm khi báo
lỗi. Hướng dẫn đầy đủ: **Hỗ trợ › Hướng dẫn sử dụng** (<kbd>F1</kbd>) trong phần mềm.

---

**Giải pháp số Sao Mai** · [saomaiso.com](https://saomaiso.com) · hotro@saomaiso.com

Tài liệu cho người phát triển (build, kiến trúc, kiểm thử): [README.md](README.md).
