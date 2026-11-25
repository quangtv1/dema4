import { IconName } from '../shared/icons';

/**
 * Nội dung "Hướng dẫn sử dụng" (HT-01). Bố cục theo prototype `HD` / `mHuongDan()` (L1491–1520); nội dung viết
 * theo đúng chức năng của bản này (không mô tả chức năng chưa có). Mỗi mục là đoạn văn thuần; `**…**` in đậm,
 * `[[Ctrl+K]]` vẽ thành phím. Sửa chức năng thì sửa luôn ở đây.
 */
export interface HelpBlock {
  h: string;
  type: 'ol' | 'ul';
  items: string[];
}

export interface HelpTopic {
  id: string;
  icon: IconName;
  name: string;
  intro: string;
  blocks: HelpBlock[];
}

export const HELP: HelpTopic[] = [
  {
    id: 'bat-dau',
    icon: 'home',
    name: 'Bắt đầu nhanh',
    intro: 'Các bước để ký văn bản lần đầu với Sao Mai Ký số. Văn bản được ký ngay trên máy, không gửi lên máy chủ.',
    blocks: [
      {
        h: 'Chuẩn bị',
        type: 'ul',
        items: [
          'Cài driver cho USB Token: vào **Hỗ trợ › Cài đặt driver USB Token**. App kiểm tra máy; nếu chưa có driver, bấm **Mở trang tải phần mềm** để tải ở trang của Ban Cơ yếu Chính phủ.',
          'Cắm USB Token chứa chứng thư số vào máy.',
          'Nếu muốn hiện ảnh chữ ký hoặc con dấu trên văn bản, chuẩn bị ảnh .png hoặc .jpg (nên dùng .png nền trong).',
        ],
      },
      {
        h: 'Ký lần đầu',
        type: 'ol',
        items: [
          'Vào **Chức năng › Ký số văn bản** ([[Ctrl+K]]).',
          'Bấm **1 file** hoặc **Thư mục** để nạp văn bản PDF; cũng có thể kéo thả file / thư mục vào cửa sổ.',
          'Chọn ảnh chữ ký / con dấu và kiểu **Hiển thị**; xem khung chữ ký trên bản xem trước bên phải.',
          'Bấm **Ký số**, chọn chứng thư số trong hộp **Xác nhận ký**, nhập mã PIN rồi bấm **Ký số**.',
          'File đã ký nằm trong thư mục **DaKySo** cạnh file gốc, tên thêm **_signed**. Ký cả thư mục thì lưu vào thư mục **<tên thư mục>_DaKySo** cùng cấp, giữ nguyên các thư mục con. File gốc giữ nguyên.',
        ],
      },
    ],
  },
  {
    id: 'ky-so',
    icon: 'pen',
    name: 'Ký số văn bản',
    intro: 'Ký một file hoặc cả thư mục PDF bằng USB Token.',
    blocks: [
      {
        h: 'Nạp văn bản',
        type: 'ul',
        items: [
          '**1 file**: chọn một file PDF. **Thư mục**: lấy mọi file PDF trong thư mục, kể cả các thư mục con (bỏ qua thư mục kết quả DaKySo của lần ký trước).',
          'Danh sách ở cột trái: tìm theo tên, lọc theo trạng thái; [[Ctrl+←]] / [[Ctrl+→]] để chuyển văn bản.',
          'Bản xem trước bên phải tự cuộn tới khung chữ ký; giữ [[Ctrl]] và lăn chuột để phóng to / thu nhỏ.',
          'Văn bản nhiều trang có cột ảnh thu nhỏ các trang: bấm một trang để lướt tới; trang có khung chữ ký có chấm xanh.',
        ],
      },
      {
        h: 'Ký nháy và ký chính',
        type: 'ul',
        items: [
          'Mỗi lần ký, mỗi văn bản nhận **một** chữ ký của một chứng thư, hiện ở mọi khung trên trang. Văn bản đã ký vẫn ký tiếp được bằng chứng thư của người khác.',
          'Chọn lượt ký **Chính** hoặc **Nháy** trên thanh xem trước. Ký nháy mặc định có khung ở mọi trang, góc dưới đối diện chữ ký chính.',
          'Văn bản cần nháy và ký chính của hai người: cắm USB của người nháy, chọn **Nháy** rồi ký; sau đó nạp thư mục kết quả (**DaKySo**, hoặc **<tên thư mục>_DaKySo** nếu đã ký cả thư mục), chọn **Chính** và ký bằng USB của người ký chính.',
        ],
      },
      {
        h: 'Ký',
        type: 'ol',
        items: [
          'Bấm **Ký số**. Nếu nơi lưu đã có file trùng tên, chọn **Giữ cả hai** (file mới thêm (1), (2)…) hoặc **Ghi đè**.',
          'Trong hộp **Xác nhận ký**: chọn chứng thư số trên USB Token, nhập mã PIN (để trống thì nhập ở hộp của token).',
          'Theo dõi trạng thái từng file; tab **Nhật ký** ghi chi tiết. Bấm **Dừng** để dừng, sau đó **Ký tiếp** các file còn lại.',
        ],
      },
      {
        h: 'Kết quả',
        type: 'ul',
        items: [
          'Mỗi file được kiểm lại chữ ký ngay sau khi ký; file lỗi ghi rõ lý do ở danh sách và Nhật ký.',
          'Nơi lưu và hậu tố tên file đổi ở **Cấu hình › Thư mục & tên file**.',
          '**Xóa file gốc** (ô góc trên phải): thay file gốc bằng file đã ký, bản gốc vào Thùng rác, chỉ khi chữ ký kiểm lại hợp lệ. Ô này chỉ có tác dụng trong lần mở app hiện tại.',
          'Rút USB Token khi đang ký: app dừng lô ký và báo các file chưa ký.',
        ],
      },
    ],
  },
  {
    id: 'vi-tri',
    icon: 'frame',
    name: 'Vị trí & kiểu chữ ký',
    intro: 'Chữ ký hiện ở đâu trên trang, to bao nhiêu và hiện những gì.',
    blocks: [
      {
        h: 'Kiểu hiển thị',
        type: 'ul',
        items: [
          '**Chỉ thông tin**: dòng "Ký bởi" và "Ngày ký".',
          '**Ảnh + thông tin**: ảnh chữ ký / con dấu và dòng "Ký bởi / Ngày ký". **Chỉ ảnh**: chỉ ảnh, phủ cả khung.',
          '**Vị trí ảnh** so với thông tin: **Trái** (mặc định), **Phải**, **Trên**, **Dưới**. Đổi bằng chuột phải vào khung chữ ký, hoặc ở **Cấu hình › Thông tin chữ ký**. Ảnh trên / dưới thì khung mặc định hẹp và cao hơn (130 × 66 pt trên trang A4).',
          '**Ẩn**: văn bản vẫn có chữ ký số nhưng không hiện gì trên trang.',
        ],
      },
      {
        h: 'Theo mẫu khổ giấy',
        type: 'ul',
        items: [
          'Mỗi khổ A1–A5 có một mẫu: góc đặt, khoảng cách tới mép, tỉ lệ, trang ký. App chọn mẫu theo khổ giấy của từng văn bản.',
          'Mặc định: góc dưới phải trang cuối. Sửa ở **Cấu hình › Chữ ký theo khổ giấy**.',
        ],
      },
      {
        h: 'Đặt tay',
        type: 'ul',
        items: [
          'Kéo khung trên bản xem trước để dời chỗ, kéo cạnh hoặc góc để đổi cỡ; văn bản có một khung thì vị trí này áp cho cả lô. Nút ↻ cạnh ô phóng to để quay về theo mẫu.',
          '**Chuột phải** vào khung: **Xoá khung**, hoặc thêm khung giống thế vào mọi trang. Chuột phải vào chỗ trống: **Thêm khung chữ ký ở đây**. Bấm chọn khung rồi [[Delete]] cũng xoá. Thêm / xoá khung chỉ áp cho văn bản đang xem. Xoá hết khung thì văn bản **Chưa gán chữ ký**: có cảnh báo, **bỏ qua khi ký số**; thêm khung (hoặc bấm ↻) để gán lại.',
          'Nút **Cấu hình vị trí** mở màn **Xem trước & vị trí chữ ký**: chọn **Tất cả file** (một khung chung) hoặc **Từng file** (khung và kiểu hiển thị riêng cho từng văn bản), chỉnh thanh **Cỡ**, hoặc **Nâng cao › Theo mẫu / Vị trí theo mẫu**.',
          'Kiểu **Ảnh + thông tin**: ở **Nâng cao** chọn **Vị trí ảnh** (Trái / Phải / Trên / Dưới), kéo **Ảnh chiếm** (phần khung dành cho ảnh) và **Khoảng cách** (giữa ảnh và thông tin); **Tự động** để chia như mặc định. Áp cho mọi văn bản, bấm **Áp dụng** mới lưu.',
          'Muốn tự đặt: bấm **Tự do** ở hàng Vị trí ảnh, kéo ô ảnh và ô thông tin trong khung phóng to bên dưới (kéo cạnh để đổi cỡ theo chiều ngang / dọc, kéo góc để đổi cả hai, chồng lên nhau được; kéo cạnh / góc khung ngoài để đổi cỡ cả khung chữ ký). Hàng này luôn hiện, chỉ có tác dụng khi Hiển thị là Ảnh + thông tin.',
          'Văn bản nhiều trang: cột **ảnh thu nhỏ** từng trang bên trái trang xem trước (trang có khung có dấu chấm). **Chuột phải vào ảnh thu nhỏ**: thêm khung vào trang đó (cùng chỗ khung đang có) hoặc xoá khung ở trang đó. **Chuột phải trên trang**: thêm khung ở chỗ bấm, xoá khung, xoá hết khung, thêm khung giống thế vào mọi trang. Xoá hết khung thì văn bản **Chưa gán chữ ký** và được bỏ qua khi ký số.',
          'Bấm **Áp dụng** để lưu và về màn Ký số; **Thoát** để bỏ thay đổi.',
        ],
      },
    ],
  },
  {
    id: 'tao-van-ban',
    icon: 'tao',
    name: 'Tạo văn bản từ mẫu',
    intro: 'Điền dữ liệu vào mẫu Word để tạo hàng loạt văn bản.',
    blocks: [
      {
        h: 'Chuẩn bị mẫu',
        type: 'ul',
        items: [
          'Mẫu là file Word **.docx**; chỗ cần điền viết dạng **{ten_bien}**, ví dụ {ho_ten}, {so_quyet_dinh}.',
          'Dữ liệu là file **.xlsx**, **.xls** hoặc **.csv**, mỗi dòng là một văn bản, dòng tiêu đề là tên cột.',
        ],
      },
      {
        h: 'Từ file dữ liệu',
        type: 'ol',
        items: [
          'Vào **Chức năng › Tạo văn bản** ([[Ctrl+N]]).',
          'Chọn mẫu văn bản và file dữ liệu (chọn sheet và dòng tiêu đề nếu cần).',
          'Kiểm tra phần ghép trường: app tự khớp biến với cột theo tên; chỉnh tay chỗ chưa khớp.',
          'Xem trước từng văn bản, chọn định dạng **PDF** hoặc **DOCX**, đặt mẫu tên file rồi bấm **Tạo văn bản**.',
        ],
      },
      {
        h: 'Nhập trực tiếp và lưu ý',
        type: 'ul',
        items: [
          'Chọn nguồn **Nhập trực tiếp** rồi điền giá trị từng biến để tạo một văn bản.',
          'Cần sửa mẫu: bấm **Sửa mẫu** để mở bằng Word, sửa xong bấm Lưu trong Word rồi bấm **Tải lại**; phần ghép biến và dòng đang xem được giữ.',
          'Bôi đen chữ trên bản xem trước rồi [[Ctrl+C]] để sao chép.',
          'Xuất PDF cần Microsoft Word trên máy; không có Word thì chỉ xuất được DOCX.',
          'Văn bản PDF vừa tạo có thể ký ở màn Ký số (nạp thư mục kết quả).',
        ],
      },
    ],
  },
  {
    id: 'kiem-tra',
    icon: 'shield',
    name: 'Kiểm tra chữ ký',
    intro: 'Xác minh văn bản đã ký có hợp lệ và còn nguyên vẹn.',
    blocks: [
      {
        h: 'Các bước',
        type: 'ol',
        items: [
          'Chức năng › **Kiểm tra chữ ký số** ([[Ctrl+E]]).',
          'Bấm **Chọn file** (hoặc kéo thả) một PDF đã ký. App kiểm mọi chữ ký và dấu thời gian trong file.',
          'Xem từng chữ ký ở tab **Chữ ký** ([[Ctrl+↑]] / [[Ctrl+↓]] để chuyển); tab **Văn bản** cho thấy vị trí, khung cam là chữ ký đang chọn.',
          '**Xuất báo cáo** để lưu kết quả ra PDF.',
        ],
      },
      {
        h: 'App kiểm những gì',
        type: 'ul',
        items: [
          'Theo 5 điều kiện của **Nghị định 68/2024/NĐ-CP, Điều 30**: văn bản toàn vẹn; người ký khớp chứng thư; thời gian ký (dấu thời gian Ban Cơ yếu); chứng thư còn hiệu lực **tại thời điểm ký** (kể cả thu hồi); chuỗi chứng thư về CA gốc Ban Cơ yếu (Root G1 / G2).',
          '**Sửa sau khi ký:** ký xong mà có người sửa nội dung rồi ký thêm, chữ ký trước báo **"Văn bản bị sửa sau khi ký"**. Thêm chữ ký, dấu thời gian, dữ liệu kiểm tra dài hạn sau đó là bình thường; thêm chú thích, điền biểu mẫu thì có lưu ý.',
          '**Kiểm tra thu hồi** (bật sẵn, cần mạng): hỏi máy chủ CA xem chứng thư có bị thu hồi. Bị thu hồi **sau** thời điểm trên dấu thời gian thì chữ ký vẫn hợp lệ. Mất mạng thì báo "chưa kiểm tra được thu hồi".',
          'Chữ ký **không có dấu thời gian** vẫn kiểm người ký, theo giờ máy người ký tự ghi (không chứng minh được).',
          '**Ký dài hạn:** khi ký có mạng, app nhúng sẵn kết quả kiểm thu hồi (OCSP / CRL của Ban Cơ yếu) vào văn bản. Sau này kiểm không cần mạng, kể cả khi chứng thư đã hết hạn; Chi tiết chứng thư ghi "Ký dài hạn: Có".',
          'Chỉ tin chứng thư do CA Ban Cơ yếu cấp. Chữ ký số công cộng (VNPT, Viettel, …) hiện báo "Người ký không thuộc CA Ban Cơ yếu".',
        ],
      },
      {
        h: 'Khi ký số',
        type: 'ul',
        items: [
          'Khi ký, app đã tự kiểm lại từng file vừa ký (chữ ký, dấu thời gian, chứng thư về CA của Ban Cơ yếu) và ghi kết quả ở Nhật ký.',
          'Giờ ký ("Ngày ký" trên khối chữ ký và giờ ký trong file) lấy theo máy chủ dấu thời gian Ban Cơ yếu, mỗi văn bản có dấu thời gian. **Không có mạng** (hoặc máy chủ không trả lời trong 15 giây) thì app ký theo **giờ hiện tại của máy tính**, văn bản **không có dấu thời gian**; Nhật ký và hộp kết quả ghi rõ.',
        ],
      },
    ],
  },
  {
    id: 'pin',
    icon: 'usb',
    name: 'USB Token & mã PIN',
    intro: 'Chứng thư số, mã PIN và việc lưu PIN trên máy.',
    blocks: [
      {
        h: 'Chứng thư số',
        type: 'ul',
        items: [
          'Danh sách chứng thư ở **Cấu hình › USB Token & chứng thư số**; bấm **Làm mới** sau khi cắm token.',
          'Nút thùng rác xoá chứng thư khỏi danh sách của app (không xoá khỏi Windows); **Hiện lại tất cả** để hiện lại.',
          'Chỉ chứng thư của CA Ban Cơ yếu Chính phủ, còn hạn, có khoá trên token mới ký được.',
        ],
      },
      {
        h: 'Mã PIN',
        type: 'ul',
        items: [
          'Nhập mã PIN ở hộp **Xác nhận ký**; để trống thì nhập ở hộp của token.',
          '**Lưu PIN trên máy này** (tắt sẵn): PIN được mã hoá theo tài khoản Windows, lần sau app tự nhập. Ai dùng được tài khoản Windows này và có token cắm vào máy đều ký được mà không cần biết PIN.',
          'Xoá PIN đã lưu ở hộp Xác nhận ký hoặc nút chìa khoá ở **Cấu hình › USB Token**.',
          'PIN đã lưu mà sai thì app xoá ngay và không tự thử lại. Nhập sai nhiều lần token sẽ bị khoá; khi đó liên hệ nơi cấp chứng thư.',
          'Có loại token không nhận PIN do app đưa vào; khi đó hộp PIN của token vẫn hiện như thường và app không lưu PIN.',
        ],
      },
    ],
  },
  {
    id: 'cau-hinh',
    icon: 'gear',
    name: 'Cấu hình',
    intro: 'Mẫu chữ ký, chứng thư số, nơi lưu file.',
    blocks: [
      {
        h: 'Các mục',
        type: 'ul',
        items: [
          '**Chữ ký theo khổ giấy**: mẫu A1–A5; phông, cỡ, màu chữ; lý do ký, nơi ký; ảnh chữ ký / con dấu.',
          '**USB Token & chứng thư số**: chứng thư trên máy, PIN đã lưu.',
          '**Thư mục & tên file**: thư mục con cạnh file gốc hoặc một thư mục cố định; hậu tố tên file; ghi đè khi trùng tên.',
          'Thay đổi chỉ có tác dụng khi bấm **Lưu cấu hình**; **Khôi phục mặc định** để về cấu hình ban đầu.',
          'Cấu hình lưu ở %APPDATA%\\SaoMai-KySo\\config.json. File hỏng thì app giữ bản cũ (đổi tên .hong-…) và dùng mặc định.',
        ],
      },
    ],
  },
  {
    id: 'thong-bao',
    icon: 'send',
    name: 'Thông báo Telegram',
    intro: 'Gửi thông tin sử dụng về hệ thống quản trị của đơn vị qua Telegram. Chỉ có khi đơn vị đã cấu hình cho ứng dụng.',
    blocks: [
      {
        h: 'Đăng ký thiết bị',
        type: 'ul',
        items: [
          'Lần mở đầu tiên trên máy, app đăng ký thiết bị với hệ thống quản trị: gửi một lần mã thiết bị, tên máy, tài khoản Windows, hệ điều hành, kiến trúc CPU, phiên bản ứng dụng, IP nội bộ và địa chỉ MAC.',
          'Tuỳ cấu hình của đơn vị, app **tự đăng ký** (bản phát hành của đơn vị) hoặc hiện hộp **Đăng ký thiết bị** để bạn chọn: **Đồng ý và đăng ký**, **Không đồng ý** (không gửi gì, không hỏi lại), **Để sau**.',
          'Mã thiết bị là số ngẫu nhiên tạo trên máy, không lấy từ phần cứng.',
        ],
      },
      {
        h: 'Những gì được gửi',
        type: 'ul',
        items: [
          'Sau mỗi lượt ký số hoặc tạo văn bản: tên máy, tài khoản Windows, thời gian, tên file, kết quả (thành công, thất bại, hủy) và mã lỗi nếu có. Một lượt nhiều văn bản gửi một tin.',
          'Không bao giờ gửi nội dung văn bản, file PDF, mã PIN, khoá bí mật hay dữ liệu chữ ký.',
          'Mất mạng hoặc Telegram lỗi thì việc ký vẫn chạy bình thường; thông báo nằm trong hàng đợi và được gửi lại sau.',
        ],
      },
      {
        h: 'Bật, tắt, kiểm tra',
        type: 'ul',
        items: [
          'Vào **Hỗ trợ › Thông báo Telegram** để bật / tắt từng loại thông báo, **Kiểm tra kết nối**, **Gửi tin thử**, xem trạng thái đăng ký và hàng đợi.',
          '**Hủy đăng ký** để ngừng gửi từ máy này; đăng ký lại được bất cứ lúc nào.',
          'Nơi nhận (Bot Token, nhóm Telegram hoặc địa chỉ gateway) do đơn vị đặt sẵn trong ứng dụng hoặc trong file notify.json; app chỉ hiện dạng đã che và không cho sửa.',
          'Mọi sự kiện đều được ghi trên máy ở %APPDATA%\\SaoMai-KySo\\nhat-ky-su-kien, kể cả khi thông báo đang tắt.',
        ],
      },
    ],
  },
  {
    id: 'hoi-dap',
    icon: 'help',
    name: 'Câu hỏi thường gặp',
    intro: 'Lỗi hay gặp và cách xử lý.',
    blocks: [
      {
        h: 'Không thấy chứng thư / không nhận USB Token',
        type: 'ul',
        items: [
          'Cắm lại token hoặc đổi cổng USB, rồi bấm **Quét lại** (hộp Xác nhận ký) hoặc **Làm mới** (Cấu hình › USB Token).',
          'Kiểm tra driver ở **Hỗ trợ › Cài đặt driver USB Token**; chưa có thì tải ở trang của Ban Cơ yếu Chính phủ.',
          'Chứng thư đã bị xoá khỏi danh sách: **Cấu hình › USB Token › Hiện lại tất cả**.',
        ],
      },
      {
        h: 'Ký lỗi một số file',
        type: 'ul',
        items: [
          'File đang mở ở chương trình khác: đóng file rồi bấm **Ký tiếp**.',
          'PDF đặt mật khẩu / mã hoá: bỏ mật khẩu trước khi ký.',
          'PDF đã khoá không cho ký thêm (người ký trước chọn "không cho sửa"): không ký thêm được.',
        ],
      },
      {
        h: 'Cần hỗ trợ',
        type: 'ul',
        items: [
          'Nhật ký của app ở %LOCALAPPDATA%\\SaoMai-KySo\\logs (không chứa mã PIN hay nội dung văn bản); gửi kèm khi báo lỗi.',
          'Email hỗ trợ: hotro@saomaiso.com · Website: saomaiso.com.',
        ],
      },
    ],
  },
];

/** Tách đoạn văn thành phần thường / in đậm / phím để vẽ bằng template (không dùng innerHTML). */
export type HelpPart = { t: 'text' | 'b' | 'kbd'; v: string };

export function helpParts(text: string): HelpPart[] {
  const out: HelpPart[] = [];
  const re = /\*\*(.+?)\*\*|\[\[(.+?)\]\]/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) {
      out.push({ t: 'text', v: text.slice(last, m.index) });
    }
    out.push(m[1] !== undefined ? { t: 'b', v: m[1] } : { t: 'kbd', v: m[2] });
    last = re.lastIndex;
  }
  if (last < text.length) {
    out.push({ t: 'text', v: text.slice(last) });
  }
  return out;
}
