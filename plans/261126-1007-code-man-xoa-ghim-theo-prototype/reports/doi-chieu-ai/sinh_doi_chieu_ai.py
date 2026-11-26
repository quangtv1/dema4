"""Sinh dữ liệu đối chiếu AI nhận diện layout (XoaGhim Python ↔ dema4 Rust).

Dùng đúng lớp `YOLODocLayNetONNXDetector` của XoaGhim (core/layout_detector.py) — không chép lại thuật toán.

Chạy:
    python3 sinh_doi_chieu_ai.py <layout_detector.py> <mo_hinh.onnx> <thu_muc_pdf> <thu_muc_ra>

Mỗi trang mẫu ghi vào <thu_muc_ra>/<ten>/:
    vao.png   ảnh trang RGB (Rust đọc lại đúng ảnh này)
    x.npy     tensor letterbox 1×3×1024×1024 float32 (_preprocess)
    y.npy     đầu ra thô 1×15×21504 float32 (session.run)
    hop.json  các hộp sau _postprocess (mọi lớp): x1 y1 x2 y2 (float, toạ độ trang), tin_cay, lop
Kèm <thu_muc_ra>/mau.json liệt kê các trang. Thư mục ra lớn (~15 MB / trang) — không đưa lên git.
"""

import importlib.util
import json
import os
import sys
import time

import cv2
import fitz  # PyMuPDF
import numpy as np


def nap_lop(duong_dan):
    spec = importlib.util.spec_from_file_location("layout_detector", duong_dan)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m.YOLODocLayNetONNXDetector


def ve_trang(pdf, so, dpi):
    doc = fitz.open(pdf)
    pix = doc[so].get_pixmap(dpi=dpi, colorspace=fitz.csRGB, alpha=False)
    rgb = np.frombuffer(pix.samples, dtype=np.uint8).reshape(pix.height, pix.width, 3).copy()
    return rgb


def main():
    ld, mo_hinh, thu_muc_pdf, ra = sys.argv[1:5]
    Lop = nap_lop(ld)
    dt = Lop()
    dt._get_model_path = lambda: mo_hinh
    assert dt._load_model(), dt.get_load_error()
    ten_vao = dt.session.get_inputs()[0].name

    # (tên, file pdf, trang từ 0, dpi, xoay ngang)
    mau = [
        ("hs1_t1_300", "HS_001.pdf", 0, 300, False),
        ("hs2_t2_200", "HS_002.pdf", 1, 200, False),
        ("hs3_t1_150_ngang", "HS_003.pdf", 0, 150, True),
    ]
    ds = []
    for ten, pdf, so, dpi, ngang in mau:
        rgb = ve_trang(os.path.join(thu_muc_pdf, pdf), so, dpi)
        if ngang:
            rgb = np.ascontiguousarray(np.rot90(rgb))
        d = os.path.join(ra, ten)
        os.makedirs(d, exist_ok=True)
        bgr = rgb[:, :, ::-1].copy()
        cv2.imwrite(os.path.join(d, "vao.png"), bgr)
        bgr = cv2.imread(os.path.join(d, "vao.png"))  # đúng ảnh Rust đọc lại
        x, scale, pad = dt._preprocess(bgr)
        t0 = time.time()
        y = dt.session.run(None, {ten_vao: x})[0]
        ms = (time.time() - t0) * 1000
        hop = dt._postprocess(y, scale, pad, bgr.shape[:2])
        np.save(os.path.join(d, "x.npy"), x.astype(np.float32))
        np.save(os.path.join(d, "y.npy"), y.astype(np.float32))
        with open(os.path.join(d, "hop.json"), "w") as f:
            json.dump([{"x1": float(a), "y1": float(b), "x2": float(c), "y2": float(e), "tin_cay": float(p), "lop": int(k)}
                       for a, b, c, e, p, k in hop], f, indent=1)
        ds.append({"ten": ten, "rong": int(bgr.shape[1]), "cao": int(bgr.shape[0]), "so_hop": len(hop), "ms_python": round(ms, 1)})
        print(ten, bgr.shape[1], "x", bgr.shape[0], "hộp:", len(hop), f"{ms:.0f} ms")
    with open(os.path.join(ra, "mau.json"), "w") as f:
        json.dump(ds, f, indent=1, ensure_ascii=False)


if __name__ == "__main__":
    main()
