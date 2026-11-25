"""Spike S2: kiểm chữ ký PDF bằng pyHanko (bộ kiểm độc lập với code ký).

Gốc tin cậy: CA thử của spike (cho người ký) và Root G2 của Ban Cơ yếu (cho dấu thời gian).
Không tải gì từ mạng; không kiểm thu hồi.

Cách dùng: python validate.py <ca thử .pem> <thư mục certs của repo> <file.pdf>...
Mã thoát 0 khi mọi chữ ký của mọi file đều đạt.
"""

import pathlib
import sys

from pyhanko.keys import load_cert_from_pemder
from pyhanko.pdf_utils.reader import PdfFileReader
from pyhanko.sign.validation import validate_pdf_signature
from pyhanko_certvalidator import ValidationContext

test_ca = load_cert_from_pemder(sys.argv[1])
certs = pathlib.Path(sys.argv[2])
root_g2 = load_cert_from_pemder(str(certs / "rootcag2.crt"))
intermediates = [load_cert_from_pemder(str(certs / n)) for n in ("dcscag2.crt", "cpg2.crt")]


def context():
    return ValidationContext(
        trust_roots=[test_ca, root_g2], other_certs=intermediates, allow_fetching=False
    )


def attr_names(attrs):
    return sorted(a["type"].native for a in attrs) if attrs else []


def validate_all(handle, strict):
    reader = PdfFileReader(handle, strict=strict)
    signatures = reader.embedded_signatures
    return [
        (sig, validate_pdf_signature(sig, context(), ts_validation_context=context()))
        for sig in signatures
    ]


all_ok = True
for path in sys.argv[3:]:
    print("=====", path)
    with open(path, "rb") as handle:
        # pyHanko ở chế độ chặt (mặc định) không kiểm chữ ký trong file hybrid-reference
        # (bảng xref kèm /XRefStm, kiểu Word xuất). Khi gặp thì thử lại ở chế độ không chặt và ghi rõ.
        try:
            results = validate_all(handle, strict=True)
            print("che do doc: chat (strict)")
        except Exception as error:  # noqa: BLE001 - in ra đúng lý do pyHanko từ chối
            if "hybrid" not in str(error):
                raise
            print("che do doc: KHONG chat, vi file la hybrid-reference:", error)
            handle.seek(0)
            results = validate_all(handle, strict=False)
        print("so chu ky:", len(results))
        if not results:
            all_ok = False
        for index, (sig, status) in enumerate(results, start=1):
            info = sig.signer_info
            signed = attr_names(info["signed_attrs"])
            unsigned = attr_names(info["unsigned_attrs"])
            ts = status.timestamp_validity
            checks = {
                "noi dung nguyen ven (intact)": status.intact,
                "chu ky dung (valid)": status.valid,
                "nguoi ky tin cay (trusted)": status.trusted,
                # Chữ ký cuối phải phủ hết file; chữ ký trước đó chỉ được có thay đổi hợp lệ phía sau.
                "pham vi hop le": (status.coverage.name == "ENTIRE_FILE") if index == len(results)
                else status.modification_level.name != "OTHER",
                "SubFilter ETSI.CAdES.detached": str(sig.sig_object["/SubFilter"]) == "/ETSI.CAdES.detached",
                "co signingCertificateV2": "signing_certificate_v2" in signed,
                "KHONG co signing-time": "signing_time" not in signed,
                "co dau thoi gian": ts is not None,
                "dau thoi gian dung (valid)": bool(ts and ts.valid),
                "TSA tin cay ve Root G2 (trusted)": bool(ts and ts.trusted),
            }
            print(f"--- chu ky {index}: field {sig.field_name}")
            print("  nguoi ky        :", status.signing_cert.subject.human_friendly)
            print("  thuat toan      :", status.pkcs7_signature_mechanism, "/", status.md_algorithm)
            print("  signed attrs    :", signed)
            print("  unsigned attrs  :", unsigned)
            print("  /M tu khai      :", status.signer_reported_dt)
            if ts:
                print("  gio TSA         :", ts.timestamp)
                print("  TSA             :", ts.signing_cert.subject.human_friendly)
            print("  pham vi / thay doi:", status.coverage.name, "/", status.modification_level.name)
            print("  bottom_line     :", status.bottom_line)
            for name, ok in checks.items():
                print(f"  [{'DAT' if ok else 'KHONG'}] {name}")
            if not (all(checks.values()) and status.bottom_line):
                all_ok = False

print("KET QUA CHUNG:", "DAT" if all_ok else "KHONG DAT")
sys.exit(0 if all_ok else 1)
