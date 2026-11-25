# ADR 0001 (D1): Công nghệ giao diện

- **Trạng thái:** Đã chấp nhận, 18/11/2026 (master prompt bước 0.2).
- **Bối cảnh:** Cần giao diện giống prototype `App_Ky_So/sao-mai-ky-so_1.html`; nhóm đã quen Angular; bản cũ
  có sẵn logic kéo thả khung và xem PDF bằng pdf.js.
- **Quyết định:** Tauri 2 làm vỏ, lõi Rust, giao diện Angular 21 (standalone, signals) chạy trong WebView2.
  CSS lấy từ prototype, không dùng PrimeNG.
- **Phương án đã loại:** Tauri + Leptos (phải viết lại toàn bộ giao diện); Slint/egui (khó giống prototype,
  phải tự làm xem trước PDF).
- **Hệ quả:** Cần WebView2 Runtime trên máy người dùng (bộ cài tự cài). Mọi lời gọi lõi đi qua IPC có kiểu
  sinh từ Rust.
- **Ghi chú khảo sát:** crates.io đang có `tauri 3.0.0-alpha.6`; dự án ở lại nhánh 2.x ổn định. Máy phát
  triển hiện chưa build được Tauri (B-01). Prototype có chú thích nhắc `logo_sao_mai.rs`; xem
  `docs/implementation/PROTOTYPE_ANALYSIS.md`.
