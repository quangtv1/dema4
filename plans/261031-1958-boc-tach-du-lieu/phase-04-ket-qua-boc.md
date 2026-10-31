---
phase: 4
title: "Tab Kết quả bóc + chạy hàng loạt + popup xong"
status: completed
priority: P1
dependencies: [3]
---

# Giai đoạn 4: Tab Kết quả bóc

## Yêu cầu (theo prototype `#boc-tach-ket-qua`, `-dang-chay`, `-loi`, `-loc`, `-gon`, `-xong`)
- Sidebar: # | Tên file | Trang | Trường ("x/6", "Đọc…" xanh, "-", "Lỗi" đỏ); vạch tiến độ mảnh dưới tên file đang bóc; kéo độ rộng cột; ô tìm lọc theo tên file hoặc cột Trường (lỗi, 5/6, đọc, -); nút thu gọn sidebar bên phải ô tìm (thu còn cột #); kéo thanh chia rộng/hẹp (tối thiểu 200 px).
- Chi tiết: thanh 30 px "‹ n/N › tên_file · Đã đọc: 2,4 s" (chữ thường) · − 100% + · Lưu. Hai vùng PDF | các trường, kéo ranh giới. Ô nền trắng; đang bóc: ô hiện vòng xoay nhỏ + "Đang bóc" (12 px, xám, `egui::Spinner`); chưa đọc: ô trống. Sửa tay → bật Lưu; Lưu ghi lại Excel + Sheet (dòng của file đó); chuyển file khi chưa Lưu = bỏ.
- Bảng "Kết quả bóc" đáy: # | Tên file | Số trang | các trường; "Đang bóc…", "Lỗi: …" đỏ, ô sửa tay màu nâu; lọc theo ô tìm; header ghi "Lọc …: x/N file"; mép phải "Mở Excel: <tên>.xlsx (xám khi chưa có) · Mở GSheet"; kéo cột / chiều cao / thu gọn như dải đọc thử.
- Chạy: Bắt đầu = mọi file; Chạy lại (n) = các file đang hiện (lọc hay không); danh sách giữ cố định trong lúc chạy; sidebar + bảng tự cuộn dòng đang bóc vào giữa; chi tiết đi theo file đang bóc; Dừng = sau file đang đọc.
- Thanh trạng thái: "Đang đọc k/n file · đã bóc X trường · thời gian đã bóc hh:mm:ss · dự kiến còn ~"; xong: "Đã bóc tách a/N file (b lỗi) · X/Y trường · thời gian đã bóc".
- Popup "Đã bóc tách xong": Văn bản, Trang đã đọc, Trường, Mô hình đọc, Thời gian (TB/file), Ghi ra; "Thông tin bóc tách đã được ghi lại"; Xem file lỗi (lọc sẵn "lỗi") · Mở Excel · Mở GSheet · Đóng.

## Kiến trúc
- `src/gui_boc_tach/ket_qua.rs` (sidebar + chi tiết + bảng đáy), `src/gui_boc_tach/hop_xong.rs`.
- Trạng thái file `TrangThaiFile { Cho, DangBoc, Xong(KetQua), Loi(String) }` + bản sửa tay; kết quả ghi `jsonl` trong `.sm-dema4/boc_tach/<nguồn>/` sau mỗi file (nguồn sự thật, dựng lại Excel được).
- License: `bat_dau(ChucNang::BocTach)` khi Bắt đầu / Chạy lại; hoàn lượt khi lỗi trước file đầu (như Sinh mục lục). **Dời sang giai đoạn 6** cùng trạm (`TEN_CN` thêm `boc_tach`) — trạm phải triển khai trước app.
- Đã làm (2026-11-01): `boc_tach/luu.rs` (jsonl, bản ghi sau thắng, chịu dòng ghi dở), `gui_boc_tach/ket_qua.rs`; Mở Excel / Mở GSheet chờ giai đoạn 5; câu "Thông tin bóc tách đã được ghi lại" đã có trong popup, ghi thống kê thật ở giai đoạn 6.

## Các bước
1. Sidebar + lọc + thu gọn + kéo cột.
2. Chi tiết + Lưu sửa tay.
3. Bảng đáy.
4. Chạy / Dừng / Chạy lại theo lọc + tự cuộn + thanh trạng thái.
5. Popup xong.
6. Test: luồng chạy với mô hình giả (trả lỗi cho 2 file đầu lần 1, thành công lần chạy lại), lọc, Lưu; ảnh chụp các hash.

## Tiêu chí xong
- [ ] Đúng hành vi chạy lại theo lọc, đếm số trường / lỗi đúng.
- [ ] Tắt app giữa chừng → mở lại thấy kết quả đã đọc (từ jsonl).
