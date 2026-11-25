"""Sinh bộ PDF mẫu cho test kiểm tra chữ ký (TD-57) vào fixtures/pdf/verify/.

Ký bằng pyHanko (độc lập với code ký của app) với khoá thử của spike S2 và máy chủ dấu thời gian thật của Ban
Cơ yếu (cần mạng). Khoá thử không nằm trong repo; file sinh ra chỉ chứa chứng thư thử công khai.

Cách dùng: python -I tools/sinh_mau_kiem_tra.py <thư mục khoá thử, mặc định spikes/s2-cms/out/pki>
"""

import io
import pathlib
import sys

sys.stdout.reconfigure(encoding="utf-8")

from pyhanko.keys import load_cert_from_pemder, load_private_key_from_pemder_data
from pyhanko.pdf_utils import generic
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter
from pyhanko.sign import fields, signers, timestamps
from pyhanko.sign.signers.pdf_signer import PdfTimeStamper
from pyhanko.sign.validation.dss import DocumentSecurityStore
from pyhanko_certvalidator.registry import SimpleCertificateStore

repo = pathlib.Path(__file__).resolve().parent.parent
pki = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else repo / "spikes/s2-cms/out/pki"
out = repo / "fixtures/pdf/verify"
out.mkdir(parents=True, exist_ok=True)
src = repo / "fixtures/pdf/table-xref.pdf"

cert = load_cert_from_pemder(str(pki / "signer.cert.der"))
ca = load_cert_from_pemder(str(pki / "ca.cert.der"))
key = load_private_key_from_pemder_data((pki / "signer.key.pk8.der").read_bytes(), passphrase=None)
tsa = timestamps.HTTPTimeStamper("http://tsa.ca.gov.vn/", timeout=30)


def signer(pss=False):
    return signers.SimpleSigner(
        signing_cert=cert, signing_key=key, cert_registry=SimpleCertificateStore.from_certs([ca]),
        prefer_pss=pss,
    )


def sign(data_in, path_out, name, *, stamp=True, pss=False, certify=None):
    meta = signers.PdfSignatureMetadata(
        field_name=name, subfilter=fields.SigSeedSubFilter.PADES, md_algorithm="sha256",
        certify=certify is not None, docmdp_permissions=certify or fields.MDPPerm.FILL_FORMS,
    )
    with open(data_in, "rb") as h:
        w = IncrementalPdfFileWriter(h, strict=False)
        with open(path_out, "wb") as o:
            signers.PdfSigner(meta, signer(pss), timestamper=tsa if stamp else None).sign_pdf(w, output=o)


def modify_page(data_in, path_out):
    """Bản cập nhật tăng dần chèn một dòng chữ vào trang 1 (sửa nội dung sau khi ký)."""
    with open(data_in, "rb") as h:
        w = IncrementalPdfFileWriter(h, strict=False)
        font = generic.DictionaryObject({
            generic.NameObject("/Type"): generic.NameObject("/Font"),
            generic.NameObject("/Subtype"): generic.NameObject("/Type1"),
            generic.NameObject("/BaseFont"): generic.NameObject("/Helvetica"),
        })
        ref = w.add_object(font)
        w.add_stream_to_page(
            0, generic.StreamObject(stream_data=b"BT /FX 30 Tf 60 400 Td (SO TIEN: 999.000.000) Tj ET"),
            resources=generic.DictionaryObject({generic.NameObject("/Font"): generic.DictionaryObject({generic.NameObject("/FX"): ref})}),
        )
        with open(path_out, "wb") as o:
            w.write(o)


def add_note(data_in, path_out):
    """Bản cập nhật thêm một chú thích (Text) vào trang 1."""
    with open(data_in, "rb") as h:
        w = IncrementalPdfFileWriter(h, strict=False)
        page_ref = w.root["/Pages"]["/Kids"][0]
        page = page_ref.get_object()
        annot = w.add_object(generic.DictionaryObject({
            generic.NameObject("/Type"): generic.NameObject("/Annot"),
            generic.NameObject("/Subtype"): generic.NameObject("/Text"),
            generic.NameObject("/Rect"): generic.ArrayObject([generic.NumberObject(n) for n in (50, 700, 70, 720)]),
            generic.NameObject("/Contents"): generic.TextStringObject("Ghi chú sau khi ký"),
            generic.NameObject("/P"): page_ref,
        }))
        annots = page.get("/Annots")
        if isinstance(annots, generic.IndirectObject):
            arr = annots.get_object()
            arr.append(annot)
            w.mark_update(annots)
        else:
            arr = generic.ArrayObject(list(annots or []))
            arr.append(annot)
            page[generic.NameObject("/Annots")] = arr
            w.update_container(page)
        with open(path_out, "wb") as o:
            w.write(o)


def add_dss(data_in, path_out):
    """Bản cập nhật chỉ thêm DSS (dữ liệu kiểm tra dài hạn), như Adobe "Add verification information"."""
    buf = io.BytesIO(pathlib.Path(data_in).read_bytes())
    DocumentSecurityStore.add_dss(buf, sig_contents=None, certs=[cert, ca], force_write=True)
    pathlib.Path(path_out).write_bytes(buf.getvalue())


def doc_timestamp(data_in, path_out):
    with open(data_in, "rb") as h:
        w = IncrementalPdfFileWriter(h, strict=False)
        with open(path_out, "wb") as o:
            PdfTimeStamper(tsa).timestamp_pdf(w, "sha256", output=o)


base = out / "ky-tsa.pdf"
sign(src, base, "ChuKy1")
modify_page(base, out / "_tmp-sua.pdf")
sign(out / "_tmp-sua.pdf", out / "sua-roi-ky-them.pdf", "ChuKy2")
(out / "_tmp-sua.pdf").unlink()
doc_timestamp(base, out / "dau-thoi-gian-tai-lieu.pdf")
add_note(base, out / "them-chu-thich.pdf")
add_dss(base, out / "them-dss.pdf")
sign(src, out / "_tmp-p1.pdf", "ChungNhan", certify=fields.MDPPerm.NO_CHANGES)
# pyHanko (và app) không cho ký thêm lên bản P=1, nên thêm một chú thích: vượt quyền P=1.
add_note(out / "_tmp-p1.pdf", out / "chung-nhan-p1-them-chu-thich.pdf")
(out / "_tmp-p1.pdf").unlink()
sign(src, out / "rsa-pss.pdf", "ChuKyPss", stamp=False, pss=True)
for p in sorted(out.glob("*.pdf")):
    print(p.name, p.stat().st_size)
