---
phase: 7
title: "AI bảo vệ chữ (ONNX, tải model lần đầu)"
status: pending
priority: P2
dependencies: [6]
---

# Phase 7: AI bảo vệ chữ

## Overview
Model `yolov12s-doclaynet.onnx` (user chấp nhận AGPL) qua ONNX Runtime đã nhúng (`ort` rc.10, liên kết tĩnh — không cần `onnxruntime.dll`). Đã thử: nạp 0,05 s, ~0,27 s / trang (Mac).

## Requirements
- **Model**: tải khi bật lần đầu từ Release riêng `mo-hinh-xoa-ghim-1` (repo công khai) bằng `ureq`; tiến độ, Huỷ; kiểm cỡ 37.544.909 B + SHA-256 `b5c0defa…7fe927a` lúc tải / chọn file (user chốt: chỉ kiểm lúc này); `.tmp` → đổi tên vào `<goc_du_lieu()>/mo_hinh/`; máy không mạng: "Chọn file mô hình…" hoặc chép tay. SHA dùng `rsa::sha2` đã có (không thêm crate).
- **Suy luận**: phiên dùng chung `OnceLock<Mutex<Session>>` (user chốt giữ), luồng ONNX 80% số nhân; khi AI bật, pool rayon Xóa ghim giới hạn ~½ số nhân; không gọi AI trên luồng UI.
  - Letterbox 1024 giữ tỉ lệ (song tuyến như `cv2.resize` mặc định), đệm 114, căn giữa, RGB / 255, CHW; hậu xử lý `[1, 15, 21504]`, ngưỡng 0,1, NMS IoU 0,5 theo lớp; nhóm lớp như `LABEL_MAPPING`; mặc định bảo vệ tiêu đề, chữ, bảng, chú thích, công thức.
  - **Luôn chạy AI** ở mọi trang có vùng áp dụng khi AI bật (user chốt — không bỏ qua trang).
  - Hộp AI nhớ **trong RAM** theo phiên (id PDF + trang); không lưu kho.
- **Ghép**: hộp AI + vùng bảo vệ tay nới 5 px → trừ khỏi vùng xoá (vùng xoá vô đối bỏ qua).
- Màn: ô "Bảo vệ chữ bằng AI" (tắt tới khi có model; hộp tải lần đầu); khung vùng chữ AI trên xem trước (bật / tắt).
- Bỏ (user chốt): máy chủ GPU từ xa, 4 bộ nhận diện cũ.

## Đối chiếu (red team #1)
- Script Python xuất **tensor letterbox** + đầu ra thô + hộp; Rust: (1) cùng tensor → so đầu ra thô (sai số float nhỏ); (2) so tensor letterbox Rust ↔ Python; (3) ghép 1-1 hộp tin cậy ≥ 0,25 (recall / precision ≥ 0,95; hộp 0,1–0,25 chỉ báo cáo); ảnh kết quả bật AI: IoU mặt nạ ≥ 0,98.

## Related Code Files
- Create: `dema4_rs/src/xoa_ghim/ai.rs`, `dema4_rs/src/xoa_ghim/mo_hinh.rs`
- Modify: `dema4_rs/src/xoa_ghim/xu_ly.rs`, `dema4_rs/src/gui_xoa_ghim.rs`; không đổi `tach_vb/huong_trang.rs`

## Tests (viết trước)
Letterbox / hậu xử lý / NMS trên tensor giả; toạ độ ngược trang dọc / ngang; kiểm model sai cỡ / sai SHA → từ chối, không để lại file; tải qua server cục bộ trong test; không mạng / không model → app không lỗi, xoá ghim thường chạy; đo ms / trang 300 DPI trên **Windows** + bộ nhớ đỉnh.

## Success Criteria
- [ ] Bật AI: chữ / bảng sát mép không bị xoá trên bộ mẫu; đối chiếu đạt.

## Risk Assessment
- AGPL: user chấp nhận. Máy Windows yếu ~1 s / trang → hiện ước lượng thời gian.
- Pool ½ số nhân + phiên chung có khoá: luồng chờ AI vẫn giữ ảnh trang → theo dõi bộ nhớ ở test đo (user giữ cách này).
