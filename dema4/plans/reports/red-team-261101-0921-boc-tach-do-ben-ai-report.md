# Red-team: độ bền pipeline AI của "Bóc tách dữ liệu"

Ngày: 2026-11-01 · Phạm vi: `dema4_rs/src/boc_tach/{anh,cau_lenh,mo_hinh,doc_json,chuan_hoa,chay,loai_tai_lieu}.rs`, `src/gui_boc_tach/xem_pdf.rs`, `src/gui_boc_tach.rs::quet_pdf`, `src/cau_hinh.rs` (giới hạn bóc tách). Chỉ báo cáo, không sửa code.

Cách kiểm chứng: đọc code, cộng 1 test tạm thêm vào cuối `chuan_hoa.rs` chạy `cargo test --release` (đã gỡ lại; md5 trùng với bản sao lưu, `git status` không có thay đổi trong `dema4_rs/src`). Nhãn **[đã chạy]** = có kết quả test thật, **[đọc code]** = chắc chắn khi đọc code, **[có thể xảy ra]** = phụ thuộc hành vi của máy chủ hoặc dữ liệu thật.

## Tóm tắt

Phần render ảnh và client HTTP làm khá cẩn thận: không có lỗi panic hay lỗi an toàn luồng nào kích hoạt được. Rủi ro thật nằm ở **tầng chuẩn hoá và gộp kết quả**. Ở đó có 4 lỗi cho ra **giá trị sai nhưng nhìn hợp lệ**: loại văn bản sai, ngày bị đảo ngày/tháng, mẫu câu lệnh bị chép lại thành dữ liệu, trường lấy nhầm trang. Người dùng không nhận ra được, nên các lỗi này nguy hiểm hơn lỗi văn bản báo rõ. Thêm 2 lỗi ở tầng vận hành: một trang không trả JSON làm hỏng cả file, và `max_tokens` dùng chung cho mọi nhà cung cấp làm vượt ngữ cảnh 8192 của Vintern.

## Các phát hiện (xếp theo mức độ)

### 1. [High] Danh sách chọn "giá trị dài nhất là chuỗi con" nên ra loại văn bản sai **[đã chạy]**
- `chuan_hoa.rs:56` (`khop_danh_sach`): khi không khớp đúng, hàm lọc mọi giá trị là chuỗi con của câu trả lời rồi lấy giá trị **dài nhất**. `dang_so` còn bỏ cả khoảng trắng nên khớp được qua ranh giới giữa các từ.
- Đầu vào → đầu ra thực tế:
  - `"BÁO CÁO Kết quả thực hiện kế hoạch năm 2015"` → `Kế hoạch` (đúng phải là Báo cáo)
  - `"Tờ trình về việc ban hành quyết định"` → `Quyết định` (đúng phải là Tờ trình)
  - `"Dự thảo quyết định"` → `Quyết định`
- Mô hình nhỏ thường trả cả dòng tên loại kèm trích yếu, nên lỗi này gặp thường xuyên.
- Sửa: chọn giá trị xuất hiện **sớm nhất** (theo vị trí của `k.find(..)`, nếu hoà thì lấy dài hơn). Tốt hơn nữa là chỉ chấp nhận giá trị là **tiền tố** của `k`, còn lại để trống để người dùng kiểm tra.

### 2. [High] `ngay()` lấy nhầm bộ 3 số có năm đứng đầu nên đảo ngày/tháng **[đã chạy]**
- `chuan_hoa.rs:63-70`: duyệt các cửa sổ 3 số liên tiếp, và với mỗi cửa sổ thì thử thêm dạng `yyyy m d` **trước khi** sang cửa sổ sau.
  - `"Số 12/2015/QĐ-UBND ngày 3/4/2015"` → `04/03/2015` (đúng: 03/04/2015)
  - `"Căn cứ Nghị định 30/2020/NĐ-CP ngày 05/03/2020"` → `03/05/2020` (đúng: 05/03/2020)
- Kết quả có định dạng hợp lệ nên vượt qua bước Kiểm tra.
- Sửa: duyệt hết các cửa sổ để tìm dạng `d m yyyy` trước, chỉ quay về dạng `yyyy m d` khi không có. Dạng ISO thì chỉ nhận khi giữa các số là `-` (ví dụ dùng regex `\d{4}-\d{1,2}-\d{1,2}`).

### 3. [High] Mô hình chép lại mẫu JSON nên mô tả trường thành "dữ liệu", riêng Danh sách thì ra luôn "Quyết định" **[đã chạy]**
- `cau_lenh.rs:25` đặt mô tả trường làm **giá trị** của mẫu JSON. Với trang trắng, mờ hoặc trang phụ lục, mô hình 1B hay trả nguyên mẫu. Kết quả chuẩn hoá thực tế:
  - `loai_van_ban` = `"Loại văn bản ở trang đầu, chỉ chọn một trong danh sách: Quyết định | Công văn | …"`. `bo_nhan_lap` bỏ phần trước dấu `:`, rồi `khop_danh_sach` chọn chuỗi dài nhất, ra **`Quyết định`**.
  - `trich_yeu` → `"văn bản về việc gì (dòng dưới tên loại văn bản)"`
  - `so_ky_hieu`, `ngay_thang`, `nguoi_ky` giữ nguyên câu mô tả.
- Các trường này có giá trị nên ô "x/6" vẫn báo đủ và bước Kiểm tra không bắt được.
- Sửa (nhỏ): trong `chuan_hoa`, nếu `dang_so(s)` trùng hoặc là tiền tố của `dang_so(mo_ta_json(mo_ta))`, hoặc chứa ≥2 giá trị của danh sách (có ký tự `|`), thì trả rỗng. Có thể bỏ luôn ví dụ `123/QĐ-UBND` trong mô tả có sẵn vì mô hình cũng hay chép lại ví dụ này **[có thể xảy ra]**.

### 4. [High] Một trang không trả JSON làm cả file lỗi và mất luôn câu trả lời thô **[đọc code]**
- `chay.rs:76`: `theo_trang.push(m.ok_or_else(..))?` thoát ra ngay. Kết quả các trang đã đọc, kể cả lượt gọi API đã trả phí, và `tho` đều bị bỏ, nên log không còn câu trả lời thô để soát lại. `chay.rs:72` (`jpeg_trang`) cũng vậy: chỉ một trang bị mã hoá hoặc hỏng là cả file hỏng.
- Ví dụ: loại "Hợp đồng" (`TatCa`) có 30 trang, Vintern lặp vô hạn ở trang 17 nên không có JSON. Kết quả: mất 29 trang tốt, file báo lỗi.
- Sửa: trang lỗi coi là map rỗng (vẫn ghi `tho` kèm ghi chú lỗi). Chỉ trả `Err` khi **mọi** trang đều lỗi. Với `DauCuoi` thì chỉ trả `Err` khi trang đầu lỗi.

### 5. [High] `max_tokens` dùng chung cho mọi nhà cung cấp (tối đa 16384), vượt ngữ cảnh 8192 của Vintern **[đọc code + có thể xảy ra]**
- `cau_hinh.rs:139` `TOKEN_KHOANG = (256, 16384)` là một giá trị toàn cục (`cau_hinh.rs:151,205`). Người dùng tăng lên 4096–8192 để Gemini 2.5 hoặc GPT-5 có chỗ suy luận, rồi chuyển sang Nội bộ. Khi đó tổng `ảnh InternVL (tới 13×256 ≈ 3.3k token) + câu lệnh (~0.4k) + max_tokens > 8192`, và vLLM trả 400 `"maximum context length is 8192 tokens…"`.
- Thông báo này không có chữ "image" nên rơi vào `LoiDoc::May(400)` (`mo_hinh.rs:241`). Lỗi này không kích hoạt `DungSom` (`chay.rs:130-136`), nên **mọi file** lần lượt báo lỗi.
- Cùng hệ quả khi máy chủ vLLM chạy với `--max-model-len 4096` để tiết kiệm VRAM: mặc định 2048 + ~3.7k đã vượt.
- Sửa: lưu `max_tokens` theo từng nhà cung cấp (Nội bộ mặc định 512–1024 là đủ cho JSON 6 trường). Hoặc khi gặp 400 chứa `context length` / `max_tokens` thì thử lại một lần với `max_tokens/2`.

### 6. [Medium] Xử lý 429 yếu với Gemini/OpenRouter gói miễn phí **[có thể xảy ra]**
- `mo_hinh.rs:228` chỉ đọc `Retry-After` dạng số giây, bỏ qua dạng HTTP-date. Gemini đặt thời gian chờ trong thân phản hồi (`details[].retryDelay: "37s"`), không đặt ở header. Hệ quả: chờ 2+4+8 = 14 s rồi trả `QuaTai` (`chay.rs:43`), mà gói miễn phí giới hạn theo phút nên lần thử nào cũng còn trong cửa sổ bị chặn.
- `QuaTai` không dừng hàng đợi: luồng chuyển ngay sang file kế tiếp và tiếp tục gọi, nên hàng loạt file bị đánh "quá tải".
- Gemini OpenAI-compat có lúc trả thân lỗi dạng **mảng** `[{"error":{…}}]`. Khi đó `v["error"]["message"]` (`mo_hinh.rs:232`) ra rỗng và chỉ còn 200 ký tự thô, có thể cắt mất chữ "temperature" hoặc "image" mà nhánh nhận diện lỗi cần.
- Sửa: đọc thêm `retryDelay` trong thân phản hồi (cả dạng `v[0]`) và HTTP-date (`httpdate`, hoặc bỏ qua dạng này nhưng tăng thời gian chờ dự phòng lên 15/30/60 s). Dừng hàng đợi khi có N file `QuaTai` liên tiếp. Chuẩn hoá `v = v[0]` nếu `v` là mảng.

### 7. [Medium] Câu trả lời bị cắt do `finish_reason=length` nhưng vẫn có nội dung bị báo là "không trả JSON" **[đọc code]**
- `mo_hinh.rs:261-264` (`het_token`) chỉ báo lỗi hết token khi nội dung **rỗng**. Trường hợp hay gặp nhất là JSON bị cắt giữa chừng (Vintern lặp chữ, hoặc mô hình suy luận dùng gần hết ngân sách). Khi đó trả `Ok`, rồi `chay.rs:76` báo "Mô hình không trả JSON ở trang t", nên người dùng không biết phải tăng hay giảm token.
- Sửa: trả cờ `het` cùng nội dung, hoặc trong `het_token` khi `het && doc_json::doc(&s).is_none()` thì trả thông báo hết token.

### 8. [Medium] `lay_trang` so chuỗi "trang cuối" nguyên văn nên hỏng với NFD và với cách viết khác **[đã chạy]**
- `loai_tai_lieu.rs:88`: `to_lowercase().contains("trang cuối")`. Mô tả gõ bằng Unikey ở chế độ "Unicode tổ hợp" (NFD) cho ra `LayTrang::Dau` (đã chạy test với mô tả `nguoi_ky` ở dạng NFD). Cách viết "cuối văn bản" / "trang sau cùng" cũng không khớp.
- Hệ quả: với "Ngày ký … ở trang cuối" của Hồ sơ đất đai, trang đầu thường cũng có ngày (ngày cấp, ngày đơn), nên lấy nhầm ngày trang đầu.
- Sửa: `dang_so(&mo_ta).contains("trangcuoi")`. Hàm này đã bỏ dấu tổ hợp và khoảng trắng, nên xử lý được cả NFD lẫn chữ hoa.

### 9. [Medium] `timeout_global` tính cả upload và sinh chữ; lỗi mạng thử lại gây trả phí 2 lần và có thể dừng hàng đợi nhầm **[có thể xảy ra]**
- `mo_hinh.rs:123`: một giới hạn thời gian cho cả request (mặc định đám mây 60 s). Ảnh 4096 px chất lượng 100 cỡ 8–10 MB, base64 lên 11–13 MB. Qua đường lên 2 Mbps riêng phần upload đã khoảng 50 s. Mô hình suy luận (gemini-2.5-pro, gpt-5) cũng hay vượt 60 s.
- Hết giờ thì ra `Mang`, `goi` gửi lại ngay (`chay.rs:53`) trong khi máy chủ vẫn đang tính, nên trả phí 2 lần. Sau 3 file liên tiếp như vậy thì `DungSom` báo "Mất kết nối", sai nguyên nhân (thực ra là chậm).
- Sửa: phân biệt lỗi hết giờ (`ureq::Error::Timeout`) với lỗi không kết nối. Khi hết giờ thì không thử lại, báo "quá thời gian chờ X s — tăng Thời gian chờ hoặc giảm Cạnh tối đa".

### 10. [Medium] Anthropic: ảnh lớn hơn 5 MB bị từ chối, không tự thu nhỏ **[có thể xảy ra]**
- `cau_hinh.rs:137-138` cho phép `canh_toi_da` tới 4096 và JPEG tới 100. Trang scan màu ở mức này dễ vượt 5 MB. Anthropic trả 400 `"image exceeds 5 MB maximum"`, thành `AnhKhongNhan` (`mo_hinh.rs:240`), và mọi file lớn đều lỗi. Claude còn tự thu cạnh dài về khoảng 1568 px, nên gửi trên 2048 px chỉ tốn băng thông và thời gian.
- Sửa: với Claude, giới hạn cạnh ≤1568–2048. Nếu JPEG lớn hơn khoảng 3.7 MB (vì base64 ×1.33) thì mã hoá lại với chất lượng thấp hơn hoặc cạnh nhỏ hơn trước khi gửi.

### 11. [Medium] Khung xem PDF: chỉ co ảnh theo bề rộng, cache không bao giờ giải phóng, trang dài có thể vượt giới hạn texture **[có thể xảy ra]**
- `xem_pdf.rs:187`: `s = bac / width`, không giới hạn chiều cao. Ở 300% trên màn hình 2x thì `bac` = 4096. Bản vẽ dạng dải có tỉ lệ cao/rộng ≥2 cho ra ảnh 4096×≥8192. Với GPU có `max_texture_side` 8192, `load_texture` (`xem_pdf.rs:84`) gặp lỗi wgpu validation hoặc panic. Dải 100×14400 pt cho pixmap khoảng 7 GB, nên hết bộ nhớ.
- `self.anh` (`xem_pdf.rs:33`) giữ mọi bậc ảnh của mọi trang đã xem tới khi đổi file. Mỗi bản A0 ở 4096 px tốn khoảng 95 MB RGBA, nên cuộn và đổi thu phóng trên file nhiều trang làm RAM tăng liên tục.
- Nếu luồng `luong_ve` panic thì luồng chết im lặng: `cho` không bao giờ được xoá, và giao diện vẽ lại mỗi 30 ms mãi mãi.
- Sửa: `s = min(bac/w, MAX_SIDE/h)` với `MAX_SIDE = ctx.input(|i| i.max_texture_side)`. Khi chèn bậc mới thì xoá các bậc khác của cùng trang.

### 12. [Medium] `doc_json::doc` lấy object đầu tiên bất kể có chứa khoá mong đợi hay không **[đã chạy]**
- `doc_json.rs:7-11`: `{"result": {"so_ky_hieu": "1/QĐ"}}` ra map chỉ có khoá `result`, nên mọi trường rỗng nhưng **không báo lỗi**. Câu trả lời dạng `Mẫu: {…mẫu…} Kết quả: {…}` thì lấy object mẫu.
- Sửa: truyền danh sách `bien` vào hàm. Ưu tiên object (ở cấp đầu hoặc lồng 1 cấp) chứa nhiều khoá trùng nhất. Không object nào có khoá trùng thì coi như không có JSON.

### 13. [Low] "Viết hoa chữ đầu" làm hỏng tên tổ chức viết tắt **[đã chạy]**
- `chuan_hoa.rs:76-84`, áp cho trường có sẵn "Người sử dụng đất" (mô tả ghi "người / tổ chức"): `"CÔNG TY TNHH ABC; UBND XÃ Y"` → `"Công Ty Tnhh Abc; Ubnd Xã Y"`.
- Sửa: giữ nguyên các từ toàn chữ hoa dài ≤5 ký tự có trong danh sách viết tắt (TNHH, UBND, HĐND, CP, MTV…), hoặc đổi trường có sẵn này sang `GiuNguyen`.

### 14. [Low] Các trường hợp biên của `ngay()` **[đã chạy]**
- `chuan_hoa.rs:64`: `parse::<u32>().ok()?` thoát cả hàm khi gặp số dài (mã hồ sơ hoặc số điện thoại ≥10 chữ số), nên `"99999999999 6/4/2015"` được giữ nguyên, không chuẩn hoá. Sửa: dùng `continue` thay cho `?`.
- `31/02/2015` được coi là hợp lệ. Năm 2 chữ số `6/4/15` không chuẩn hoá. Mô hình trả kiểu Mỹ `04/06/2015` không phân biệt được (cần nhắc rõ trong câu lệnh, việc này không sửa được ở bước chuẩn hoá).

### 15. [Low] Các lỗi cấu hình chắc chắn lặp lại vẫn chạy hết hàng đợi; vài chi phí lặp **[đọc code]**
- `chay.rs:130-136`: chỉ `Khoa` và 3 lần `Mang` mới dừng hàng đợi. Các lỗi như 404 (sai model hoặc sai địa chỉ), `AnhKhongNhan "At most 1 image(s)"`, 400 vượt ngữ cảnh (#5) khiến hàng nghìn file lần lượt báo lỗi giống hệt nhau. Sửa: dừng khi K file liên tiếp có cùng `(mã, thông báo)`.
- `mo_hinh.rs:195`: model không nhận `temperature` thì **mỗi trang** tốn 2 request. Sửa: nhớ cờ `bo_temperature` (AtomicBool) trong `KetNoi`.
- `noi_dung_openai` (`mo_hinh.rs:245-254`): `finish_reason = "content_filter"` (Gemini chặn vì an toàn hoặc RECITATION trên giấy tờ có thông tin cá nhân) chỉ báo chung "Mô hình không trả nội dung". Sửa: đưa `finish_reason` vào thông báo.
- `gui_boc_tach.rs:688`: `quet_pdf` mở **tuần tự** mọi PDF để đếm trang, không huỷ được. Với thư mục trên Google Drive (CloudStorage) có vài nghìn file, mỗi lần mở có thể kéo file về, nên bước quét kéo dài nhiều phút.

## Đã kiểm tra, không phải lỗi
- **An toàn luồng của mupdf**: `mupdf 0.8` dùng context `thread_local!` (`context.rs:28`) và `Document` không phải `Send`. Mỗi luồng (hàng đợi, `luong_ve`, `quet_pdf`) tự mở Document riêng, và trình biên dịch bắt buộc điều này.
- **Tràn u16 của jpeg-encoder**: `CANH_KHOANG` ≤4096 và `s` đã bị giới hạn theo cạnh dài (`anh.rs:25`), nên `as u16` không tràn. Trang có kích thước 0 thì jpeg-encoder trả lỗi, không panic.
- **Stride / alpha của pixmap**: `alpha=false` + `device_rgb` cho `n == 3`. mupdf cấp phát `stride = w·n` không đệm, nên `&rgb[..w*h*3]` an toàn.
- **Trang xoay**: `bounds()` và `to_pixmap` của mupdf đã tính `/Rotate`. Ảnh scan bị nghiêng hoặc xoay mà không có `/Rotate` là giới hạn của dữ liệu, không phải lỗi code.
- **PDF 0 trang / hỏng / có mật khẩu**: `cac_trang(0)` rỗng thì báo "PDF không có trang nào". Trang không mở được thì báo lỗi rõ (nhưng xem #4).
- **`to_string_lossy` trên Windows**: chỉ hỏng khi đường dẫn có surrogate lẻ (gần như không gặp). Tên tiếng Việt vẫn là UTF-8 hợp lệ.
- **`bo_chu_so` với "Sở…" hoặc NFD**: so sánh nguyên văn "số"/"so". Với NFD, phần còn lại bắt đầu bằng dấu tổ hợp nên không bị cắt (thất bại an toàn).

## Thứ tự sửa đề xuất
1. #1, #2, #3 (chuẩn hoá cho giá trị sai mà người dùng không nhận ra), mỗi lỗi chỉ cần sửa vài dòng trong `chuan_hoa.rs`, kèm test như ví dụ ở trên.
2. #4, #7 (`chay.rs`, `mo_hinh.rs`): giữ kết quả từng phần và thông báo lỗi đúng nguyên nhân.
3. #5, #8: `max_tokens` theo từng nhà cung cấp, `lay_trang` dùng `dang_so`.
4. Các mục còn lại theo nhà cung cấp đang dùng thật.

## Câu hỏi chưa giải quyết
- vLLM nội bộ chạy với `--max-model-len` và `max_dynamic_patch` bằng bao nhiêu? Hai thông số này quyết định #5 có xảy ra ngay với cấu hình mặc định hay không.
- Khi một trang lỗi (#4), sản phẩm muốn giữ kết quả từng phần kèm cờ Kiểm tra, hay muốn báo lỗi cả file?
