"""Sinh dữ liệu mẫu lớn cho chức năng Tạo văn bản.

Mỗi file Excel mẫu trong `mau-tao-van-ban/` được nhân thành một file mới có 2 sheet, mỗi sheet khoảng
100 người. Tiêu đề cột, định dạng (màu tiêu đề, độ rộng cột, ô ngày dd/mm/yyyy, tiền và CCCD lưu dạng
chữ) giữ nguyên như file gốc để ghép được với đúng mẫu .docx.

Toàn bộ họ tên, ngày sinh, CCCD, số điện thoại, địa chỉ là dữ liệu giả, sinh ngẫu nhiên có seed cố định
nên chạy lại ra đúng kết quả cũ.

Cách chạy (từ thư mục gốc repo):
    python -I tools/sinh_du_lieu_mau.py
    python -I tools/sinh_du_lieu_mau.py --src mau-tao-van-ban --out mau-tao-van-ban/du-lieu-100-nguoi --rows 100
"""

from __future__ import annotations

import argparse
import calendar
import random
import shutil
import sys
from copy import copy
from datetime import date, timedelta
from pathlib import Path

import openpyxl

SEED = 20261120
THU = ["thứ Hai", "thứ Ba", "thứ Tư", "thứ Năm", "thứ Sáu", "thứ Bảy", "Chủ nhật"]

HO = (
    ["Nguyễn"] * 12 + ["Trần"] * 5 + ["Lê"] * 4 + ["Phạm"] * 3 + ["Hoàng"] * 2 + ["Vũ"] * 2
    + ["Huỳnh", "Phan", "Võ", "Đặng", "Bùi", "Đỗ", "Hồ", "Ngô", "Dương", "Lý", "Đinh", "Trịnh",
       "Mai", "Lâm", "Tạ", "Cao", "Hà", "Lương", "Tô", "Đào", "Kiều", "Thái"]
)
DEM_NAM = ["Văn", "Đức", "Minh", "Quốc", "Hữu", "Thành", "Công", "Xuân", "Gia", "Hoàng", "Anh", "Quang",
           "Thế", "Trọng", "Tuấn", "Đình", "Mạnh", "Ngọc", "Hải", "Bảo"]
DEM_NU = ["Thị", "Thu", "Ngọc", "Thanh", "Minh", "Phương", "Bích", "Kim", "Thùy", "Hồng", "Mai",
          "Diệu", "Khánh", "Hải", "Lan", "Bảo", "Thị Thu", "Thị Ngọc", "Thị Thanh", "Thị Minh"]
TEN_NAM = ["An", "Bình", "Cường", "Dũng", "Đạt", "Hải", "Hiếu", "Hùng", "Huy", "Khang", "Khoa", "Kiên",
           "Long", "Lâm", "Mạnh", "Minh", "Nam", "Nghĩa", "Phong", "Phúc", "Quân", "Sơn", "Tài",
           "Thắng", "Thành", "Trung", "Tuấn", "Tùng", "Việt", "Vinh", "Duy", "Hoàng", "Toàn", "Thịnh",
           "Khải", "Đông", "Hưng", "Lộc", "Quang", "Tiến"]
TEN_NU = ["Anh", "Châu", "Chi", "Dung", "Giang", "Hà", "Hạnh", "Hằng", "Hiền", "Hoa", "Huyền", "Hương",
          "Lan", "Linh", "Loan", "Mai", "My", "Nga", "Ngân", "Nhung", "Oanh", "Phương", "Quỳnh", "Thảo",
          "Trang", "Trinh", "Tú", "Uyên", "Vân", "Yến", "Ngọc", "Thủy", "Hồng", "Diệp", "Nhi", "Thư"]

PHONG_BAN_HN = ["Ban Giám đốc", "Văn phòng Công ty", "Phòng Hành chính - Nhân sự",
                "Phòng Tài chính - Kế toán", "Phòng Kinh doanh", "Phòng Công nghệ thông tin",
                "Phòng Chăm sóc khách hàng", "Phòng Pháp chế", "Phòng Kế hoạch - Đầu tư",
                "Phòng Marketing"]
CHI_NHANH = ["Chi nhánh Đà Nẵng", "Chi nhánh Hồ Chí Minh", "Chi nhánh Hải Phòng", "Chi nhánh Cần Thơ"]

CHUC_DANH = {
    "Ban Giám đốc": ["Phó Giám đốc", "Trợ lý Giám đốc", "Thư ký Ban Giám đốc"],
    "Văn phòng Công ty": ["Chánh Văn phòng", "Văn thư", "Chuyên viên tổng hợp", "Nhân viên lễ tân"],
    "Phòng Hành chính - Nhân sự": ["Chuyên viên nhân sự", "Chuyên viên tuyển dụng",
                                   "Chuyên viên tiền lương", "Nhân viên hành chính"],
    "Phòng Tài chính - Kế toán": ["Kế toán viên", "Kế toán tổng hợp", "Kế toán thuế", "Thủ quỹ"],
    "Phòng Kinh doanh": ["Nhân viên kinh doanh", "Chuyên viên phát triển thị trường",
                         "Trưởng nhóm kinh doanh", "Chuyên viên dự án"],
    "Phòng Công nghệ thông tin": ["Kỹ sư phần mềm", "Lập trình viên", "Kỹ sư hệ thống",
                                  "Chuyên viên an toàn thông tin", "Kỹ sư kiểm thử"],
    "Phòng Chăm sóc khách hàng": ["Nhân viên chăm sóc khách hàng", "Trưởng nhóm", "Chuyên viên hỗ trợ"],
    "Phòng Pháp chế": ["Chuyên viên pháp chế", "Chuyên viên hợp đồng"],
    "Phòng Kế hoạch - Đầu tư": ["Chuyên viên kế hoạch", "Chuyên viên phân tích đầu tư"],
    "Phòng Marketing": ["Chuyên viên marketing", "Chuyên viên truyền thông", "Thiết kế đồ hoạ"],
}
CHUC_DANH_CHI_NHANH = ["Nhân viên kinh doanh", "Trưởng nhóm kinh doanh", "Kế toán viên",
                       "Nhân viên hành chính", "Chuyên viên chăm sóc khách hàng", "Kỹ thuật viên",
                       "Nhân viên kho vận"]

THANH_TICH = {
    "Phòng Công nghệ thông tin": [
        "hoàn thành xuất sắc dự án số hoá hồ sơ, đưa hệ thống ký số vào vận hành trước tiến độ",
        "xây dựng thành công nền tảng quản lý văn bản điện tử dùng chung toàn Công ty",
        "bảo đảm hệ thống công nghệ thông tin vận hành an toàn, không gián đoạn trong năm 2026",
        "triển khai sao lưu dữ liệu tập trung, rút ngắn thời gian khôi phục sự cố còn dưới 1 giờ",
    ],
    "Phòng Tài chính - Kế toán": [
        "hoàn thành quyết toán thuế năm 2025 đúng hạn, không phát sinh sai sót",
        "chuẩn hoá quy trình thanh toán, rút ngắn thời gian xử lý chứng từ 30%",
        "triển khai hoá đơn điện tử cho toàn bộ chi nhánh đúng kế hoạch",
    ],
    "Phòng Kinh doanh": [
        "vượt 135% chỉ tiêu doanh số năm 2026",
        "phát triển thành công 25 khách hàng doanh nghiệp mới",
        "đạt doanh số cao nhất khối kinh doanh trong 3 quý liên tiếp",
    ],
    "Phòng Chăm sóc khách hàng": [
        "duy trì tỷ lệ hài lòng của khách hàng trên 98% trong 12 tháng liên tục",
        "xử lý dứt điểm 100% phản ánh của khách hàng trong thời hạn cam kết",
    ],
    "Phòng Hành chính - Nhân sự": [
        "xây dựng và triển khai Quy chế thi đua, khen thưởng mới của Công ty",
        "tổ chức tuyển dụng đủ nhân sự cho các dự án trọng điểm năm 2026",
        "triển khai thành công chương trình đào tạo nội bộ cho hơn 200 lượt người",
    ],
}
THANH_TICH_CHUNG = [
    "có nhiều sáng kiến cải tiến quy trình làm việc, được áp dụng rộng rãi trong Công ty",
    "hoàn thành xuất sắc nhiệm vụ được giao năm 2026",
    "tích cực tham gia phong trào thi đua, gương mẫu chấp hành nội quy lao động",
    "hoàn thành vượt mức kế hoạch công tác được giao trong năm 2026",
    "có đóng góp nổi bật trong công tác chuyển đổi số của đơn vị",
]

DIA_CHI = {
    "Hà Nội": ["Đống Đa", "Cầu Giấy", "Thanh Xuân", "Hai Bà Trưng", "Hoàng Mai", "Long Biên",
               "Nam Từ Liêm", "Bắc Từ Liêm", "Hà Đông", "Tây Hồ", "Ba Đình"],
    "Đà Nẵng": ["Hải Châu", "Thanh Khê", "Sơn Trà", "Ngũ Hành Sơn", "Liên Chiểu", "Cẩm Lệ"],
    "TP. Hồ Chí Minh": ["Quận 1", "Quận 3", "Quận 7", "Quận 10", "Bình Thạnh", "Phú Nhuận", "Tân Bình",
                        "Gò Vấp", "Thủ Đức"],
    "Hải Phòng": ["Lê Chân", "Ngô Quyền", "Hồng Bàng", "Hải An", "Kiến An"],
    "Cần Thơ": ["Ninh Kiều", "Bình Thủy", "Cái Răng", "Ô Môn"],
}
DUONG = ["Trần Duy Hưng", "Nguyễn Trãi", "Lê Duẩn", "Trường Chinh", "Láng Hạ", "Hoàng Quốc Việt",
         "Nguyễn Văn Linh", "Lê Lợi", "Hai Bà Trưng", "Điện Biên Phủ", "Phạm Văn Đồng", "Võ Văn Kiệt",
         "Trần Phú", "Nguyễn Huệ", "Lạch Tray", "Cách Mạng Tháng Tám", "Kim Mã", "Xuân Thủy"]
CHI_NHANH_THANH_PHO = {
    "Chi nhánh Đà Nẵng": "Đà Nẵng",
    "Chi nhánh Hồ Chí Minh": "TP. Hồ Chí Minh",
    "Chi nhánh Hải Phòng": "Hải Phòng",
    "Chi nhánh Cần Thơ": "Cần Thơ",
}
MA_TINH = {"Hà Nội": "001", "Hải Phòng": "031", "Đà Nẵng": "048", "TP. Hồ Chí Minh": "079",
           "Cần Thơ": "092"}
MA_TINH_KHAC = ["034", "035", "036", "037", "038", "040", "042", "049", "052", "075", "083", "089"]
DAU_SO = ["090", "091", "093", "094", "096", "097", "098", "086", "088", "089", "032", "033", "034",
          "035", "036", "037", "038", "039", "070", "076", "077", "078", "079", "081", "082", "083",
          "084", "085"]

CHU_SO = ["không", "một", "hai", "ba", "bốn", "năm", "sáu", "bảy", "tám", "chín"]


def doc_ba_so(n: int, day_du: bool) -> str:
    tram, chuc, dv = n // 100, (n // 10) % 10, n % 10
    tu: list[str] = []
    if day_du or tram:
        tu += [CHU_SO[tram], "trăm"]
    if chuc == 0:
        if dv:
            if day_du or tram:
                tu.append("linh")
            tu.append(CHU_SO[dv])
    elif chuc == 1:
        tu.append("mười")
        if dv:
            tu.append("lăm" if dv == 5 else CHU_SO[dv])
    else:
        tu += [CHU_SO[chuc], "mươi"]
        if dv:
            tu.append({1: "mốt", 4: "tư", 5: "lăm"}.get(dv, CHU_SO[dv]))
    return " ".join(tu)


def tien_bang_chu(so: int) -> str:
    """10_500_000 -> 'Mười triệu năm trăm nghìn đồng'."""
    if so == 0:
        return "Không đồng"
    nhom = [(1_000_000_000, "tỷ"), (1_000_000, "triệu"), (1_000, "nghìn"), (1, "")]
    tu: list[str] = []
    con = so
    da_co = False
    for gia_tri, ten in nhom:
        phan = con // gia_tri
        con %= gia_tri
        if phan:
            tu.append(doc_ba_so(phan, day_du=da_co))
            if ten:
                tu.append(ten)
            da_co = True
    chuoi = " ".join(t for t in tu if t)
    return chuoi[0].upper() + chuoi[1:] + " đồng"


def tien(so: int) -> str:
    return f"{so:,}".replace(",", ".")


def ngay_chu(d: date) -> str:
    return f"{d.day:02d} tháng {d.month} năm {d.year}"


def thoi_gian(gio: int, phut: int, d: date) -> str:
    return f"{gio:02d} giờ {phut:02d} phút, {THU[d.weekday()]}, ngày {d:%d/%m/%Y}"


def cong_thang(d: date, thang: int) -> date:
    m = d.month - 1 + thang
    y, m = d.year + m // 12, m % 12 + 1
    return date(y, m, min(d.day, calendar.monthrange(y, m)[1]))


class Nguoi:
    def __init__(self, rng: random.Random, da_dung: set[str]):
        while True:
            self.nam = rng.random() < 0.5
            dem = rng.choice(DEM_NAM if self.nam else DEM_NU)
            ten = rng.choice(TEN_NAM if self.nam else TEN_NU)
            self.ho_ten = f"{rng.choice(HO)} {dem} {ten}"
            if self.ho_ten not in da_dung:
                da_dung.add(self.ho_ten)
                break
        self.ngay_sinh = date(rng.randint(1972, 2003), rng.randint(1, 12), rng.randint(1, 28))


def lay_khuon(src: Path, ten_file: str):
    """Đọc file gốc: workbook, kiểu chữ tiêu đề và kiểu chữ từng cột ở dòng dữ liệu đầu."""
    wb = openpyxl.load_workbook(src / ten_file)
    ws = wb.worksheets[0]
    kieu_cot = []
    for c in ws[2]:
        kieu_cot.append((copy(c.font), copy(c.alignment), c.number_format, copy(c.border), copy(c.fill)))
    ws.delete_rows(2, ws.max_row)
    return wb, ws, kieu_cot


def ghi_sheet(ws, ten: str, kieu_cot, dong: list[list]):
    ws.title = ten
    ws.freeze_panes = "A2"
    for i, gia_tri in enumerate(dong, start=2):
        for j, v in enumerate(gia_tri, start=1):
            c = ws.cell(row=i, column=j, value=v)
            font, align, fmt, border, fill = kieu_cot[j - 1]
            c.font, c.alignment, c.border, c.fill = copy(font), copy(align), copy(border), copy(fill)
            c.number_format = fmt if isinstance(v, date) else "General"


def tao_file(src: Path, out: Path, ten_file: str, ten_moi: str, sheets: list[tuple[str, list[list]]]):
    wb, ws1, kieu_cot = lay_khuon(src, ten_file)
    ws2 = wb.copy_worksheet(ws1)
    for ws, (ten, dong) in zip((ws1, ws2), sheets):
        ghi_sheet(ws, ten, kieu_cot, dong)
    wb.active = 0
    wb.save(out / ten_moi)
    return [(ten, len(dong)) for ten, dong in sheets]


# ---------------------------------------------------------------- 1. Giấy mời họp


def du_lieu_giay_moi(rng: random.Random, so_dong: int):
    def nguoi_moi(n: int, da_dung: set[str]):
        dong = []
        for _ in range(n):
            p = Nguoi(rng, da_dung)
            if rng.random() < 0.2:
                don_vi = rng.choice(CHI_NHANH)
                chuc_vu = rng.choice(["Giám đốc chi nhánh", "Phó Giám đốc chi nhánh", "Trưởng nhóm",
                                      "Chuyên viên"])
            else:
                don_vi = rng.choice(PHONG_BAN_HN)
                chuc_vu = (rng.choice(["Phó Giám đốc", "Trợ lý Giám đốc"]) if don_vi == "Ban Giám đốc"
                           else rng.choice(["Trưởng phòng", "Phó trưởng phòng", "Trưởng nhóm",
                                            "Chuyên viên", "Chuyên viên chính", "Nhân viên"]))
            dong.append((("Ông" if p.nam else "Bà"), p.ho_ten, chuc_vu, don_vi))
        return dong

    cuoc_hop_1 = [
        dict(so="130/GM-ABC", ky=date(2026, 12, 5),
             ten="Hội nghị tổng kết công tác năm 2026 và triển khai nhiệm vụ năm 2027",
             noi_dung="Tổng kết kết quả sản xuất kinh doanh năm 2026, biểu dương tập thể, cá nhân tiêu "
                      "biểu và triển khai kế hoạch năm 2027.",
             tg=thoi_gian(8, 0, date(2026, 12, 26)),
             dd="Hội trường tầng 8, trụ sở Công ty, số 12 Trần Duy Hưng, Cầu Giấy, Hà Nội",
             tp="Ban Giám đốc, Trưởng, Phó các phòng ban, chi nhánh và cá nhân được khen thưởng"),
        dict(so="131/GM-ABC", ky=date(2026, 12, 5),
             ten="Hội nghị người lao động năm 2027",
             noi_dung="Báo cáo thực hiện Thoả ước lao động tập thể, lấy ý kiến về Nội quy lao động và "
                      "Quy chế trả lương năm 2027.",
             tg=thoi_gian(14, 0, date(2026, 12, 26)),
             dd="Hội trường tầng 8, trụ sở Công ty, số 12 Trần Duy Hưng, Cầu Giấy, Hà Nội",
             tp="Đại biểu người lao động các phòng ban, chi nhánh"),
    ]
    cuoc_hop_2 = [
        dict(so="140/GM-ABC", ky=date(2026, 12, 8),
             ten="Tập huấn sử dụng phần mềm ký số văn bản điện tử (đợt 1)",
             noi_dung="Hướng dẫn tạo văn bản hàng loạt từ mẫu, ký số bằng USB token Ban Cơ yếu Chính phủ "
                      "và kiểm tra chữ ký số trên văn bản PDF.",
             tg=thoi_gian(8, 30, date(2026, 12, 15)),
             dd="Phòng đào tạo tầng 3, trụ sở Công ty",
             tp="Văn thư, cán bộ phụ trách hành chính các phòng ban"),
        dict(so="141/GM-ABC", ky=date(2026, 12, 8),
             ten="Tập huấn sử dụng phần mềm ký số văn bản điện tử (đợt 2)",
             noi_dung="Hướng dẫn tạo văn bản hàng loạt từ mẫu, ký số bằng USB token Ban Cơ yếu Chính phủ "
                      "và kiểm tra chữ ký số trên văn bản PDF.",
             tg=thoi_gian(14, 0, date(2026, 12, 17)),
             dd="Phòng họp trực tuyến (Microsoft Teams) và điểm cầu các chi nhánh",
             tp="Cán bộ phụ trách hành chính các chi nhánh"),
    ]

    def sheet(cuoc_hop, chia):
        da_dung: set[str] = set()
        dong = []
        for ch, n in zip(cuoc_hop, chia):
            for danh_xung, ho_ten, chuc_vu, don_vi in nguoi_moi(n, da_dung):
                dong.append([ch["so"], ngay_chu(ch["ky"]), danh_xung, ho_ten, chuc_vu, don_vi, ch["ten"],
                             ch["noi_dung"], ch["tg"], ch["dd"], ch["tp"]])
        return dong

    n1 = so_dong * 3 // 5
    n2 = so_dong // 2
    return [("TongKet_2026", sheet(cuoc_hop_1, [n1, so_dong - n1])),
            ("TapHuan_KySo", sheet(cuoc_hop_2, [n2, so_dong - n2]))]


# ---------------------------------------------------------------- 2. Quyết định khen thưởng


def du_lieu_khen_thuong(rng: random.Random, so_dong: int):
    def sheet(so_dau: int, ngay_ky: date, chi_nhanh_ty_le: float):
        da_dung: set[str] = set()
        dong = []
        for i in range(so_dong):
            p = Nguoi(rng, da_dung)
            if rng.random() < chi_nhanh_ty_le:
                phong = rng.choice(CHI_NHANH)
                chuc_vu = rng.choice(CHUC_DANH_CHI_NHANH)
            else:
                phong = rng.choice([k for k in CHUC_DANH if k != "Ban Giám đốc"])
                chuc_vu = rng.choice(CHUC_DANH[phong])
            thanh_tich = rng.choice(THANH_TICH.get(phong, []) + THANH_TICH_CHUNG)
            muc = rng.randrange(2_000_000, 15_000_001, 500_000)
            dong.append([f"{so_dau + i}/QĐ-ABC", ngay_chu(ngay_ky), "ông" if p.nam else "bà", p.ho_ten,
                         p.ngay_sinh, chuc_vu, phong, thanh_tich, tien(muc), tien_bang_chu(muc), None])
        # Hai dòng cố ý trùng họ tên với dòng khác để thử quy tắc đặt tên file khi trùng.
        for nguon, dich in ((4, 37), (12, 81)):
            if dich < len(dong):
                dong[dich][3] = dong[nguon][3]
                dong[dich][10] = f"Trùng tên với dòng {nguon + 2} – để thử quy tắc đặt tên file"
        return dong

    return [("CaNhan_Nam2026", sheet(301, date(2026, 12, 15), 0.25)),
            ("CaNhan_Quy4_2026", sheet(301 + so_dong, date(2026, 12, 28), 0.4))]


# ---------------------------------------------------------------- 3. Hợp đồng lao động


def du_lieu_hop_dong(rng: random.Random, so_dong: int):
    def dia_chi(thanh_pho: str) -> str:
        quan = rng.choice(DIA_CHI[thanh_pho])
        kieu = rng.random()
        if kieu < 0.5:
            return f"Số {rng.randint(1, 350)} {rng.choice(DUONG)}, {quan}, {thanh_pho}"
        if kieu < 0.85:
            return (f"Số {rng.randint(1, 60)} ngõ {rng.randint(10, 250)} {rng.choice(DUONG)}, {quan}, "
                    f"{thanh_pho}")
        return f"Tổ {rng.randint(1, 30)}, phường {rng.randint(1, 15)}, {quan}, {thanh_pho}"

    def cccd(p: Nguoi, thanh_pho: str) -> str:
        tinh = MA_TINH[thanh_pho] if rng.random() < 0.55 else rng.choice(MA_TINH_KHAC)
        the_ky = 0 if p.ngay_sinh.year < 2000 else 2
        gioi = the_ky + (0 if p.nam else 1)
        return f"{tinh}{gioi}{p.ngay_sinh.year % 100:02d}{rng.randint(0, 999999):06d}"

    def sheet(so_dau: int, bat_dau: date, ngay_ky: date, chi_nhanh: bool):
        da_dung: set[str] = set()
        dong = []
        for i in range(so_dong):
            p = Nguoi(rng, da_dung)
            if chi_nhanh:
                phong = rng.choice(CHI_NHANH)
                thanh_pho = CHI_NHANH_THANH_PHO[phong]
                chuc_danh = rng.choice(CHUC_DANH_CHI_NHANH)
                noi_lam = f"Chi nhánh Công ty tại {thanh_pho}"
            else:
                phong = rng.choice([k for k in CHUC_DANH if k != "Ban Giám đốc"])
                thanh_pho = "Hà Nội"
                chuc_danh = rng.choice(CHUC_DANH[phong])
                noi_lam = "Trụ sở Công ty tại Hà Nội"
            thang = rng.choice([12, 12, 24, 24, 36])
            ket_thuc = cong_thang(bat_dau, thang) - timedelta(days=1)
            ngay_cap = date(rng.randint(2021, 2025), rng.randint(1, 12), rng.randint(1, 28))
            luong = rng.randrange(8_000_000, 35_000_001, 500_000)
            phu_cap = rng.randrange(500_000, 5_000_001, 250_000)
            dong.append([f"HĐ-2026-{so_dau + i:04d}", ngay_chu(ngay_ky), p.ho_ten, p.ngay_sinh,
                         "Nam" if p.nam else "Nữ", cccd(p, thanh_pho), ngay_cap,
                         "Cục Cảnh sát QLHC về TTXH", dia_chi(thanh_pho),
                         f"{rng.choice(DAU_SO)}{rng.randint(0, 9_999_999):07d}", chuc_danh, phong, noi_lam,
                         f"{thang} tháng", bat_dau, ket_thuc, tien(luong), tien(phu_cap)])
        return dong

    return [("HD_TruSo_HaNoi", sheet(101, date(2026, 12, 1), date(2026, 12, 1), False)),
            ("HD_ChiNhanh", sheet(101 + so_dong, date(2026, 12, 15), date(2026, 12, 15), True))]


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--src", type=Path, default=Path("mau-tao-van-ban"))
    ap.add_argument("--out", type=Path, default=Path("mau-tao-van-ban/du-lieu-100-nguoi"))
    ap.add_argument("--rows", type=int, default=100, help="số người mỗi sheet")
    a = ap.parse_args()

    a.out.mkdir(parents=True, exist_ok=True)
    rng = random.Random(SEED)
    viec = [
        ("1_giay_moi_hop", du_lieu_giay_moi),
        ("2_quyet_dinh_khen_thuong", du_lieu_khen_thuong),
        ("3_hop_dong_lao_dong", du_lieu_hop_dong),
    ]
    for ten, sinh in viec:
        sheets = sinh(rng, a.rows)
        ten_moi = f"{ten}_{a.rows}_nguoi.xlsx"
        ket_qua = tao_file(a.src, a.out, f"{ten}.xlsx", ten_moi, sheets)
        shutil.copy2(a.src / f"{ten}.docx", a.out / f"{ten}.docx")
        print(f"{ten_moi}: " + ", ".join(f"{s} = {n} dòng" for s, n in ket_qua))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
