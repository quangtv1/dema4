import sys
import os
import datetime
import threading
import pandas as pd
from PyQt5 import QtCore, QtGui, QtWidgets
from PyQt5.QtWidgets import (
    QFileDialog, QVBoxLayout, QHBoxLayout, QPushButton, QTextEdit, QLineEdit,
    QLabel, QWidget, QApplication, QDesktopWidget, QStatusBar, QProgressBar,
    QMenuBar, QAction, QDialog, QTabWidget, QMessageBox
)
import fitz  # PyMuPDF library

class CaiDatDialog(QDialog):
    def __init__(self, settings1, settings2, parent=None):
        super().__init__(parent)
        self.settings1 = settings1
        self.settings2 = settings2
        self.init_ui()

    def init_ui(self):
        self.setWindowTitle('Cài đặt')
        self.setGeometry(100, 100, 500, 300)

        layout = QVBoxLayout()

        tab_widget = QTabWidget(self)
        tab1 = QWidget()
        tab2 = QWidget()

        tab_widget.addTab(tab1, "Tổng hợp Excel")
        tab_widget.addTab(tab2, "Đếm trang PDF")

        self.init_tab1(tab1)
        self.init_tab2(tab2)

        layout.addWidget(tab_widget)

        button_box = QHBoxLayout()
        save_button = QPushButton('Lưu', self)
        save_button.setFixedWidth(80)  # Thêm dòng này để đặt chiều rộng cho nút "Lưu"
        save_button.clicked.connect(self.save_settings)
        button_box.addWidget(save_button)

        cancel_button = QPushButton('Hủy', self)
        cancel_button.setFixedWidth(80)
        cancel_button.clicked.connect(self.reject)
        button_box.addWidget(cancel_button)

        layout.addLayout(button_box)
        self.setLayout(layout)

    def init_tab1(self, tab):
        layout = QVBoxLayout()
        self.inputs1 = {}
        labels1 = {
            "ho_so_col": "Cột 'Mã hồ sơ' - Cột 2, trong Sheet Files              :",
            "van_ban_col": "Cột 'Mã văn bản' - Cột 2, trong sheet Docs         :",
            "trang_quy_doi_col": "Cột 'Số lượng trang quy đổi' - Cột 17 sheet Docs:"
        }

        for key, label_text in labels1.items():
            hbox = QHBoxLayout()
            label = QLabel(label_text, self)
            hbox.addWidget(label)

            input_field = QLineEdit(self)
            input_field.setText(self.settings1[key])
            hbox.addWidget(input_field)

            self.inputs1[key] = input_field
            layout.addLayout(hbox)

        tab.setLayout(layout)

    def init_tab2(self, tab):
        layout = QVBoxLayout()
        self.inputs2 = {}
        labels2 = {
            "A0": "Hệ số của trang A0/A4:",
            "A1": "Hệ số của trang A1/A4:",
            "A2": "Hệ số của trang A2/A4:",
            "A3": "Hệ số của trang A3/A4:",
            "A4": "Hệ số của trang A4/A4:",
            "A5": "Hệ số của trang A5/A4:",
            "A6": "Hệ số của trang A6/A4:",
            "A7": "Hệ số của trang A7/A4:"
        }

        for key, label_text in labels2.items():
            hbox = QHBoxLayout()
            label = QLabel(label_text, self)
            hbox.addWidget(label)

            input_field = QLineEdit(self)
            input_field.setText(str(self.settings2[key]))
            hbox.addWidget(input_field)

            self.inputs2[key] = input_field
            layout.addLayout(hbox)

        tab.setLayout(layout)

    def save_settings(self):
        for key, input_field in self.inputs1.items():
            self.settings1[key] = input_field.text()
        for key, input_field in self.inputs2.items():
            self.settings2[key] = float(input_field.text())
        self.accept()

    def center_dialog(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

class AppDemPDF(QWidget):
    def __init__(self):
        super().__init__()
        self.settings1 = {
            "ho_so_col": "B",  # Chỉ số cột mặc định là B (cột 2)
            "van_ban_col": "B",  # Chỉ số cột mặc định là B (cột 2)
            "trang_quy_doi_col": "Q"  # Chỉ số cột mặc định là Q (cột 17)
        }
        self.settings2 = {
            "A0": 16, "A1": 8, "A2": 4, "A3": 2, "A4": 1, "A5": 0.5, "A6": 0.25, "A7": 0.125
        }
        self.dung_event = threading.Event()
        self.khoi_tao_giao_dien()

    def khoi_tao_giao_dien(self):
        self.setWindowTitle('Đếm số trang A4 - SaoMai')
        self.setWindowIcon(QtGui.QIcon('icon.png'))  # Đặt biểu tượng cho cửa sổ
        self.setGeometry(100, 100, 800, 400)
        self.canh_giua()  # Gọi phương thức để căn giữa cửa sổ

        layout = QVBoxLayout()

        menubar = QMenuBar(self)
        chuc_nang_menu = menubar.addMenu('Chức năng')

        dem_trang_pdf_action = QAction('1. Đếm trang A4', self)
        dem_trang_pdf_action.triggered.connect(self.hien_thi_giao_dien_dem_pdf)
        chuc_nang_menu.addAction(dem_trang_pdf_action)

        tong_hop_excel_action = QAction('2. Thống kê file Metadata', self)
        tong_hop_excel_action.triggered.connect(self.hien_thi_giao_dien_excel)
        chuc_nang_menu.addAction(tong_hop_excel_action)

        ky_so_action = QAction('3. Ký số thư mục', self)
        ky_so_action.triggered.connect(self.ky_so_thu_muc)
        chuc_nang_menu.addAction(ky_so_action)


        cai_dat_action = QAction('Cài đặt', self)
        cai_dat_action.triggered.connect(self.mo_cai_dat)
        menubar.addAction(cai_dat_action)


        layout.setMenuBar(menubar)

        self.dem_pdf_widget = self.tao_giao_dien_dem_pdf()
        self.tong_hop_excel_widget = self.tao_giao_dien_excel()

        self.stacked_layout = QtWidgets.QStackedLayout()
        self.stacked_layout.addWidget(self.dem_pdf_widget)
        self.stacked_layout.addWidget(self.tong_hop_excel_widget)

        layout.addLayout(self.stacked_layout)

        self.setLayout(layout)
        self.hien_thi_giao_dien_dem_pdf()

    def tao_giao_dien_dem_pdf(self):
        widget = QWidget()
        layout = QVBoxLayout()

        # Thêm label tiêu đề và căn giữa
        title_layout = QHBoxLayout()
        label = QLabel('1. Đếm số trang A4 trong thư mục', self)
        label.setStyleSheet("font-size: 12px; font-style: italic; ")
        title_layout.addStretch()
        title_layout.addWidget(label)
        title_layout.addStretch()
        layout.addLayout(title_layout)

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

        layout.addLayout(hbox)

        self.output_text_pdf = QTextEdit(self)
        self.output_text_pdf.setReadOnly(True)
        layout.addWidget(self.output_text_pdf)

        self.progress_bar_pdf = QProgressBar(self)
        self.progress_bar_pdf.setMaximumHeight(20)
        self.progress_bar_pdf.setStyleSheet(
            "QProgressBar {border: 1px solid grey; border-radius: 5px; text-align: center; background-color: rgba(240, 240, 240, 200);}"
            "QProgressBar::chunk {background-color: #C0C0C0; width: 20px;}"
        )

        self.status_bar_pdf = QStatusBar(self)
        self.status_bar_pdf.setFixedHeight(25)
        self.status_bar_pdf.setStyleSheet("QStatusBar {background: rgba(240, 240, 240, 200);}")

        self.status_bar_pdf.addPermanentWidget(self.progress_bar_pdf, 1)
        layout.addWidget(self.status_bar_pdf)

        widget.setLayout(layout)
        return widget

    def tao_giao_dien_excel(self):
        widget = QWidget()
        layout = QVBoxLayout()

        # Thêm label tiêu đề với link và căn giữa
        title_layout = QHBoxLayout()
        label = QLabel('2. Thống kê file Metadata theo <a href="https://docs.google.com/spreadsheets/d/1ScZh8mByLlgNHQgiFKAIG_TNDHlASO7G3jbSSK_ECnU/edit?usp=sharing">mẫu link</a>', self)
        label.setOpenExternalLinks(True)  # Cho phép mở liên kết ngoài
        label.setStyleSheet("font-size: 12px; font-style: italic; ")
        title_layout.addStretch()
        title_layout.addWidget(label)
        title_layout.addStretch()
        layout.addLayout(title_layout)

        hbox = QHBoxLayout()
        self.nhan_duong_dan_excel = QLabel('Chọn file metadata:', self)
        hbox.addWidget(self.nhan_duong_dan_excel)

        self.hop_duong_dan_excel = QLineEdit(self)
        hbox.addWidget(self.hop_duong_dan_excel)

        self.nut_chon_excel = QPushButton('Chọn', self)
        self.nut_chon_excel.setFixedWidth(80)
        self.nut_chon_excel.clicked.connect(self.chon_file_excel)
        hbox.addWidget(self.nut_chon_excel)

        self.nut_thuc_hien_excel = QPushButton('Thực hiện', self)
        self.nut_thuc_hien_excel.setFixedWidth(80)
        self.nut_thuc_hien_excel.clicked.connect(self.tong_hop_file_excel)
        hbox.addWidget(self.nut_thuc_hien_excel)

        layout.addLayout(hbox)

        self.output_text_excel = QTextEdit(self)
        self.output_text_excel.setReadOnly(True)
        layout.addWidget(self.output_text_excel)

        self.progress_bar_excel = QProgressBar(self)
        self.progress_bar_excel.setMaximumHeight(20)
        self.progress_bar_excel.setStyleSheet(
            "QProgressBar {border: 1px solid grey; border-radius: 5px; text-align: center; background-color: rgba(240, 240, 240, 200);}"
            "QProgressBar::chunk {background-color: #C0C0C0; width: 20px;}"
        )

        self.status_bar_excel = QStatusBar(self)
        self.status_bar_excel.setFixedHeight(25)
        self.status_bar_excel.setStyleSheet("QStatusBar {background: rgba(240, 240, 240, 200);}")

        self.status_bar_excel.addPermanentWidget(self.progress_bar_excel, 1)
        layout.addWidget(self.status_bar_excel)

        widget.setLayout(layout)
        return widget

    def hien_thi_giao_dien_dem_pdf(self):
        self.stacked_layout.setCurrentWidget(self.dem_pdf_widget)

    def hien_thi_giao_dien_excel(self):
        self.stacked_layout.setCurrentWidget(self.tong_hop_excel_widget)

    def canh_giua(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

    def chon_thu_muc(self):
        duong_dan_thu_muc = QFileDialog.getExistingDirectory(self, 'Chọn thư mục')
        if duong_dan_thu_muc:
            self.hop_duong_dan.setText(duong_dan_thu_muc.replace('/', '\\'))
            self.reset_status_bar_pdf()

    def chon_file_excel(self):
        duong_dan_file, _ = QFileDialog.getOpenFileName(self, 'Chọn file Excel', '', 'Excel Files (*.xlsx)')
        if duong_dan_file:
            self.hop_duong_dan_excel.setText(duong_dan_file.replace('/', '\\'))
            self.reset_status_bar_excel()

    def dem_cac_tap_tin_pdf(self):
        duong_dan_thu_muc = self.hop_duong_dan.text()
        if duong_dan_thu_muc:
            self.dung_event.clear()
            self.nut_dem.setText("Dừng")
            self.nut_dem.setStyleSheet("background-color: red; color: white;")
            self.nut_dem.clicked.disconnect()
            self.nut_dem.clicked.connect(self.dung_dem)

            # Xóa nội dung các tệp trước khi ghi
            self.output_text_pdf.clear()

            self.cong_viec = DemA4(duong_dan_thu_muc, self.dung_event, self.settings2)
            self.cong_viec.ket_qua_tin_hieu.connect(self.cap_nhat_ket_qua_pdf)
            self.cong_viec.tong_hop_tin_hieu.connect(self.hien_thi_tong_hop_pdf)
            self.cong_viec.hoan_thanh_tin_hieu.connect(self.hoan_thanh_dem_pdf)
            self.cong_viec.cap_nhat_tien_do.connect(self.cap_nhat_tien_do_pdf)

            self.thread = QtCore.QThread()
            self.cong_viec.moveToThread(self.thread)
            self.thread.started.connect(self.cong_viec.phan_tich_cac_tap_tin_pdf_trong_thu_muc)
            self.thread.start()

    def tong_hop_file_excel(self):
        duong_dan_file = self.hop_duong_dan_excel.text()
        if duong_dan_file:
            try:
                self.analyze_excel_file(duong_dan_file)
            except Exception as e:
                self.output_text_excel.append(f"Lỗi khi phân tích file Excel: {e}")
                QMessageBox.critical(self, "Lỗi", f"Lỗi khi phân tích file Excel: {e}")

    def read_excel_fast(self, filepath, sheet_name, skiprows):
        return pd.read_excel(filepath, sheet_name=sheet_name, engine='openpyxl', skiprows=skiprows)

    def col_letter_to_index(self, col_letter):
        """Chuyển đổi chữ cái cột thành chỉ số cột, ví dụ 'A' -> 0, 'B' -> 1, ..., 'Z' -> 25, 'AA' -> 26, ..."""
        col_letter = col_letter.upper()
        index = 0
        for i, char in enumerate(reversed(col_letter)):
            index += (ord(char) - ord('A') + 1) * (26 ** i)
        return index - 1

    def analyze_excel_file(self, duong_dan_file):
        try:
            xls = pd.ExcelFile(duong_dan_file)
            
            # Đọc dữ liệu từ sheet Files với tên cột tùy chỉnh
            files_df = self.read_excel_fast(duong_dan_file, 'Files', skiprows=0)

            ho_so_col_letter = self.settings1["ho_so_col"]
            ho_so_col_index = self.col_letter_to_index(ho_so_col_letter)  # Chuyển đổi từ chữ cái cột sang chỉ số cột
            ho_so_col_name = files_df.columns[ho_so_col_index]  # Lấy tên cột dựa trên chỉ số cột

            # Tính tổng số giá trị duy nhất trong cột
            unique_ho_so_count = files_df[ho_so_col_name].iloc[1:].nunique()

            # Đọc dữ liệu từ sheet Docs với tên cột tùy chỉnh
            docs_df = self.read_excel_fast(duong_dan_file, 'Docs', skiprows=0)

            van_ban_col_letter = self.settings1["van_ban_col"]
            van_ban_col_index = self.col_letter_to_index(van_ban_col_letter)  # Chuyển đổi từ chữ cái cột sang chỉ số cột
            van_ban_col_name = docs_df.columns[van_ban_col_index]  # Lấy tên cột dựa trên chỉ số cột

            # Tính tổng số giá trị duy nhất trong cột
            unique_van_ban_count = docs_df[van_ban_col_name].iloc[1:].nunique()

            # Tính tổng số trang quy đổi
            trang_quy_doi_col_letter = self.settings1["trang_quy_doi_col"]
            trang_quy_doi_col_index = self.col_letter_to_index(trang_quy_doi_col_letter)
            docs_df_copy = docs_df.copy()
            docs_df_copy.iloc[:, trang_quy_doi_col_index] = pd.to_numeric(docs_df_copy.iloc[:, trang_quy_doi_col_index], errors='coerce')
            total_trang_quy_doi = docs_df_copy.iloc[1:, trang_quy_doi_col_index].sum()

            # Tính tổng số trường trong hồ sơ
            files_df_copy = files_df.copy()
            total_fields_files = files_df_copy.notna().sum().sum()  # Tính tất cả các ô khác rỗng

            # Tính tổng số trường văn bản
            docs_df_copy = docs_df.copy()
            total_fields_docs = docs_df_copy.notna().sum().sum()  # Tính tất cả các ô khác rỗng

            # Định dạng và in kết quả ra QTextEdit
            self.output_text_excel.setText(
                f'KẾT QUẢ ĐỌC FILE METADATA\n'
                f'==============================\n'
                f'- Tổng số hồ sơ Files: {unique_ho_so_count}\n'
                f'- Tổng số văn bản Docs: {unique_van_ban_count}\n'
                f'- Tổng số trang quy đổi: {total_trang_quy_doi}\n'
                f'- Tổng số trường trong hồ sơ: {total_fields_files}\n'
                f'- Tổng số trường văn bản: {total_fields_docs}\n'
                f'==============================\n'
                f'==> Đã đếm xong file: {duong_dan_file} !!!'
            )
        except Exception as e:
            raise ValueError(f"Lỗi khi đọc file Excel: {e}")

    def reset_status_bar_pdf(self):
        self.progress_bar_pdf.reset()
        self.progress_bar_pdf.setFormat("")
        self.status_bar_pdf.clearMessage()

    def reset_status_bar_excel(self):
        self.progress_bar_excel.reset()
        self.progress_bar_excel.setFormat("")
        self.status_bar_excel.clearMessage()

    def dung_dem(self):
        self.dung_event.set()
        self.nut_dem.setText("Đếm")
        self.nut_dem.setStyleSheet("background-color: green; color: white;")
        self.nut_dem.clicked.disconnect()
        self.nut_dem.clicked.connect(self.dem_cac_tap_tin_pdf)

    def cap_nhat_ket_qua_pdf(self, text):
        self.output_text_pdf.append(text)

    def hien_thi_tong_hop_pdf(self, tong_hop):
        self.output_text_pdf.append(tong_hop)

    def hoan_thanh_dem_pdf(self):
        self.nut_dem.setText("Đếm")
        self.nut_dem.setStyleSheet("background-color: green; color: white;")
        self.nut_dem.clicked.disconnect()
        self.nut_dem.clicked.connect(self.dem_cac_tap_tin_pdf)
        self.thread.quit()
        self.thread.wait()
        self.ask_save_results_pdf()

    def cap_nhat_tien_do_pdf(self, so_file_da_xu_ly, tong_so_file):
        phan_tram_hoan_thanh = int((so_file_da_xu_ly / tong_so_file) * 100)
        self.progress_bar_pdf.setValue(phan_tram_hoan_thanh)
        self.progress_bar_pdf.setFormat(f'{phan_tram_hoan_thanh}% ({so_file_da_xu_ly}/{tong_so_file} files)')

    def ask_save_results_pdf(self):
        reply = QMessageBox.question(self, 'Lưu kết quả đếm', 'Bạn có muốn lưu kết quả đếm không?', QMessageBox.Yes | QMessageBox.No, QMessageBox.No)
        if reply == QMessageBox.Yes:
            # Gợi ý lưu ngay trong thư mục vừa đếm với tên <thư mục>_dem.txt; trùng tên thì ghi đè, không hỏi lại
            thu_muc_dem = os.path.normpath(self.hop_duong_dan.text())
            ten_thu_muc = os.path.basename(thu_muc_dem) or "ket_qua"
            default_file_name = os.path.join(thu_muc_dem, f"{ten_thu_muc}_dem.txt")
            file_path, _ = QFileDialog.getSaveFileName(self, "Chọn đường dẫn lưu kết quả", default_file_name, "Text Files (*.txt)",
                                                       options=QFileDialog.DontConfirmOverwrite)
            if file_path:
                duong_dan_chi_tiet = self.save_results_pdf(file_path)
                self.output_text_pdf.append(f"Kết quả đếm ghi tại file:\n {os.path.normpath(file_path)}\n {os.path.normpath(duong_dan_chi_tiet)}\n===========================\n\n")
        self.output_text_pdf.append(f"==> Đã đếm xong thư mục {os.path.abspath(self.hop_duong_dan.text())} !!!\n")

    def save_results_pdf(self, file_path):
        """Ghi file tổng hợp (mỗi file PDF một dòng) vào file_path, ví dụ <thư mục>_dem.txt, và file chi tiết
        (mỗi trang một dòng) vào <tên file>_chitiet.txt cùng thư mục, ví dụ <thư mục>_dem_chitiet.txt.
        Các cột cách nhau bởi tab; file đã có thì ghi đè. Trả về đường dẫn file chi tiết."""
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write("STT\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4\tVi tri page\n")
            for item in self.cong_viec.ket_qua_phan_tich:
                f.write('\t'.join(item) + '\n')
            f.write(self.cong_viec.tong_hop)

        goc, duoi = os.path.splitext(file_path)
        duong_dan_chi_tiet = f"{goc}_chitiet{duoi or '.txt'}"
        with open(duong_dan_chi_tiet, 'w', encoding='utf-8') as f:
            f.write(self.cong_viec.TIEU_DE_CHI_TIET + '\n')
            for trang in self.cong_viec.ket_qua_chi_tiet:
                f.write('\t'.join(trang) + '\n')
        return duong_dan_chi_tiet

    def mo_cai_dat(self):
        dialog = CaiDatDialog(self.settings1, self.settings2, self)
        dialog.center_dialog()  # Di chuyển hộp thoại về giữa màn hình
        if dialog.exec_():
            self.settings1 = dialog.settings1
            self.settings2 = dialog.settings2

    def ky_so_thu_muc(self):
        # Hiện tại chưa làm gì khi hành động được kích hoạt
        pass

class DemA4(QtCore.QObject):
    ket_qua_tin_hieu = QtCore.pyqtSignal(str)
    tong_hop_tin_hieu = QtCore.pyqtSignal(str)
    hoan_thanh_tin_hieu = QtCore.pyqtSignal()
    cap_nhat_tien_do = QtCore.pyqtSignal(int, int)
    TIEU_DE_CHI_TIET = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tKho giay\tDPI\tCanh bao"

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
        # Kết quả riêng của lần đếm này (mỗi lần đếm tạo đối tượng mới nên không lẫn kết quả lần trước)
        self.ket_qua_phan_tich = []  # Mỗi file PDF một dòng
        self.ket_qua_chi_tiet = []  # Mỗi trang một dòng

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
        ten_pdf = os.path.basename(duong_dan_pdf)
        try:
            tap_tin_pdf = fitz.open(duong_dan_pdf)
        except:
            self.ket_qua_tin_hieu.emit(f"Lỗi mở tệp: {duong_dan_pdf}")
            return [], [], {}

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

            thong_tin_trang = [f"{stt}/{tong_so_tap_tin}", f"{os.path.abspath(duong_dan_pdf)} => trang: {chi_so_trang + 1}", f"{chieu_rong}", f"{chieu_cao}", f"{huong_trang}", f"{kich_co_a}", dpi_str, canh_bao]
            ket_qua.append(thong_tin_trang)
            self.ket_qua_chi_tiet.append(thong_tin_trang)
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

        quy_a4 = (dem_kich_co["A0"] * self.he_so["A0"] + dem_kich_co["A1"] * self.he_so["A1"] + dem_kich_co["A2"] * self.he_so["A2"] +
                  dem_kich_co["A3"] * self.he_so["A3"] + dem_kich_co["A4"] * self.he_so["A4"] + dem_kich_co["A5"] * self.he_so["A5"] +
                  dem_kich_co["A6"] * self.he_so["A6"] + dem_kich_co["A7"] * self.he_so["A7"])

        thong_tin_tong_hop = [f"{stt}/{tong_so_tap_tin}", f"{os.path.abspath(duong_dan_pdf)}", f"{tong_so_trang}", f"{dem_kich_co['A0']}", f"{dem_kich_co['A1']}",
                              f"{dem_kich_co['A2']}", f"{dem_kich_co['A3']}", f"{dem_kich_co['A4']}", f"{dem_kich_co['A5']}", f"{dem_kich_co['A6']}", f"{dem_kich_co['A7']}", "'=>", f"{quy_a4}", ''.join(chuoi_tong_hop_str)]
        
        self.ket_qua_phan_tich.append(thong_tin_tong_hop)
        
        return ket_qua, thong_tin_tong_hop, dem_kich_co

    def phan_tich_cac_tap_tin_pdf_trong_thu_muc(self):
        thoi_gian_bat_dau = datetime.datetime.now()  # Lưu thời gian bắt đầu

        thu_muc_hien_tai = os.path.basename(self.duong_dan_thu_muc)

        cac_tap_tin_pdf = []
        for root, _, files in os.walk(self.duong_dan_thu_muc):
            for file in files:
                if file.endswith('.pdf'):
                    cac_tap_tin_pdf.append(os.path.join(root, file))

        cac_tap_tin_pdf = sorted(cac_tap_tin_pdf)
        tong_so_tap_tin = len(cac_tap_tin_pdf)

        tieu_de_chi_tiet = self.TIEU_DE_CHI_TIET
        tieu_de_tong_hop = "STT\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\t'=>\tQuy A4\tVi tri page"

        # In tiêu đề header của file chi tiết
        self.ket_qua_tin_hieu.emit(tieu_de_chi_tiet)

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

            ket_qua, thong_tin_tong_hop, dem_kich_co = self.phan_tich_trang_pdf(tap_tin_pdf, idx + 1, tong_so_tap_tin)

            # File không mở được: ghi vào danh sách lỗi, bỏ qua và đếm tiếp file sau
            if not dem_kich_co:
                danh_sach_file_loi.append(os.path.abspath(tap_tin_pdf))
                completed_files += 1
                self.cap_nhat_tien_do.emit(completed_files, tong_so_tap_tin)
                continue

            tong_so_trang_a0 += dem_kich_co["A0"]
            tong_so_trang_a1 += dem_kich_co["A1"]
            tong_so_trang_a2 += dem_kich_co["A2"]
            tong_so_trang_a3 += dem_kich_co["A3"]
            tong_so_trang_a4 += dem_kich_co["A4"]
            tong_so_trang_a5 += dem_kich_co["A5"]
            tong_so_trang_a6 += dem_kich_co["A6"]
            tong_so_trang_a7 += dem_kich_co["A7"]
            tong_so_quy_a4 += (dem_kich_co["A0"] * self.he_so["A0"] + dem_kich_co["A1"] * self.he_so["A1"] +
                               dem_kich_co["A2"] * self.he_so["A2"] + dem_kich_co["A3"] * self.he_so["A3"] +
                               dem_kich_co["A4"] * self.he_so["A4"] + dem_kich_co["A5"] * self.he_so["A5"] +
                               dem_kich_co["A6"] * self.he_so["A6"] + dem_kich_co["A7"] * self.he_so["A7"])

            completed_files += 1
            self.cap_nhat_tien_do.emit(completed_files, tong_so_tap_tin)

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
        self.tong_hop = (
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
            f"Cảnh báo cần kiểm tra (xem cột Canh bao ở danh sách trang):\n"
            f" - Số trang sát ngưỡng chuyển khổ (>= {self.NGUONG_CANH_BAO:.0%} nửa khoảng): {self.so_trang_sat_nguong}\n"
            f" - Số trang có DPI khác {self.DPI_CHUAN}: {self.so_trang_lech_dpi}\n"
            f"------------------------------------------------------\n"
        )
        if danh_sach_file_loi:
            self.tong_hop += (
                f"Danh sách file PDF bị lỗi ({len(danh_sach_file_loi)} file, không được đếm):\n"
                + "\n".join(danh_sach_file_loi) + "\n"
                f"------------------------------------------------------\n"
            )
        self.tong_hop_tin_hieu.emit(self.tong_hop)
        self.hoan_thanh_tin_hieu.emit()

def main():
    app = QApplication(sys.argv)
    ex = AppDemPDF()
    ex.show()
    sys.exit(app.exec_())

if __name__ == '__main__':
    main()
