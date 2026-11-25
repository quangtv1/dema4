"""Spike S2: tạo CA thử và chứng thư người ký thử (RSA 2048) bằng thư viện cryptography.

CHỈ DÙNG CHO THỬ NGHIỆM. Khoá bí mật ghi không mã hoá vào thư mục kết quả (không commit).
Cách dùng: python gen_pki.py <thư mục kết quả>
"""

import datetime
import pathlib
import sys

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID

out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
now = datetime.datetime.now(datetime.timezone.utc)


def name(common_name):
    return x509.Name(
        [
            x509.NameAttribute(NameOID.COUNTRY_NAME, "VN"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "Sao Mai - THU NGHIEM"),
            x509.NameAttribute(NameOID.COMMON_NAME, common_name),
        ]
    )


ca_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
ca_cert = (
    x509.CertificateBuilder()
    .subject_name(name("CA thử nghiệm Sao Mai (không dùng thật)"))
    .issuer_name(name("CA thử nghiệm Sao Mai (không dùng thật)"))
    .public_key(ca_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(now - datetime.timedelta(days=1))
    .not_valid_after(now + datetime.timedelta(days=365))
    .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
    .add_extension(
        x509.KeyUsage(
            digital_signature=False, content_commitment=False, key_encipherment=False,
            data_encipherment=False, key_agreement=False, key_cert_sign=True, crl_sign=True,
            encipher_only=False, decipher_only=False,
        ),
        critical=True,
    )
    .add_extension(x509.SubjectKeyIdentifier.from_public_key(ca_key.public_key()), critical=False)
    .sign(ca_key, hashes.SHA256())
)

signer_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
signer_cert = (
    x509.CertificateBuilder()
    .subject_name(name("Nguyễn Văn Thử Nghiệm"))
    .issuer_name(ca_cert.subject)
    .public_key(signer_key.public_key())
    .serial_number(x509.random_serial_number())
    .not_valid_before(now - datetime.timedelta(days=1))
    .not_valid_after(now + datetime.timedelta(days=90))
    .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
    .add_extension(
        x509.KeyUsage(
            digital_signature=True, content_commitment=True, key_encipherment=False,
            data_encipherment=False, key_agreement=False, key_cert_sign=False, crl_sign=False,
            encipher_only=False, decipher_only=False,
        ),
        critical=True,
    )
    .add_extension(
        x509.AuthorityKeyIdentifier.from_issuer_public_key(ca_key.public_key()), critical=False
    )
    .sign(ca_key, hashes.SHA256())
)

der = serialization.Encoding.DER
pem = serialization.Encoding.PEM
(out / "ca.cert.der").write_bytes(ca_cert.public_bytes(der))
(out / "ca.cert.pem").write_bytes(ca_cert.public_bytes(pem))
(out / "signer.cert.der").write_bytes(signer_cert.public_bytes(der))
(out / "signer.key.pk8.der").write_bytes(
    signer_key.private_bytes(der, serialization.PrivateFormat.PKCS8, serialization.NoEncryption())
)
print("Da tao CA thu va chung thu nguoi ky thu trong", out)
