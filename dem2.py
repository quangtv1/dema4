import fitz  # PyMuPDF library
import os
import concurrent.futures
import datetime
from PyQt5 import QtCore, QtWidgets
from PyQt5.QtWidgets import QFileDialog, QVBoxLayout, QHBoxLayout, QPushButton, QTextEdit, QLineEdit, QLabel, QWidget, QApplication, QDesktopWidget, QMessageBox

class Worker(QtCore.QObject):
    result_signal = QtCore.pyqtSignal(str)
    summary_signal = QtCore.pyqtSignal(str)

    def __init__(self, folder_path):
        super().__init__()
        self.folder_path = folder_path

    def xac_dinh_kho_a(self, longer_side):
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

        if longer_side > a_sizes[0][0]:
            return "A0"
        if longer_side < a_sizes[-1][0]:
            return "A7"

        for i in range(len(a_sizes) - 1):
            if a_sizes[i + 1][0] <= longer_side <= a_sizes[i][0]:
                mid_point = (a_sizes[i][0] + a_sizes[i + 1][0]) / 2
                if longer_side >= mid_point:
                    return a_sizes[i][1]
                else:
                    return a_sizes[i + 1][1]
        return "Unknown"

    def phan_tich_trang_pdf(self, pdf_path, stt, total_files):
        start_time = datetime.datetime.now().strftime('%d-%m-%Y %H:%M:%S')
        pdf_name = os.path.splitext(os.path.basename(pdf_path))[0]
        pdf_file = fitz.open(pdf_path)
        results = []
        size_count = {"A0": 0, "A1": 0, "A2": 0, "A3": 0, "A4": 0, "A5": 0, "A6": 0, "A7": 0}
        a3_pages = []
        total_pages = len(pdf_file)

        for page_index in range(total_pages):
            page = pdf_file[page_index]
            page_rect = page.rect
            rounded_width = round(page_rect.width, 3)
            rounded_height = round(page_rect.height, 3)

            if rounded_width > rounded_height:
                orientation = "Ngang"
                longer_side = rounded_width
            else:
                orientation = "Doc"
                longer_side = rounded_height

            abs_diff = round(abs(rounded_width - rounded_height), 3)
            a_size = self.xac_dinh_kho_a(longer_side)
            size_count[a_size] += 1

            if a_size == "A3":
                a3_pages.append(str(page_index + 1))

            page_info = f"{stt}/{total_files}\t{pdf_name}_page_{page_index + 1}\t{rounded_width}\t{rounded_height}\t{orientation}\t{abs_diff}\t{a_size}"
            results.append(page_info)

        pdf_file.close()

        quy_a4 = (size_count["A0"] * 16 + size_count["A1"] * 8 + size_count["A2"] * 4 + size_count["A3"] * 2 +
                  size_count["A4"] + size_count["A5"] * 0.5 + size_count["A6"] * 0.25 + size_count["A7"] * 0.125)

        dema4_info = (
            f"{stt}/{total_files}\t{start_time}\t{os.path.abspath(pdf_path)}\t{total_pages}\t{size_count['A0']}\t{size_count['A1']}\t"
            f"{size_count['A2']}\t{size_count['A3']}\t{size_count['A4']}\t{size_count['A5']}\t"
            f"{size_count['A6']}\t{size_count['A7']}\t{','.join(a3_pages)}\t{quy_a4}"
        )
        results.append(dema4_info)
        self.result_signal.emit('\t'.join(dema4_info.split('\t')[:-2] + dema4_info.split('\t')[-1:]))  # Bỏ cột 'Vi tri A3' khi in ra giao diện

        return results, dema4_info, size_count

    def phan_tich_cac_file_pdf_trong_thu_muc(self):
        current_directory = os.path.basename(self.folder_path)
        chitiet_file_path = os.path.join(self.folder_path, f'{current_directory}_chitiet.txt')
        dema4_file_path = os.path.join(self.folder_path, f'{current_directory}_tonghop.txt')

        pdf_files = sorted([f for f in os.listdir(self.folder_path) if f.endswith('.pdf')])
        total_files = len(pdf_files)

        chitiet_buffer = []
        dema4_buffer = []

        chitiet_header = "STT\tTrang\tChieu rong\tChieu cao\tHuong trang\tRong-Cao\tKho giay"
        chitiet_buffer.append(chitiet_header)

        dema4_header = "STT\tThoi gian doc file\tDuong dan\tSo trang (len)\tA0\tA1\tA2\tA3\tA4\tA5\tA6\tA7\tVi tri A3\tQuy A4"
        dema4_buffer.append(dema4_header)
        self.result_signal.emit(dema4_header.replace("\tVi tri A3", ""))  # Bỏ cột 'Vi tri A3' khi in ra giao diện

        total_pages_a0 = 0
        total_pages_a1 = 0
        total_pages_a2 = 0
        total_pages_a3 = 0
        total_pages_a4 = 0
        total_pages_a5 = 0
        total_pages_a6 = 0
        total_pages_a7 = 0
        total_quy_a4 = 0

        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_to_pdf = {executor.submit(self.phan_tich_trang_pdf, os.path.join(self.folder_path, pdf_file), idx + 1, total_files): pdf_file for idx, pdf_file in enumerate(pdf_files)}

            for future in concurrent.futures.as_completed(future_to_pdf):
                pdf_file = future_to_pdf[future]
                try:
                    results, dema4_info, size_count = future.result()
                    chitiet_buffer.extend(results)
                    dema4_buffer.append(dema4_info)

                    total_pages_a0 += size_count["A0"]
                    total_pages_a1 += size_count["A1"]
                    total_pages_a2 += size_count["A2"]
                    total_pages_a3 += size_count["A3"]
                    total_pages_a4 += size_count["A4"]
                    total_pages_a5 += size_count["A5"]
                    total_pages_a6 += size_count["A6"]
                    total_pages_a7 += size_count["A7"]
                    total_quy_a4 += (size_count["A0"] * 16 + size_count["A1"] * 8 + size_count["A2"] * 4 + size_count["A3"] * 2 +
                                     size_count["A4"] + size_count["A5"] * 0.5 + size_count["A6"] * 0.25 + size_count["A7"] * 0.125)

                    if len(chitiet_buffer) >= 1000:
                        self.ghi_du_lieu(chitiet_buffer, chitiet_file_path)
                        chitiet_buffer = []

                    if len(dema4_buffer) >= 1000:
                        self.ghi_du_lieu(dema4_buffer, dema4_file_path)
                        dema4_buffer = []

                except Exception as exc:
                    self.result_signal.emit(f'{pdf_file} generated an exception: {exc}\n')

        if chitiet_buffer:
            self.ghi_du_lieu(chitiet_buffer, chitiet_file_path)

        if dema4_buffer:
            self.ghi_du_lieu(dema4_buffer, dema4_file_path)

        # Hiển thị thông tin tổng hợp trên giao diện
        summary = (
            f"\n==========================================\n"
            f"Số file pdf trong thư mục '{current_directory}': {total_files}\n"
            f"Tổng số trang quy ra A4: {total_quy_a4}\n"
            f"------------------------------------------\n"
            f"Trong đó, bao gồm:\n"
            f" - Tổng số trang A0: {total_pages_a0}\n"
            f" - Tổng số trang A1: {total_pages_a1}\n"
            f" - Tổng số trang A2: {total_pages_a2}\n"
            f" - Tổng số trang A3: {total_pages_a3}\n"
            f" - Tổng số trang A4: {total_pages_a4}\n"
            f" - Tổng số trang A5: {total_pages_a5}\n"
            f" - Tổng số trang A6: {total_pages_a6}\n"
            f" - Tổng số trang A7: {total_pages_a7}\n"
            f"==========================================\n"
        )
        self.summary_signal.emit(summary)

    def ghi_du_lieu(self, buffer, file_path):
        with open(file_path, 'a', encoding='utf-8') as file:
            file.write('\n'.join(buffer) + '\n')

class PDFAnalyzerApp(QWidget):
    def __init__(self):
        super().__init__()

        self.initUI()

    def initUI(self):
        self.setWindowTitle('Đếm trang A4')
        self.setGeometry(100, 100, 600, 400)
        self.center()  # Gọi phương thức để căn giữa cửa sổ

        layout = QVBoxLayout()

        hbox = QHBoxLayout()
        self.path_label = QLabel('Chọn thư mục:', self)
        hbox.addWidget(self.path_label)

        self.path_edit = QLineEdit(self)
        hbox.addWidget(self.path_edit)

        self.choose_btn = QPushButton('Chọn', self)
        self.choose_btn.setFixedWidth(80)
        self.choose_btn.clicked.connect(self.select_folder)
        hbox.addWidget(self.choose_btn)

        self.analyze_btn = QPushButton('Đếm', self)
        self.analyze_btn.setFixedWidth(80)
        self.analyze_btn.setStyleSheet("background-color: green; color: white;")  # Đổi màu nền và màu chữ
        self.analyze_btn.clicked.connect(self.analyze_pdfs)
        hbox.addWidget(self.analyze_btn)

        layout.addLayout(hbox)

        self.output_text = QTextEdit(self)
        self.output_text.setReadOnly(True)
        layout.addWidget(self.output_text)

        self.setLayout(layout)

    def center(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

    def select_folder(self):
        folder_path = QFileDialog.getExistingDirectory(self, 'Chọn thư mục')
        if folder_path:
            self.path_edit.setText(folder_path)

    def analyze_pdfs(self):
        folder_path = self.path_edit.text()
        if folder_path:
            # Xóa nội dung các tệp trước khi ghi
            current_directory = os.path.basename(folder_path)
            open(os.path.join(folder_path, f'{current_directory}_chitiet.txt'), 'w').close()
            open(os.path.join(folder_path, f'{current_directory}_tonghop.txt'), 'w').close()

            self.output_text.clear()

            self.worker = Worker(folder_path)
            self.worker.result_signal.connect(self.update_output)
            self.worker.summary_signal.connect(self.update_summary)

            self.thread = QtCore.QThread()
            self.worker.moveToThread(self.thread)
            self.thread.started.connect(self.worker.phan_tich_cac_file_pdf_trong_thu_muc)
            self.thread.start()

    def update_output(self, text):
        # Bỏ cột 'Vi tri A3' khi hiển thị ra giao diện
        columns = text.split('\t')
        if len(columns) == 13:  # Kiểm tra nếu có cột "Vi tri A3"
            text = '\t'.join(columns[:9] + columns[10:])  # Loại bỏ cột thứ 9 (Vị trí A3)
        self.output_text.append(text)

    def update_summary(self, summary):
        self.output_text.append(summary)
        self.thread.quit()
        self.thread.wait()

def main():
    import sys
    app = QApplication(sys.argv)
    ex = PDFAnalyzerApp()
    ex.show()
    sys.exit(app.exec_())

if __name__ == '__main__':
    main()
