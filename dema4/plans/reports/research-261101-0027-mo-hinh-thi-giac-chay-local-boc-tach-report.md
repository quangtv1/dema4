# Mô hình thị giác chạy local cho Bóc tách dữ liệu

Ngày: 2026-11-01. Mục tiêu: chạy trên máy văn phòng (nhiều máy chỉ CPU, 8–16 GB RAM), đọc văn bản hành chính tiếng Việt, trả JSON các trường (loại, số ký hiệu, ngày, tác giả, trích yếu, người ký).

## Kết luận ngắn
- App không cần đổi kiến trúc: chạy mô hình qua **Ollama / LM Studio / llama.cpp server** (đều có địa chỉ chuẩn OpenAI `http://localhost:…/v1`) → dùng ô "Máy chủ nội bộ" sẵn có. Không nhúng mô hình vào app (file vài GB, cần cập nhật riêng).
- Ứng viên chính: **Qwen3-VL-4B** (cân bằng chính xác / kích thước, có sẵn trong Ollama). Máy yếu: **Qwen3-VL-2B** hoặc **Vintern-1B-v3.5** (chuyên tiếng Việt, rất nhẹ). Có GPU: Qwen3-VL-8B.
- Chưa có benchmark độc lập nào trên văn bản hành chính Việt Nam → **phải tự đo** trên bộ mẫu của đơn vị trước khi chọn mặc định (đề xuất ở cuối).

## Ứng viên

| Mô hình | Cỡ | Điểm công bố (nguồn) | Tiếng Việt | Chạy local | Ghi chú |
|---|---|---|---|---|---|
| Qwen3-VL-4B | ~4B, Q4 ≈ 3 GB | OCRBench 873, DocVQA ~91–95 (báo cáo kỹ thuật / Qianfan-OCR, số liệu bên thứ ba lệch nhau) | Tốt (đa ngữ) | Ollama `qwen3-vl` (2b/4b/8b) | Làm theo chỉ dẫn, trả JSON tốt — hợp bài toán trích trường |
| Qwen3-VL-2B | ~2B, ≈ 1.9 GB | Thấp hơn 4B (OCRVQA 59.3 vs 64.7) | Khá | Ollama | Cho máy chỉ CPU |
| Vintern-1B-v3.5 (5CD-AI) | 1B | vi-MTVQA 41.9, DocVQA 78.8, OCRBench 706 (thẻ mô hình — tác giả tự đo) | Chuyên tiếng Việt, văn bản VN | GGUF cộng đồng (dekthedev) + mmproj, chạy llama.cpp / Ollama | Nhẹ nhất; kém Qwen3-VL ở bài chung; cần xác nhận bản GGUF cho kết quả như bản gốc |
| Gemma 4 E2B / E4B | 2B / 4B (4/2026, Apache 2.0) | Google: "excelling at OCR"; chưa có số độc lập | Đa ngữ | Hỗ trợ ảnh trong Ollama chưa xác nhận | Theo dõi, thử khi Ollama hỗ trợ ổn |
| PaddleOCR-VL 0.9B (1.5/1.6) | 0.9B | Dẫn đầu OmniDocBench (blog, chưa kiểm chứng) | Có (109–111 ngôn ngữ) | Paddle / GGUF cộng đồng | Mạnh **chuyển trang thành chữ**, không phải trích trường theo câu lệnh → cần bước 2: chữ → LLM chữ nhỏ (Qwen3 1.7B/4B) |
| InternVL3.5-1B/2B, MiniCPM-V | 1–8B | — | Đa ngữ | GGUF có | Dự phòng nếu Qwen3-VL không hợp |

## Hai cách dùng
1. **Một bước (khuyên dùng):** ảnh trang → VLM → JSON. Khớp kiến trúc app hiện tại, câu lệnh và loại tài liệu dùng chung với ChatGPT / Gemini / Claude.
2. **Hai bước:** PaddleOCR-VL (chữ đầy đủ) → LLM chữ nhỏ trích trường. Chữ chính xác hơn với bản scan xấu, nhưng 2 mô hình, chậm hơn, thêm việc. Chỉ làm nếu đo thấy cách 1 sai nhiều ở số ký hiệu / ngày.

## Tốc độ (ước lượng, cần đo)
- 1–2B trên CPU 8 lõi: vài giây đến ~15 s / trang. 4B trên CPU: ~20–60 s / trang. GPU 6–8 GB: 1–3 s / trang. Bóc tách đọc 1–2 trang / văn bản → hàng nghìn văn bản trên CPU là chạy qua đêm.
- Giảm DPI (150) / cạnh tối đa (1280 px) cho mô hình nhỏ để nhanh hơn — cho phép chỉnh trong Cấu hình › Nâng cao (đã có).

## Đề xuất bước tiếp
- Thêm vào giai đoạn 1 một **công cụ đo** (lệnh ẩn): chạy bộ ~50 trang thật đã gán nhãn tay (không đưa vào repo) qua từng mô hình + lượng tử hoá, in độ chính xác từng trường + giây/trang. Chạy với Qwen3-VL 2B/4B/8B, Vintern-1B (GGUF), Gemma 4 E4B (nếu Ollama hỗ trợ).
- Chọn mặc định theo số đo; ô "Nội bộ" trong Cấu hình gợi ý sẵn `http://localhost:11434/v1` + model khuyên dùng.
- Kiểm tra giấy phép từng mô hình trước khi khuyên dùng chính thức (Qwen3-VL, Gemma 4, PaddleOCR-VL: Apache 2.0 theo trang công bố; Vintern: xem thẻ mô hình).

## Nguồn
- https://huggingface.co/5CD-AI/Vintern-1B-v3_5
- https://huggingface.co/dekthedev/Vintern-1B-v3_5-GGUF
- https://huggingface.co/5CD-AI/Vintern-3B-beta
- https://arxiv.org/pdf/2511.21631 (Qwen3-VL Technical Report)
- https://arxiv.org/pdf/2603.13398 (Qianfan-OCR — bảng so Qwen3-VL)
- https://codersera.com/blog/qwen3-vl-4b-vs-qwen3-vl-8b-benchmarks-vram-guide/
- https://arxiv.org/pdf/2510.14528 (PaddleOCR-VL), https://arxiv.org/pdf/2601.21957 (PaddleOCR-VL-1.5)
- https://www.docsumo.com/blog/best-ocr-models, https://roboflow.com/blog/best-open-source-ocr-models
- https://simonwillison.net/b/9404 (Gemma 4)
- https://arxiv.org/html/2508.13680v1 (ViExam)

## Câu hỏi còn mở
- Máy người dùng phổ biến có GPU không? Quyết định mặc định 2B hay 4B.
- Có bộ ~50 trang thật để gán nhãn đo không (giữ ngoài repo)?
