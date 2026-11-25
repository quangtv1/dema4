# S1: Ký bằng USB token từ Rust

- **Kết luận:** CHƯA CHẠY (18/11/2026).
- **Chặn bởi:** B-01 (không build được code gọi Windows API), B-02 (không có token và middleware).

## Mục tiêu

Ký được 32 byte ngẫu nhiên bằng khoá trên token **bit4id** và **SafeNet** từ một chương trình Rust; hộp PIN
của middleware hiện phía trên cửa sổ ứng dụng, không bị chìm.

## Phương án kỹ thuật

1. Mở `CurrentUser\My` bằng `CertOpenStore`, tìm chứng thư theo thumbprint.
2. `CryptAcquireCertificatePrivateKey` với `CRYPT_ACQUIRE_ALLOW_NCRYPT_KEY_FLAG`; xem `dwKeySpec` để biết đi
   đường CNG (`CERT_NCRYPT_KEY_SPEC`) hay CAPI.
3. CNG: đặt thuộc tính `"HWND Handle"`, gọi `NCryptSignHash` với `BCRYPT_PKCS1_PADDING_INFO { "SHA256" }`
   (RSA) hoặc không padding (ECDSA).
4. CAPI: `CryptSetProvParam(NULL, PP_CLIENT_HWND)`, `CryptCreateHash(CALG_SHA_256)`,
   `CryptSetHashParam(HP_HASHVAL)`, `CryptSignHashW`, đảo thứ tự byte.
5. Xác minh chữ ký vừa tạo bằng khoá công khai trong chứng thư (crate `rsa`, `p256`).
6. Lặp lại với cửa sổ Tauri thật để kiểm vị trí hộp PIN.

Tham chiếu bản cũ: `StoreSigningKey.cs`, `ktl.be.desktop/Program.cs` (xem `MIGRATION_MAP.md` 3.2).

## Câu hỏi phải trả lời

- Mỗi loại token đi đường CNG hay CAPI? Tên provider thực tế có khớp Phụ lục A.6 không?
- Ký liên tiếp nhiều lần có bị hỏi PIN lại không? Sau bao lâu phiên hết hạn?
- Rút token giữa chừng thì API trả mã lỗi nào?
- ECDSA: chữ ký trả về dạng `r‖s` đúng như dự đoán không?

## Code thử nghiệm, kết quả, bằng chứng

Chưa có.

## Rủi ro còn lại

Toàn bộ R1 còn nguyên cho tới khi spike này chạy.
