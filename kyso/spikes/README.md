# Mã spike

Mã thử nghiệm của giai đoạn 0. **Không phải code sản phẩm**: không dùng lại nguyên trạng, không build cùng
workspace chính, không bị lint của workspace ràng buộc.

Quy ước:

- Mỗi spike một thư mục `spikes/<mã>-<tên>/`. Project Rust trong đó tự khai `[workspace]` rỗng để đứng riêng.
- Kế hoạch, kết quả và kết luận của spike ghi ở `docs/spikes/<mã>-*.md`.
- Kết quả chạy cần giữ làm bằng chứng thì chép vào `docs/spikes/evidence/<mã>/`.
- Thư mục `out/`, `data/`, `target/`, `bin/`, `obj/` không commit.
- Không để khoá bí mật, PIN, dữ liệu thật của khách hàng trong thư mục spike.

| Thư mục | Spike | Trạng thái |
|---|---|---|
| `s2-cms/` | S2: CMS + TSA + ghi nối bản | S2a xong: PASS theo pyHanko. Chạy `gen_pki.py` trước để có khoá thử |
| `s3-font/` | S3: subset font tiếng Việt trong Form XObject | S3a đã chạy: đạt 4/5 font, Georgia thiếu glyph. `verify/` cần `npm install` (pdf.js) |
| `s5-printpdf/` | S5: WebView2 PrintToPdf từ Tauri | Xong: PASS |
| `s7-word/` | S7: Word COM trên luồng STA | Xong: PASS |
| `s4-excel/` | S4: calamine so với ClosedXML | Xong: PASS với 3 file mẫu; ô số cần tự áp định dạng |
