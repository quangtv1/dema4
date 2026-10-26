import fitz  # PyMuPDF library
import os
import concurrent.futures
import datetime
import threading

class DemA4:
    def __init__(self, duong_dan_thu_muc, dung_event):
        self.duong_dan_thu_muc = duong_dan_thu_muc
        self.dung_event = dung_event

    def xac_dinh_kho_a(self, canh_dai_hon):
        a_sizes = [
            (3370.39, "A0"),
            (2383.94, "A1"),
            (1683.78, "A2"),
            (1190.55, "A3"),
            (841.89, "A4"),
            (595.28, "A5"),
            (419.53, "A6"),
            (298.74, "A7"),
        ]

        if canh_dai_hon > a_sizes[0][0]:
            return "A0"
        if canh_dai_hon < a_sizes[-1][0]:
            return "A7"

        for i in range(len(a_sizes) - 1):
            if a_sizes[i + 1][0] <= canh_dai_hon <= a_sizes[i][0]:
                mid_point = (a_sizes[i][0] + a_sizes[i + 1][0]) / 2
                if canh_dai_hon >= mid_point:
                    return a_sizes[i][1]
                else:
                    return a_sizes[i + 1][1]
        return "Unknown"

    def phan_tich_trang_pdf(self, duong_dan_pdf, stt, tong_so_tap_tin):
        ten_pdf = os.path.splitext(os.path.basename(duong_dan_pdf))[0]
        tap_tin_pdf = fitz.open(duong_dan_pdf)
        ket_qua = []
        dem_kich_co = {"A0": 0, "A1": 0, "A2": 0, "A3": 0, "A4": 0, "A5": 0, "A6": 0, "A7": 0}
        cac_trang_a3 = []
        tong_so_trang = len(tap_tin_pdf)

        for chi_so_trang in range(tong_so_trang):
            if self.dung_event.is_set():
                break
            trang = tap_tin_pdf[chi_so_trang]
            hinh_chu_nhat_trang = trang.rect
            chieu_rong = round(hinh_chu_nhat_trang.width, 3)
            chieu_cao = round(hinh_chu_nhat_trang.height, 3)

            if chieu_rong > chieu_cao:
                huong_trang = "Ngang"
                canh_dai_hon = chieu_rong
            else:
                huong_trang = "Doc"
                canh_dai_hon = chieu_cao

            chenh_lech = round(abs(chieu_rong - chieu_cao), 3)
            kich_co_a = self.xac_dinh_kho_a(canh_dai_hon)
            dem_kich_co[kich_co_a] += 1

            if kich_co_a == "A3":
                cac_trang_a3.append(str(chi_so_trang + 1))

            thong_tin_trang = [f"{stt}/{tong_so_tap_tin}", f"{ten_pdf}_page_{chi_so_trang + 1}", f"{chieu_rong}", f"{chieu_cao}", f"{huong_trang}", f"{kich_co_a}"]
            ket_qua.append(thong_tin_trang)

        tap_tin_pdf.close()

        quy_a4 = (dem_kich_co["A0"] * 16 + dem_kich_co["A1"] * 8 + dem_kich_co["A2"] * 4 + dem_kich_co["A3"] * 2 +
                  dem_kich_co["A4"] + dem_kich_co["A5"] * 0.5 + dem_kich_co["A6"] * 0.25 + dem_kich_co["A7"] * 0.125)

        thong_tin_tong_hop = [f"{stt}/{tong_so_tap_tin}", f"{os.path.abspath(duong_dan_pdf)}", f"{tong_so_trang}", f"{dem_kich_co['A0']}", f"{dem_kich_co['A1']}",
                              f"{dem_kich_co['A2']}", f"{dem_kich_co['A3']}", f"{dem_kich_co['A4']}", f"{dem_kich_co['A5']}", f"{dem_kich_co['A6']}", f"{dem_kich_co['A7']}", "'=>", f"{quy_a4}"]
        return ket_qua, thong_tin_tong_hop, dem_kich_co

    def phan_tich_cac_tap_tin_pdf_trong_thu_muc(self):
        thoi_gian_bat_dau = datetime.datetime.now()  # Lưu thời gian bắt đầu

        thu_muc_hien_tai = os.path.basename(self.duong_dan_thu_muc)
        duong_dan_chi_tiet = os.path.join(self.duong_dan_thu_muc, f'{thu_muc_hien_tai}_dema4_chitiet.txt')
        duong_dan_tong_hop = os.path.join(self.duong_dan_thu_muc, f'{thu_muc_hien_tai}_dema4_tonghop.txt')

        cac_tap_tin_pdf = []
        for root, _, files in os.walk(self.duong_dan_thu_muc):
            for file in files:
                if file.endswith('.pdf'):
                    cac_tap_tin_pdf.append(os.path.join(root, file))

        cac_tap_tin_pdf = sorted(cac_tap_tin_pdf)
        tong_so_tap_tin = len(cac_tap_tin_pdf)

        tieu_de_chi_tiet = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tKho giay"
        tieu_de_tong_hop = "STT\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4"

        # In tiêu đề header của file chi tiết
        print(tieu_de_chi_tiet)
        with open(duong_dan_chi_tiet, 'w', encoding='utf-8') as f_chi_tiet:
            f_chi_tiet.write(tieu_de_chi_tiet + '\n')

        with open(duong_dan_tong_hop, 'w', encoding='utf-8') as f_tong_hop:
            f_tong_hop.write(tieu_de_tong_hop + '\n')

        tong_so_trang_a0 = 0
        tong_so_trang_a1 = 0
        tong_so_trang_a2 = 0
        tong_so_trang_a3 = 0
        tong_so_trang_a4 = 0
        tong_so_trang_a5 = 0
        tong_so_trang_a6 = 0
        tong_so_trang_a7 = 0
        tong_so_quy_a4 = 0

        completed_files = 0

        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_to_pdf = {executor.submit(self.phan_tich_trang_pdf, tap_tin_pdf, idx + 1, tong_so_tap_tin): tap_tin_pdf for idx, tap_tin_pdf in enumerate(cac_tap_tin_pdf)}

            for future in concurrent.futures.as_completed(future_to_pdf):
                if self.dung_event.is_set():
                    break
                tap_tin_pdf = future_to_pdf[future]
                try:
                    ket_qua, thong_tin_tong_hop, dem_kich_co = future.result()

                    with open(duong_dan_chi_tiet, 'a', encoding='utf-8') as f_chi_tiet:
                        for trang in ket_qua:
                            f_chi_tiet.write('\t'.join(trang) + '\n')

                    with open(duong_dan_tong_hop, 'a', encoding='utf-8') as f_tong_hop:
                        f_tong_hop.write('\t'.join(thong_tin_tong_hop) + '\n')

                    tong_so_trang_a0 += dem_kich_co["A0"]
                    tong_so_trang_a1 += dem_kich_co["A1"]
                    tong_so_trang_a2 += dem_kich_co["A2"]
                    tong_so_trang_a3 += dem_kich_co["A3"]
                    tong_so_trang_a4 += dem_kich_co["A4"]
                    tong_so_trang_a5 += dem_kich_co["A5"]
                    tong_so_trang_a6 += dem_kich_co["A6"]
                    tong_so_trang_a7 += dem_kich_co["A7"]
                    tong_so_quy_a4 += (dem_kich_co["A0"] * 16 + dem_kich_co["A1"] * 8 + dem_kich_co["A2"] * 4 + dem_kich_co["A3"] * 2 +
                                       dem_kich_co["A4"] + dem_kich_co["A5"] * 0.5 + dem_kich_co["A6"] * 0.25 + dem_kich_co["A7"] * 0.125)

                    completed_files += 1

                except Exception as exc:
                    print(f'{tap_tin_pdf} generated an exception: {exc}')

        # Hiển thị thông tin tổng hợp trên giao diện
        thoi_gian_ket_thuc = datetime.datetime.now()  # Lưu thời gian kết thúc
        thoi_gian_thuc_thi = thoi_gian_ket_thuc - thoi_gian_bat_dau # Tính toán thời gian thực thi
        total_seconds = round(thoi_gian_thuc_thi.total_seconds(), 3)  # Tính tổng số giây và làm tròn đến 3 chữ số thập phân
        hours, remainder = divmod(total_seconds, 3600)
        minutes, seconds = divmod(remainder, 60)
        thoi_gian_thuc_thi_str = f"{int(hours)}:{int(minutes)}:{seconds:.3f}"  # Định dạng thành chuỗi giờ, phút, giây

        duong_dan_tuyet_doi = os.path.abspath(self.duong_dan_thu_muc)
        tong_so_trang = tong_so_trang_a0 + tong_so_trang_a1 + tong_so_trang_a2 + tong_so_trang_a3 + tong_so_trang_a4 + tong_so_trang_a5 + tong_so_trang_a6 + tong_so_trang_a7
        so_trang_tang_them = tong_so_quy_a4 - tong_so_trang
        tong_hop = (
            f"\nKẾT QUẢ ĐẾM\n"
            f"===========================\n"
            f"Tổng số file pdf trong thư mục {thu_muc_hien_tai}: {tong_so_tap_tin} \n"
            f"Tổng số trang {tong_so_trang} các loại => quy ra A4: {tong_so_quy_a4} (+{so_trang_tang_them})\n"
            f"Thời gian đếm: {thoi_gian_thuc_thi_str}\n"
            f"------------------------------------------------------\n"
            f"Trong đó, bao gồm:\n"
            f" - Tổng số trang A0: {tong_so_trang_a0}\n"
            f" - Tổng số trang A1: {tong_so_trang_a1}\n"
            f" - Tổng số trang A2: {tong_so_trang_a2}\n"
            f" - Tổng số trang A3: {tong_so_trang_a3}\n"
            f" - Tổng số trang A4: {tong_so_trang_a4}\n"
            f" - Tổng số trang A5: {tong_so_trang_a5}\n"
            f" - Tổng số trang A6: {tong_so_trang_a6}\n"
            f" - Tổng số trang A7: {tong_so_trang_a7}\n"
            f"------------------------------------------------------\n"
            f"Chi tiết đếm trang ghi tại file:\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dema4_tonghop.txt\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dema4_chitiet.txt\n"
            f"===========================\n\n"
            f"==> Đã đếm xong thư mục {duong_dan_tuyet_doi} !!!\n"
        )
        print(tong_hop)

def main():
    dung_event = threading.Event()
    duong_dan_thu_muc = os.getcwd()  # Đặt thư mục hiện tại
    dem_a4 = DemA4(duong_dan_thu_muc, dung_event)
    dem_a4.phan_tich_cac_tap_tin_pdf_trong_thu_muc()

if __name__ == '__main__':
    main()
