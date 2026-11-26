---
phase: 8
title: "Cải tiến C2–C4, N3, N5 (theo số đo)"
status: pending
priority: P3
dependencies: [7]
---

# Phase 8: Cải tiến theo số đo

## Overview
User chốt làm tất cả; mỗi cải tiến bật sau khi **đạt ngưỡng số đặt trước** trên bộ mẫu thật (không chỉ trang tổng hợp); chưa đạt thì để sau tuỳ chọn.

## Requirements
- **C2** màu nền cục bộ (trung vị dải nền quanh vùng) — ngưỡng: lệch màu vá (ΔE trung bình so nền quanh) giảm ≥ 50%.
- **C3** lọc theo mảng liền (luật cụ thể: chạm mép trang / vùng, dài mảnh, lớn nhạt → xoá; cỡ / tỉ lệ / độ đậm giống ký tự → giữ) — ngưỡng: điểm chữ / bút chì / dấu nhạt bị mất giảm ≥ 50%, vết còn sót tăng ≤ 5%.
- **C4** tô nền chuyển dần — ngưỡng: user chấm trên trang so sánh trước / sau.
- **N3** thay ảnh tại chỗ (red team #7): chỉ khi ảnh **DeviceRGB / DeviceGray 8 bit**, dùng ở **đúng 1 trang**, vẽ 1 lần, không `/Decode` / `/Mask` / `/ImageMask` / JBIG2Globals, không trong Form XObject; còn lại (gồm 1 bit, CMYK, ICC) vẽ trang như cũ; dùng `PdfPage::replace_image` (mupdf 0.8) + lưu có dọn rác; trang đã xử lý bỏ `/Thumb`; ngưỡng: dung lượng ≤ gốc + 10% (đầu vào chỉ là PDF scan thuần ảnh, không có lớp OCR).
- **N5** model INT8 — ngưỡng: recall hộp ≥ 0,95 so FP32 và nhanh ≥ 1,3× trên Windows; đạt → model mới ở **tag Release mới** (không thay asset cũ), app mang bảng (URL, cỡ, SHA).
- Đo: trang tổng hợp (mặt nạ biết trước) + trang so sánh trước / sau (HTML) cho user chấm.

## Tests
Mỗi cải tiến: test đơn vị + số đo trên bộ mẫu; N3: PDF tài nguyên dùng chung 2 trang, `/Rotate` 90/180/270, ảnh lật, CMYK, `/Decode`, 1 bit → đều đi đường vẽ trang hoặc đúng.

## Success Criteria
- [ ] Mỗi cải tiến đạt ngưỡng của nó mới thành mặc định.
