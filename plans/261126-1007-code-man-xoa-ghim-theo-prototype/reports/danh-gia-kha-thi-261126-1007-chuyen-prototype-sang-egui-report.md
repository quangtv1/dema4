# Đánh giá khả thi: chuyển prototype màn Xóa ghim (HTML) sang Rust / egui 0.37

Ngày 26/11/2026 · Nguồn: `plans/261126-0727-prototype-man-xoa-ghim/prototype-xoa-ghim.html` (chốt) · Mã hiện tại: `dema4_rs/src/gui_xoa_ghim/*`, `xoa_ghim/*`, `gui.rs`, `gui_tach/hop_cau_hinh.rs`.

## Kết luận
**Khả thi.** Mọi thành phần và hành vi của prototype đều làm được bằng egui 0.37 + crate đang có (`egui_extras` svg, `rfd`, `ureq`, `mupdf`, `ort`), **không cần thư viện giao diện mới**. Về mặt bố cục, kích thước, màu, biểu tượng, chữ, hành vi: đạt **100%**, kiểm bằng checklist ở phase 9. Về mặt **điểm ảnh tuyệt đối**: không thể 100% vì bộ vẽ chữ của egui khác trình duyệt. Sai khác này nhỏ và không làm đổi bố cục.

Phần nặng nhất không nằm ở giao diện mà ở **lõi**:
1. Kiểu ghi "Cạnh file gốc", thư mục đích tuỳ chọn, Ghi đè: đổi cách tính đích, trạng thái và định dạng kho.
2. Nhận diện layout bằng AI: thuộc phase 7 của kế hoạch Xóa ghim, chưa làm.

## Bảng đối chiếu từng thành phần
| Thành phần prototype | Cách làm trong egui | Đã có sẵn? | Mức khó | Giống được |
|---|---|---|---|---|
| Thanh menu chung + dropdown | `MenuBar` + `MenuButton` (đang dùng) | Một phần (thiếu Tệp tin) | Thấp | 100% |
| Biểu tượng SVG | `egui_extras` svg + `include_image!` dùng **nguyên SVG** prototype | Feature svg đã bật | Thấp | 100% |
| Biểu tượng trang Góc / Cạnh / Tùy biến, bấm từng phần | `Painter` + `interact` từng rect | Có bản cũ (cần làm lại) | TB | 100% |
| Danh sách kéo rộng / thu nhỏ, chữ dọc | Panel trái + tay nắm tự làm; `TextShape.angle` | Panel có, phần còn lại mới | TB | 100% (chữ dọc có thể lệch baseline 1 px) |
| Thanh công cụ chia đôi thẳng ranh giới | tự tính bề rộng 2 nửa, chung hàm với khung xem | Mới | TB | 100% |
| Thanh trượt rãnh mảnh + nút tròn + ô số | `Slider` (`trailing_fill`, `handle_shape` tròn, visuals) + `DragValue` tuỳ biến; hiệu ứng phóng nút khi rê bằng `animate_value` | Một phần | TB | 98–100% (bóng nút vẽ tay) |
| Ô lọc gộp (chọn hoặc gõ) | `TextEdit` + nút ▾ + `Popup` | Mới | TB | 100% |
| Tooltip tức thì (kể cả nút mờ) | `on_hover_ui` + `tooltip_delay = 0` cục bộ hoặc vẽ vào `Order::Tooltip` | Mới | Thấp | 100% |
| 2 khung cuộn đồng bộ | 2 `ScrollArea` + `scroll_offset` | Đang là 1 khung chung → đổi | TB | 100% |
| Vừa ngang / vừa cao chính xác | tính từ `available_size` trong frame (egui không bị trễ như HTML) | Có bản cũ | Thấp | 100% |
| Vẽ vùng, 8 tay nắm, kéo, đổi cỡ | painter + `Sense::drag` (đang có) | Có (chọn 1) | TB | 100% |
| Chọn nhiều, Shift, Ctrl+A, quét Window / Crossing, kéo nhóm | máy trạng thái kéo, `Shape::dashed_line` | Mới | **Cao** (nhiều nhánh) | 100% |
| Menu chuột phải | `Response::context_menu` (đang dùng ở Tách) | Có mẫu | Thấp | 100% |
| Vô đối, phạm vi trang theo vùng chọn | `LoaiVung::VoDoi`, `ApTrang` đã có | Lõi có | Thấp | 100% |
| Hộp xác nhận, Cấu hình, tiến độ, F1 | `egui::Modal` (`khung_hop::hop`) | Có | Thấp | 100% |
| Toast | `Area` + `Order::Foreground` + hẹn giờ | Mới | Thấp | 100% |
| Logo Sao Mai → Trang chủ | texture logo có sẵn + `chuyen_man` | Có | Thấp | 100% |
| Lọc nâng cao theo khổ / chiều trang | đọc `bounds` + `/Rotate` từng trang ở luồng nền (mupdf) | Phân loại khổ có ở Đếm trang | TB | 100% |
| 4 trạng thái file | `xuat::TrangThai` đã có | Có | Thấp | 100% |
| Kiểu ghi Cạnh file gốc / thư mục đích / Ghi đè | `dich_cua()` mới, lọc `*_xoaghim.pdf`, trạng thái ghi đè qua `nguon_theo_ket_qua`, kho có phiên bản | **Chưa** | **Cao** (dữ liệu) | 100% |
| Tải mô hình AI (tiến độ, huỷ, SHA, chọn file) | `ureq` luồng nền + `sha2` (đã có trong cây qua `rsa`) | Chưa | TB | 100% |
| Tô đỏ khối chữ + bảo vệ khi Nhận diện layout bật | YOLO DocLayNet qua `ort` (đã nhúng) | **Chưa** — phase 7 kế hoạch Xóa ghim | **Cao** | 100% (sau khi AI xong) |

## Khác biệt không tránh được (sẽ ghi rõ khi bàn giao)
1. **Vẽ chữ**: egui dùng ab_glyph. Độ dày nét, khử răng cưa và kerning khác Chrome. Phông chọn giống nhau (Segoe UI trên Windows), cỡ giống nhau. Chiều rộng chữ có thể chênh vài %, nên chữ dài như đường dẫn có thể bị cắt "…" ở điểm hơi khác.
2. **Trang văn bản**: prototype dùng trang HTML giả. App hiển thị ảnh quét thật, còn Đích là kết quả xoá thật, không phải lớp phủ trắng mô phỏng.
3. **Bóng đổ / bo góc**: dùng `Shadow` và `CornerRadius` của egui, gần giống CSS, khác vài điểm ảnh ở mép mờ.
4. **Ô tick / radio**: vẽ theo kiểu egui, tô màu `#2563eb` cho giống. Hình dấu tick có thể khác Chrome một chút. Nếu cần trùng khít thì vẽ tay (thêm khoảng 0,5 ngày).

## Ước lượng
| Phần | Ngày công |
|---|---|
| Giao diện (phase 1–5, 7 UI, 8) | 8–9 |
| Lõi đầu ra + kho (phase 6) | 1,5 |
| Tải mô hình (phase 7) | 0,5 |
| Kiểm thử, đối chiếu, tài liệu (phase 9) | 1,5 |
| AI nhận diện (phase 10, gộp vào) | 3 |
| **Tổng kế hoạch này** | **≈ 14–16** |

## Rủi ro chính & cách giảm
| Rủi ro | Giảm thiểu |
|---|---|
| Đổi menu chung làm vỡ màn khác | Làm ở phase 1, chạy kittest các màn, mục Tệp tin mờ ở màn chưa hỗ trợ |
| Kho mới làm bản cũ đọc hỏng | Khối đuôi có phiên bản; test đọc ngược bằng bytes bản 2.5.0 |
| Ghi đè làm mất file gốc | Ghi tạm rồi đổi tên nguyên tử; cảnh báo trong Cấu hình; mặc định tắt |
| Máy trạng thái chọn / kéo phức tạp → lỗi khó thấy | Hàm thuần trong `sua_vung.rs` có test đơn vị; kittest mô phỏng chuột cho từng hàng của bảng hành vi |
| "Giống 100%" khó nghiệm thu | Checklist phase 9 + ảnh ghép cạnh nhau cho người dùng duyệt |

## Câu hỏi đã chốt (26/11/2026)
1. Lô 10 trang → song song tối đa 10 trang / file (mô tả dòng sửa thành "Nhanh hơn, tốn RAM hơn").
2. F1 → bảng thao tác nhanh đúng prototype.
3. AI → gộp vào kế hoạch này (phase 10, +3 ngày) → tổng ≈ 14–16 ngày công.
4. Phông Segoe UI chỉ màn Xóa ghim; menu Tệp tin ở đầu mọi module (chạy thật); thêm thẻ Xóa ghim trong Cấu hình.
