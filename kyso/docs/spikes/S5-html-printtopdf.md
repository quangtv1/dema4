# S5: In HTML ra PDF bằng WebView2

- **Kết luận (18/11/2026): PASS**, kèm một điều kiện về cách truyền khổ giấy (xem "Phát hiện").

## Mục tiêu

Từ ứng dụng Tauri, in một mẫu HTML ra PDF đúng khổ A4, đúng font, đúng lề theo CSS `@page`.

## Phương án kỹ thuật đã làm

Mã: `spikes/s5-printpdf/` (ứng dụng Tauri 2.12 tối thiểu, `webview2-com` 0.39.1 cùng phiên bản Tauri đang dùng).

1. Cửa sổ ẩn (`visible: false`).
2. Chờ trang khởi đầu của ứng dụng tải xong (`Builder::on_page_load`, sự kiện `Finished`).
3. `with_webview` → lấy `ICoreWebView2` từ controller → đăng ký `NavigationCompleted` → `NavigateToString(html)`.
4. Trong `NavigationCompleted` của lần nạp đó (nguồn `about:blank`): gọi `ICoreWebView2_7::PrintToPdf(file, cài đặt, handler)`.
5. Hai chế độ: `css` (không truyền cài đặt in) và `settings` (`ICoreWebView2Environment6::CreatePrintSettings`:
   A4 = 8,27 × 11,69 inch, bốn lề 0, in nền, không đầu trang/chân trang).

Mẫu thử `sample.html`: `@page { size: A4; margin: 20mm 15mm 20mm 30mm }`, Times New Roman 14pt, một khối có nền
màu, một bảng, một ngắt trang, đủ bộ nguyên âm có dấu.

## Kết quả thực tế

| Mục | Chế độ `css` | Chế độ `settings` |
|---|---|---|
| `PrintToPdf` | thành công, 0,13s | thành công, 0,12s |
| Số trang | 2 | 2 |
| Khổ trang (`/MediaBox`) | **612 × 792 (Letter)** | **595 × 841,9 (A4)** |
| Font | Times New Roman thường và đậm, được nhúng (subset) | như bên |
| Chữ trích bằng pdf.js | đúng toàn bộ tiếng Việt | đúng toàn bộ tiếng Việt |
| Lề trái đo trên trang 2 | 105,7pt | 84,7pt (30mm = 85,0pt) |
| Kích thước file | 65.178 byte | 65.209 byte |

Kiểm bằng: pyHanko 0.38.1 (số trang, khổ, font nhúng), pdf.js 6.4.299 (trích chữ), và ảnh trang 1 do pdf.js vẽ
trong Edge chạy ẩn (nền màu được in, bảng và dấu tiếng Việt đúng).

Bằng chứng: `docs/spikes/evidence/S5/` (`css.pdf`, `settings.pdf`, `settings-trang1.png`).

### Lần chạy đầu thất bại

Bản đầu của spike in ngay ở sự kiện `NavigationCompleted` đầu tiên có nguồn `about:blank`. Đó là trang trống
ban đầu của webview, không phải HTML cần in: kết quả là PDF 864 byte, một trang trắng, trong khi
`PrintToPdf` vẫn báo thành công. Sửa bằng cách chỉ bắt đầu sau khi trang khởi đầu của ứng dụng tải xong.

## Phát hiện

1. **Khổ giấy phải truyền qua cài đặt in.** Không truyền cài đặt thì ra khổ Letter dù CSS ghi `size: A4`.
2. **Lề của CSS `@page` được tôn trọng** khi cài đặt in để lề 0. Vậy: khổ giấy lấy từ cài đặt in, lề lấy từ CSS.
3. **`PrintToPdf` báo thành công cả khi in nhầm trang trắng.** Bản sản phẩm phải kiểm kết quả (số trang, có
   chữ hay không) chứ không tin cờ thành công.
4. Một phiên `PrintToPdf` mất khoảng 0,12s sau khi webview sẵn sàng.

## Trả lời các câu hỏi của spike

- **`@page` có được tôn trọng không?** Lề: có. Khổ: không, phải đặt qua `PrintSettings`.
- **In nhiều tài liệu liên tiếp trên cùng một WebView2?** **Chưa thử** (spike in một tài liệu rồi thoát).
- **Font tiếng Việt có được nhúng đúng không?** Có, với Times New Roman.

## Chưa thử

- Nhiều tài liệu liên tiếp trên một webview; huỷ giữa chừng.
- HTML có ảnh (`NavigateToString` không có URL gốc nên ảnh đường dẫn tương đối không nạp được; cần `data:`
  hoặc `SetVirtualHostNameToFolderMapping`).
- HTML lớn hơn giới hạn 2MB của `NavigateToString`.
- Font không có trên máy; khổ khác A4; trang ngang.

## Rủi ro còn lại

R7 giảm. Việc cần làm ở giai đoạn 5: cơ chế nạp ảnh, hàng đợi in tuần tự, và kiểm kết quả sau khi in.
