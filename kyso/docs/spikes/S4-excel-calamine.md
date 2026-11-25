# S4: Đọc Excel bằng calamine

- **Kết luận (18/11/2026): PASS theo tiêu chí của kế hoạch.** `calamine 0.38.0` đọc 3 file mẫu khớp ClosedXML
  **193/193 ô**, đúng tiêu đề, đúng số dòng, đúng số cột.
- **Kèm điều kiện:** với file có ô số, calamine chỉ trả giá trị thô và mã định dạng. Muốn giống bản cũ phải
  tự viết phần áp định dạng số (25/57 ô của `tricky.xlsx` lệch khi chưa áp), và phải chốt vùng văn hoá (Q-08).
- Chạy bằng toolchain MSVC sau khi B-01 được gỡ.

## Mục tiêu

`calamine` đọc 3 file `fixtures/merge/*.xlsx` cho ra đúng từng giá trị như `ExcelReader.cs` của bản cũ
(ClosedXML 0.105.1): ngày, số có số 0 đứng đầu, số tiền, phần trăm.

## Phương án kỹ thuật

1. **Chuẩn so sánh 1 (ClosedXML):** `spikes/s4-excel/closedxml-ref/`, chép nguyên logic `ExcelReader.Read`,
   cùng phiên bản ClosedXML 0.105.1, xuất JSON kèm kiểu dữ liệu và định dạng số của từng ô. Chạy ở hai vùng
   văn hoá `en-US` và `vi-VN`.
2. **Chuẩn so sánh 2 (Excel):** `spikes/s4-excel/make-tricky.ps1` dùng Excel 16 qua COM để tạo
   `fixtures/table/tricky.xlsx` (20 cột định dạng dễ lệch, tiêu đề ở dòng 2, bắt đầu từ cột B, một dòng
   trống, một ô gộp, một cột không tiêu đề, một sheet thứ hai) và ghi lại chữ Excel hiển thị (`Range.Text`).
3. **Bản Rust:** `spikes/s4-excel/calamine-read/`, cùng quy tắc, cố ý chưa áp định dạng số để thấy calamine trả gì.
4. **So sánh:** `spikes/s4-excel/compare.py` so từng ô.

## Kết quả thực tế

### Đã chạy

| Lệnh | Kết quả |
|---|---|
| `make-tricky.ps1` | Tạo được `tricky.xlsx` (11.113 byte) và `excel-text.json`. Excel không tự thoát sau `Quit()`; script buộc đóng đúng tiến trình do nó mở |
| `dotnet build -c Release` (closedxml-ref) | 0 cảnh báo, 0 lỗi |
| `closedxml-ref en-US …` và `closedxml-ref vi-VN …` trên 4 file | Ghi được 2 file JSON |
| `cargo run --release` (calamine-read), toolchain GNU | **Không build được**: `error calling dlltool`, `could not compile getrandom`, `windows-sys` |
| `cargo run --release` (calamine-read), toolchain MSVC, sau khi gỡ B-01 | Chạy được, ghi `calamine.json` |
| `python compare.py closedxml.en-US.json calamine.json` | 225/250 ô khớp; 3 file mẫu khớp 193/193 |

Bằng chứng: `docs/spikes/evidence/S4/closedxml.en-US.json`, `closedxml.vi-VN.json`, `excel-text.json`.

### Ba file mẫu (theo ClosedXML)

| File | Tiêu đề | Dòng dữ liệu | Kiểu ô | Lệch giữa en-US và vi-VN |
|---|---|---|---|---|
| `1_giay_moi_hop.xlsx` | 11 | 6 | 66 Text | 0 |
| `2_quyet_dinh_khen_thuong.xlsx` | 11 | 5 | 46 Text, 5 DateTime (`dd/mm/yyyy`), 4 Blank | 0 |
| `3_hop_dong_lao_dong.xlsx` | 18 | 4 | 56 Text, 16 DateTime (`dd/mm/yyyy`) | 0 |

Ba file mẫu **không có ô số nào**: số tiền, số thứ tự đều được lưu dạng văn bản (inline string). Chúng chỉ
thử được chuỗi và ngày, không thử được định dạng số mà rủi ro R5 nói tới.

### File `tricky.xlsx`: bản cũ thực sự trả về gì

| Loại ô | ClosedXML en-US | ClosedXML vi-VN | Excel hiển thị |
|---|---|---|---|
| Văn bản `0001` | `0001` | `0001` | `0001` |
| Số 1, định dạng `0000` | `0001` | `0001` | `0001` |
| 1234567, `#,##0` | `1,234,567` | **`1.234.567`** | `1,234,567` |
| 1234567.5, `#,##0.00` | `1,234,567.50` | **`1.234.567,50`** | `1,234,567.50` |
| 0.125, `0.00%` | `12.50%` | **`12,50%`** | `12.50%` |
| Ngày, `dd/mm/yyyy` | `05/03/2026` | `05/03/2026` | `05/03/2026` |
| Ngày, định dạng ngắn của hệ thống (id 14) | `05/03/2026` | `05/03/2026` | `3/5/2026` |
| Ngày giờ, `dd/mm/yyyy hh:mm` | `05/03/2026` (mất giờ) | `05/03/2026` | `05/03/2026 14:30` |
| Số General 0.1 | `0.1` | **`0,1`** | `0.1` |
| Số General 123456789012 | `123456789012` | `123456789012` | `1.23457E+11` |
| Số General 12345678901234567 | `1.23456789012346E+16` | **`1,23456789012346E+16`** | `1.23457E+16` |
| Số General 1e-7 | `1E-07` | `1E-07` | `0.0000001` |
| Công thức `=J3*2` | `30000000` | `30000000` | `30000000` |
| Văn bản `"  abc  "` | `abc` (đã cắt) | `abc` | `  abc  ` |
| Logic | `TRUE` / `FALSE` | như bên | như bên |
| Giờ, `hh:mm` | `14:30` | `14:30` | `14:30` |
| 2500000, `#,##0 "VND"` | `2,500,000 VND` | **`2.500.000 VND`** | `2,500,000 VND` |
| -1500, `#,##0;(#,##0)` | `(1,500)` | **`(1.500)`** | `(1,500)` |
| Ô gộp, ô thứ hai | rỗng | rỗng | rỗng |

Cấu trúc: tiêu đề nhận đúng ở dòng 2; số cột là số thật trên sheet (2 đến 21); cột không tiêu đề bị bỏ (19
trên 20 cột); dòng trống bị bỏ (dòng dữ liệu 3, 4, 6); sheet thứ hai không được đọc.

### calamine so với ClosedXML (`compare.py`)

| File | Sheet, tiêu đề, số dòng | Ô khớp (chuẩn en-US) |
|---|---|---|
| `1_giay_moi_hop.xlsx` | khớp | 66/66 |
| `2_quyet_dinh_khen_thuong.xlsx` | khớp | 55/55 |
| `3_hop_dong_lao_dong.xlsx` | khớp | 72/72 |
| `tricky.xlsx` | khớp (tiêu đề dòng 2, cột 2–21, bỏ cột không tiêu đề, bỏ dòng trống) | 32/57 |

Với chuẩn `vi-VN`: 222/250 ô khớp (thêm 3 ô số General có dấu thập phân).

25 ô lệch của `tricky.xlsx`, theo nhóm:

| Nhóm | calamine trả | Bản cũ trả | Mã định dạng calamine đọc được |
|---|---|---|---|
| Số có định dạng | `1`, `1234567`, `1234567.5`, `0.125`, `2500000`, `-1500` | `0001`, `1,234,567`, `1,234,567.50`, `12.50%`, `2,500,000 VND`, `(1,500)` | `0000`, `#,##0`, `#,##0.00`, `0.00%`, `#,##0 "VND"`, `#,##0;(#,##0)` |
| Số General rất lớn hoặc rất nhỏ | `12345678901234568`, `0.0000001`, `0.000001234` | `1.23456789012346E+16`, `1E-07`, `1.234E-06` | `General` |
| Logic | `true` / `false` | `TRUE` / `FALSE` | `General` |
| Giờ (`hh:mm`) | được nhận là ngày giờ, in thô ra `31/12/1899` | `14:30` | `hh:mm` |

Những gì calamine làm đúng sẵn: chuỗi (kể cả inline string), ngày, giá trị đã tính của ô công thức, ô gộp,
số cột và số dòng thật, và **mã định dạng của từng ô** qua `worksheet_style` (đủ để tự áp định dạng).

Bằng chứng: `docs/spikes/evidence/S4/calamine.json`.

## Điều rút ra

1. **Kết quả đọc số của bản cũ phụ thuộc vùng văn hoá của Windows.** 16 trên 57 ô của `tricky.xlsx` khác
   nhau giữa `en-US` và `vi-VN`. `GetFormattedString()` dùng `CultureInfo.CurrentCulture`. Bản Rust phải chọn
   một chính sách (Q-08): theo cài đặt vùng của máy như bản cũ, hay cố định `vi-VN`.
2. Ô ngày giờ luôn ra `dd/MM/yyyy`, bất kể định dạng của ô; phần giờ bị bỏ.
3. Số General ra tối đa 15 chữ số có nghĩa, chuyển sang dạng `E+16` / `E-07` theo quy tắc `G15` của .NET.
   Khác với Excel hiển thị (Excel còn phụ thuộc bề rộng cột).
4. Bản Rust phải tự áp ít nhất các định dạng: `General`, `0000`, `#,##0`, `#,##0.00`, `0.00%`, chuỗi cố định
   trong ngoặc kép, định dạng hai phần `dương;âm`, `hh:mm`.
5. Văn bản bị cắt khoảng trắng hai đầu.

## Việc phải làm khi viết `smks-merge::table`

- Viết bộ áp định dạng số cho: `General` (15 chữ số có nghĩa, dạng `E+16`/`E-07`), số 0 đệm, nhóm nghìn, số
  thập phân cố định, phần trăm, chuỗi cố định trong ngoặc kép hoặc sau dấu gạch chéo ngược, định dạng nhiều
  phần `dương;âm`, giờ `hh:mm`. Định dạng chưa hỗ trợ thì trả giá trị thô và ghi cảnh báo.
- Ô ngày giờ luôn ra `dd/MM/yyyy`; ô chỉ có giờ (định dạng không có ngày) ra theo định dạng giờ.
- Logic ra `TRUE` / `FALSE`.
- Dùng `fixtures/table/tricky.xlsx` cùng hai bộ kết quả mong đợi làm test hồi quy.

## Rủi ro còn lại

R5 thu hẹp lại thành một việc cụ thể: bộ áp định dạng số, có test hồi quy. Còn rủi ro về vùng văn hoá (K-11,
Q-08) và về các định dạng hiếm gặp trong file thật của khách hàng mà `tricky.xlsx` chưa phủ.
