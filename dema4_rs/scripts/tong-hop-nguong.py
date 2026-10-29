#!/usr/bin/env python3
"""Tổng hợp thống kê ngưỡng trắng gửi về từ sm-dema4 → đề xuất ngưỡng mặc định mới.

Đầu vào (1 hoặc nhiều đường dẫn):
  - thư mục / file `nguong_*.csv` tải từ nhóm Telegram (dòng đầu `# may=…;…`, sau đó `v_tu;v_den;trang;noi_dung`);
  - file CSV tải từ tab "nguong" của Google Sheet Monitor_dema4 (cột `histogram` = `trắng|nội dung` theo ô).

Ví dụ:  python3 tong-hop-nguong.py ~/Downloads/nguong/  nguong-sheet.csv  --t1 1.0 --t2 3.2

In: số lô, tổng trang, t1/t2 đề xuất (toàn bộ + theo máy), tỷ lệ trắng lọt / Trắng sai tại ngưỡng hiện tại.
Thuật toán đề xuất giống app (src/tach_vb/thong_ke_nguong.rs). Đổi mặc định vẫn do người quyết.
"""
import argparse
import csv
import glob
import os
import sys

SO_O = 101
NOI_DUNG_TOI_DA = 0.002
LOT_TOI_DA = 0.01
TRANG_TOI_THIEU = 50


def de_xuat(o):
    tong_trang = sum(t for t, _ in o)
    if tong_trang < TRANG_TOI_THIEU:
        return None
    ct = cn = t1 = 0
    for i, (tr, nd) in enumerate(o):
        ct += tr
        cn += nd
        if ct + cn > 0 and cn / (ct + cn) <= NOI_DUNG_TOI_DA:
            t1 = i + 1
    con_tren, t2 = tong_trang, len(o)
    for i, (tr, _) in enumerate(o):
        if con_tren / tong_trang <= LOT_TOI_DA:
            t2 = i
            break
        con_tren -= tr
    return min(t1 * 0.1, 10.0), min(max(t1, t2) * 0.1, 10.0)


def doc_csv_telegram(duong_dan):
    """Trả (máy, histogram) từ file nguong_*.csv."""
    may, o = "?", [[0, 0] for _ in range(SO_O)]
    with open(duong_dan, encoding="utf-8") as f:
        dau = f.readline().lstrip("#").strip()
        for cap in dau.split(";"):
            if cap.startswith("may="):
                may = cap[4:]
        for i, dong in enumerate(csv.DictReader(f, delimiter=";")):
            if i < SO_O:
                o[i][0] += int(dong["trang"])
                o[i][1] += int(dong["noi_dung"])
    return may, o


def doc_csv_sheet(duong_dan):
    """Trả danh sách (máy, histogram) từ CSV xuất Google Sheet."""
    kq = []
    with open(duong_dan, encoding="utf-8") as f:
        for dong in csv.DictReader(f):
            o = [[0, 0] for _ in range(SO_O)]
            for i, cap in enumerate(dong.get("histogram", "").split(",")[:SO_O]):
                if "|" in cap:
                    a, b = cap.split("|")
                    o[i] = [int(a), int(b)]
            kq.append((dong.get("may", "?"), o))
    return kq


def cong(a, b):
    for i in range(SO_O):
        a[i][0] += b[i][0]
        a[i][1] += b[i][1]


def danh_gia(o, t1, t2):
    """Tỷ lệ trắng lọt (trắng có v ≥ t2) và Trắng sai (chữ có v < t1) tại ngưỡng (%)."""
    o1, o2 = round(t1 * 10), round(t2 * 10)
    tong_trang = sum(t for t, _ in o) or 1
    duoi_t1 = sum(t + n for t, n in o[:o1]) or 1
    lot = sum(t for t, _ in o[o2:]) / tong_trang
    sai = sum(n for _, n in o[:o1]) / duoi_t1
    return lot, sai


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("duong_dan", nargs="+", help="thư mục / file CSV")
    ap.add_argument("--t1", type=float, default=1.0, help="ngưỡng trắng hiện tại (%%)")
    ap.add_argument("--t2", type=float, default=3.2, help="ngưỡng nghi trắng hiện tại (%%)")
    a = ap.parse_args()

    lo = []
    for p in a.duong_dan:
        tep = sorted(glob.glob(os.path.join(p, "*.csv"))) if os.path.isdir(p) else [p]
        for t in tep:
            with open(t, encoding="utf-8") as f:
                dau = f.readline()
            lo.extend([doc_csv_telegram(t)] if dau.startswith("#") else doc_csv_sheet(t))
    if not lo:
        sys.exit("Không có dữ liệu")

    tong = [[0, 0] for _ in range(SO_O)]
    theo_may = {}
    for may, o in lo:
        cong(tong, o)
        cong(theo_may.setdefault(may, [[0, 0] for _ in range(SO_O)]), o)

    def in_dong(ten, o):
        so_trang = sum(t + n for t, n in o)
        so_trang_trang = sum(t for t, _ in o)
        dx = de_xuat(o)
        dx_s = f"{dx[0]:.1f} / {dx[1]:.1f} %" if dx else "chưa đủ dữ liệu"
        lot, sai = danh_gia(o, a.t1, a.t2)
        print(f"{ten:<24} {so_trang:>8} trang ({so_trang_trang} trắng)  đề xuất {dx_s:<18} "
              f"tại {a.t1}/{a.t2}: trắng lọt {lot:.2%}, Trắng sai {sai:.2%}")

    print(f"{len(lo)} lô tách\n")
    in_dong("TOÀN BỘ", tong)
    for may in sorted(theo_may):
        in_dong(may, theo_may[may])


if __name__ == "__main__":
    main()
