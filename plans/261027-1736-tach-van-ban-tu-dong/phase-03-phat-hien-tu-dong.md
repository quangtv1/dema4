---
phase: 3
title: "Phát hiện tự động (đọc dung lượng trước, phân tích nền, cache)"
status: pending
priority: P1
dependencies: [2]
effort: "2,5 ngày"
---

# Phase 3: Phát hiện tự động

## Overview
Gắn nhãn index/blank (+ cờ **Nghi trắng**, cờ **Nghi xoay** kèm góc đề xuất) cho mọi trang của một thư mục PDF theo 2 bước: (1) **đọc dung lượng + kích thước** mọi trang (vài giây/100k trang) để GUI có lưới ngay; (2) **phân tích nền**, gửi kết quả từng trang về GUI để người dùng duyệt trong lúc chạy. Kết quả + ảnh nhỏ ghi **cache** trong thư mục đầu vào. Xuất `index_and_blank.txt`.

## Số đo làm căn cứ
| Mẫu | Kết quả |
|---|---|
| Mẫu 1 `temp/function2/mau` (896 trang, xám) | index 209/209; 1 trắng; index 18–94 KB vs chữ ≥ 229 KB |
| Mẫu 2 + **nhãn tay** `~/Desktop/mau2/index_and_blank (mau2).txt` (1.780 index, 3.561 trắng) | Quy trình dưới: **index 1.780/1.780, 0 thừa**; Trắng tự động 2.732 trang đúng 99,85 %; Nghi trắng 842; trắng lọt Content 0,95 %. Báo cáo `reports/do-chinh-xac-mau2-voi-nhan-tay-261028-1919-report.md` |
| Mẫu 2 `~/Desktop/mau2` (234 PDF, 21.113 trang, **màu**) — số đo cũ | 484 s/10 lõi khi đọc mã mọi trang (~23 ms/trang thực; 100k ≈ 38 phút). Đọc mã 185 ms/trang/lõi (~90 % thời gian), dò trắng 20 ms, đọc dung lượng 0,2 ms. 1.777 index (`NAMSAO-TACH` 1.747, đọc lệch ký tự 28, `Van-ban-khac` 2); 25 % cần 150 dpi |

Kết luận từ mẫu 2 (báo cáo `reports/do-mau2-trang-trang-index-261028-0010-report.md`):
- **Không lọc trang đọc mã theo dung lượng**: 63 % index mẫu 2 nặng ≥ 35 % trung vị hồ sơ (scan màu). Đọc mã **mọi trang**.
- **Chỉ số trắng cũ** (40 dpi xám, lệch nền ≥ 60, ngưỡng 0,20 %) gọi nhầm ~20 % trang chữ nhạt/bảng mờ là trắng; **chỉ số v2** (lệch nền ≥ 18) đếm cả chữ thấm/vân giấy là mực → chỉ bắt 21,7 % trang trắng tay → thay bằng **chỉ số v3** + 2 ngưỡng (dưới).
- **Đọc mã 1 lần 150 dpi** chỉ với LocalAverage sót 23/1.780; thêm GlobalHistogram + mã lỗi checksum + ảnh thu 0,9 → 1.780/1.780 (một trang chỉ đọc được ở 120/175/200 dpi, không đọc được ở đúng 150 dpi do răng cưa lấy mẫu).

## Requirements

### Bước 1 — Đọc dung lượng + kích thước (ngay khi bấm Phân tích)
- Dung lượng trang = tổng `/Length` các luồng ảnh nhúng (đọc từ từ điển, không đọc/giải nén ảnh); trang không ảnh → độ dài luồng nội dung. Kích thước trang (bounds) để tính Quy A4 bằng `dem_pdf::quy_a4_trang` như màn Đếm.
- Song song theo file; gửi GUI 1 lô/file. Mục tiêu 100k trang ≤ 10 s trên ổ cục bộ. Dung lượng chỉ dùng cho Sắp xếp (không dùng để bỏ qua trang).

### Bật/tắt từng loại quét (Cấu hình `quet_trang`, `quet_ma`, `quet_xoay`, mặc định bật)
- Cả hai bật: 1 lần render/trang như dưới. Chỉ trắng: render 100 dpi RGB (đủ cho chỉ số v3 + ảnh nhỏ), bỏ đọc mã → nhanh ~3 lần (đo: 100 dpi 34 ms vs 150 dpi 117 ms/trang). Chỉ mã: render như cả hai, bỏ dò trắng (gần như không nhanh hơn). Tắt cả hai: bỏ bước 2, chỉ bước 1 + ảnh nhỏ theo yêu cầu lưới (~14 dpi).
- Loại bị tắt không sinh nhãn tự động → người dùng gán tay. Cache ghi loại quét đã chạy cho từng file; bật thêm 1 loại rồi Phân tích → chỉ chạy phần còn thiếu cho file chưa có (không xoá nhãn đã sửa); Phân tích lại vẫn làm lại từ đầu.
- Test: tắt mã ⇒ không có nhãn index, không gọi bộ đọc mã; tắt cả hai ⇒ không render trang đầy đủ; bật lại mã ⇒ chỉ đọc mã, giữ nhãn tay.

### Bước 2 — Phân tích nền
- Hàng đợi theo thứ tự hồ sơ/trang; **trang đang hiện trên lưới được ưu tiên** (người dùng duyệt tới đâu có nhãn tới đó). Trang đã Xoá khỏi lưới → bỏ khỏi hàng đợi.
- **Một lần render 150 dpi RGB mỗi trang** dùng chung cho: đọc mã (chuyển xám), dò trắng (thu về 100 dpi), ảnh nhỏ cho lưới. Không render lần 2 (đo mẫu 2: render chiếm ~85 % thời gian; đọc mã ~11 ms, ~32 ms khi chạy đủ các lượt dự phòng).
- **Index** (Code 128, crate **`zxing-cpp`** bundled — rxing sót 4/68 ở hồ sơ khó, không đạt yêu cầu index đúng 100 %): đọc mã **mọi trang**, các lượt trên cùng ảnh 150 dpi xám, dừng ở lượt đầu ra mã hợp lệ: (1) Binarizer LocalAverage + `ReturnErrors` (giữ cả mã lỗi checksum); (2) Binarizer GlobalHistogram; (3) ảnh thu 0,5 / 0,8 / 0,9 (trung bình vùng, trong bộ nhớ) + `ReturnErrors` — đã có ở `tach_vb::doc_ma` (phase 1: mẫu 1 209/209, mẫu 2 1.778/1.780 + 2 `Van-ban-khac`, 0 thừa). **Index = mã hợp lệ hoặc mã lỗi checksum có giá trị khớp mã index trong Cấu hình**, chấp nhận **sai lệch ≤ 3 ký tự** (Levenshtein; mẫu 2 có `NAM5AO-T,CH`, mã lỗi `NA_SAO-TACH`); mặc định `NAMSAO-TACH`; ô trống = mọi Code 128 hợp lệ. **Đổi mã index trong Cấu hình → gán lại index theo giá trị mã đã lưu, không render lại** (cache lưu mọi giá trị đọc được, kể cả mã lỗi). Cấu hình chọn loại mã khác Code 128 (QR code / Cụm ký tự — chưa hỗ trợ) → bỏ bước đọc mã, chỉ dò trắng. Mã khác (vd `Van-ban-khac`) → **không** gán index, để người duyệt tự xem và quyết; vẫn lưu giá trị mã (cache + log).
- **Trắng — chỉ số v3** (ảnh 100 dpi RGB): bỏ 8 % mỗi mép; nền cục bộ = trung vị khối 16 px; `dam` = tỷ lệ khối 8×8 có ≥ 6 điểm **tối hơn nền ≥ 60** và không phải mực màu (chữ thấm mặt sau, vân giấy pơ-luya, bìa màu đều nhạt → bị loại); `mau` = tỷ lệ khối 8×8 có ≥ 6 điểm mực đỏ (R−G > 40 và R−B > 40) hoặc xanh (B−R > 40 và B−G > 25); **chỉ số = dam + 0,3 × mau**. Hằng số chốt theo nhãn tay mẫu 2 (AUC 0,998; v2 là 0,994).
- **2 ngưỡng** (Cấu hình, mặc định **1,0 % / 3,2 %**, chốt theo nhãn tay mẫu 2 — Rust thu ảnh 150 → 100 dpi nên lệch Python một chút: ở 3,0 % trắng lọt 1,04 %, ở 3,2 % còn 0,87 %, Nghi trắng +14 trang): `chi_so < nguong_trang` **và không có mực màu** (`mau` < 0,05 %) → nhãn **Trắng**; `chi_so < nguong_nghi` mà chưa là Trắng → cờ **nghi trắng** (không phải nhãn, không ảnh hưởng cắt; GUI có bộ lọc để người dùng tự phân loại); còn lại Content. **Trang chỉ có dấu đỏ/xanh hoặc vài dòng viết tay màu nhạt → luôn vào Nghi trắng** (người dùng chốt 2026-10-28: để người dùng tự phân loại). Số đo mẫu 2: Trắng 2.732 (99,85 % đúng, 4 trang chữ đánh máy rất mờ bị gọi Trắng), Nghi trắng 842 (799 trắng / 43 chữ), trắng lọt Content 34 (0,95 %); bỏ điều kiện mực màu thì Nghi còn 221 nhưng sai 7.
- Trang vừa có mã index vừa trắng → ưu tiên index.
- **Chiều trang — mô hình PP-LCNet_x1_0_doc_ori** (ONNX 6,8 MB, Apache-2.0, nhúng `include_bytes!`; báo cáo `reports/do-mo-hinh-nhan-huong-trang-261028-2115-report.md`). **Thư viện chạy: crate `ort` (ONNX Runtime, liên kết tĩnh vào exe)** — 8,7 ms/lần/lõi; **dự phòng `tract-onnx`** (thuần Rust, 20 ms/lần) nếu `ort` không liên kết tĩnh được trên CI Windows (kiểm ở phase 1). Hai thư viện cho cùng kết quả (khớp 6 chữ số), chỉ khác tốc độ/cách build; code gọi qua 1 hàm `du_doan_huong(&[f32]) -> [f32; 4]` để đổi thư viện không ảnh hưởng chỗ khác. Chỉ chạy trên trang Content (không Index, không Trắng, không Nghi trắng). Đầu vào: ảnh 150 dpi RGB đã render → cạnh ngắn 256 → cắt giữa 224×224 → chuẩn hoá ImageNet (0,485/0,456/0,406 ; 0,229/0,224/0,225), CHW. Lớp ra `[0, 90, 180, 270]` → góc cần xoay theo kim `[0, 270, 180, 90]`. **Hai tầng**: (1) chạy 1 lần; tin cậy ≥ **`tin_cay_xoay`** (Cấu hình, mặc định **0,8**, khoảng 0,50–0,95) → kết luận. (2) Tin cậy < `tin_cay_xoay` (~12 % trang ở 0,8) → chạy thêm **4 góc xoay**: xoay ảnh 224×224 thêm 0/90/180/270° (ngược kim), mỗi kết quả dịch ngược đúng số bước xoay, cộng 4 phân bố rồi chuẩn hoá; tin cậy ≥ **0,5** → kết luận; còn lại không gắn cờ. Mẫu 2 (800 lần đoán): quyết 780 (97,5 %), **sai 0** (chỉ tầng 1: 707); thêm bình quân ~4–5 ms/trang. Kết luận góc ≠ 0 → cờ **Nghi xoay** (góc gợi ý); **không tự xoay** (cờ không phải nhãn, không ảnh hưởng cắt). Cache lưu phân bố tầng 1 và (nếu đã chạy) tầng 2 → **đổi `tin_cay_xoay` gán lại cờ ngay, không render lại** (bỏ qua trang đã Xác nhận xoay trong `xoay_trang.txt`; trang đang chỉnh dở mà hết đạt ngưỡng → bỏ cờ, giữ góc đã chỉnh); trang mới rơi xuống dưới ngưỡng mà chưa có tầng 2 → chạy bổ sung tầng 2 nền chỉ cho các trang đó. Ngưỡng tầng 2 (0,5) cố định. Bật/tắt bằng `quet_xoay` (mặc định bật). File mô hình dùng bản đã sửa chiều batch = 1 (`dema4_rs/assets/doc_ori_b1.onnx`; bản gốc có tên chiều `DynamicDimension.0` mà tract không đọc được, ort đọc được cả hai) — ghi nguồn + giấy phép.
- Ảnh nhỏ (~165 px cạnh dài, xám JPEG) thu nhỏ từ ảnh 150 dpi — không render thêm.
- Gửi kết quả **từng trang** (nhãn, cờ nghi trắng, cờ nghi xoay + góc, giá trị mã, chỉ số trắng, ảnh nhỏ) qua mpsc. Hủy bằng `AtomicBool` < 1 s. Song song theo trang, mỗi luồng giữ sẵn Document đang mở theo file.
- Dùng lại `tim_file_pdf` (jwalk, bỏ thư mục hệ thống/ẩn, `.PDF` hoa). PDF lỗi mở → ghi lỗi, tiếp tục file khác.

### Cache trong thư mục đầu vào
- Thư mục ẩn `<thư mục đầu vào>/.sm-dema4/` (đếm trang đã bỏ qua thư mục ẩn; trên Windows đặt thuộc tính ẩn). Chứa: `index_and_blank.txt`, `ketqua_tach.txt` và các file cache (vd thư mục con `cache/`).
- Mỗi PDF 1 file cache nhị phân, khoá = đường dẫn tương đối + kích thước + thời điểm sửa của PDF; PDF đổi → bỏ cache file đó.
- Nội dung: dung lượng, kích thước, chỉ số trắng (`dam`, `mau`), nhãn/cờ tự động, giá trị mã đọc được kể cả mã lỗi checksum, phân bố hướng trang tầng 1 (4 × u8) + tầng 2 nếu đã chạy (4 × u8), **trạng thái duyệt của người dùng** (bit: giữ nội dung Nghi trắng, đã xoá khỏi lưới, Nghi xoay đang chỉnh chờ xác nhận) (~40 byte/trang) — mở lại thư mục giữ nguyên trạng thái cuối; ghi lại cache khi trạng thái đổi (gộp theo lô, ghi file tạm rồi đổi tên) + ảnh nhỏ của trang **Index / Trắng / Nghi trắng / đã từng hiện trên lưới** (thư mục nằm trên Shared Drive nên giữ nhỏ).
- Lưu chỉ số trắng thô + giá trị mã → đổi ngưỡng hoặc mã index trong Cấu hình thì gán lại nhãn tự động **không cần render lại**.
- Mở thư mục đã có cache → bỏ qua bước 1–2 với file không đổi. **Phân tích lại** → bỏ cache, chạy lại mọi trang.
- Ghi file tạm rồi đổi tên; cache hỏng → bỏ, phân tích lại, không báo lỗi. Nhãn người sửa ở `.sm-dema4/index_and_blank.txt` (nguồn sự thật). **Phân tích lại xoá file cache** và mọi sửa tay: `index_and_blank.txt` ghi lại theo nhãn tự động mới, `xoay_trang.txt` ghi lại rỗng (mất cả góc xoay và `xac_nhan`), trang đã xoá khỏi lưới hiện lại, lịch sử hoàn tác xoá; **không** xoá `ketqua_tach.txt`.

## Related Code Files
- Create/Modify: `dema4_rs/src/tach_vb.rs` (đọc dung lượng, hàng đợi phân tích, phát hiện), `dema4_rs/src/cache_trang.rs` (đọc/ghi cache `.sm-dema4/`)
- Modify: `dema4_rs/src/dem_pdf.rs` (chỉ để `tim_file_pdf` thành `pub(crate)`)

## Implementation Steps (tests first)
1. Test đơn vị với PDF sinh trong test:
   - dung lượng trang = tổng `/Length` ảnh; trang không ảnh → độ dài nội dung;
   - trắng thuần, bìa màu nhạt + vệt mép + lỗ bấm, **chữ xám rất nhạt (giả thấm)** ⇒ trắng; **trang chỉ có dấu đỏ** ⇒ cờ nghi trắng; trang ít chữ đen + chữ ký ⇒ content; trang giữa 2 ngưỡng ⇒ cờ nghi trắng; đổi ngưỡng có tác dụng;
   - Chiều trang: trang chữ sinh trong test xoay 90/180/270 ⇒ cờ Nghi xoay đúng góc; trang đứng ⇒ không cờ; trang Trắng/Index ⇒ không chạy mô hình; đầu ra khớp giá trị chuẩn (tensor mẫu + kết quả ONNX Runtime Python lưu trong test, sai lệch < 1e-4) với cả `ort` và `tract`; tầng 2 chỉ chạy khi tầng 1 < 0,8 (đếm số lần gọi mô hình: 1 hoặc 5);
   - Code 128 (zxing-cpp writer) `NAMSAO-TACH` và bản lệch 2 ký tự ⇒ index; `Van-ban-khac` ⇒ không index nhưng lưu giá trị; ô mã trống ⇒ mọi mã là index (kể cả `Van-ban-khac`); đổi mã index ⇒ gán lại, không render.
2. Hàng đợi: trang đang hiện được ưu tiên; bỏ trang khỏi hàng đợi ⇒ không có kết quả; hủy ⇒ dừng < 1 s, không panic; mỗi trang render đúng 1 lần (đếm số lần render).
3. Cache: ghi/đọc khứ hồi; PDF đổi ⇒ bỏ cache file đó; cache hỏng ⇒ phân tích lại; mở lần 2 không render; giữ nội dung / xoá khỏi lưới / đang chỉnh xoay → đóng mở lại vẫn nguyên; đổi ngưỡng ⇒ nhãn tự động đổi, không render; đổi mã index ⇒ index gán lại, không render; đổi `tin_cay_xoay` ⇒ cờ Nghi xoay gán lại, không render, trang đã `xac_nhan` không có cờ; trang mới xuống dưới ngưỡng mà chưa có tầng 2 ⇒ chỉ các trang đó chạy tầng 2 nền; `quet_xoay` tắt ⇒ mô hình không được gọi lần nào (đếm số lần gọi); Phân tích lại ⇒ `xoay_trang.txt` rỗng.
4. Test vàng `#[ignore]`:
   - Mẫu 1 (`DEMA4_MAU_TACH`): index 209/209, 0 thừa; blank 1/1, 0 thừa.
   - Mẫu 2 (`DEMA4_MAU2` + nhãn tay `index_and_blank (mau2).txt`, đường dẫn qua biến môi trường `DEMA4_MAU2_NHAN`): in bảng nhầm lẫn trắng / nghi trắng / content, index, thời gian. Ngưỡng đạt: **index 1.780/1.780, 0 thừa**; Trắng tự động ≥ 99,5 % đúng; trắng lọt Content ≤ 1 %. Là **test hồi quy** mỗi khi chỉnh thuật toán (đối chiếu số đo Python ở báo cáo).
5. Hiện thực tới khi xanh; chỉnh tham số chỉ khi test vàng đòi, ghi lý do + số đo.

## Success Criteria
- [ ] Test đơn vị xanh; mẫu 1: index 209/209, blank 1/1.
- [ ] Mẫu 2 (nhãn tay): index 1.780/1.780, 0 thừa; Trắng tự động ≥ 99,5 % đúng; trắng thật lọt Content ≤ 1 %.
- [ ] Chiều trang: mẫu 2 xoay giả 200 trang × 4 chiều (bộ chọn trong báo cáo) ⇒ 2 tầng: quyết ≥ 97 % số lần, sai 0.
- [ ] Bước đọc dung lượng: 100k trang (bộ sinh giả) ≤ 10 s.
- [ ] Mẫu 2 (21.113 trang) ≤ ~8 phút trên 10 lõi với trắng + mã (Python quy trình mới đo 7,3 phút cho phần đọc mã); bật thêm chiều trang ≤ ~9 phút (+~15 ms/trang có chữ, gồm tầng 2).
- [ ] Mở lại thư mục đã có cache: kết quả có ngay, không render. Hủy < 1 s.

## Risk Assessment
- **Render là nút cổ chai** (~85 % thời gian; 100k trang ~25 phút trên 8 lõi) → chạy nền, người dùng duyệt song song; trang đang hiện ưu tiên; cache để không làm lại; tắt quét mã trong Cấu hình nếu cần nhanh. **Không hạ dpi đọc mã** (120 dpi sót tới 2/3 index ở hồ sơ khó).
- **Ngưỡng chốt trên 1 mẫu** (mẫu 2) → hồ sơ khác có thể lệch; vùng mập mờ + trang có mực màu đẩy sang Nghi trắng thay vì đoán; người dùng chỉnh ngưỡng trong Cấu hình.
- **Index phải đúng 100 %** (vị trí cắt): 1 trang mẫu 2 chỉ đọc được nhờ lượt ảnh thu 0,9 → mã vạch hỏng nặng hơn vẫn có thể sót; người duyệt gán I bằng tay. Không hạ độ khớp giá trị (> 3 ký tự) để tránh gán thừa.
- Mã vạch lạ/đọc lệch → so mã có dung sai; mã khác để người duyệt quyết.
- Cache trên Shared Drive bị đồng bộ → chỉ lưu ảnh nhỏ cần thiết; không ghi được → chạy không cache, không báo lỗi.
- RAM: render 100 dpi RGB A4 ≈ 2,8 MB/trang/luồng — chấp nhận.
