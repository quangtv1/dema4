"""Spike S4: so kết quả đọc của calamine với chuẩn ClosedXML, từng ô.

Cách dùng: python compare.py <closedxml.json> <calamine.json>
In ra từng file: tiêu đề có khớp không, số dòng có khớp không, và mọi ô lệch kèm kiểu dữ liệu, định dạng
mà mỗi bên nhìn thấy. Mã thoát 0 khi không lệch ô nào.
"""

import json
import sys


def load(path):
    with open(path, encoding="utf-8-sig") as handle:
        return json.load(handle)


def main():
    reference = load(sys.argv[1])
    candidate = load(sys.argv[2])
    print(f"chuan: {reference['reader']} ({reference['culture']}) | thu: {candidate['reader']}")

    total_cells = 0
    total_mismatches = 0
    for ref_file, got_file in zip(reference["files"], candidate["files"]):
        ref_headers = [(h["columnNumber"], h["name"]) for h in ref_file["headers"]]
        got_headers = [(h["columnNumber"], h["name"]) for h in got_file["headers"]]
        ref_rows = [r["rowNumber"] for r in ref_file["rows"]]
        got_rows = [r["rowNumber"] for r in got_file["rows"]]
        names = {str(number): name for number, name in ref_headers}

        mismatches = []
        cells = 0
        got_by_row = {r["rowNumber"]: r for r in got_file["rows"]}
        for ref_row in ref_file["rows"]:
            got_row = got_by_row.get(ref_row["rowNumber"])
            for column, expected in ref_row["values"].items():
                cells += 1
                actual = None if got_row is None else got_row["values"].get(column)
                if actual != expected:
                    ref_meta = ref_row["meta"][column]
                    got_meta = {} if got_row is None else got_row["meta"].get(column, {})
                    mismatches.append(
                        (ref_row["rowNumber"], names.get(column, column), expected, actual, ref_meta, got_meta)
                    )

        total_cells += cells
        total_mismatches += len(mismatches)
        print(
            f"== {ref_file['file']}: sheet {'khop' if ref_file['sheet'] == got_file['sheet'] else 'LECH'}"
            f" | tieu de {'khop' if ref_headers == got_headers else 'LECH'}"
            f" | dong {'khop' if ref_rows == got_rows else f'LECH {ref_rows} / {got_rows}'}"
            f" | o lech {len(mismatches)}/{cells}"
        )
        for row, name, expected, actual, ref_meta, got_meta in mismatches:
            print(
                f"   dong {row} {name:<15} chuan={expected!r:<26} calamine={actual!r:<26}"
                f" | chuan: {ref_meta.get('type')}, {ref_meta.get('format') or ref_meta.get('numFmtId')}"
                f" | calamine: {got_meta.get('type')}, {got_meta.get('format')}"
            )

    print(f"TONG: {total_cells - total_mismatches}/{total_cells} o khop, {total_mismatches} o lech")
    return 0 if total_mismatches == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
