import fitz  # PyMuPDF library
import os
import concurrent.futures
import datetime
from PyQt5 import QtCore, QtGui, QtWidgets
from PyQt5.QtWidgets import QFileDialog, QVBoxLayout, QHBoxLayout, QPushButton, QTextEdit, QLineEdit, QLabel, QWidget, QApplication, QDesktopWidget

class DemA4(QtCore.QObject):
    ket_qua_tin_hieu = QtCore.pyqtSignal(str)
    tong_hop_tin_hieu = QtCore.pyqtSignal(str)

    def __init__(self, duong_dan_thu_muc):
        super().__init__()
        self.duong_dan_thu_muc = duong_dan_thu_muc

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
        thoi_gian_bat_dau = datetime.datetime.now().strftime('%d-%m-%Y %H:%M:%S')
        ten_pdf = os.path.splitext(os.path.basename(duong_dan_pdf))[0]
        tap_tin_pdf = fitz.open(duong_dan_pdf)
        ket_qua = []
        dem_kich_co = {"A0": 0, "A1": 0, "A2": 0, "A3": 0, "A4": 0, "A5": 0, "A6": 0, "A7": 0}
        cac_trang_a3 = []
        tong_so_trang = len(tap_tin_pdf)

        for chi_so_trang in range(tong_so_trang):
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

            thong_tin_trang = [f"{stt}/{tong_so_tap_tin}", f"{ten_pdf}_page_{chi_so_trang + 1}", f"{chieu_rong}", f"{chieu_cao}", f"{huong_trang}", f"{chenh_lech}", f"{kich_co_a}"]
            ket_qua.append(thong_tin_trang)

        tap_tin_pdf.close()

        quy_a4 = (dem_kich_co["A0"] * 16 + dem_kich_co["A1"] * 8 + dem_kich_co["A2"] * 4 + dem_kich_co["A3"] * 2 +
                  dem_kich_co["A4"] + dem_kich_co["A5"] * 0.5 + dem_kich_co["A6"] * 0.25 + dem_kich_co["A7"] * 0.125)

        thong_tin_tong_hop = [f"{stt}/{tong_so_tap_tin}", f"{thoi_gian_bat_dau}", f"{os.path.abspath(duong_dan_pdf)}", f"{tong_so_trang}", f"{dem_kich_co['A0']}", f"{dem_kich_co['A1']}",
                              f"{dem_kich_co['A2']}", f"{dem_kich_co['A3']}", f"{dem_kich_co['A4']}", f"{dem_kich_co['A5']}", f"{dem_kich_co['A6']}", f"{dem_kich_co['A7']}", f"{','.join(cac_trang_a3)}", f"{quy_a4}"]
        ket_qua.append(thong_tin_tong_hop)
        self.ket_qua_tin_hieu.emit('\t'.join(thong_tin_tong_hop[:-2] + thong_tin_tong_hop[-1:]))  # Bỏ cột 'Vi tri A3' khi in ra giao diện

        return ket_qua, thong_tin_tong_hop, dem_kich_co

    def phan_tich_cac_tap_tin_pdf_trong_thu_muc(self):
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

        bo_dem_chi_tiet = []
        bo_dem_tong_hop = []

        tieu_de_chi_tiet = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tRong-Cao\tKho giay"
        bo_dem_chi_tiet.append(tieu_de_chi_tiet)

        tieu_de_tong_hop = "STT\tThoi gian doc file\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\tVi tri A3\tQuy A4"
        bo_dem_tong_hop.append(tieu_de_tong_hop)
        self.ket_qua_tin_hieu.emit(tieu_de_tong_hop.replace("\tVi tri A3", ""))  # Bỏ cột 'Vi tri A3' khi in ra giao diện

        tong_so_trang_a0 = 0
        tong_so_trang_a1 = 0
        tong_so_trang_a2 = 0
        tong_so_trang_a3 = 0
        tong_so_trang_a4 = 0
        tong_so_trang_a5 = 0
        tong_so_trang_a6 = 0
        tong_so_trang_a7 = 0
        tong_so_quy_a4 = 0

        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_to_pdf = {executor.submit(self.phan_tich_trang_pdf, tap_tin_pdf, idx + 1, tong_so_tap_tin): tap_tin_pdf for idx, tap_tin_pdf in enumerate(cac_tap_tin_pdf)}

            for future in concurrent.futures.as_completed(future_to_pdf):
                tap_tin_pdf = future_to_pdf[future]
                try:
                    ket_qua, thong_tin_tong_hop, dem_kich_co = future.result()
                    bo_dem_tong_hop.append('\t'.join(thong_tin_tong_hop))  # Chuyển đổi danh sách thành chuỗi trước khi thêm vào bo_dem_tong_hop
                    bo_dem_chi_tiet.extend(['\t'.join(trang) for trang in ket_qua[:-1]])  # Thêm thông tin trang vào bo_dem_chi_tiet

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

                    if len(bo_dem_tong_hop) >= 1000:
                        self.ghi_du_lieu(bo_dem_tong_hop, duong_dan_tong_hop)
                        bo_dem_tong_hop = []

                    if len(bo_dem_chi_tiet) >= 1000:
                        self.ghi_du_lieu(bo_dem_chi_tiet, duong_dan_chi_tiet)
                        bo_dem_chi_tiet = []

                except Exception as exc:
                    self.ket_qua_tin_hieu.emit(f'{tap_tin_pdf} generated an exception: {exc}\n')

        if bo_dem_tong_hop:
            self.ghi_du_lieu(bo_dem_tong_hop, duong_dan_tong_hop)

        if bo_dem_chi_tiet:
            self.ghi_du_lieu(bo_dem_chi_tiet, duong_dan_chi_tiet)

        # Hiển thị thông tin tổng hợp trên giao diện
        duong_dan_tuyet_doi = os.path.abspath(self.duong_dan_thu_muc)
        tong_hop = (
            f"\n===========================\n"
            f"Số file pdf trong thư mục '{thu_muc_hien_tai}': {tong_so_tap_tin}\n"
            f"Tổng số trang quy ra A4: {tong_so_quy_a4}\n"
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
            f"\n===========================\n"
            f"Chi tiết xem thêm tại đường dẫn:\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dema4_tonghop.txt\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dema4_chitiet.txt\n"
        )
        self.tong_hop_tin_hieu.emit(tong_hop)

    def ghi_du_lieu(self, buffer, file_path):
        with open(file_path, 'a', encoding='utf-8') as file:
            file.write('\n'.join(buffer) + '\n')

class AppDemPDF(QWidget):
    def __init__(self):
        super().__init__()

        self.khoi_tao_giao_dien()

    def khoi_tao_giao_dien(self):
        self.setWindowTitle('Đếm trang A4')
        self.setWindowIcon(QtGui.QIcon('icon.png'))  # Đặt biểu tượng cho cửa sổ
        self.setGeometry(100, 100, 600, 400)
        self.canh_giua()  # Gọi phương thức để căn giữa cửa sổ

        layout = QVBoxLayout()

        hbox = QHBoxLayout()
        self.nhan_duong_dan = QLabel('Chọn thư mục:', self)
        hbox.addWidget(self.nhan_duong_dan)

        self.hop_duong_dan = QLineEdit(self)
        hbox.addWidget(self.hop_duong_dan)

        self.nut_chon = QPushButton('Chọn', self)
        self.nut_chon.setFixedWidth(80)
        self.nut_chon.clicked.connect(self.chon_thu_muc)
        hbox.addWidget(self.nut_chon)

        self.nut_dem = QPushButton('Đếm', self)
        self.nut_dem.setFixedWidth(80)
        self.nut_dem.setStyleSheet("background-color: green; color: white;")  # Đổi màu nền và màu chữ
        self.nut_dem.clicked.connect(self.dem_cac_tap_tin_pdf)
        hbox.addWidget(self.nut_dem)

        layout.addLayout(hbox)

        self.output_text = QTextEdit(self)
        self.output_text.setReadOnly(True)
        layout.addWidget(self.output_text)

        self.setLayout(layout)

    def canh_giua(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

    def chon_thu_muc(self):
        duong_dan_thu_muc = QFileDialog.getExistingDirectory(self, 'Chọn thư mục')
        if duong_dan_thu_muc:
            self.hop_duong_dan.setText(duong_dan_thu_muc)

    def dem_cac_tap_tin_pdf(self):
        duong_dan_thu_muc = self.hop_duong_dan.text()
        if duong_dan_thu_muc:
            # Xóa nội dung các tệp trước khi ghi
            thu_muc_hien_tai = os.path.basename(duong_dan_thu_muc)
            open(os.path.join(duong_dan_thu_muc, f'{thu_muc_hien_tai}_dema4_chitiet.txt'), 'w').close()
            open(os.path.join(duong_dan_thu_muc, f'{thu_muc_hien_tai}_dema4_tonghop.txt'), 'w').close()

            self.output_text.clear()

            self.cong_viec = DemA4(duong_dan_thu_muc)
            self.cong_viec.ket_qua_tin_hieu.connect(self.cap_nhat_ket_qua)
            self.cong_viec.tong_hop_tin_hieu.connect(self.hien_thi_tong_hop)

            self.thread = QtCore.QThread()
            self.cong_viec.moveToThread(self.thread)
            self.thread.started.connect(self.cong_viec.phan_tich_cac_tap_tin_pdf_trong_thu_muc)
            self.thread.start()

    def cap_nhat_ket_qua(self, text):
        # Bỏ cột 'Vi tri A3' khi hiển thị ra giao diện
        columns = text.split('\t')
        if len(columns) == 13:  # Kiểm tra nếu có cột "Vi tri A3"
            text = '\t'.join(columns[:9] + columns[10:])  # Loại bỏ cột thứ 10 (Vị trí A3)
        self.output_text.append(text)

    def hien_thi_tong_hop(self, tong_hop):
        self.output_text.append(tong_hop)
        self.thread.quit()
        self.thread.wait()

def main():
    import sys
    app = QApplication(sys.argv)
    ex = AppDemPDF()
    ex.show()
    sys.exit(app.exec_())

if __name__ == '__main__':
    main()
