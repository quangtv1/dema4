import fitz  # PyMuPDF library
import os
import datetime
import threading
from PyQt5 import QtCore, QtGui, QtWidgets
from PyQt5.QtWidgets import QFileDialog, QVBoxLayout, QHBoxLayout, QPushButton, QTextEdit, QLineEdit, QLabel, QWidget, QApplication, QDesktopWidget, QStatusBar, QProgressBar, QCheckBox

class DemA4(QtCore.QObject):
    ket_qua_tin_hieu = QtCore.pyqtSignal(str)
    tong_hop_tin_hieu = QtCore.pyqtSignal(str)
    hoan_thanh_tin_hieu = QtCore.pyqtSignal()
    cap_nhat_tien_do = QtCore.pyqtSignal(int, int)

    # Cạnh dài chuẩn (pt) của từng khổ, từ lớn đến nhỏ
    A_SIZES = [
        (3370.39, "A0"),
        (2383.94, "A1"),
        (1683.78, "A2"),
        (1190.55, "A3"),
        (841.89, "A4"),
        (595.28, "A5"),
        (419.53, "A6"),
        (297.64, "A7"),
    ]
    NGUONG_CANH_BAO = 0.8  # Cảnh báo khi cạnh dài đã lệch >= 80% nửa khoảng tới khổ kề bên
    DPI_CHUAN = 300
    DPI_SAI_SO = 0.03  # Cảnh báo khi DPI lệch quá 3% so với DPI chuẩn

    def __init__(self, duong_dan_thu_muc, dung_event, he_so):
        super().__init__()
        self.duong_dan_thu_muc = duong_dan_thu_muc
        self.dung_event = dung_event
        self.he_so = he_so
        self.so_trang_sat_nguong = 0
        self.so_trang_lech_dpi = 0

    def muc_sat_nguong(self, chieu_rong, chieu_cao, kich_co_a):
        """Trả về (tỷ lệ đã dùng của nửa khoảng tới khổ kề bên, tên khổ kề bên).
        Tỷ lệ 0 = đúng kích thước chuẩn, gần 1 = sắp bị xếp sang khổ kề bên."""
        canh_dai = max(chieu_rong, chieu_cao)
        ten_kho = [ten for _, ten in self.A_SIZES]
        i = ten_kho.index(kich_co_a)
        chuan = self.A_SIZES[i][0]
        if canh_dai > chuan:
            if i == 0:
                return 0.0, None  # Lớn hơn A0 vẫn tính A0
            ke_ben = self.A_SIZES[i - 1]
        else:
            if i == len(self.A_SIZES) - 1:
                return 0.0, None  # Nhỏ hơn A7 vẫn tính A7
            ke_ben = self.A_SIZES[i + 1]
        return abs(canh_dai - chuan) / (abs(ke_ben[0] - chuan) / 2), ke_ben[1]

    def tinh_dpi(self, trang):
        """DPI thực tế của ảnh scan lớn nhất trên trang = số pixel / số inch ảnh chiếm trên trang.
        Tính theo ma trận đặt ảnh nên không bị ảnh hưởng khi trang hoặc ảnh bị xoay.
        Trả về (dpi_ngang, dpi_doc), hoặc None nếu trang không có ảnh."""
        cac_anh = [anh for anh in trang.get_image_info() if anh["width"] > 0 and anh["height"] > 0]
        if not cac_anh:
            return None
        anh = max(cac_anh, key=lambda a: a["width"] * a["height"])
        a, b, c, d, _, _ = anh["transform"]
        rong_inch = (a * a + b * b) ** 0.5 / 72
        cao_inch = (c * c + d * d) ** 0.5 / 72
        if rong_inch == 0 or cao_inch == 0:
            return None
        return anh["width"] / rong_inch, anh["height"] / cao_inch

    def canh_bao_trang(self, trang, chieu_rong, chieu_cao, kich_co_a):
        """Trả về (chuỗi DPI, chuỗi cảnh báo) cho một trang và cập nhật bộ đếm cảnh báo."""
        canh_bao = []

        ty_le, kho_ke_ben = self.muc_sat_nguong(chieu_rong, chieu_cao, kich_co_a)
        if ty_le >= self.NGUONG_CANH_BAO:
            # Làm tròn xuống để trang chưa vượt ngưỡng không bị hiển thị 100%
            canh_bao.append(f"Sat nguong {kho_ke_ben} ({int(ty_le * 100)}%)")
            self.so_trang_sat_nguong += 1

        dpi = self.tinh_dpi(trang)
        if dpi is None:
            dpi_str = "-"
        else:
            dpi_ngang, dpi_doc = round(dpi[0]), round(dpi[1])
            dpi_str = f"{dpi_ngang}" if dpi_ngang == dpi_doc else f"{dpi_ngang}x{dpi_doc}"
            if any(abs(x - self.DPI_CHUAN) > self.DPI_CHUAN * self.DPI_SAI_SO for x in dpi):
                canh_bao.append(f"DPI {dpi_str} khac {self.DPI_CHUAN}")
                self.so_trang_lech_dpi += 1

        return dpi_str, '; '.join(canh_bao)

    def xac_dinh_kho_a(self, chieu_rong, chieu_cao):
        a_sizes = self.A_SIZES

        if chieu_rong > chieu_cao:
            huong_trang = "Ngang"
            canh_dai = chieu_rong
        else:
            huong_trang = "Doc"
            canh_dai = chieu_cao

        if canh_dai > a_sizes[0][0]:
            return "A0"
        if canh_dai < a_sizes[-1][0]:
            return "A7"

        for i in range(len(a_sizes) - 1):
            if a_sizes[i + 1][0] <= canh_dai <= a_sizes[i][0]:
                mid_point = (a_sizes[i][0] + a_sizes[i + 1][0]) / 2
                if canh_dai >= mid_point:
                    detected_size = a_sizes[i][1]
                else:
                    detected_size = a_sizes[i + 1][1]

                return detected_size

        return "Unknown"

    def phan_tich_trang_pdf(self, duong_dan_pdf, stt, tong_so_tap_tin):
        try:
            tap_tin_pdf = fitz.open(duong_dan_pdf)
        except Exception as e:
            return [], [], {"A0": 0, "A1": 0, "A2": 0, "A3": 0, "A4": 0, "A5": 0, "A6": 0, "A7": 0}, duong_dan_pdf

        ten_pdf = os.path.basename(duong_dan_pdf)
        ket_qua = []
        dem_kich_co = {"A0": 0, "A1": 0, "A2": 0, "A3": 0, "A4": 0, "A5": 0, "A6": 0, "A7": 0}
        tong_so_trang = len(tap_tin_pdf)

        kich_co_trang_truoc = None
        chuoi_tong_hop = []

        for chi_so_trang in range(tong_so_trang):
            if self.dung_event.is_set():
                break
            trang = tap_tin_pdf[chi_so_trang]
            hinh_chu_nhat_trang = trang.rect
            chieu_rong = round(hinh_chu_nhat_trang.width, 3)
            chieu_cao = round(hinh_chu_nhat_trang.height, 3)

            if chieu_rong > chieu_cao:
                huong_trang = "Ngang"
            else:
                huong_trang = "Doc"

            kich_co_a = self.xac_dinh_kho_a(chieu_rong, chieu_cao)
            dem_kich_co[kich_co_a] += 1
            dpi_str, canh_bao = self.canh_bao_trang(trang, chieu_rong, chieu_cao, kich_co_a)

            thong_tin_trang = [
                f"{stt}/{tong_so_tap_tin}",
                f"{os.path.abspath(duong_dan_pdf)} => trang: {chi_so_trang + 1}",
                f"{chieu_rong:.2f}".replace('.', ','),
                f"{chieu_cao:.2f}".replace('.', ','),
                f"{huong_trang}",
                f"{kich_co_a}",
                dpi_str,
                canh_bao
            ]

            ket_qua.append(thong_tin_trang)
            self.ket_qua_tin_hieu.emit('\t'.join(thong_tin_trang))  # Emit signal to update GUI

            if kich_co_trang_truoc is None or kich_co_a != kich_co_trang_truoc[1]:
                if kich_co_trang_truoc is not None:
                    chuoi_tong_hop.append(f"{kich_co_trang_truoc[0]}{kich_co_trang_truoc[1]}")
                kich_co_trang_truoc = [1, kich_co_a]
            else:
                kich_co_trang_truoc[0] += 1

        if kich_co_trang_truoc is not None:
            chuoi_tong_hop.append(f"{kich_co_trang_truoc[0]}{kich_co_trang_truoc[1]}")
        chuoi_tong_hop_str = ', '.join(chuoi_tong_hop)

        tap_tin_pdf.close()

        quy_a4 = (
            dem_kich_co["A0"] * self.he_so["A0"] +
            dem_kich_co["A1"] * self.he_so["A1"] +
            dem_kich_co["A2"] * self.he_so["A2"] +
            dem_kich_co["A3"] * self.he_so["A3"] +
            dem_kich_co["A4"] * self.he_so["A4"] +
            dem_kich_co["A5"] * self.he_so["A5"] +
            dem_kich_co["A6"] * self.he_so["A6"] +
            dem_kich_co["A7"] * self.he_so["A7"]
        )

        kich_thuoc_file = os.path.getsize(duong_dan_pdf) // 1024  # Kích thước file tính bằng KB
        quy_a4_str = f"{quy_a4:.2f}".replace('.', ',')  # Chuyển dấu chấm thành dấu phẩy

        thong_tin_tong_hop = [f"{stt}/{tong_so_tap_tin}", f"{os.path.abspath(duong_dan_pdf)}", f"{kich_thuoc_file}KB", f"{tong_so_trang}", f"{dem_kich_co['A0']}", f"{dem_kich_co['A1']}",
                              f"{dem_kich_co['A2']}", f"{dem_kich_co['A3']}", f"{dem_kich_co['A4']}", f"{dem_kich_co['A5']}", f"{dem_kich_co['A6']}", f"{dem_kich_co['A7']}", "'=>", quy_a4_str, ''.join(chuoi_tong_hop_str)]
        return ket_qua, thong_tin_tong_hop, dem_kich_co, None

    def phan_tich_cac_tap_tin_pdf_trong_thu_muc(self):
        thoi_gian_bat_dau = datetime.datetime.now()  # Lưu thời gian bắt đầu

        thu_muc_hien_tai = os.path.basename(self.duong_dan_thu_muc)
        duong_dan_chi_tiet = os.path.join(self.duong_dan_thu_muc, f'{thu_muc_hien_tai}_dempdf_chitiet.txt')
        duong_dan_tong_hop = os.path.join(self.duong_dan_thu_muc, f'{thu_muc_hien_tai}_dempdf_tonghop.txt')

        cac_tap_tin_pdf = []
        for root, _, files in os.walk(self.duong_dan_thu_muc):
            for file in files:
                if file.endswith('.pdf'):
                    cac_tap_tin_pdf.append(os.path.join(root, file))

        cac_tap_tin_pdf = sorted(cac_tap_tin_pdf)
        tong_so_tap_tin = len(cac_tap_tin_pdf)

        tieu_de_chi_tiet = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tKho giay\tDPI\tCanh bao"
        tieu_de_tong_hop = "STT\tDuong dan\tKich thuoc (KB)\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4\tVi tri page"

        # In tiêu đề header của file chi tiết
        self.ket_qua_tin_hieu.emit(tieu_de_chi_tiet)

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
        danh_sach_file_loi = []

        for idx, tap_tin_pdf in enumerate(cac_tap_tin_pdf):
            if self.dung_event.is_set():
                break

            ket_qua, thong_tin_tong_hop, dem_kich_co, file_loi = self.phan_tich_trang_pdf(tap_tin_pdf, idx + 1, tong_so_tap_tin)

            if file_loi:
                danh_sach_file_loi.append(file_loi)
                continue

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
            tong_so_quy_a4 += (
                dem_kich_co["A0"] * self.he_so["A0"] +
                dem_kich_co["A1"] * self.he_so["A1"] +
                dem_kich_co["A2"] * self.he_so["A2"] +
                dem_kich_co["A3"] * self.he_so["A3"] +
                dem_kich_co["A4"] * self.he_so["A4"] +
                dem_kich_co["A5"] * self.he_so["A5"] +
                dem_kich_co["A6"] * self.he_so["A6"] +
                dem_kich_co["A7"] * self.he_so["A7"]
            )

            completed_files += 1
            self.cap_nhat_tien_do.emit(completed_files, tong_so_tap_tin)

        # Hiển thị thông tin tổng hợp trên giao diện
        thoi_gian_ket_thuc = datetime.datetime.now()  # Lưu thời gian kết thúc
        thoi_gian_thuc_thi = thoi_gian_ket_thuc - thoi_gian_bat_dau # Tính toán thời gian thực thi
        total_seconds = round(thoi_gian_thuc_thi.total_seconds(), 3)  # Tính tổng số giây và làm tròn đến 3 chữ số thập phân
        hours, remainder = divmod(total_seconds, 3600)
        minutes, seconds = divmod(remainder, 60)
        thoi_gian_thuc_thi_str = f"{int(hours)}:{int(minutes)}:{seconds:.2f}"  # Định dạng thành chuỗi giờ, phút, giây

        duong_dan_tuyet_doi = os.path.abspath(self.duong_dan_thu_muc)
        tong_so_trang = tong_so_trang_a0 + tong_so_trang_a1 + tong_so_trang_a2 + tong_so_trang_a3 + tong_so_trang_a4 + tong_so_trang_a5 + tong_so_trang_a6 + tong_so_trang_a7
        so_trang_tang_them = tong_so_quy_a4 - tong_so_trang

        if danh_sach_file_loi:
            danh_sach_file_loi_str = "\n".join(danh_sach_file_loi)
            loi_thong_bao = f"\nDanh sách file PDF bị lỗi:\n{danh_sach_file_loi_str}\n"
        else:
            loi_thong_bao = ""

        tong_hop = (
            f"\nKẾT QUẢ ĐẾM\n"
            f"===========================\n"
            f"Tổng số file pdf trong thư mục {thu_muc_hien_tai}: {tong_so_tap_tin} \n"
            f"Tổng số trang {tong_so_trang} các loại => quy ra A4: {tong_so_quy_a4:.2f}".replace('.', ',') + f" (+{so_trang_tang_them:.2f}".replace('.', ',') + ")\n"
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
            f"Cảnh báo cần kiểm tra (xem cột Canh bao trong file chi tiết):\n"
            f" - Số trang sát ngưỡng chuyển khổ (>= {self.NGUONG_CANH_BAO:.0%} nửa khoảng): {self.so_trang_sat_nguong}\n"
            f" - Số trang có DPI khác {self.DPI_CHUAN}: {self.so_trang_lech_dpi}\n"
            f"------------------------------------------------------"
            f"{loi_thong_bao}\n"
            f"\nChi tiết đếm trang ghi tại file:\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dempdf_tonghop.txt\n"
            f" {duong_dan_tuyet_doi}\\{thu_muc_hien_tai}_dempdf_chitiet.txt\n"
            f"===========================\n\n"
            f"==> Đã đếm xong thư mục {duong_dan_tuyet_doi} !!!\n"
        )
        self.tong_hop_tin_hieu.emit(tong_hop)
        with open(duong_dan_tong_hop, 'a', encoding='utf-8') as f_tong_hop:
            f_tong_hop.write(tong_hop)
        self.hoan_thanh_tin_hieu.emit()


class AppDemPDF(QWidget):
    def __init__(self):
        super().__init__()
        self.dung_event = threading.Event()
        self.khoi_tao_giao_dien()

    def khoi_tao_giao_dien(self):
        self.setWindowTitle('Đếm PDF - SaoMai')
        self.setWindowIcon(QtGui.QIcon('icon.png'))  # Đặt biểu tượng cho cửa sổ
        self.setGeometry(100, 100, 800, 400)
        self.canh_giua()  # Gọi phương thức để căn giữa cửa sổ

        layout = QVBoxLayout()

        hbox = QHBoxLayout()
        self.nhan_duong_dan = QLabel('Chọn thư mục pdf:', self)
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



        hbox_heso = QHBoxLayout()
        self.checkbox_heso = QCheckBox('Hệ số:', self)
        self.checkbox_heso.stateChanged.connect(self.toggle_he_so_inputs)
        hbox_heso.addWidget(self.checkbox_heso)

        layout.addLayout(hbox)        

        self.he_so_labels = []
        self.he_so_inputs = []
        for i in range(8):
            label = QLabel(f"A{i}: ", self)

            label.setStyleSheet("color: gray")

            input_box = QLineEdit(self)
            input_box.setFixedWidth(50)
            input_box.setText(str(16 / (2**i)))
            input_box.setEnabled(False)
            self.he_so_labels.append(label)
            self.he_so_inputs.append(input_box)
            hbox_heso.addWidget(label)
            hbox_heso.addWidget(input_box)
        layout.addLayout(hbox_heso)
        
        input_box.setStyleSheet("color: gray")

        self.output_text = QTextEdit(self)
        self.output_text.setReadOnly(True)
        layout.addWidget(self.output_text)

        self.progress_bar = QProgressBar(self)
        self.progress_bar.setMaximumHeight(20)
        self.progress_bar.setStyleSheet(
            "QProgressBar {border: 1px solid grey; border-radius: 5px; text-align: center; background-color: rgba(240, 240, 240, 200);}"
            "QProgressBar::chunk {background-color: #C0C0C0; width: 20px;}"
        )

        self.status_bar = QStatusBar(self)
        self.status_bar.setFixedHeight(25)
        self.status_bar.setStyleSheet("QStatusBar {background: rgba(240, 240, 240, 200);}")

        self.status_bar.addPermanentWidget(self.progress_bar, 1)
        layout.addWidget(self.status_bar)

        self.setLayout(layout)

    def toggle_he_so_inputs(self):
        enabled = self.checkbox_heso.isChecked()

        color = "black" if enabled else "gray"
        for label, input_box in zip(self.he_so_labels, self.he_so_inputs):
            input_box.setEnabled(enabled)
            input_box.setStyleSheet(f"color: {color}")
            label.setStyleSheet(f"color: {color}")


    def canh_giua(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

    def chon_thu_muc(self):
        duong_dan_thu_muc = QFileDialog.getExistingDirectory(self, 'Chọn thư mục')
        if duong_dan_thu_muc:
            self.hop_duong_dan.setText(duong_dan_thu_muc.replace('/', '\\'))
            self.reset_status_bar()

    def dem_cac_tap_tin_pdf(self):
        duong_dan_thu_muc = self.hop_duong_dan.text()
        if duong_dan_thu_muc:
            self.dung_event.clear()
            self.nut_dem.setText("Dừng")
            self.nut_dem.setStyleSheet("background-color: red; color: white;")
            self.nut_dem.clicked.disconnect()
            self.nut_dem.clicked.connect(self.dung_dem)

            # Xóa nội dung các tệp trước khi ghi
            thu_muc_hien_tai = os.path.basename(duong_dan_thu_muc)
            open(os.path.join(duong_dan_thu_muc, f'{thu_muc_hien_tai}_dempdf_chitiet.txt'), 'w').close()
            open(os.path.join(duong_dan_thu_muc, f'{thu_muc_hien_tai}_dempdf_tonghop.txt'), 'w').close()

            self.output_text.clear()

            he_so = {f"A{i}": float(input_box.text()) for i, input_box in enumerate(self.he_so_inputs)}

            self.cong_viec = DemA4(duong_dan_thu_muc, self.dung_event, he_so)
            self.cong_viec.ket_qua_tin_hieu.connect(self.cap_nhat_ket_qua)
            self.cong_viec.tong_hop_tin_hieu.connect(self.hien_thi_tong_hop)
            self.cong_viec.hoan_thanh_tin_hieu.connect(self.hoan_thanh_dem)
            self.cong_viec.cap_nhat_tien_do.connect(self.cap_nhat_tien_do)

            self.thread = QtCore.QThread()
            self.cong_viec.moveToThread(self.thread)
            self.thread.started.connect(self.cong_viec.phan_tich_cac_tap_tin_pdf_trong_thu_muc)
            self.thread.start()

    def reset_status_bar(self):
        self.progress_bar.reset()
        self.progress_bar.setFormat("")
        self.status_bar.clearMessage()

    def dung_dem(self):
        self.dung_event.set()
        self.nut_dem.setText("Đếm")
        self.nut_dem.setStyleSheet("background-color: green; color: white;")
        self.nut_dem.clicked.disconnect()
        self.nut_dem.clicked.connect(self.dem_cac_tap_tin_pdf)

    def cap_nhat_ket_qua(self, text):
        self.output_text.append(text)

    def hien_thi_tong_hop(self, tong_hop):
        self.output_text.append(tong_hop)

    def hoan_thanh_dem(self):
        self.nut_dem.setText("Đếm")
        self.nut_dem.setStyleSheet("background-color: green; color: white;")
        self.nut_dem.clicked.disconnect()
        self.nut_dem.clicked.connect(self.dem_cac_tap_tin_pdf)
        self.thread.quit()
        self.thread.wait()

    def cap_nhat_tien_do(self, so_file_da_xu_ly, tong_so_file):
        phan_tram_hoan_thanh = int((so_file_da_xu_ly / tong_so_file) * 100)
        self.progress_bar.setValue(phan_tram_hoan_thanh)
        self.progress_bar.setFormat(f'{phan_tram_hoan_thanh}% ({so_file_da_xu_ly}/{tong_so_file} files)')


def main():
    import sys
    app = QApplication(sys.argv)
    ex = AppDemPDF()
    ex.show()
    sys.exit(app.exec_())

if __name__ == '__main__':
    main()
