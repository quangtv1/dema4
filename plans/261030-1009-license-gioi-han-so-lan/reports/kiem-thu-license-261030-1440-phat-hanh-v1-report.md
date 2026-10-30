# Kiểm thử license — phát hành v1.0.0 (30/10/2026)

Trạm: deployment `AKfycbyR90…` (@2), Monitor_dema4. Máy thử: Mac `SM-2145-6787-A9FB` (chủ Sheet đã cấp 100/100) + mã giả `SM-0000-0000-00A1` qua curl.

## Tự động
- `cargo test --release`: 164 xanh (KhoGia: hết lượt, offline, đặt chỗ, chống trùng, 1 luồng gửi, IP đọc lại, Khoá, xoá/sửa trả lời trên đĩa; kittest popup Đếm/Tách; CLI mã 3).
- Node `loi.test.js`: 9 xanh. `kho_tram_that` (gọi trạm thật): xanh. Chữ ký Apps Script ↔ crate `rsa`: `Verified OK` + test `chu_ky_tu_apps_script_kiem_duoc`.

## Ma trận (trạm thật)
| # | Kết quả | Cách kiểm |
|---|---|---|
| 1 | Đạt | Mac: thư mục rỗng → hoàn lượt; 2 lần Đếm → Sheet 2/2; lần 3 CLI in lý do + mã máy |
| 2 | Đạt | CLI `--tach` → `da_dung_tach` +1 trên trạm; chặn: test `het_luot_tach_hien_popup_khong_tach` |
| 3 | Đạt | Chủ Sheet đặt 100/100 Đã cấp → app chạy tiếp; kittest Kiểm tra lại → đếm chạy |
| 4 | Test đơn vị | `khoa_sau_lan_chay_ke`, `khoa_offline_van_khoa` — chưa thử tay trên Sheet |
| 5–6 | Đạt | curl: gửi lại mã lần cũ + 1 mã bù offline → đã dùng +1, không trùng |
| 7 | Test đơn vị | `thoat_giua_chung_van_bi_tinh` |
| 8 | Test đơn vị | `dung_*_luot` |
| 9 | Đạt | Quyết định không gọi mạng (`quyet_dinh_khong_goi_mang`); chống trùng qua mã lần |
| 10 | Thiết kế | Mã máy từ ID hệ điều hành (`ma_may_chi_tu_id_he_dieu_hanh`) |
| 11 | Đạt | CLI hết lượt in lý do + mã máy, mã thoát 3 |
| 12 | Đạt (sau sửa) | Sửa `han_dem` trong sổ trên đĩa: **trước sửa vẫn chạy được** → đã sửa (xem dưới) |
| 13 | Gửi | curl `thong_ke_tach` với `=IMPORTXML(...)`, `<a href>` → trạm `ok`; Sheet/Telegram chờ chủ Sheet xem |
| 14 | Đạt | Tin Đếm thử tới nhóm Telegram (chủ Sheet xác nhận) |
| 15, 16 | Chưa | Đổi chat_id / 30 tin/phút — không spam nhóm thật; Node test ngân sách 15 lời gọi |
| 17 | Thiết kế | Không có hẹn giờ gọi trạm |
| 18 | Test đơn vị | `ip_doc_lai_moi_loi_goi` |
| Windows | Chưa | Chỉ CI build + test; chưa chạy tay |

## Lỗi phát hiện khi chạy thật (đã sửa)
1. Tranh chấp đặt chỗ ↔ luồng gửi lúc mở app: lượt đang chạy bị gửi như bù offline (mất hoàn lượt). Sửa: đặt chỗ + đọc sổ cùng khoá `dang_chay`; nguồn "online" theo lần xong trong phiên.
2. Xoá/sửa câu trả lời ký trên đĩa → app coi như máy mới, chạy theo luật offline mãi (CLI mỗi lần khởi động). Sửa: chưa có trả lời ký hợp lệ + chưa hỏi trạm trong phiên → chờ trạm (≤ 20 s) rồi mới quyết định; mất mạng thật mới áp luật offline.
3. Thời gian chờ 8 s < trạm lúc nghỉ (~6,5 s + mạng) → nâng 15 s.

## Còn mở (chủ Sheet)
- Vô hiệu khoá service account cũ + gỡ khỏi chia sẻ Sheet (exe v0.5.x có khoá → sửa được tab `license`).
- BotFather `/revoke` token cũ, sửa ô `telegram_token`.
- Xem tình huống 13 trên Sheet (ô chữ thuần) + Telegram (không có link); thử tay 4 (Khoá), Windows.
- Giới hạn đã biết: mất mạng + xoá sổ lượt trên đĩa vẫn chạy được từng 2 lần (bản chất chế độ offline).
