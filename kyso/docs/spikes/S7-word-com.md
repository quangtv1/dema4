# S7: Word COM từ Rust trên luồng STA

- **Kết luận (18/11/2026): PASS.** Xuất được 3 mẫu DOCX ra PDF từ Rust, kết quả trùng với bản tham chiếu, Word
  thoát sạch.
- Còn một trường hợp chưa thử: máy đang mở sẵn Word với tài liệu của người dùng.

## Mục tiêu

Xuất 3 mẫu `fixtures/merge/*.docx` ra PDF từ Rust và đóng Word sạch (không còn `WINWORD.EXE` do spike mở).

## Phương án kỹ thuật đã làm

Mã: `spikes/s7-word/` (crate `windows` 0.62.2, toolchain MSVC).

1. Một luồng riêng `word-sta` gọi `CoInitializeEx(COINIT_APARTMENTTHREADED)`; mọi lời gọi COM nằm trên luồng
   này. Luồng chính gửi việc qua `mpsc` và chờ trả lời có giới hạn thời gian (120 giây mỗi file).
2. `CLSIDFromProgID("Word.Application")` → `CoCreateInstance(CLSCTX_LOCAL_SERVER)` → `IDispatch`. Gọi muộn bằng
   `GetIDsOfNames` + `Invoke`; tham số dùng `VARIANT` của crate `windows` (tự `VariantClear` khi thả).
3. Chuỗi lệnh đúng như `WordPdfConverter.cs`: `Visible = false`, `DisplayAlerts = 0`,
   `Documents.Open(path, false, true, false)`, `ExportAsFixedFormat(pdf, 17)`, `Close(0)`, `Quit(0)`.
4. PID của Word do spike mở: chụp danh sách `WINWORD.EXE` trước và sau `CoCreateInstance` (ToolHelp).
5. Sau `Quit`: chờ tiến trình tự thoát 15 giây, không thoát thì `TerminateProcess`.

## Kết quả thực tế

| Phép thử | Kết quả |
|---|---|
| Xuất 3 mẫu | 3/3 thành công. Word sẵn sàng sau 3,33s (lần đầu), file đầu 2,04s, hai file sau 0,32s mỗi file |
| 60 lượt liên tiếp (`--repeat 20`) | 60/60 thành công; mỗi file 0,21–0,64s, trung bình 0,264s; tổng 20,09s |
| Đóng Word | Cả 3 lần chạy: luồng STA thoát, `WINWORD.EXE` **tự thoát** sau `Quit(0)`, không phải buộc đóng, không còn tiến trình sót |
| Nhận diện PID | Cả 3 lần đều tìm đúng một PID mới |
| Giả lập treo (`--kill-test`: buộc đóng Word khi đang mở file) | Lời gọi COM đang chờ trả lỗi sau 0,17s: `0x800706BE The remote procedure call failed`. Luồng STA thoát sạch, không treo |
| So với bản tham chiếu (cùng chuỗi lệnh COM, chạy từ PowerShell) | Số trang khớp (1, 1, 3), khổ A4 khớp, kích thước file khớp từng byte (158.492, 178.063, 168.067), nội dung khớp sau khi bỏ ngày tạo và ID. Đọc bằng pyHanko 0.38.1 |

Phiên bản Word trên máy thử: `Word.Application.16`, trường Producer của PDF ghi "Microsoft® Word for
Microsoft 365".

Bản tham chiếu dùng đúng các lệnh COM của bản .NET 1.4.0 nhưng không phải chính bản 1.4.0; chưa chạy bản
1.4.0 để so.

## Trả lời các câu hỏi của spike

- **Lấy PID:** so danh sách tiến trình trước và sau khi tạo là đủ trong điều kiện thử. Điểm yếu: nếu người
  dùng mở Word đúng trong khoảnh khắc đó thì sẽ thấy hai PID mới; khi ấy không được buộc đóng PID nào (spike
  in cảnh báo). Bản sản phẩm nên xác nhận thêm bằng dòng lệnh của tiến trình (`/Automation -Embedding`).
- **Thời gian:** khoảng 0,26s mỗi file sau khi Word đã khởi động; không thấy chậm dần qua 60 lượt. Chưa đo số
  handle.
- **Word treo:** buộc đóng theo PID làm lời gọi COM trả lỗi ngay, nên cơ chế "quá thời gian → kill → mở phiên
  mới" khả thi.
- **Word đang mở sẵn tài liệu của người dùng:** **chưa thử** (lúc thử máy không mở Word).

## Lưu ý khi đưa vào `smks-merge::convert::word`

- Đường dẫn truyền cho Word phải là đường dẫn tuyệt đối.
- Sau khi buộc đóng Word phải tạo phiên mới; mọi đối tượng COM cũ chỉ được thả, không được gọi tiếp.
- Mã thoát `0x800706BE` / `0x800706BA` cần được dịch thành thông báo tiếng Việt (lỗi 1054 hoặc 1046).

## Rủi ro còn lại

R6 giảm đáng kể. Còn lại: hành vi khi người dùng đang mở Word, các bản Office 2016/2019, và máy không có
Word (đường báo lỗi 1045 chưa thử vì máy thử có Word).
