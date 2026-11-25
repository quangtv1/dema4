# Red-team: Bóc tách dữ liệu — đúng/sai, đồng thời, mất dữ liệu

Ngày: 2026-11-01 · Phạm vi: `dema4_rs/src/boc_tach/{chay,luu,excel,sheet}.rs`, `gui_boc_tach.rs`, `gui_boc_tach/{ket_qua,truong,hop_loai}.rs`, `gui.rs` (phần boc_tach), `license.rs` (bat_dau/ket_thuc).
Cách làm: đọc code, lần theo luồng sự kiện mpsc → UI. Không chạy thử, không sửa code.
Nhãn: **[Đã xác minh]** = đọc code thấy đường đi chắc chắn; **[Có khả năng]** = phụ thuộc hành vi nền tảng/thao tác người dùng chưa kiểm chứng.

## Tóm tắt

Phần xử lý bất đồng bộ (chay_hang, HetHang luôn gửi, Disconnected coi như HetHang) chắc chắn. Lỗi nằm ở **ranh giới UI ↔ luồng nền**. Khoá đầu vào chỉ khoá khối 2 dòng trên cùng, nhưng **hộp sửa Loại tài liệu (tab Trường dữ liệu) và hộp chọn thư mục vẫn đổi được trạng thái trong lúc chạy**. Sự kiện nền vẫn đọc `loai_chon()`/`files`/`tt` *hiện tại* thay vì ảnh chụp lúc bấm Bắt đầu. Hậu quả: treo vĩnh viễn, panic, ghi giá trị sai cột. Ngoài ra, chiến lược "bản ghi sau thắng" làm mất kết quả tốt và giá trị sửa tay khi đọc lại bị lỗi.

| # | Mức | Vấn đề |
|---|-----|--------|
| 1 | Critical | Xoá loại tài liệu khi đang chạy → hàng đợi không bao giờ kết thúc, UI khoá vĩnh viễn, lượt license treo |
| 2 | Critical | Sửa loại (thêm/xoá/đổi thứ tự trường) khi đang chạy → giá trị ghi sai cột vào jsonl/Excel/Sheet |
| 3 | High | "Lưu thành loại mới" khi đang có bản sửa chưa Lưu → panic index out of bounds |
| 4 | High | Đổi thư mục trong lúc chạy/ReOCR (hộp chọn thư mục không chặn trên Windows) → panic hoặc ghi kết quả vào nhầm file |
| 5 | High | ReOCR / Chạy lại / Dừng bị lỗi ghi đè kết quả tốt và giá trị sửa tay (bản ghi sau thắng) |
| 6 | High | Mất giá trị sửa tay trong lúc chạy (BatDau giật file đang xem; Lưu ghi đè cả trường không sửa) |
| 7 | High | `ghi_them` đọc toàn bộ jsonl mỗi lần ghi, trên luồng UI → O(n²), đơ trên Google Drive |
| 8 | Medium | Thoát app khi đang bóc tách không hỏi → mất Excel, mất hàng đợi Sheet, kill luồng giữa chừng |
| 9 | Medium | Bộ lọc "lỗi"/"đọc"/"5/6" khớp cả tên file → "Chạy lại (n)" chạy nhầm file tốt |
| 10 | Medium | Sheet bị trùng dòng khi chạy lại; Excel/Sheet dùng chung tên/tab giữa các loại |
| 11 | Medium | Luồng gửi Sheet thử lại vô hạn, rò luồng khi tạo hàng đợi mới |
| 12 | Medium | Lỗi ghi jsonl bị nuốt (`let _ =`) → UI báo xong nhưng mở lại mất hết |
| 13 | Medium | Dừng không dừng file đang đọc (đọc hết mọi trang); Dừng trong lúc chờ 429 ghi thành lỗi |
| 14 | Low | Việc O(n) mỗi khung hình + ghi Excel/đọc jsonl trên luồng UI |
| 15 | Low | Lặt vặt: worker panic để file ở "Đọc…" mãi; `bat_dau` license chặn UI tới 20 s; lọc bị bỏ qua khi đang chạy; khoá jsonl theo dấu phân cách đường dẫn của từng OS |

---

## 1. [Critical] Xoá loại tài liệu khi đang chạy → treo vĩnh viễn — [Đã xác minh]

**Vị trí:** `gui_boc_tach/truong.rs:243-247` (nháy đúp trường mở `Hop::Sua`, không kiểm `dang_chay`); `gui_boc_tach.rs:621-631` (`XacNhanXoa` đặt `self.loai = None`); `gui_boc_tach/ket_qua.rs:219` (`nhan_chay`: `let Some(loai) = self.loai_chon().cloned() else { return };`); `gui_boc_tach.rs:253-254` (khoá khối đầu vào khi `dang_chay()`); `gui.rs:227` (menu khoá khi đang chạy).

**Tái hiện:**
1. Bấm Bắt đầu với ≥ 1 file.
2. Chuyển sang tab "Trường dữ liệu" (thanh tab không bị khoá), nháy đúp một trường, bấm Xoá loại, xác nhận.
3. `loai = None`. Từ đó `nhan_chay` thoát ngay ở dòng 219, không bao giờ `try_recv`, nên `HetHang` không được xử lý và `self.chay` giữ `Some` mãi.

**Hậu quả:**
- `dang_chay()` luôn true. Khối đầu vào bị khoá nên không chọn lại được loại. Menu bị khoá nên không đổi màn hình được.
- Nút Dừng vẫn hiện, nhưng bấm Dừng cũng không giải phóng được gì.
- `license.ket_thuc` không bao giờ được gọi: lượt nằm trong `dang_chay` set suốt phiên.
- Excel không được ghi, popup tổng kết không hiện. Người dùng chỉ còn cách thoát app.

**Sửa:**
- Chặn mở `Hop::Sua`/`XacNhanXoa`/`LuuMoi` khi `m.dang_chay()` (truong.rs:241-246).
- Đồng thời: `DangChay` giữ bản sao `loai: LoaiTaiLieu` (đã clone trong `bat_dau`), và `nhan_chay` dùng `c.loai` thay vì `self.loai_chon()`. Như vậy sự kiện luôn được rút hết dù UI đổi gì.

## 2. [Critical] Sửa loại khi đang chạy → giá trị ghi sai cột — [Đã xác minh]

**Vị trí:** `gui_boc_tach.rs:611-616` (`HanhDong::Luu`: ghi đè `ds_loai[chi_so]` rồi gọi `nap_ket_qua()`); `ket_qua.rs:219,241` (`BanGhi::tu_ket_qua(&self.files[i].ten, &loai, &kq)` với `loai` hiện tại); `luu.rs:43` (`loai.truong.iter().zip(&kq.gia_tri)`).

**Tái hiện:**
1. Đang chạy loại "Công văn" 6 trường.
2. Tab Trường → nháy đúp → "+ Thêm trường" (hoặc kéo đổi thứ tự / xoá trường) → Lưu.
3. Luồng nền vẫn dùng `loai` cũ (đã clone), nên `kq.gia_tri` theo thứ tự cũ. UI lại zip với `truong` mới.
4. Ví dụ: chèn trường ở vị trí 0 thì "Số ký hiệu" bị ghi vào biến "Loại văn bản", v.v. Thêm vào đó, đổi tên loại khiến bản ghi mang tên loại mới.

**Hậu quả:**
- jsonl có bản ghi sai biến; "bản ghi sau thắng" nên dữ liệu sai được giữ lâu dài.
- `nap_ket_qua` giữa chừng reset toàn bộ `tt` (file đang "Đọc…" trở thành "-").
- Header của hàng đợi Sheet (tạo lúc `bat_dau`) lệch số cột với các dòng sau.

**Sửa:** như #1. Chặn sửa loại khi đang chạy/ReOCR, và dùng ảnh chụp `loai` trong `DangChay` (và trong `doc_lai`) để dựng `BanGhi`.

## 3. [High] "Lưu thành loại mới" khi đang có bản sửa chưa Lưu → panic — [Đã xác minh]

**Vị trí:** `gui_boc_tach.rs:635-646` (`LuuMoi` đổi `self.loai` sang loại mới nhưng **không** gọi `nap_ket_qua()`, nên `kq.sua` vẫn còn); `ket_qua.rs:587-590` (`can_moi = j != i || (g != goc && b == g)`); `ket_qua.rs:703,718,720` (`ban[k]` theo `loai.truong` mới); `ket_qua.rs:734-735` (`ban[k]`, `goc[k]`).

**Tái hiện:**
1. Tab Kết quả: mở 1 file đã Xong, gõ sửa 1 ô, chưa bấm Lưu.
2. Tab Trường → nháy đúp → "+ Thêm trường" → "Lưu thành loại mới" → OK.
3. Quay lại tab Kết quả. Lúc này `j == i` và `b != g`, nên `can_moi = false` và `sua` giữ vector 6 phần tử.
4. `form` lặp 7 trường và gọi `ban[6]`, gây panic và app sập.

Biến thể: không panic nhưng sai dữ liệu khi loại mới ít trường hơn (`luu_sua` ghi biến lệch vị trí).

**Sửa:**
- Gọi `self.nap_ket_qua()` (hoặc ít nhất `self.kq.sua = None`) trong nhánh `LuuMoi`.
- Thêm điều kiện độ dài vào `can_moi`: `|| b.len() != loai.truong.len()`.

## 4. [High] Đổi thư mục trong lúc chạy hoặc ReOCR → panic / ghi nhầm file — [Có khả năng] (đường code đã xác minh; phụ thuộc hộp chọn thư mục không modal)

**Vị trí:**
- `gui_tach.rs:302-303` (Windows: `rfd` chạy ở luồng riêng, không gắn cửa sổ cha, nên cửa sổ chính vẫn bấm được).
- `gui_boc_tach.rs:209-214` (`nhan_nen` gọi `chon_thu_muc` bất kể `dang_chay`).
- `gui_boc_tach.rs:172-176` (`files.clear(); tt.clear()`).
- `ket_qua.rs:235,241,250,413` (`self.tt[i]`, `self.files[i]`); `ket_qua.rs:346,349` (ReOCR `self.files[i]`).

**Tái hiện (Windows):**
1. Đã có file. Bấm "Chọn" và để hộp chọn mở.
2. Bấm "Bắt đầu" trên cửa sổ chính (nút không phụ thuộc `chon_tm`).
3. Chọn thư mục khác trong hộp. Khung kế tiếp `files`/`tt` bị rỗng, rồi sự kiện `BatDau(i)` gọi `self.tt[i]`, gây **panic**.

Với ReOCR:
- Nếu thư mục mới có ≥ i+1 file, kết quả của file cũ được ghi vào jsonl của **thư mục mới**, dưới **tên file khác**, rồi `ghi_de` lên Sheet: **hỏng dữ liệu im lặng**.
- Nếu thư mục mới ít file hơn: panic.

**Sửa:**
- Trong `nhan_nen`, bỏ qua (hoặc hoãn) kết quả `chon_tm`/`quet` khi `dang_chay()`.
- Hoặc vô hiệu nút Bắt đầu/ReOCR khi `chon_tm.is_some() || quet.is_some()`.
- Phòng thủ thêm: sự kiện mang theo `ten`/`PathBuf` thay vì chỉ chỉ số; dùng `get()` và bỏ qua khi lệch.

## 5. [High] Đọc lại bị lỗi ghi đè kết quả tốt và giá trị sửa tay — [Đã xác minh]

**Vị trí:** `ket_qua.rs:347-357` (ReOCR `Err` → `BanGhi::loi`, ghi jsonl, `tt[i] = Loi`, ghi lại Excel, `ghi_de` Sheet); `ket_qua.rs:249-253` (Chạy lại: `Loi` ghi đè); `luu.rs:80-84` (bản ghi sau thắng, không phân biệt lỗi/thành công); `chay.rs:46-48` (Dừng trong lúc chờ 429 trả `Err(QuaTai)`, thành bản ghi lỗi).

**Tái hiện:**
1. File A đã Xong, người dùng đã sửa tay và Lưu (`sua=["nguoi_ky"]`).
2. Bấm ReOCR khi mạng chập chờn hoặc máy chủ trả 500.
3. A trở thành "Lỗi". Excel được ghi lại với dòng A trống, Sheet bị ghi đè thành dòng lỗi. Mở lại app: `luu::doc` lấy bản ghi lỗi.

Tương tự: "Chạy lại" (không lọc: chạy **mọi** file, `gui_boc_tach.rs:572-578`) khi đứt mạng biến hàng loạt file tốt thành lỗi. Chạy lại thành công thì cũng **xoá sạch giá trị sửa tay** (`tu_ket_qua` đặt `sua: vec![]`) mà không cảnh báo.

**Sửa:**
- Lỗi không được thay thế kết quả Xong đã có. Giữ `tt` cũ và chỉ báo lỗi (toast/tooltip). Trong jsonl, ghi bản ghi lỗi kiểu "lần thử lỗi" mà `doc()` không cho thắng bản Xong.
- Chạy lại/ReOCR thành công: hoặc giữ các biến trong `sua` (merge), hoặc hỏi xác nhận "N file có sửa tay sẽ bị ghi đè".
- Dừng: phân biệt `Err` do dừng và không ghi bản ghi.

## 6. [High] Mất giá trị sửa tay trong lúc chạy — [Đã xác minh]

**Vị trí:** `ket_qua.rs:237` (`BatDau` đặt `i_xem = i`); `ket_qua.rs:587-594` (`j != i` thì bỏ bản sửa); `ket_qua.rs:734-740` (`luu_sua` ghi **mọi** trường từ `ban`, không chỉ trường đã sửa); `ket_qua.rs:246` (`Xong(i)` ghi đè `tt[i]`).

**Tái hiện:**
- **(a)** Đang chạy 4 luồng: chọn một file đã Xong, gõ sửa. Vài giây sau có `BatDau` của file khác, `i_xem` bị đổi và bản sửa mất không báo. Gần như không thể sửa trong lúc chạy.
- **(b)** File X còn ở "-" (trong `c.ds`): người dùng gõ 1 ô (form cho sửa vì chỉ `Dang` mới hiện spinner) rồi Lưu. Sau đó hàng đợi tới X, `Xong(X)` ghi đè và bản sửa mất.
- **(c)** X đang xem và có bản sửa chưa Lưu, rồi `BatDau(X)` → `Xong(X)`. Vì `b != g` nên `can_moi=false` và bộ đệm cũ được giữ. Bấm Lưu: `luu_sua` lấy `b` = kết quả mô hình mới nhưng ghi đè **tất cả** trường bằng `ban` cũ (các ô chưa gõ là ""). Kết quả mô hình bị xoá.

**Sửa:**
- Trong lúc chạy, `BatDau` chỉ đổi `i_xem` khi không có bản sửa đang dở (`g == b`).
- `luu_sua` chỉ ghi các trường có `ban[k] != goc[k]`, áp lên `b` hiện tại.
- Khi `goc` thay đổi dưới bộ đệm đang sửa, báo xung đột thay vì giữ im lặng.
- Hoặc khoá form khi file nằm trong `c.ds` mà chưa xong.

## 7. [High] `ghi_them` đọc toàn bộ jsonl mỗi lần ghi, trên luồng UI — [Đã xác minh]

**Vị trí:** `luu.rs:70` (`std::fs::read(f)` toàn file chỉ để xem byte cuối); gọi từ `ket_qua.rs:242,250,351,742` trên luồng UI. `nap_ket_qua` (`ket_qua.rs:164`) parse toàn bộ jsonl (gồm `tho` thô) trên luồng UI.

**Tái hiện:**
- Mỗi bản ghi có `tho` (câu trả lời thô từng trang), cỡ vài KB.
- 10.000 file khoảng 30-60 MB jsonl. Mỗi sự kiện Xong/Loi đọc lại toàn bộ file, tổng O(n²) byte.
- Thư mục nguồn nằm trên Google Drive (CloudStorage). Đọc file lớn có thể kéo mạng, nên UI đơ hàng giây sau mỗi file và Dừng cũng phản hồi chậm.
- Mỗi lần chạy lại thêm bản ghi mới mà không có cơ chế nén, nên file chỉ phình ra.

**Sửa:**
- Mở file, `seek(SeekFrom::End(-1))`, đọc 1 byte.
- Đưa ghi jsonl sang luồng ghi riêng (channel), hoặc ghi ngay trong worker.
- `nap_ket_qua` chạy nền.
- Cân nhắc tách `tho` ra file log riêng, và nén jsonl (viết lại các bản ghi thắng) khi mở thư mục.

## 8. [Medium] Thoát app khi đang bóc tách không hỏi — [Đã xác minh]

**Vị trí:** `gui.rs:301-309`: điều kiện hỏi chỉ có `self.tach.dang_chay() || self.muc_luc.dang_chay()`, **thiếu `self.boc_tach.dang_chay()`**. `gui.rs:137-143` (thông điệp cũng không có nhánh "bóc tách").

**Tái hiện:** đang chạy 500 file, bấm nút đóng cửa sổ. App thoát ngay.

**Hậu quả:**
- Excel không được ghi (chỉ ghi ở `HetHang`).
- Các dòng Sheet đang chờ trong `HangDoi` (bộ nhớ) mất hẳn, và không có cơ chế đồng bộ bù từ jsonl.
- Đặt chỗ license (`DatCho`) không được `ket_thuc`; theo chú thích ở `license.rs:488`, lần sau có thể bị coi là lần bỏ dở.
- Lời gọi API đang chạy bị cắt (đã tính tiền nhưng không lưu).

**Sửa:**
- Thêm `self.boc_tach.dang_chay()` vào điều kiện, và nhánh "bóc tách dữ liệu" trong `hop_hoi_dong_chay`.
- Khi thoát luôn: set `dung` và chờ ngắn, hoặc ít nhất ghi Excel từ `tt` hiện có.

## 9. [Medium] Bộ lọc trạng thái khớp cả tên file → "Chạy lại (n)" chạy nhầm — [Đã xác minh]

**Vị trí:** `ket_qua.rs:133`: `theo_tt || (… dang_so(ten).contains(&dang_so(&q)))`.

**Tái hiện:**
- Popup "Xem file lỗi" đặt `loc="lỗi"`, `dang_so` cho ra `"loi"`. Các file **Xong** có tên chứa "loi" (ví dụ "Báo cáo lợi nhuận.pdf", "Loi_noi_dau.pdf") cũng hiện. "Chạy lại (n)" đọc lại chúng, mất lượt API và, kết hợp #5, có thể mất giá trị sửa tay hoặc thành lỗi.
- `"5/6"` thành `"56"`: khớp "VB56.pdf" dù file đó 6/6.
- `"doc"` khớp mọi tên chứa "doc"/"đốc".

**Sửa:** khi `q` là từ khoá trạng thái (lỗi/đọc/-/x/y), chỉ lọc theo trạng thái, không cộng thêm khớp tên. Có thể dùng tiền tố cho tìm tên.

## 10. [Medium] Sheet trùng dòng; Excel/Sheet đè chéo giữa các loại — [Đã xác minh]

**Vị trí:**
- `ket_qua.rs:247,253,274-277` (`day_sheet` dùng `them` = append cho mọi Xong/Loi).
- `sheet.rs:149-151` (chỉ tạo tiêu đề khi dòng 1 trống).
- `excel.rs:25-28` (tên `<nguồn>_boc_tach.xlsx` không chứa tên loại); `sheet.rs:106` (tab mặc định chung).

**Tái hiện:**
- Lần 1, file B lỗi: Sheet có dòng lỗi B. Chạy lại (n), B thành công: dòng B **thứ hai** được append. Chạy lại toàn bộ thư mục làm gấp đôi mọi dòng.
- Chạy loại "Hợp đồng" trên cùng thư mục: `…_boc_tach.xlsx` của loại "Công văn" bị ghi đè. Các dòng Hợp đồng được append dưới tiêu đề cột của Công văn trên cùng tab.

**Sửa:**
- Trong `day_sheet`, dùng `ghi_de` theo tên file khi file đã có trong Sheet (hoặc luôn upsert theo cột A; nên gom theo lô để giảm số lần GET `A:A`).
- Đưa tên loại vào tên file Excel và tab mặc định, hoặc kiểm tiêu đề dòng 1 khớp `tieu_de(loai)` và báo lỗi khi lệch.

## 11. [Medium] Luồng gửi Sheet thử lại vô hạn, rò luồng — [Đã xác minh]

**Vị trí:** `sheet.rs:252-297`: chỉ thoát khi `Disconnected && cho.is_empty()`; lỗi thì thử lại mãi (tối đa 60 s một lần). `ket_qua.rs:393,402` (`tao_sheet` thay `HangDoi` cũ ở mỗi Bắt đầu, nên sender cũ bị drop nhưng luồng cũ còn `cho` thì vẫn chạy).

**Tái hiện:**
1. Service account chưa được chia sẻ quyền (403 vĩnh viễn), hoặc tên tab không hợp lệ (400).
2. Mỗi lần Bắt đầu sinh một luồng mới. Các luồng cũ vẫn chạy và thử lại mỗi 60 s đến khi đóng app, trạng thái của chúng không còn hiện trên UI.
3. Khi người dùng sửa quyền, mọi luồng cũ cùng đẩy lô cũ lên, gây trùng dòng (cộng với #10).
4. `dat_lai()` (đổi màn) cũng để luồng cũ chạy.

**Sửa:**
- Phân loại lỗi vĩnh viễn (4xx trừ 429) để dừng và báo.
- Giới hạn số lần thử, hoặc thoát khi `Disconnected` sau N lần thử.
- Khi thay `HangDoi`, chuyển `cho` còn tồn sang hàng đợi mới thay vì để luồng cũ tự chạy.

## 12. [Medium] Lỗi ghi jsonl bị nuốt — [Đã xác minh]

**Vị trí:** `ket_qua.rs:242,250,351,742` (`let _ = luu::ghi_them(...)`).

**Tái hiện:** thư mục nguồn chỉ đọc (Shared drive quyền Người xem), Drive mất kết nối, hoặc đường dẫn quá dài trên Windows. Mọi file vẫn hiện Xong, Lưu sửa tay vẫn báo xong. Mở lại app thì `nap_ket_qua` trả rỗng và toàn bộ kết quả + sửa tay mất. jsonl là "nguồn sự thật" theo chú thích ở `luu.rs:1-3`.

**Sửa:** lưu lỗi ghi vào trạng thái (giống `loi_sheet`), hiện đỏ ở dòng đáy/cảnh báo. Lỗi đầu tiên thì dừng hàng đợi (`DungSom`) hoặc ít nhất cảnh báo trước khi đóng.

## 13. [Medium] Dừng không dừng file đang đọc — [Đã xác minh]

**Vị trí:** `chay.rs:71-77` (vòng trang không kiểm `dung`); `chay.rs:118` (chỉ kiểm giữa các file); `chay.rs:46-48` (kiểm trong lúc chờ 429 nhưng trả lỗi).

**Tái hiện:**
- Loại đọc mọi trang (Hợp đồng) với file 150 trang: bấm Dừng, nút hiện "Đang dừng…" trong nhiều phút cho tới khi đọc xong mọi trang của mọi luồng đang chạy, vẫn tốn API.
- Dừng đúng lúc đang chờ 429: file đó bị ghi Loi "quá tải" (xem #5).

**Sửa:** kiểm `dung` đầu mỗi vòng trang và trả một biến thể `LoiDoc::Dung`. `chay_hang` nhận biến thể này thì không gửi `Loi` (file giữ trạng thái cũ).

## 14. [Low] Việc O(n) mỗi khung hình và IO trên luồng UI — [Đã xác minh]

**Vị trí:**
- `ket_qua.rs:465` và `gui_boc_tach.rs:572`: `hien()` gọi 2 lần/khung, mỗi lần O(n) `khop_loc`.
- `ket_qua.rs:133`: `dang_so(&q)` tính 2 lần cho mỗi file, `dang_so(ten)` cấp phát mỗi khung.
- `ket_qua.rs:447-450` (`canh_bao` duyệt `tt` × trường), `426-430`, `760-764`: thêm 3 lần fold `tt` mỗi khung.
- Trong lúc chạy có repaint 100 ms.
- `ghi_excel` (`ket_qua.rs:381-389`) viết lại toàn bộ xlsx đồng bộ ở mỗi Lưu/ReOCR. Nếu file Excel đang mở (Windows khoá file), mỗi lần Lưu tạo thêm một `_YYYYmmdd_HHMMSS.xlsx` mới (`excel.rs:54-57`).

**Ảnh hưởng:** với 10-50k file, khung hình chậm thấy rõ khi ô lọc có chữ; Lưu bị khựng.

**Sửa:**
- Cache `hien` và số liệu tổng (ok/lỗi/thiếu), chỉ tính lại khi `tt`/`loc`/`files` đổi (cờ dirty).
- Tiền tính `dang_so(ten)` trong `FilePdf`.
- Ghi Excel ở luồng nền (debounce).

## 15. [Low] Lặt vặt

- **Worker panic** (ví dụ mupdf panic với PDF hỏng) — [Có khả năng]: `thread::scope` (`chay.rs:115-146`) vẫn kết thúc qua `Disconnected` nên không treo. Nhưng file đó không có `Xong`/`Loi`, nên `tt` mắc ở "Đọc…" sau khi chạy xong (đếm sai ở `dong_trang_thai`, `ket_qua.rs:413`). Sửa: bọc `doc_file` bằng `catch_unwind` và gửi `Loi`.
- **`license.bat_dau` chặn UI tới 20 s** (`license.rs:483-485`, gọi từ `ket_qua.rs:192` trên luồng UI) khi máy chưa có trả lời ký. Gây đơ nhưng không gây sai dữ liệu. [Đã xác minh]
- **`hien()` bỏ qua bộ lọc khi đang chạy** (`ket_qua.rs:178-180`): chú thích nói "vẫn áp lọc tên" nhưng code trả nguyên `c.ds`. Ô lọc không có tác dụng trong lúc chạy, và nhãn "Lọc …: n/N" hiển thị sai. [Đã xác minh]
- **Khoá jsonl theo dấu phân cách của từng OS** (`gui_boc_tach.rs:689`: `ten` = `strip_prefix` + `to_string_lossy`). Cùng thư mục Shared drive mở trên Windows thì ra `HS01\a.pdf`, trên macOS thì ra `HS01/a.pdf`. Kết quả do máy kia bóc không khớp, coi như chưa đọc; cột A trên Sheet cũng lệch nên `ghi_de` không tìm thấy dòng. Sửa: chuẩn hoá `ten` về `/`. [Đã xác minh]

---

## Việc nên làm (theo thứ tự)

1. Chụp trạng thái lúc chạy: `DangChay { loai, goc, ten_file: Vec<String> }`; `nhan_chay`/`nhan_doc_lai` chỉ dùng ảnh chụp này. Chặn `Hop::*` và kết quả chọn/quét thư mục khi `dang_chay()` (#1, #2, #4).
2. Reset `kq.sua` khi loại đổi (mọi nhánh), và kiểm độ dài `ban` (#3).
3. Lỗi không thay thế Xong; giữ/merge `sua` khi đọc lại (#5). `luu_sua` chỉ ghi trường đã sửa; không giật `i_xem` khi đang có bản sửa (#6).
4. `ghi_them` dùng seek, đưa IO jsonl/Excel khỏi luồng UI (#7, #14).
5. Thêm `boc_tach.dang_chay()` vào hỏi khi thoát (#8). Báo lỗi ghi jsonl (#12).
6. Sửa lọc trạng thái (#9). Sheet upsert theo tên file, đưa tên loại vào Excel/tab (#10). Luồng Sheet có giới hạn thử lại (#11). Dừng giữa các trang (#13).

## Câu hỏi còn mở

- Trên Windows, hộp chọn `rfd` chạy ở luồng riêng có thực sự để cửa sổ chính bấm được không? Cần thử trên máy thật để chốt #4. Trên macOS hộp chạy ở luồng chính nên ít khả năng xảy ra.
- "Chạy lại" (không lọc) đọc lại **mọi** file kể cả file đã sửa tay. Đây là chủ ý sản phẩm hay nên mặc định chỉ đọc file Cho/Lỗi?
- Đặt chỗ license bị bỏ dở khi thoát app: theo thiết kế có tính lượt không? Điều này ảnh hưởng mức độ của #8.
