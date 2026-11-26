---
phase: 10
title: "AI nhận diện layout (bảo vệ chữ) — gộp từ kế hoạch Xóa ghim"
status: completed
priority: P1
dependencies: [4, 7]
---

# Phase 10: AI nhận diện layout

## Overview
Gộp [phase 7 của kế hoạch Xóa ghim](../261125-1750-chuc-nang-xoa-ghim/phase-07-ai-bao-ve-chu.md) vào kế hoạch này để ô **Nhận diện layout** có tác dụng đúng như prototype:
- trang Gốc tô đỏ các khối chữ / bảng được bảo vệ (`NHAN_DIEN_TO`);
- vùng xoá thường không xoá vào khối đó;
- vùng **Vô đối** vẫn xoá đè lên khối;
- khung Đích là kết quả thật.

Tải / kiểm / chọn file mô hình đã làm ở [phase 7](./phase-07-cau-hinh-va-mo-hinh-ai.md) của kế hoạch này.

## Requirements (giữ nguyên các quyết định đã chốt trong kế hoạch cũ)
- **Mô hình**: `yolov12s-doclaynet.onnx` (AGPL — đã chấp nhận) qua `ort` rc.10 đã nhúng (không cần `onnxruntime.dll`). Đã thử nghiệm: nạp 0,05 s, ~0,27 s / trang (Mac).
- **Suy luận**:
  - Phiên dùng chung `OnceLock<Mutex<Session>>`; luồng ONNX 80% số nhân.
  - Khi AI bật, pool rayon Xóa ghim giới hạn ~½ số nhân.
  - Không gọi AI trên luồng giao diện.
  - Letterbox 1024 giữ tỉ lệ (nội suy song tuyến như `cv2.resize` mặc định), đệm 114, căn giữa, RGB / 255, CHW.
  - Hậu xử lý đầu ra `[1, 15, 21504]`: ngưỡng 0,1, NMS IoU 0,5 theo lớp. Nhóm lớp như `LABEL_MAPPING`; mặc định bảo vệ tiêu đề, chữ, bảng, chú thích, công thức.
  - **Luôn chạy AI** ở mọi trang có vùng áp dụng khi bật.
  - Hộp AI nhớ trong RAM theo (id PDF, trang), không lưu kho.
- **Ghép vào xoá ghim**:
  - Hộp AI + vùng bảo vệ tay nới 5 px rồi trừ khỏi vùng xoá.
  - Vùng xoá `VoDoi` bỏ qua bước trừ này, giữ đúng nghĩa đang có trong `xu_ly.rs`.
  - Xem trước và file xuất dùng **cùng** hàm → khớp từng điểm ảnh.
- **Hiển thị** (theo prototype):
  - Bật Nhận diện layout → trên trang Gốc tô `rgba(232,84,96,.30)` + viền `rgba(200,40,55,.75)` mỗi hộp AI, nằm **dưới** lớp vùng.
  - Đang tính → góc khung Đích ghi "đang cập nhật…" (đã có).
  - Tắt → không tô, không trừ.
- `TuyChon.nhan_dien` (phase 7) vào `ma_cau_hinh` → bật / tắt làm file đã xoá thành "đã đổi vùng".
- **Lô 10 trang** (đã chốt): bật = xử lý song song tối đa 10 trang / file (có AI: dùng chung phiên, giới hạn bởi pool ½ nhân).

## Đối chiếu với Python (giữ từ kế hoạch cũ)
Script Python xuất tensor letterbox, đầu ra thô và các hộp. Phía Rust kiểm 4 bước:
1. Cùng tensor đầu vào → so đầu ra thô (cho phép sai số float nhỏ).
2. So tensor letterbox Rust ↔ Python.
3. Ghép 1–1 các hộp tin cậy ≥ 0,25: recall / precision ≥ 0,95. Hộp 0,1–0,25 chỉ báo cáo.
4. Ảnh kết quả khi bật AI: IoU mặt nạ ≥ 0,98.

## Related Code Files
- Create: `dema4_rs/src/xoa_ghim/ai.rs`
- Modify: `xoa_ghim/xu_ly.rs` (nhận hộp bảo vệ thêm — tham số `bao_ve_them` đã có), `xoa_ghim/xuat.rs` (song song lô 10 trang, gọi AI), `gui_xoa_ghim/xem_truoc.rs` (trả hộp AI cùng ảnh), `gui_xoa_ghim/khung_trang.rs` (tô khối)
- Không đổi: `tach_vb/huong_trang.rs`

## Tests (viết trước)
- Letterbox, hậu xử lý, NMS trên tensor giả.
- Toạ độ ngược về trang dọc / ngang.
- Không mạng / không mô hình → app không lỗi, xoá ghim thường vẫn chạy.
- Bật / tắt AI đổi `ma_cau_hinh`.
- Vô đối xoá đè khối AI.
- Đo ms / trang ở 300 DPI và bộ nhớ đỉnh trên **Windows**, có / không lô 10 trang.

## Success Criteria
- [ ] Bật AI: chữ / bảng sát mép không bị xoá trên bộ mẫu; 4 bước đối chiếu đạt.
- [ ] Xem trước tô khối giống prototype; khung Đích = file xuất.
- [ ] Máy yếu Windows ~1 s / trang → hộp tiến độ hiện ước lượng thời gian.

## Risk Assessment
- Bộ nhớ khi song song 10 trang + AI: đo ở test hiệu năng; nếu vượt ngưỡng, giảm còn 4 trang khi AI bật (ghi trong mô tả tuỳ chọn).
- Giấy phép AGPL của mô hình: ghi NOTICE kèm mô hình tải về.
