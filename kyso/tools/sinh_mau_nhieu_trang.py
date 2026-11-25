"""Sinh bộ mẫu Tạo văn bản có độ dài khác nhau (1–4 trang), mỗi mẫu kèm file Excel khoảng 100 dòng.

Mỗi mẫu là một cặp `.docx` + `.xlsx`:
    4_thong_bao_nghi_phep       1 trang  (thông báo chấp thuận nghỉ phép năm)
    5_quyet_dinh_nang_luong     2 trang  (quyết định nâng bậc lương)
    6_thoa_thuan_bao_mat        3 trang  (thỏa thuận bảo mật thông tin)
    7_hop_dong_dao_tao          4 trang  (hợp đồng đào tạo, cam kết làm việc)

Khung văn bản (tiêu ngữ, bảng chữ ký, cỡ chữ, lề trang) lấy từ các mẫu gốc trong `mau-tao-van-ban/`
để in ra giống bộ mẫu cũ. Tên cột Excel là tên biến viết có dấu (ví dụ cột "Số thông báo" ghép với
`{so_thong_bao}`), sheet `DuLieu`, cố định dòng 1, ô ngày dạng dd/mm/yyyy, tiền và CCCD lưu dạng chữ.

Toàn bộ họ tên, ngày sinh, CCCD, số điện thoại, địa chỉ là dữ liệu giả, sinh ngẫu nhiên có seed cố định.

Cách chạy (từ thư mục gốc repo):
    python -I tools/sinh_mau_nhieu_trang.py
    python -I tools/sinh_mau_nhieu_trang.py --out mau-tao-van-ban/mau-nhieu-trang --rows 100
"""

from __future__ import annotations

import argparse
import random
import re
import sys
import unicodedata
from copy import deepcopy
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True  # không để lại tools/__pycache__ khi nhập sinh_du_lieu_mau

import docx  # noqa: E402  (python-docx)
import openpyxl  # noqa: E402
from openpyxl.styles import Alignment, Border, Font, PatternFill, Side  # noqa: E402
from openpyxl.utils import get_column_letter  # noqa: E402

from sinh_du_lieu_mau import (  # noqa: E402
    CHI_NHANH, CHI_NHANH_THANH_PHO, CHUC_DANH, CHUC_DANH_CHI_NHANH, DAU_SO, DIA_CHI, DUONG, MA_TINH,
    MA_TINH_KHAC, Nguoi, cong_thang, ngay_chu, tien, tien_bang_chu,
)

SEED = 20261123
NGHI_LE_2026 = {date(2026, 12, 31)}
NGHI_LE_2027 = {date(2027, 1, 1), date(2027, 2, 5), date(2027, 2, 8), date(2027, 2, 9), date(2027, 2, 10),
                date(2027, 2, 11), date(2027, 4, 16), date(2027, 4, 30), date(2027, 5, 1)}
NGHI_LE = NGHI_LE_2026 | NGHI_LE_2027

# ====================================================================== khung văn bản Word


class Khung:
    """Các đoạn mẫu (định dạng) lấy từ 3 file .docx gốc."""

    def __init__(self, src: Path):
        self.src = src
        gm = docx.Document(src / "1_giay_moi_hop.docx").paragraphs
        qd = docx.Document(src / "2_quyet_dinh_khen_thuong.docx").paragraphs
        hd = docx.Document(src / "3_hop_dong_lao_dong.docx").paragraphs
        self.mau = {
            "title": qd[0],    # QUYẾT ĐỊNH – đậm 14, giữa
            "sub": qd[1],      # Về việc ... – đậm, giữa
            "subi": hd[1],     # (Xác định thời hạn ...) – nghiêng, giữa
            "sep": qd[2],      # ———————
            "cb": qd[3],       # GIÁM ĐỐC CÔNG TY ... – đậm, giữa
            "qd": qd[9],       # QUYẾT ĐỊNH: – đậm, giữa, cách trên dưới
            "center": gm[2],   # Kính gửi: ... – giữa
            "cancu": hd[2],    # Căn cứ ... – nghiêng, đều hai bên, thụt đầu dòng
            "body": hd[5],     # đoạn thường, đều hai bên, thụt đầu dòng, cách trên
            "dieu": qd[10],    # Điều 1. ... – đều hai bên, thụt đầu dòng
            "party": hd[6],    # BÊN A – ... – đậm, trái, cách trên
            "head": hd[20],    # Điều 1. Tên điều – đậm, trái, cách trên
            "kv": hd[7],       # Nhãn: giá trị – trái
            "bl": hd[21],      # - gạch đầu dòng, đều hai bên, thụt đầu dòng
        }

    def doan(self, kieu: str, text: str):
        """Tạo phần tử <w:p> theo kiểu `kieu`. `**chữ**` trong `text` là chữ đậm."""
        mau = self.mau[kieu]
        p = deepcopy(mau._p)
        runs = p.findall(docx.oxml.ns.qn("w:r"))
        rpr = runs[0].find(docx.oxml.ns.qn("w:rPr")) if runs else None
        for r in runs:
            p.remove(r)
        phan = text.split("**")
        co_dam = len(phan) > 1
        for i, chu in enumerate(phan):
            if not chu:
                continue
            r = docx.oxml.OxmlElement("w:r")
            if rpr is not None:
                r.append(deepcopy(rpr))
            if co_dam:
                _dat_dam(r, i % 2 == 1)
            t = docx.oxml.OxmlElement("w:t")
            t.set(docx.oxml.ns.qn("xml:space"), "preserve")
            t.text = chu
            r.append(t)
            p.append(r)
        return p

    def tao(self, goc: str, so_bien: str, noi_dung: list[tuple[str, str]], chu_ky: dict, chan_trang: str | None,
            out: Path):
        """Dựng văn bản từ file gốc `goc`: giữ bảng tiêu ngữ, thay thân bài, sửa bảng chữ ký."""
        d = docx.Document(self.src / goc)
        body = d.element.body
        bang = body.findall(docx.oxml.ns.qn("w:tbl"))
        tieu_ngu, ky = bang[0], bang[-1]
        for p in body.findall(docx.oxml.ns.qn("w:p")):
            body.remove(p)
        for kieu, text in noi_dung:
            ky.addprevious(self.doan(kieu, text))

        for t in tieu_ngu.iter(docx.oxml.ns.qn("w:t")):
            t.text = re.sub(r"\{so_[a-z_]+\}", "{" + so_bien + "}", t.text)

        o_trai, o_phai = d.tables[-1].rows[0].cells
        if chu_ky["kieu"] == "noi_nhan":
            dong = o_trai.paragraphs
            mau_dong = dong[1]._p
            for p in dong[1:]:
                p._p.getparent().remove(p._p)
            for text in chu_ky["noi_nhan"]:
                p = deepcopy(mau_dong)
                p.findall(docx.oxml.ns.qn("w:r"))[0].find(docx.oxml.ns.qn("w:t")).text = text
                o_trai._tc.append(p)
        else:  # hai bên ký
            _dat_chu(o_trai.paragraphs[0], chu_ky["trai"])
            _dat_chu(o_phai.paragraphs[0], chu_ky["phai"])
            if "ten_trai" in chu_ky:
                _dat_chu(o_trai.paragraphs[-1], chu_ky["ten_trai"])
        # Chân trang của mẫu hợp đồng gốc ("Hợp đồng lao động số ... – Trang x/y"): đổi tên loại văn bản.
        for sec in d.sections:
            for p in sec.footer.paragraphs:
                if p.runs and p.runs[0].text.startswith("Hợp đồng lao động số"):
                    p.runs[0].text = chan_trang + " – Trang "
        d.core_properties.title = noi_dung[0][1]
        d.save(out)


def _dat_dam(r, dam: bool):
    rpr = r.find(docx.oxml.ns.qn("w:rPr"))
    if rpr is None:
        rpr = docx.oxml.OxmlElement("w:rPr")
        r.insert(0, rpr)
    for tag in ("w:b", "w:bCs"):
        cu = rpr.find(docx.oxml.ns.qn(tag))
        if cu is not None:
            rpr.remove(cu)
    if dam:
        rpr.insert(0, docx.oxml.OxmlElement("w:b"))


def _dat_chu(p, text: str):
    p.runs[0].text = text
    for r in p.runs[1:]:
        r.text = ""


# ====================================================================== dữ liệu dùng chung


def la_ngay_lam(d: date) -> bool:
    return d.weekday() < 5 and d not in NGHI_LE


def cong_ngay_lam(d: date, n: int) -> date:
    """Ngày làm việc thứ n tính từ d (d là ngày thứ 1)."""
    while not la_ngay_lam(d):
        d += timedelta(days=1)
    dem = 1
    while dem < n:
        d += timedelta(days=1)
        if la_ngay_lam(d):
            dem += 1
    return d


def ngay_lam_tiep(d: date) -> date:
    d += timedelta(days=1)
    while not la_ngay_lam(d):
        d += timedelta(days=1)
    return d


def phong_va_chuc(rng: random.Random, ty_le_chi_nhanh: float) -> tuple[str, str]:
    if rng.random() < ty_le_chi_nhanh:
        return rng.choice(CHI_NHANH), rng.choice(CHUC_DANH_CHI_NHANH)
    phong = rng.choice([k for k in CHUC_DANH if k != "Ban Giám đốc"])
    return phong, rng.choice(CHUC_DANH[phong])


def thanh_pho_cua(phong: str) -> str:
    return CHI_NHANH_THANH_PHO.get(phong, "Hà Nội")


def dia_chi(rng: random.Random, thanh_pho: str) -> str:
    quan = rng.choice(DIA_CHI[thanh_pho])
    kieu = rng.random()
    if kieu < 0.5:
        return f"Số {rng.randint(1, 350)} {rng.choice(DUONG)}, {quan}, {thanh_pho}"
    if kieu < 0.85:
        return f"Số {rng.randint(1, 60)} ngõ {rng.randint(10, 250)} {rng.choice(DUONG)}, {quan}, {thanh_pho}"
    return f"Tổ {rng.randint(1, 30)}, phường {rng.randint(1, 15)}, {quan}, {thanh_pho}"


def cccd(rng: random.Random, p: Nguoi, thanh_pho: str) -> str:
    tinh = MA_TINH[thanh_pho] if rng.random() < 0.55 else rng.choice(MA_TINH_KHAC)
    gioi = (0 if p.ngay_sinh.year < 2000 else 2) + (0 if p.nam else 1)
    return f"{tinh}{gioi}{p.ngay_sinh.year % 100:02d}{rng.randint(0, 999999):06d}"


def sdt(rng: random.Random) -> str:
    return f"{rng.choice(DAU_SO)}{rng.randint(0, 9_999_999):07d}"


def khong_dau(s: str) -> str:
    s = unicodedata.normalize("NFD", s.replace("Đ", "D").replace("đ", "d"))
    return "".join(c for c in s if not unicodedata.combining(c))


def email(p: Nguoi, da_dung: set[str]) -> str:
    phan = khong_dau(p.ho_ten).lower().split()
    goc = phan[-1] + "." + "".join(x[0] for x in phan[:-1])
    ten, i = goc, 1
    while ten in da_dung:
        i += 1
        ten = f"{goc}{i}"
    da_dung.add(ten)
    return f"{ten}@abc.com.vn"


def ma_nv(i: int) -> str:
    return f"ABC{i:05d}"


# ====================================================================== 4. Thông báo nghỉ phép – 1 trang

TB_COT = ["Số thông báo", "Ngày ký", "Danh xưng", "Họ tên", "Chức vụ", "Phòng ban", "Ngày đơn",
          "Số ngày nghỉ", "Từ ngày", "Đến ngày", "Ngày đi làm", "Lý do", "Phép còn lại",
          "Người nhận bàn giao", "Số điện thoại"]

TB_NOI_DUNG = [
    ("title", "THÔNG BÁO"),
    ("sub", "Về việc chấp thuận nghỉ phép năm"),
    ("sep", "———————"),
    ("center", "Kính gửi: {danh_xung} **{ho_ten}** – {chuc_vu}, {phong_ban}."),
    ("body", "Căn cứ Bộ luật Lao động số 45/2019/QH14 và Nội quy lao động của Công ty Cổ phần ABC; xét Đơn "
             "xin nghỉ phép ngày {ngay_don} của {danh_xung} {ho_ten} và ý kiến của người quản lý trực tiếp, "
             "Công ty thông báo như sau:"),
    ("dieu", "**1.** Chấp thuận cho {danh_xung} {ho_ten} nghỉ phép năm **{so_ngay_nghi} ngày làm việc**, từ "
             "ngày {tu_ngay} đến hết ngày {den_ngay}. Lý do: {ly_do}."),
    ("dieu", "**2.** Thời gian nghỉ được trừ vào số ngày nghỉ hằng năm và được hưởng nguyên lương theo quy "
             "định. Số ngày phép năm còn lại sau đợt nghỉ: {phep_con_lai} ngày."),
    ("dieu", "**3.** Trước khi nghỉ, {danh_xung} có trách nhiệm bàn giao đầy đủ công việc, hồ sơ đang xử lý "
             "cho {nguoi_nhan_ban_giao} và giữ liên lạc qua số điện thoại {so_dien_thoai} khi cần trao đổi "
             "công việc gấp."),
    ("dieu", "**4.** {danh_xung} trở lại làm việc từ ngày **{ngay_di_lam}**. Trường hợp cần kéo dài thời gian "
             "nghỉ, phải báo cáo bằng văn bản với người quản lý trực tiếp và Phòng Hành chính - Nhân sự "
             "trước ít nhất 02 ngày làm việc."),
    ("body", "Phòng Hành chính - Nhân sự, Trưởng đơn vị nơi {danh_xung} {ho_ten} công tác và cá nhân có tên "
             "trên chịu trách nhiệm thực hiện Thông báo này./."),
]
TB_CHU_KY = {"kieu": "noi_nhan", "noi_nhan": ["- Như trên;", "- Phòng HCNS (theo dõi chấm công);",
                                              "- Lưu: VT."]}

LY_DO = ["giải quyết việc gia đình", "về quê thăm bố mẹ", "đi du lịch cùng gia đình", "nghỉ ngơi, phục hồi "
         "sức khỏe sau đợt cao điểm công việc cuối năm", "đưa con đi khám bệnh định kỳ", "tổ chức đám cưới "
         "cho em trai", "sửa chữa nhà ở", "chăm sóc người thân đang điều trị tại bệnh viện",
         "giải quyết thủ tục hành chính cá nhân", "tham gia lễ giỗ tổ của dòng họ tại quê",
         "về quê ăn Tết Nguyên đán sớm", "chuyển nhà"]


def du_lieu_thong_bao(rng: random.Random, so_dong: int):
    da_dung: set[str] = set()
    dong = []
    for i in range(so_dong):
        p = Nguoi(rng, da_dung)
        phong, chuc = phong_va_chuc(rng, 0.25)
        ngay_don = date(2026, 12, 1) + timedelta(days=rng.randint(0, 25))
        while not la_ngay_lam(ngay_don):
            ngay_don += timedelta(days=1)
        ky = ngay_lam_tiep(ngay_don)
        tu = ky + timedelta(days=rng.randint(3, 30))
        so_ngay = rng.choice([1, 1, 2, 2, 3, 3, 4, 5, 5, 6, 7, 10])
        tu = cong_ngay_lam(tu, 1)
        den = cong_ngay_lam(tu, so_ngay)
        nguoi_nhan = Nguoi(rng, set(da_dung))
        dong.append([f"{450 + i}/TB-ABC", ngay_chu(ky), "Ông" if p.nam else "Bà", p.ho_ten, chuc, phong,
                     ngay_don, so_ngay, tu, den, ngay_lam_tiep(den), rng.choice(LY_DO),
                     rng.randint(0, 14 - min(so_ngay, 12)),
                     f"{'ông' if nguoi_nhan.nam else 'bà'} {nguoi_nhan.ho_ten}", sdt(rng)])
    return dong


# ====================================================================== 5. Quyết định nâng lương – 2 trang

NL_COT = ["Số quyết định", "Ngày ký", "Danh xưng", "Họ tên", "Ngày sinh", "Mã nhân viên", "Chức danh",
          "Phòng ban", "Số hợp đồng", "Ngạch lương", "Số bậc", "Bậc cũ", "Lương cũ", "Ngày hưởng cũ",
          "Bậc mới", "Lương mới", "Lương mới bằng chữ", "Ngày hưởng mới", "Thời gian giữ bậc", "Xếp loại",
          "Nhận xét", "Ngày họp hội đồng"]

NL_NOI_DUNG = [
    ("title", "QUYẾT ĐỊNH"),
    ("sub", "Về việc nâng bậc lương đối với người lao động"),
    ("sep", "———————"),
    ("cb", "GIÁM ĐỐC CÔNG TY CỔ PHẦN ABC"),
    ("cancu", "Căn cứ Bộ luật Lao động số 45/2019/QH14 ngày 20 tháng 11 năm 2019;"),
    ("cancu", "Căn cứ Nghị định số 145/2020/NĐ-CP ngày 14 tháng 12 năm 2020 của Chính phủ quy định chi tiết "
              "và hướng dẫn thi hành một số điều của Bộ luật Lao động về điều kiện lao động và quan hệ lao "
              "động;"),
    ("cancu", "Căn cứ Điều lệ tổ chức và hoạt động của Công ty Cổ phần ABC;"),
    ("cancu", "Căn cứ Thang lương, bảng lương của Công ty ban hành kèm theo Quyết định số 02/QĐ-ABC ngày "
              "02/01/2026;"),
    ("cancu", "Căn cứ Quy chế trả lương và nâng bậc lương của Công ty ban hành kèm theo Quyết định số "
              "03/QĐ-ABC ngày 02/01/2026;"),
    ("cancu", "Căn cứ Biên bản họp Hội đồng xét nâng bậc lương ngày {ngay_hop_hoi_dong};"),
    ("cancu", "Theo đề nghị của Trưởng phòng Hành chính - Nhân sự."),
    ("qd", "QUYẾT ĐỊNH:"),
    ("dieu", "**Điều 1.** Nâng bậc lương đối với {danh_xung} **{ho_ten}**, sinh ngày {ngay_sinh}, mã nhân "
             "viên {ma_nhan_vien}, chức danh {chuc_danh} – {phong_ban}, cụ thể như sau:"),
    ("bl", "- Ngạch, bậc lương đang hưởng: {ngach_luong}, bậc {bac_cu}/{so_bac}, mức lương {luong_cu} "
           "đồng/tháng, hưởng từ ngày {ngay_huong_cu}."),
    ("bl", "- Ngạch, bậc lương được nâng: {ngach_luong}, bậc {bac_moi}/{so_bac}, mức lương **{luong_moi} "
           "đồng/tháng** (Bằng chữ: {luong_moi_bang_chu})."),
    ("bl", "- Thời điểm hưởng bậc lương mới: kể từ ngày {ngay_huong_moi}."),
    ("bl", "- Thời điểm tính xét nâng bậc lương lần sau: tính từ ngày {ngay_huong_moi}, khi đủ thời gian giữ "
           "bậc và đáp ứng các điều kiện theo Quy chế trả lương và nâng bậc lương của Công ty."),
    ("dieu", "**Điều 2.** Căn cứ xét nâng bậc lương: {danh_xung} {ho_ten} đã giữ bậc lương đang hưởng đủ "
             "{thoi_gian_giu_bac} tháng; kết quả đánh giá mức độ hoàn thành nhiệm vụ: {xep_loai}; trong thời "
             "gian giữ bậc không bị xử lý kỷ luật lao động. Nhận xét của đơn vị: {nhan_xet}."),
    ("dieu", "**Điều 3.** Chế độ liên quan đến mức lương mới:"),
    ("bl", "- Tiền lương đóng bảo hiểm xã hội, bảo hiểm y tế, bảo hiểm thất nghiệp được điều chỉnh theo mức "
           "lương mới kể từ tháng có thời điểm hưởng bậc lương mới."),
    ("bl", "- Tiền lương làm thêm giờ, tiền lương ngày nghỉ hằng năm, ngày nghỉ lễ, Tết và các khoản trợ cấp "
           "tính trên tiền lương được tính theo mức lương mới kể từ thời điểm hưởng."),
    ("bl", "- Phần chênh lệch tiền lương của các tháng trước (nếu có) được Phòng Tài chính - Kế toán truy "
           "lĩnh vào kỳ trả lương gần nhất sau ngày ký Quyết định này."),
    ("bl", "- Thuế thu nhập cá nhân đối với phần thu nhập tăng thêm được khấu trừ theo quy định của pháp luật "
           "về thuế thu nhập cá nhân."),
    ("bl", "- Điều khoản tiền lương của Hợp đồng lao động số {so_hop_dong} được sửa đổi tương ứng bằng Phụ "
           "lục hợp đồng lao động ký kèm theo Quyết định này."),
    ("dieu", "**Điều 4.** Quyết định này có hiệu lực kể từ ngày ký. Trưởng phòng Hành chính - Nhân sự, Kế "
             "toán trưởng, Trưởng {phong_ban} và {danh_xung} {ho_ten} chịu trách nhiệm thi hành Quyết định "
             "này./."),
]
NL_CHU_KY = {"kieu": "noi_nhan", "noi_nhan": ["- Như Điều 4;", "- Ban Giám đốc;", "- Công đoàn Công ty;",
                                              "- Lưu: VT, HCNS, hồ sơ nhân sự."]}

NGACH = [("Chuyên viên", 8, 9_000_000, 1_200_000), ("Chuyên viên chính", 6, 16_000_000, 1_800_000),
         ("Kỹ sư", 8, 11_000_000, 1_400_000), ("Nhân viên", 10, 6_500_000, 700_000),
         ("Cán sự", 10, 7_500_000, 900_000), ("Kỹ thuật viên", 9, 7_000_000, 800_000)]
XEP_LOAI = ["Hoàn thành xuất sắc nhiệm vụ"] * 2 + ["Hoàn thành tốt nhiệm vụ"] * 5 + ["Hoàn thành nhiệm vụ"]
NHAN_XET = [
    "có tinh thần trách nhiệm cao, chủ động trong công việc",
    "nắm vững chuyên môn, hoàn thành công việc đúng tiến độ, phối hợp tốt với đồng nghiệp",
    "chấp hành tốt nội quy, quy chế của Công ty",
    "tích cực học tập nâng cao trình độ chuyên môn, có sáng kiến cải tiến quy trình làm việc được đơn vị "
    "áp dụng",
    "luôn hoàn thành khối lượng công việc lớn với chất lượng tốt, được khách hàng và đối tác đánh giá cao",
    "nhiệt tình, hỗ trợ đồng nghiệp mới, góp phần xây dựng tập thể đoàn kết",
]


def du_lieu_nang_luong(rng: random.Random, so_dong: int):
    da_dung: set[str] = set()
    dong = []
    ky = date(2026, 12, 21)
    for i in range(so_dong):
        p = Nguoi(rng, da_dung)
        phong, chuc = phong_va_chuc(rng, 0.3)
        ngach, so_bac, goc, buoc = rng.choice(NGACH)
        bac_cu = rng.randint(1, so_bac - 1)
        luong_cu = goc + buoc * (bac_cu - 1)
        luong_moi = luong_cu + buoc
        giu = rng.choice([24, 24, 36, 36, 36, 18])
        huong_moi = date(2027, 1, 1) if rng.random() < 0.7 else date(2026, 12, 1)
        huong_cu = cong_thang(huong_moi, -giu)
        nhan_xet = "; ".join(rng.sample(NHAN_XET, rng.randint(1, 3)))
        dong.append([f"{520 + i}/QĐ-ABC", ngay_chu(ky), "ông" if p.nam else "bà", p.ho_ten, p.ngay_sinh,
                     ma_nv(rng.randint(100, 2400)), chuc, phong,
                     f"HĐ-{rng.randint(2019, 2025)}-{rng.randint(1, 900):04d}", ngach, so_bac, bac_cu,
                     tien(luong_cu), huong_cu, bac_cu + 1, tien(luong_moi), tien_bang_chu(luong_moi), huong_moi,
                     giu, rng.choice(XEP_LOAI), nhan_xet, date(2026, 12, 16)])
    return dong


# ====================================================================== 6. Thỏa thuận bảo mật – 3 trang

BM_COT = ["Số thỏa thuận", "Ngày ký", "Số hợp đồng", "Họ tên", "Ngày sinh", "Số CCCD", "Ngày cấp",
          "Nơi cấp", "Địa chỉ", "Chức danh", "Phòng ban", "Điện thoại", "Email", "Phạm vi riêng",
          "Thời hạn bảo mật", "Mức phạt", "Mức phạt bằng chữ"]

BEN_A = [
    ("party", "BÊN A – CÔNG TY CỔ PHẦN ABC"),
    ("kv", "Đại diện: Ông Trần Minh Quang"),
    ("kv", "Chức vụ: Giám đốc"),
    ("kv", "Địa chỉ: Số 12 Trần Duy Hưng, phường Trung Hòa, quận Cầu Giấy, Hà Nội"),
    ("kv", "Mã số thuế: 0101234567"),
]

BM_NOI_DUNG = [
    ("title", "THỎA THUẬN BẢO MẬT THÔNG TIN"),
    ("subi", "(Kèm theo Hợp đồng lao động số {so_hop_dong})"),
    ("cancu", "Căn cứ Bộ luật Dân sự số 91/2015/QH13 ngày 24 tháng 11 năm 2015;"),
    ("cancu", "Căn cứ Bộ luật Lao động số 45/2019/QH14 ngày 20 tháng 11 năm 2019;"),
    ("cancu", "Căn cứ Luật Sở hữu trí tuệ và Nghị định số 13/2023/NĐ-CP về bảo vệ dữ liệu cá nhân;"),
    ("cancu", "Căn cứ Quy chế bảo mật thông tin và an toàn dữ liệu của Công ty Cổ phần ABC."),
    ("body", "Hôm nay, ngày {ngay_ky}, tại trụ sở CÔNG TY CỔ PHẦN ABC, chúng tôi gồm:"),
    *BEN_A,
    ("party", "BÊN B – NGƯỜI LAO ĐỘNG"),
    ("kv", "Họ và tên: **{ho_ten}**"),
    ("kv", "Ngày sinh: {ngay_sinh}"),
    ("kv", "Số CCCD: {so_cccd}, cấp ngày {ngay_cap}, tại {noi_cap}"),
    ("kv", "Địa chỉ thường trú: {dia_chi}"),
    ("kv", "Chức danh: {chuc_danh} – {phong_ban}"),
    ("kv", "Điện thoại: {dien_thoai}; thư điện tử: {email}"),
    ("body", "Hai bên thống nhất ký kết Thỏa thuận bảo mật thông tin (sau đây gọi là “Thỏa thuận”) với các "
             "điều khoản sau:"),
    ("head", "Điều 1. Giải thích từ ngữ"),
    ("bl", "- “Thông tin bảo mật” là mọi thông tin, tài liệu, dữ liệu ở bất kỳ hình thức nào (văn bản, điện "
           "tử, hình ảnh, lời nói) mà Bên B được biết, được giao hoặc tự tạo ra trong quá trình làm việc tại "
           "Bên A và chưa được Bên A công bố công khai."),
    ("bl", "- “Tiết lộ” là việc cung cấp, chuyển giao, sao chép, đăng tải hoặc để người khác tiếp cận thông "
           "tin bảo mật dưới bất kỳ hình thức nào, kể cả qua mạng xã hội và ứng dụng nhắn tin."),
    ("bl", "- “Bên thứ ba” là tổ chức, cá nhân không phải là Bên A, kể cả người lao động khác của Bên A không "
           "có thẩm quyền tiếp cận thông tin đó."),
    ("head", "Điều 2. Phạm vi thông tin bảo mật"),
    ("bl", "- Thông tin về chiến lược, kế hoạch kinh doanh, kế hoạch đầu tư, báo cáo tài chính chưa công bố, "
           "giá thành, chính sách giá và điều khoản hợp đồng với khách hàng, đối tác."),
    ("bl", "- Danh sách, thông tin liên hệ và lịch sử giao dịch của khách hàng, nhà cung cấp, đối tác."),
    ("bl", "- Mã nguồn, thiết kế hệ thống, mật khẩu, khóa truy cập, chứng thư số, quy trình nghiệp vụ và tài "
           "liệu kỹ thuật nội bộ."),
    ("bl", "- Dữ liệu cá nhân của người lao động, khách hàng mà Bên A đang quản lý, xử lý."),
    ("bl", "- Thông tin riêng gắn với vị trí công việc của Bên B: {pham_vi_rieng}."),
    ("bl", "- Các thông tin khác được Bên A đóng dấu, ghi chú “Mật”, “Nội bộ” hoặc thông báo là thông tin "
           "bảo mật."),
    ("head", "Điều 3. Nghĩa vụ của Bên B"),
    ("bl", "- Chỉ sử dụng thông tin bảo mật để thực hiện công việc được giao; không sử dụng cho mục đích cá "
           "nhân hoặc vì lợi ích của bên thứ ba."),
    ("bl", "- Không tiết lộ thông tin bảo mật cho bên thứ ba khi chưa có sự đồng ý bằng văn bản của người có "
           "thẩm quyền của Bên A."),
    ("bl", "- Không sao chép, chụp ảnh, lưu trữ thông tin bảo mật trên thiết bị, tài khoản cá nhân hoặc dịch "
           "vụ lưu trữ trực tuyến không do Bên A cấp."),
    ("bl", "- Bảo quản tài liệu, thiết bị, tài khoản được giao; không cho người khác mượn tài khoản, thiết bị "
           "ký số hoặc mã PIN của mình."),
    ("bl", "- Thông báo ngay cho người quản lý trực tiếp và Phòng Công nghệ thông tin khi phát hiện thông tin "
           "bảo mật bị lộ, bị mất hoặc có nguy cơ bị lộ, mất."),
    ("bl", "- Tuân thủ Quy chế bảo mật thông tin, quy định về an toàn thông tin mạng và bảo vệ dữ liệu cá "
           "nhân của Bên A trong từng thời kỳ."),
    ("head", "Điều 4. Trường hợp không bị coi là vi phạm"),
    ("bl", "- Thông tin đã được Bên A công bố công khai hoặc đã trở thành thông tin phổ biến không do lỗi của "
           "Bên B."),
    ("bl", "- Thông tin Bên B đã biết một cách hợp pháp trước khi làm việc tại Bên A và có tài liệu chứng "
           "minh."),
    ("bl", "- Thông tin phải cung cấp theo yêu cầu bằng văn bản của cơ quan nhà nước có thẩm quyền; trong "
           "trường hợp này Bên B phải thông báo cho Bên A trước khi cung cấp, nếu pháp luật không cấm."),
    ("head", "Điều 5. Thời hạn bảo mật"),
    ("bl", "- Nghĩa vụ bảo mật có hiệu lực từ ngày ký Thỏa thuận này, trong suốt thời gian Bên B làm việc tại "
           "Bên A và tiếp tục kéo dài **{thoi_han_bao_mat}** kể từ ngày chấm dứt hợp đồng lao động, vì bất kỳ "
           "lý do gì."),
    ("bl", "- Đối với mã nguồn, mật khẩu, khóa truy cập và dữ liệu cá nhân, nghĩa vụ bảo mật không giới hạn "
           "thời gian."),
    ("head", "Điều 6. Hoàn trả tài liệu, thiết bị"),
    ("bl", "- Khi chấm dứt hợp đồng lao động hoặc khi Bên A yêu cầu, Bên B phải bàn giao lại toàn bộ tài "
           "liệu, thiết bị, tài khoản và xóa các bản sao thông tin bảo mật (nếu có) trong vòng 03 ngày làm "
           "việc."),
    ("bl", "- Việc bàn giao được lập thành biên bản có xác nhận của người quản lý trực tiếp và Phòng Hành chính "
           "- Nhân sự."),
    ("head", "Điều 7. Vi phạm và bồi thường"),
    ("bl", "- Bên B vi phạm Thỏa thuận này bị xử lý kỷ luật lao động theo Nội quy lao động của Bên A; trường "
           "hợp nghiêm trọng có thể bị sa thải theo quy định của Bộ luật Lao động."),
    ("bl", "- Ngoài việc bị xử lý kỷ luật, Bên B phải nộp tiền phạt vi phạm là **{muc_phat} đồng** (Bằng chữ: "
           "{muc_phat_bang_chu}) cho mỗi lần vi phạm và bồi thường toàn bộ thiệt hại thực tế phát sinh."),
    ("bl", "- Trường hợp hành vi vi phạm có dấu hiệu tội phạm, Bên A có quyền đề nghị cơ quan có thẩm quyền xử "
           "lý theo quy định của pháp luật."),
    ("head", "Điều 8. Giải quyết tranh chấp"),
    ("bl", "- Tranh chấp phát sinh từ Thỏa thuận này trước hết được giải quyết thông qua thương lượng, hòa "
           "giải giữa hai bên."),
    ("bl", "- Trường hợp không thương lượng được, một trong hai bên có quyền yêu cầu Tòa án nhân dân có thẩm "
           "quyền nơi Bên A đặt trụ sở chính giải quyết."),
    ("head", "Điều 9. Điều khoản thi hành"),
    ("bl", "- Thỏa thuận này là phần không tách rời của Hợp đồng lao động số {so_hop_dong} và có hiệu lực kể "
           "từ ngày ký."),
    ("bl", "- Thỏa thuận được lập thành 02 bản có giá trị pháp lý như nhau, mỗi bên giữ 01 bản; được ký bằng "
           "chữ ký số có giá trị tương đương bản giấy."),
]
BM_CHU_KY = {"kieu": "hai_ben", "trai": "BÊN B", "phai": "BÊN A"}

PHAM_VI = {
    "Phòng Công nghệ thông tin": [
        "mã nguồn phần mềm, sơ đồ hạ tầng mạng, cấu hình máy chủ và nhật ký hệ thống",
        "tài khoản quản trị, khóa mã hóa, chứng thư số máy chủ và kết quả kiểm thử bảo mật",
    ],
    "Phòng Tài chính - Kế toán": [
        "số liệu kế toán, báo cáo tài chính, thông tin tài khoản ngân hàng và bảng lương",
        "hồ sơ thuế, chứng từ thanh toán và kế hoạch dòng tiền của Công ty",
    ],
    "Phòng Kinh doanh": [
        "danh sách khách hàng, báo giá, chính sách chiết khấu và hợp đồng đang đàm phán",
        "kế hoạch doanh số, dữ liệu thị trường và thông tin đối thủ cạnh tranh do Công ty thu thập",
    ],
    "Phòng Hành chính - Nhân sự": [
        "hồ sơ nhân sự, bảng lương, kết quả đánh giá và thông tin kỷ luật của người lao động",
    ],
    "Phòng Pháp chế": ["hồ sơ tranh chấp, ý kiến pháp lý và dự thảo hợp đồng của Công ty"],
    "Phòng Marketing": ["kế hoạch truyền thông, ngân sách quảng cáo và dữ liệu khách hàng tiềm năng"],
    "Phòng Kế hoạch - Đầu tư": ["hồ sơ dự án đầu tư, phương án tài chính và kết quả thẩm định dự án"],
    "Phòng Chăm sóc khách hàng": ["thông tin cá nhân, lịch sử khiếu nại và ghi âm cuộc gọi của khách hàng"],
    "Văn phòng Công ty": ["văn bản đến, văn bản đi chưa ban hành, lịch công tác và biên bản họp Ban Giám đốc"],
}
PHAM_VI_CHI_NHANH = ["danh sách khách hàng, công nợ và giá bán áp dụng tại chi nhánh",
                     "số liệu tồn kho, kế hoạch nhập hàng và hợp đồng với nhà cung cấp tại địa phương"]


def du_lieu_bao_mat(rng: random.Random, so_dong: int):
    da_dung: set[str] = set()
    mail: set[str] = set()
    dong = []
    for i in range(so_dong):
        p = Nguoi(rng, da_dung)
        phong, chuc = phong_va_chuc(rng, 0.25)
        tp = thanh_pho_cua(phong)
        muc = rng.choice([20, 30, 50, 50, 100]) * 1_000_000
        ky = date(2026, 12, 1) + timedelta(days=rng.randint(0, 20))
        while not la_ngay_lam(ky):
            ky += timedelta(days=1)
        dong.append([f"{i + 1:03d}/2026/TTBM-ABC", ngay_chu(ky),
                     f"HĐ-{rng.randint(2020, 2026)}-{rng.randint(1, 900):04d}", p.ho_ten, p.ngay_sinh,
                     cccd(rng, p, tp), date(rng.randint(2021, 2025), rng.randint(1, 12), rng.randint(1, 28)),
                     "Cục Cảnh sát QLHC về TTXH", dia_chi(rng, tp), chuc, phong, sdt(rng), email(p, mail),
                     rng.choice(PHAM_VI.get(phong, PHAM_VI_CHI_NHANH)),
                     rng.choice(["12 tháng", "24 tháng", "24 tháng", "36 tháng"]), tien(muc), tien_bang_chu(muc)])
    return dong


# ====================================================================== 7. Hợp đồng đào tạo – 4 trang

DT_COT = ["Số hợp đồng", "Ngày ký", "Họ tên", "Ngày sinh", "Giới tính", "Số CCCD", "Ngày cấp", "Nơi cấp",
          "Địa chỉ", "Điện thoại", "Chức danh", "Phòng ban", "Tên khóa học", "Cơ sở đào tạo", "Hình thức",
          "Địa điểm đào tạo", "Từ ngày", "Đến ngày", "Số giờ", "Chứng chỉ", "Chi phí", "Chi phí bằng chữ",
          "Thời gian cam kết", "Ngày bắt đầu cam kết"]

DT_NOI_DUNG = [
    ("title", "HỢP ĐỒNG ĐÀO TẠO"),
    ("subi", "(Đào tạo nâng cao trình độ, kỹ năng nghề cho người lao động)"),
    ("cancu", "Căn cứ Bộ luật Dân sự số 91/2015/QH13 ngày 24 tháng 11 năm 2015;"),
    ("cancu", "Căn cứ Bộ luật Lao động số 45/2019/QH14 ngày 20 tháng 11 năm 2019, các Điều 60, 61 và 62 về "
              "đào tạo, bồi dưỡng, nâng cao trình độ, kỹ năng nghề;"),
    ("cancu", "Căn cứ Quy chế đào tạo của Công ty Cổ phần ABC ban hành kèm theo Quyết định số 05/QĐ-ABC ngày "
              "05/01/2026;"),
    ("cancu", "Căn cứ Kế hoạch đào tạo năm 2026 đã được Giám đốc Công ty phê duyệt."),
    ("body", "Hôm nay, ngày {ngay_ky}, tại trụ sở CÔNG TY CỔ PHẦN ABC, chúng tôi gồm:"),
    *BEN_A,
    ("party", "BÊN B – NGƯỜI LAO ĐỘNG ĐƯỢC CỬ ĐI ĐÀO TẠO"),
    ("kv", "Họ và tên: **{ho_ten}**"),
    ("kv", "Ngày sinh: {ngay_sinh}; giới tính: {gioi_tinh}"),
    ("kv", "Số CCCD: {so_cccd}, cấp ngày {ngay_cap}, tại {noi_cap}"),
    ("kv", "Địa chỉ thường trú: {dia_chi}"),
    ("kv", "Điện thoại: {dien_thoai}"),
    ("kv", "Chức danh: {chuc_danh} – {phong_ban}"),
    ("body", "Hai bên thỏa thuận ký kết Hợp đồng đào tạo với các điều khoản sau:"),
    ("head", "Điều 1. Nội dung đào tạo"),
    ("bl", "- Tên khóa đào tạo: **{ten_khoa_hoc}**."),
    ("bl", "- Cơ sở đào tạo: {co_so_dao_tao}."),
    ("bl", "- Hình thức đào tạo: {hinh_thuc}; địa điểm: {dia_diem_dao_tao}."),
    ("bl", "- Thời gian đào tạo: từ ngày {tu_ngay} đến ngày {den_ngay}, tổng thời lượng {so_gio} giờ."),
    ("bl", "- Văn bằng, chứng chỉ được cấp sau khóa học: {chung_chi}."),
    ("head", "Điều 2. Chi phí đào tạo"),
    ("bl", "- Tổng chi phí đào tạo do Bên A chi trả: **{chi_phi} đồng** (Bằng chữ: {chi_phi_bang_chu})."),
    ("bl", "- Chi phí đào tạo gồm: học phí, lệ phí thi và cấp chứng chỉ, tài liệu học tập, chi phí đi lại, "
           "ăn ở trong thời gian học (nếu có) và các khoản chi khác có chứng từ hợp lệ theo Quy chế đào tạo "
           "của Bên A."),
    ("bl", "- Tiền lương trả cho Bên B trong thời gian đi học không tính vào chi phí đào tạo phải bồi hoàn, "
           "trừ trường hợp hai bên có thỏa thuận khác bằng văn bản."),
    ("bl", "- Bên A thanh toán trực tiếp cho cơ sở đào tạo hoặc tạm ứng cho Bên B theo đề nghị có kèm chứng "
           "từ; Bên B quyết toán tạm ứng trong vòng 10 ngày làm việc kể từ ngày kết thúc khóa học."),
    ("head", "Điều 3. Quyền và nghĩa vụ của Bên A"),
    ("bl", "- Bố trí thời gian để Bên B tham gia đầy đủ khóa đào tạo; giữ nguyên vị trí công việc của Bên B "
           "trong thời gian đi học."),
    ("bl", "- Chi trả đầy đủ, đúng hạn chi phí đào tạo theo Điều 2 của Hợp đồng này."),
    ("bl", "- Trả lương cho Bên B trong thời gian đi học theo mức lương ghi trong hợp đồng lao động; bảo đảm "
           "các chế độ bảo hiểm xã hội, bảo hiểm y tế, bảo hiểm thất nghiệp theo quy định."),
    ("bl", "- Theo dõi, đánh giá kết quả học tập của Bên B; yêu cầu cơ sở đào tạo cung cấp thông tin về tình "
           "hình học tập khi cần thiết."),
    ("bl", "- Bố trí công việc phù hợp với chuyên môn được đào tạo cho Bên B sau khi kết thúc khóa học."),
    ("bl", "- Yêu cầu Bên B bồi hoàn chi phí đào tạo trong các trường hợp quy định tại Điều 6 của Hợp đồng "
           "này."),
    ("head", "Điều 4. Quyền và nghĩa vụ của Bên B"),
    ("bl", "- Tham gia đầy đủ chương trình đào tạo, chấp hành nội quy, quy chế của cơ sở đào tạo; số buổi "
           "vắng mặt không vượt quá mức cho phép của khóa học."),
    ("bl", "- Hoàn thành khóa học và đạt văn bằng, chứng chỉ theo Điều 1; nộp bản sao văn bằng, chứng chỉ cho "
           "Phòng Hành chính - Nhân sự trong vòng 15 ngày kể từ ngày được cấp."),
    ("bl", "- Báo cáo bằng văn bản kết quả học tập và đề xuất áp dụng kiến thức đã học vào công việc trong "
           "vòng 10 ngày làm việc sau khi kết thúc khóa học."),
    ("bl", "- Chia sẻ, phổ biến lại kiến thức đã học cho người lao động khác của Bên A khi được yêu cầu."),
    ("bl", "- Bảo quản tài liệu, giáo trình do Bên A cấp; không sử dụng tài liệu, kiến thức của khóa học để "
           "làm việc cho đơn vị cạnh tranh với Bên A trong thời gian cam kết."),
    ("bl", "- Được hưởng lương và các chế độ theo hợp đồng lao động trong thời gian đi học; thời gian đi học "
           "được tính là thời gian làm việc."),
    ("bl", "- Được Bên A xem xét bố trí công việc, nâng bậc lương phù hợp với trình độ sau đào tạo theo Quy chế "
           "trả lương của Công ty."),
    ("bl", "- Thực hiện cam kết làm việc và nghĩa vụ bồi hoàn theo Điều 5 và Điều 6 của Hợp đồng này."),
    ("head", "Điều 5. Cam kết làm việc sau đào tạo"),
    ("bl", "- Sau khi kết thúc khóa học, Bên B cam kết làm việc cho Bên A tối thiểu **{thoi_gian_cam_ket} "
           "tháng**, tính từ ngày {ngay_bat_dau_cam_ket}."),
    ("bl", "- Thời gian Bên B nghỉ không hưởng lương, tạm hoãn thực hiện hợp đồng lao động không được tính vào "
           "thời gian cam kết làm việc."),
    ("bl", "- Trường hợp hợp đồng lao động hết hạn trước khi kết thúc thời gian cam kết, Bên A đề nghị giao kết "
           "hợp đồng mới thì Bên B có nghĩa vụ tiếp tục giao kết để hoàn thành thời gian cam kết."),
    ("head", "Điều 6. Bồi hoàn chi phí đào tạo"),
    ("bl", "- Bên B phải bồi hoàn **toàn bộ** chi phí đào tạo nếu tự ý bỏ học, bị cơ sở đào tạo buộc thôi học "
           "hoặc không đạt văn bằng, chứng chỉ vì lý do chủ quan."),
    ("bl", "- Bên B phải bồi hoàn chi phí đào tạo **tương ứng với thời gian cam kết còn lại** nếu đơn phương "
           "chấm dứt hợp đồng lao động trái pháp luật, bị sa thải hoặc tự ý nghỉ việc trước khi hết thời gian "
           "cam kết. Số tiền bồi hoàn = Tổng chi phí đào tạo × (Số tháng cam kết còn lại ÷ "
           "{thoi_gian_cam_ket})."),
    ("bl", "- Việc bồi hoàn được thực hiện một lần, trong vòng 30 ngày kể từ ngày chấm dứt hợp đồng lao động "
           "hoặc ngày có kết luận về việc không hoàn thành khóa học; hai bên có thể thỏa thuận trả nhiều lần "
           "bằng văn bản."),
    ("bl", "- Bên A không được khấu trừ chi phí bồi hoàn vào tiền lương của Bên B khi chưa có sự đồng ý bằng "
           "văn bản của Bên B."),
    ("head", "Điều 7. Trường hợp không phải bồi hoàn"),
    ("bl", "- Bên B không hoàn thành khóa học hoặc phải nghỉ việc do ốm đau, tai nạn có xác nhận của cơ sở "
           "khám bệnh, chữa bệnh có thẩm quyền."),
    ("bl", "- Bên A đơn phương chấm dứt hợp đồng lao động vì lý do không thuộc lỗi của Bên B, hoặc Bên A thay "
           "đổi cơ cấu, công nghệ dẫn đến không còn vị trí công việc phù hợp."),
    ("bl", "- Bên B đơn phương chấm dứt hợp đồng lao động trong các trường hợp không cần báo trước theo khoản 2 "
           "Điều 35 Bộ luật Lao động."),
    ("bl", "- Cơ sở đào tạo hủy, hoãn khóa học hoặc các trường hợp bất khả kháng khác theo quy định của pháp "
           "luật."),
    ("head", "Điều 8. Sửa đổi, chấm dứt hợp đồng"),
    ("bl", "- Mọi sửa đổi, bổ sung Hợp đồng này phải được hai bên thỏa thuận bằng văn bản."),
    ("bl", "- Hợp đồng chấm dứt khi Bên B hoàn thành thời gian cam kết làm việc, hoặc khi Bên B hoàn thành "
           "nghĩa vụ bồi hoàn theo Điều 6, hoặc khi hai bên thỏa thuận chấm dứt."),
    ("bl", "- Khóa học bị thay đổi về thời gian, địa điểm nhưng không thay đổi nội dung chính thì hai bên tiếp "
           "tục thực hiện Hợp đồng; Bên A thông báo cho Bên B bằng văn bản."),
    ("head", "Điều 9. Giải quyết tranh chấp"),
    ("bl", "- Tranh chấp phát sinh từ Hợp đồng này trước hết được giải quyết thông qua thương lượng, hòa giải "
           "tại Công đoàn cơ sở hoặc hòa giải viên lao động."),
    ("bl", "- Trường hợp hòa giải không thành, một trong hai bên có quyền yêu cầu Hội đồng trọng tài lao động "
           "hoặc Tòa án nhân dân có thẩm quyền giải quyết theo quy định của pháp luật."),
    ("head", "Điều 10. Điều khoản thi hành"),
    ("bl", "- Hợp đồng này có hiệu lực kể từ ngày ký. Những nội dung không ghi trong Hợp đồng được thực hiện "
           "theo Quy chế đào tạo của Bên A và quy định của pháp luật."),
    ("bl", "- Hợp đồng được lập thành 02 bản có giá trị pháp lý như nhau, mỗi bên giữ 01 bản; được ký bằng chữ "
           "ký số có giá trị tương đương bản giấy."),
]
DT_CHU_KY = {"kieu": "hai_ben", "trai": "BÊN B", "phai": "BÊN A"}

KHOA_HOC = [
    ("Quản trị dự án chuyên nghiệp (PMP)", "Viện Quản trị dự án Việt Nam", "Chứng chỉ PMP", 120, 35_000_000),
    ("Kế toán trưởng doanh nghiệp", "Trường Đại học Kinh tế Quốc dân", "Chứng chỉ Kế toán trưởng", 240,
     12_000_000),
    ("An toàn thông tin mạng nâng cao", "Học viện Kỹ thuật Mật mã", "Chứng chỉ An toàn thông tin", 160,
     28_000_000),
    ("Điện toán đám mây AWS Solutions Architect", "Trung tâm Đào tạo CNTT Sao Mai",
     "Chứng chỉ AWS Solutions Architect – Associate", 80, 22_000_000),
    ("Kỹ năng quản lý cấp trung", "Trường Doanh nhân PACE", "Giấy chứng nhận hoàn thành khóa học", 64,
     18_500_000),
    ("Tiếng Anh thương mại (IELTS 6.5)", "Hội đồng Anh tại Việt Nam", "Chứng chỉ IELTS", 180, 26_000_000),
    ("Pháp luật hợp đồng thương mại", "Học viện Tư pháp", "Giấy chứng nhận hoàn thành khóa học", 48,
     9_500_000),
    ("Phân tích dữ liệu kinh doanh với Power BI", "Trung tâm Đào tạo CNTT Sao Mai",
     "Chứng chỉ Microsoft PL-300", 60, 15_000_000),
    ("Quản trị nhân sự chiến lược", "Trường Đại học Lao động - Xã hội", "Chứng chỉ Quản trị nhân sự", 120,
     14_000_000),
    ("Kiểm thử phần mềm ISTQB", "Hội Kiểm thử phần mềm Việt Nam", "Chứng chỉ ISTQB Foundation", 72,
     11_000_000),
    ("Marketing số thực chiến", "Học viện Báo chí và Tuyên truyền", "Giấy chứng nhận hoàn thành khóa học", 90,
     13_500_000),
]
HINH_THUC = [("Tập trung, học ban ngày", None), ("Học ngoài giờ hành chính", None),
             ("Trực tuyến kết hợp tập trung", "trực tuyến và tại cơ sở đào tạo")]


def du_lieu_dao_tao(rng: random.Random, so_dong: int):
    da_dung: set[str] = set()
    dong = []
    for i in range(so_dong):
        p = Nguoi(rng, da_dung)
        phong, chuc = phong_va_chuc(rng, 0.2)
        tp = thanh_pho_cua(phong)
        ten, co_so, chung_chi, gio, phi = rng.choice(KHOA_HOC)
        phi = round(phi * rng.uniform(0.9, 1.15) / 500_000) * 500_000
        hinh_thuc, noi = rng.choice(HINH_THUC)
        noi = noi or f"tại cơ sở của {co_so}, {rng.choice(['Hà Nội', 'Hà Nội', 'TP. Hồ Chí Minh', 'Đà Nẵng'])}"
        tu = date(2027, 1, 4) + timedelta(weeks=rng.randint(0, 10))
        den = tu + timedelta(weeks=max(2, gio // 16)) - timedelta(days=3)
        cam_ket = 12 if phi < 15_000_000 else (24 if phi < 25_000_000 else 36)
        ky = date(2026, 12, 14) + timedelta(days=rng.randint(0, 10))
        while not la_ngay_lam(ky):
            ky += timedelta(days=1)
        dong.append([f"HĐĐT-2026-{i + 1:04d}", ngay_chu(ky), p.ho_ten, p.ngay_sinh, "Nam" if p.nam else "Nữ",
                     cccd(rng, p, tp), date(rng.randint(2021, 2025), rng.randint(1, 12), rng.randint(1, 28)),
                     "Cục Cảnh sát QLHC về TTXH", dia_chi(rng, tp), sdt(rng), chuc, phong, ten, co_so,
                     hinh_thuc, noi, tu, den, gio, chung_chi, tien(phi), tien_bang_chu(phi), cam_ket,
                     ngay_lam_tiep(den)])
    return dong


# ====================================================================== ghi Excel


def ghi_excel(out: Path, cot: list[str], dong: list[list]):
    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "DuLieu"
    vien = Border(*(Side(style="thin", color="FFBFBFBF"),) * 4)
    for j, ten in enumerate(cot, start=1):
        c = ws.cell(row=1, column=j, value=ten)
        c.font = Font(name="Calibri", size=11, bold=True, color="FFFFFFFF")
        c.fill = PatternFill("solid", fgColor="FF1F4E78")
        c.alignment = Alignment(horizontal="center", vertical="center", wrap_text=True)
        c.border = vien
    for i, gia_tri in enumerate(dong, start=2):
        for j, v in enumerate(gia_tri, start=1):
            c = ws.cell(row=i, column=j, value=v)
            c.font = Font(name="Calibri", size=11)
            c.border = vien
            c.alignment = Alignment(vertical="top")
            if isinstance(v, date):
                c.number_format = "dd/mm/yyyy"
    for j, ten in enumerate(cot, start=1):
        dai = max([len(ten)] + [len(str(r[j - 1])) if not isinstance(r[j - 1], date) else 10 for r in dong])
        ws.column_dimensions[get_column_letter(j)].width = max(9, min(45, dai + 2))
    ws.freeze_panes = "A2"
    wb.save(out)


# ====================================================================== chạy

VIEC = [
    ("4_thong_bao_nghi_phep", "2_quyet_dinh_khen_thuong.docx", "so_thong_bao", TB_NOI_DUNG, TB_CHU_KY, None,
     TB_COT, du_lieu_thong_bao),
    ("5_quyet_dinh_nang_luong", "2_quyet_dinh_khen_thuong.docx", "so_quyet_dinh", NL_NOI_DUNG, NL_CHU_KY, None,
     NL_COT, du_lieu_nang_luong),
    ("6_thoa_thuan_bao_mat", "3_hop_dong_lao_dong.docx", "so_thoa_thuan", BM_NOI_DUNG, BM_CHU_KY,
     "Thỏa thuận bảo mật số {so_thoa_thuan} – {ho_ten}", BM_COT, du_lieu_bao_mat),
    ("7_hop_dong_dao_tao", "3_hop_dong_lao_dong.docx", "so_hop_dong", DT_NOI_DUNG, DT_CHU_KY,
     "Hợp đồng đào tạo số {so_hop_dong} – {ho_ten}", DT_COT, du_lieu_dao_tao),
]


def bien_trong(noi_dung: list[tuple[str, str]]) -> set[str]:
    return {m for _, t in noi_dung for m in re.findall(r"\{([a-z_]+)\}", t)}


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--src", type=Path, default=Path("mau-tao-van-ban"))
    ap.add_argument("--out", type=Path, default=Path("mau-tao-van-ban/mau-nhieu-trang"))
    ap.add_argument("--rows", type=int, default=100, help="số dòng dữ liệu mỗi file Excel")
    a = ap.parse_args()

    a.out.mkdir(parents=True, exist_ok=True)
    khung = Khung(a.src)
    rng = random.Random(SEED)
    for ten, goc, so_bien, noi_dung, chu_ky, chan_trang, cot, sinh in VIEC:
        # Mọi biến trong mẫu phải có cột cùng tên (bỏ dấu) trong Excel.
        khoa_cot = {khong_dau(c).lower().replace(" ", "_") for c in cot}
        thieu = (bien_trong(noi_dung) | {so_bien, "ngay_ky"}) - khoa_cot
        if thieu:
            raise SystemExit(f"{ten}: thiếu cột cho biến {sorted(thieu)}")
        dong = sinh(rng, a.rows)
        assert all(len(r) == len(cot) for r in dong), ten
        khung.tao(goc, so_bien, noi_dung, chu_ky, chan_trang, a.out / f"{ten}.docx")
        ghi_excel(a.out / f"{ten}.xlsx", cot, dong)
        print(f"{ten}: .docx + .xlsx ({len(dong)} dòng, {len(cot)} cột)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
