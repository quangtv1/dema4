//! Mã lỗi nghiệp vụ và thông báo tiếng Việt.
//!
//! Mã 1025–1048 giữ nguyên của bản .NET 1.4.0 (`ErrorCodes.cs`) để log và tài liệu hỗ trợ
//! không bị lệch. Mã 1049 trở đi là mới ở 1.5.0 (Phụ lục B của kế hoạch).
//! Các số 1036, 1038, 1039, 1041 bản cũ bỏ trống; không dùng lại.

use serde::{Serialize, Serializer};

macro_rules! error_codes {
    ($( $(#[$meta:meta])* $name:ident = $code:literal => $message:literal, )+) => {
        /// Mã lỗi nghiệp vụ.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        pub enum ErrorCode {
            $( $(#[$meta])* $name = $code, )+
        }

        impl ErrorCode {
            /// Toàn bộ mã lỗi, theo thứ tự khai báo.
            pub const ALL: &'static [ErrorCode] = &[ $( ErrorCode::$name, )+ ];

            /// Giá trị số của mã lỗi.
            pub const fn as_u32(self) -> u32 {
                self as u32
            }

            /// Tên định danh của mã lỗi (dùng trong log).
            pub const fn name(self) -> &'static str {
                match self { $( ErrorCode::$name => stringify!($name), )+ }
            }

            /// Thông báo tiếng Việt mặc định.
            pub const fn default_message(self) -> &'static str {
                match self { $( ErrorCode::$name => $message, )+ }
            }

            /// Tra mã lỗi theo giá trị số.
            pub const fn from_u32(code: u32) -> Option<Self> {
                match code {
                    $( $code => Some(ErrorCode::$name), )+
                    _ => None,
                }
            }
        }
    };
}

error_codes! {
    System = 1 => "Lỗi hệ thống",
    BadRequest = 400 => "Yêu cầu không hợp lệ",
    NotFound = 404 => "Không tìm thấy",
    InternalError = 500 => "Lỗi không xác định",

    TimestampInvalid = 1025 => "Dấu thời gian/chữ ký số của file không hợp lệ",
    SignatureInvalid = 1026 => "Nội dung file đã bị sửa sau khi ký số",
    CertificateNotFound = 1027 => "Không đọc được chứng thư số nào trên máy",
    CertificateNotSignable = 1028 => "Không có chứng thư số nào dùng để ký được",
    SignSourceNotFound = 1029 => "Không tìm thấy file hoặc thư mục cần ký",
    SignNoPdfFound = 1030 => "Không có file PDF nào để ký",
    SignTokenInvalid = 1031 => "Chứng thư số không dùng để ký được",
    SignFailed = 1032 => "Ký số file thất bại",
    SignPdfEncrypted = 1033 => "File PDF được mã hóa nên không ký số được",
    SignPdfLocked = 1034 => "File PDF bị khóa không cho ký thêm (/DocMDP)",
    SignPdfStructureUnsupported = 1035 => "Cấu trúc file PDF không hỗ trợ ký nối bản",
    SignPositionInvalid = 1037 => "Vị trí đặt chữ ký không hợp lệ",
    StampImageInvalid = 1040 => "Ảnh dấu đỏ hoặc chữ ký tươi không hợp lệ",
    MergeTemplateInvalid = 1042 => "File mẫu không hợp lệ",
    MergeDataInvalid = 1043 => "File dữ liệu không hợp lệ",
    MergeMappingInvalid = 1044 => "Cấu hình ghép trường không hợp lệ",
    MergeConverterUnavailable = 1045 => "Máy chưa cài Microsoft Word để xuất PDF",
    MergeFailed = 1046 => "Tạo văn bản thất bại",
    MergeStampInvalid = 1047 => "Cấu hình ảnh dấu đỏ hoặc chữ ký tươi không hợp lệ",
    /// Bản 1.5.0 không còn tải file lên; giữ chỗ để không dùng lại số này.
    FileUploadInvalid = 1048 => "File kéo thả vào ứng dụng không hợp lệ",

    TokenRemoved = 1049 => "Đã rút USB token khi đang ký nên dừng tiến trình",
    OutputExists = 1050 => "File kết quả đã tồn tại",
    OutputFolderInvalid = 1051 => "Không ghi được vào thư mục lưu kết quả",
    RevocationUnknown = 1052 => "Chưa kiểm tra được tình trạng thu hồi chứng thư",
    CertificateRevoked = 1053 => "Chứng thư số đã bị thu hồi",
    DocxConvertFailed = 1054 => "Không chuyển được DOCX sang PDF để ký",
    HtmlRenderFailed = 1055 => "Không in được mẫu HTML ra PDF",
    TableSheetNotFound = 1056 => "Không tìm thấy sheet trong file Excel",
    ConfigInvalid = 1057 => "File cấu hình hỏng, đã khôi phục mặc định",
    UpdateFailed = 1058 => "Không kiểm tra hoặc tải được bản cập nhật",
    DriverInstallFailed = 1059 => "Cài driver token không thành công",
    PinIncorrect = 1060 => "Mã PIN của token không đúng",
    PinBlocked = 1061 => "Token đã bị khoá do nhập sai mã PIN quá số lần cho phép",
    NotifyFailed = 1062 => "Không gửi được thông báo Telegram",
    SignatureUnsupported = 1063 => "Chữ ký dùng thuật toán chưa được hỗ trợ nên không kết luận được",
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.as_u32())
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.as_u32(), self.name())
    }
}

/// Lỗi nghiệp vụ: mã lỗi kèm thông báo tiếng Việt hiển thị thẳng cho người dùng.
///
/// Thông báo phải nói rõ nguyên nhân và cách xử lý. Không đưa PIN hay nội dung văn bản vào đây.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[error("{message}")]
pub struct SmksError {
    pub code: ErrorCode,
    pub message: String,
}

impl SmksError {
    /// Lỗi với thông báo cụ thể.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Lỗi dùng thông báo mặc định của mã lỗi.
    pub fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_owned(),
        }
    }
}

impl From<ErrorCode> for SmksError {
    fn from(code: ErrorCode) -> Self {
        Self::from_code(code)
    }
}

/// Kiểu kết quả dùng trong các crate lõi.
pub type Result<T> = std::result::Result<T, SmksError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Mã của bản 1.4.0 (`ErrorCodes.cs`), chép tay để phát hiện nếu ai đó đổi số.
    const LEGACY_CODES: &[(ErrorCode, u32)] = &[
        (ErrorCode::System, 1),
        (ErrorCode::BadRequest, 400),
        (ErrorCode::NotFound, 404),
        (ErrorCode::InternalError, 500),
        (ErrorCode::TimestampInvalid, 1025),
        (ErrorCode::SignatureInvalid, 1026),
        (ErrorCode::CertificateNotFound, 1027),
        (ErrorCode::CertificateNotSignable, 1028),
        (ErrorCode::SignSourceNotFound, 1029),
        (ErrorCode::SignNoPdfFound, 1030),
        (ErrorCode::SignTokenInvalid, 1031),
        (ErrorCode::SignFailed, 1032),
        (ErrorCode::SignPdfEncrypted, 1033),
        (ErrorCode::SignPdfLocked, 1034),
        (ErrorCode::SignPdfStructureUnsupported, 1035),
        (ErrorCode::SignPositionInvalid, 1037),
        (ErrorCode::StampImageInvalid, 1040),
        (ErrorCode::MergeTemplateInvalid, 1042),
        (ErrorCode::MergeDataInvalid, 1043),
        (ErrorCode::MergeMappingInvalid, 1044),
        (ErrorCode::MergeConverterUnavailable, 1045),
        (ErrorCode::MergeFailed, 1046),
        (ErrorCode::MergeStampInvalid, 1047),
        (ErrorCode::FileUploadInvalid, 1048),
    ];

    #[test]
    fn legacy_codes_keep_their_numbers() {
        for (code, number) in LEGACY_CODES {
            assert_eq!(code.as_u32(), *number, "{}", code.name());
        }
    }

    #[test]
    fn new_codes_continue_from_1049_without_gaps() {
        let mut new_codes: Vec<u32> = ErrorCode::ALL
            .iter()
            .map(|c| c.as_u32())
            .filter(|n| *n > 1048)
            .collect();
        new_codes.sort_unstable();
        assert_eq!(new_codes, (1049..=1063).collect::<Vec<u32>>());
    }

    #[test]
    fn reserved_legacy_gaps_are_not_reused() {
        for gap in [1036, 1038, 1039, 1041] {
            assert_eq!(ErrorCode::from_u32(gap), None);
        }
    }

    #[test]
    fn from_u32_round_trips_and_messages_are_not_empty() {
        for code in ErrorCode::ALL {
            assert_eq!(ErrorCode::from_u32(code.as_u32()), Some(*code));
            assert!(!code.default_message().trim().is_empty());
        }
        assert_eq!(ErrorCode::from_u32(0), None);
    }

    #[test]
    fn error_displays_message_and_serializes_numeric_code() {
        let err = SmksError::new(
            ErrorCode::SignPdfEncrypted,
            "File PDF được mã hóa (/Encrypt).",
        );
        assert_eq!(err.to_string(), "File PDF được mã hóa (/Encrypt).");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], 1033);
        assert_eq!(json["message"], "File PDF được mã hóa (/Encrypt).");

        let default = SmksError::from(ErrorCode::TokenRemoved);
        assert_eq!(default.message, ErrorCode::TokenRemoved.default_message());
    }
}
