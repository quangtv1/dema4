//! Hằng số của ứng dụng.
//!
//! Mọi giá trị ở đây được đối chiếu trực tiếp với mã nguồn bản .NET 1.4.0
//! (`Kytl-main/ktl.be/ktl.be.shared/Constants/**` và các file ghi trong chú thích).
//! Giá trị mới của bản 1.5.0 (không có ở bản cũ) được ghi rõ "mới ở 1.5.0".
//! Không viết cứng các giá trị này trong code xử lý.

/// Thông tin chung của ứng dụng.
pub mod app {
    /// Tên thư mục dữ liệu trong `%APPDATA%`, `%LOCALAPPDATA%`, `%TEMP%` (`AppConstants.AppFolderName`).
    pub const APP_FOLDER_NAME: &str = "SaoMai-KySo";
    /// Kích thước cửa sổ mặc định (`ktl.be.desktop/Program.cs`).
    pub const WINDOW_WIDTH: u32 = 1280;
    pub const WINDOW_HEIGHT: u32 = 820;
    pub const WINDOW_MIN_WIDTH: u32 = 1024;
    pub const WINDOW_MIN_HEIGHT: u32 = 700;
    /// Múi giờ Việt Nam cố định UTC+07:00, không có giờ mùa hè.
    pub const VIETNAM_UTC_OFFSET_SECONDS: i32 = 7 * 3600;
}

/// Ký số và dấu thời gian (`SigningConstants.cs`, `PdfSigner.cs`, `PdfIncrementalSigner.cs`).
pub mod signing {
    pub const TSA_URL: &str = "http://tsa.ca.gov.vn/";
    pub const TSA_CONTENT_TYPE: &str = "application/timestamp-query";
    pub const TSA_REPLY_CONTENT_TYPE: &str = "application/timestamp-reply";
    pub const TSA_TIMEOUT_SECONDS: u64 = 180;
    /// Thời hạn hỏi giờ máy chủ TSA ở đầu lô ký (TD-54). Quá hạn hoặc lỗi mạng thì ký theo giờ máy tính.
    pub const TSA_PROBE_TIMEOUT_SECONDS: u64 = 15;
    /// Số yêu cầu TSA đồng thời tối đa (`MaxTsaConcurrency = MaxDegreeOfParallelism`).
    pub const MAX_TSA_CONCURRENCY: usize = 8;
    /// Độ dài nonce RFC 3161 tính bằng byte (`TimestampClient.cs`: `GetBytes(8)`).
    pub const TSA_NONCE_BYTES: usize = 8;

    pub const PDF_FILTER: &str = "Adobe.PPKLite";
    pub const PDF_SUB_FILTER: &str = "ETSI.CAdES.detached";

    /// Chỗ trống `/Contents` tính bằng byte (`CadesSigner.GetSignatureSizeAsync`: `32 * 1024`).
    pub const SIGNATURE_CONTENTS_BYTES: usize = 32 * 1024;
    /// Độ rộng cố định của mỗi số trong `/ByteRange` (`ByteRangeFieldWidth`).
    pub const BYTE_RANGE_FIELD_WIDTH: usize = 10;
    /// Tiền tố tên field chữ ký, nối với số object của field.
    pub const SIGNATURE_FIELD_NAME_PREFIX: &str = "SaoMai_Signature_";

    pub const DEFAULT_REASON: &str = "Ký số tài liệu";
    pub const DEFAULT_LOCATION: &str = "Việt Nam";

    /// Số luồng ký: `clamp(số CPU, MIN, MAX)`.
    pub const MIN_PARALLELISM: usize = 2;
    pub const MAX_PARALLELISM: usize = 8;

    /// Chu kỳ kiểm tra token còn cắm hay không.
    pub const TOKEN_MONITOR_INTERVAL_SECONDS: u64 = 2;
    /// Kích thước dữ liệu ngẫu nhiên dùng để ký thử trước lô.
    pub const PREFLIGHT_TEST_DATA_BYTES: usize = 32;

    /// Hậu tố file kết quả của bản 1.4.0: `<tên>_daky_<yyyyMMdd_HHmmss>.pdf`.
    /// Bản 1.5.0 không dùng để đặt tên nữa, chỉ giữ để nhận ra file do bản cũ tạo.
    pub const LEGACY_SIGNED_FILE_SUFFIX: &str = "_daky_";
}

/// Thư mục và tên file kết quả. Mới ở 1.5.0 (quyết định D4).
pub mod output {
    pub const DEFAULT_SUBFOLDER_NAME: &str = "DaKySo";
    pub const DEFAULT_SIGNED_SUFFIX: &str = "_signed";
    pub const DEFAULT_FAILED_SUFFIX: &str = "_loi";
    pub const TEMP_EXTENSION: &str = ".tmp";
}

/// OID dùng khi ký và xác minh (`SigningConstants.cs`, `SignatureConstants.cs`, `PdfSigner.cs`).
pub mod oid {
    pub const SHA256: &str = "2.16.840.1.101.3.4.2.1";
    pub const SIGNATURE_TIME_STAMP_TOKEN: &str = "1.2.840.113549.1.9.16.2.14";
    pub const SIGNING_CERTIFICATE_V2: &str = "1.2.840.113549.1.9.16.2.47";
    pub const SIGNING_TIME: &str = "1.2.840.113549.1.9.5";
    pub const EKU_TIME_STAMPING: &str = "1.3.6.1.5.5.7.3.8";
}

/// Chứng thư CA của Ban Cơ yếu được ghim (`SignatureConstants.cs`).
///
/// Giá trị ghim là SHA-256 của chứng thư ở dạng DER (không phải băm của file PEM).
pub mod ca {
    pub const ROOT_G1_FILE: &str = "rootca.crt";
    pub const ROOT_G1_SHA256: &str =
        "F7BC89054CEC7F12730D7B8CD3EB7955B163835087E5EB873EDFB584763B3F90";

    pub const SUB_CA_G1_FILE: &str = "cp.crt";
    pub const SUB_CA_G1_SHA256: &str =
        "75E969C07BA35C29C72BBC1BFB880F20F3891A619B00B77EB02CA6160736A1FE";

    /// Root G2 cũng là gốc của chuỗi TSA.
    pub const ROOT_G2_FILE: &str = "rootcag2.crt";
    pub const ROOT_G2_SHA256: &str =
        "F04BA4B459A7C9A1B971AD9B1CB833E695A80C79AEA63B174B66A70EDFE2FDB8";

    pub const DCS_CA_G2_FILE: &str = "dcscag2.crt";
    pub const DCS_CA_G2_SHA256: &str =
        "E4B402E3CB4CACC0A20D21FA49BCE76BEB4EB47BA7FFB7A8F71C554DBE561A81";

    pub const CP_G2_FILE: &str = "cpg2.crt";
    pub const CP_G2_SHA256: &str =
        "D8CAB7A4AEF92E6BB2509D7C61CEE7B44F3731B2D21074963530277B366FAE4B";
}

/// Xác minh chữ ký (`SignatureConstants.cs`).
pub mod verify {
    /// Một số phần mềm ghi giờ địa phương (UTC+7) nhưng đánh dấu là UTC trong `signing-time`.
    pub const SIGNER_LOCAL_TIME_QUIRK_HOURS: i64 = 7;
    pub const SIGNER_LOCAL_TIME_QUIRK_WINDOW_SECONDS: i64 = 3600;
    /// Dung sai đồng hồ giữa giờ tự khai và giờ TSA.
    pub const TSA_CLOCK_SKEW_GRACE_SECONDS: i64 = 60;
}

/// Khối chữ ký hiển thị trên trang (`SigningConstants.cs`).
pub mod appearance {
    /// Cỡ khối mặc định tính bằng điểm, trên trang tham chiếu A4.
    pub const WIDTH_PT: f64 = 170.0;
    pub const HEIGHT_PT: f64 = 30.0;
    /// Cỡ khối mặc định khi ảnh nằm trên / dưới chữ (kiểu "Ảnh + thông tin"): hẹp và cao hơn để ảnh đủ lớn.
    pub const STACKED_WIDTH_PT: f64 = 130.0;
    pub const STACKED_HEIGHT_PT: f64 = 66.0;
    pub const REFERENCE_PAGE_WIDTH_PT: f64 = 595.0;
    pub const REFERENCE_PAGE_HEIGHT_PT: f64 = 842.0;
    /// Lề an toàn giữ khối chữ ký nằm trong trang.
    pub const PAGE_MARGIN_PT: f64 = 8.0;

    pub const DEFAULT_FONT_FAMILY: &str = "Times New Roman";
    pub const FONT_FAMILIES: [&str; 5] = [
        "Times New Roman",
        "Arial",
        "Verdana",
        "Georgia",
        "Courier New",
    ];
    pub const DEFAULT_FONT_SIZE_PT: f64 = 10.0;
    pub const MIN_FONT_SIZE_PT: f64 = 6.0;
    pub const MAX_FONT_SIZE_PT: f64 = 24.0;
    pub const DEFAULT_TEXT_COLOR: &str = "#000000";
}

/// Ảnh dấu đỏ, chữ ký tươi (`StampImageConstants.cs`).
pub mod stamp_image {
    pub const ALLOWED_EXTENSIONS: [&str; 3] = [".png", ".jpg", ".jpeg"];
    pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

    pub const DARKNESS_DEFAULT: i32 = 140;
    pub const DARKNESS_MIN: i32 = 40;
    pub const DARKNESS_MAX: i32 = 250;
    pub const DARKNESS_BRIGHTNESS_FACTOR: f64 = 0.5;
    pub const DARKNESS_MIN_BRIGHTNESS: f64 = 0.5;

    pub const STROKE_DEFAULT: i32 = 100;
    pub const STROKE_MIN: i32 = 0;
    pub const STROKE_MAX: i32 = 200;
    /// Bán kính nét = `bề rộng ảnh × STROKE_RADIUS_RATIO × nét / 100`.
    pub const STROKE_RADIUS_RATIO: f64 = 0.35 / 158.0;
    pub const STROKE_DIRECTIONS: usize = 8;
}

/// Nhận diện token phần cứng theo tên provider (`SigningConstants.cs`).
/// So khớp không phân biệt hoa thường, theo kiểu "tên provider có chứa".
pub mod token {
    pub const HARDWARE_PROVIDER_MARKERS: [&str; 10] = [
        "smart card",
        "bit4id",
        "vgca",
        "token",
        "etoken",
        "safenet",
        "gemalto",
        "thales",
        "epass",
        "feitian",
    ];

    pub const SOFTWARE_PROVIDER_MARKERS: [&str; 5] = [
        "microsoft software key storage provider",
        "microsoft enhanced cryptographic provider",
        "microsoft base cryptographic provider",
        "microsoft strong cryptographic provider",
        "microsoft enhanced rsa and aes cryptographic provider",
    ];
}

/// Tạo văn bản (`DocumentMergeConstants.cs`).
pub mod merge {
    /// Mẫu trường trong template, dạng regex: `{ten_truong}`.
    pub const FIELD_PATTERN: &str = r"\{([^{}]+)\}";
    /// Định dạng ngày khi đọc ô kiểu ngày (kiểu .NET: `dd/MM/yyyy`).
    pub const CELL_DATE_FORMAT: &str = "dd/MM/yyyy";
    /// Định dạng thời gian trong tên thư mục kết quả (kiểu .NET: `yyyyMMdd_HHmmss`).
    pub const OUTPUT_TIME_FORMAT: &str = "yyyyMMdd_HHmmss";
    pub const FILE_NAME_MAX_LENGTH: usize = 100;

    pub const TEMP_FOLDER_NAME: &str = "tao-van-ban";
    pub const PREVIEW_FOLDER_NAME: &str = "xem-truoc";
    pub const PREVIEW_RETENTION_MINUTES: u64 = 60;

    /// Dấu phân tách trong danh sách trang tuỳ chọn, ví dụ `1,3,5-7`.
    pub const CUSTOM_PAGE_SEPARATOR: char = ',';

    pub const WORD_PROG_ID: &str = "Word.Application";
    /// `wdExportFormatPDF`.
    pub const WORD_EXPORT_FORMAT_PDF: i32 = 17;
    /// `wdDoNotSaveChanges`.
    pub const WORD_DO_NOT_SAVE_CHANGES: i32 = 0;
    /// `wdAlertsNone`.
    pub const WORD_ALERTS_NONE: i32 = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ca_pins_are_64_uppercase_hex_chars() {
        for pin in [
            ca::ROOT_G1_SHA256,
            ca::SUB_CA_G1_SHA256,
            ca::ROOT_G2_SHA256,
            ca::DCS_CA_G2_SHA256,
            ca::CP_G2_SHA256,
        ] {
            assert_eq!(pin.len(), 64);
            assert!(
                pin.chars()
                    .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c))
            );
        }
    }

    #[test]
    fn default_font_is_in_allowed_list() {
        assert!(appearance::FONT_FAMILIES.contains(&appearance::DEFAULT_FONT_FAMILY));
    }

    #[test]
    fn slider_defaults_are_inside_ranges() {
        use stamp_image::*;
        assert!((DARKNESS_MIN..=DARKNESS_MAX).contains(&DARKNESS_DEFAULT));
        assert!((STROKE_MIN..=STROKE_MAX).contains(&STROKE_DEFAULT));
        assert!(
            (appearance::MIN_FONT_SIZE_PT..=appearance::MAX_FONT_SIZE_PT)
                .contains(&appearance::DEFAULT_FONT_SIZE_PT)
        );
    }

    #[test]
    fn software_markers_do_not_overlap_hardware_markers() {
        // Bản cũ loại provider phần mềm trước rồi mới xét dấu hiệu phần cứng;
        // nếu hai danh sách chồng nhau thì thứ tự kiểm tra sẽ ảnh hưởng kết quả.
        for soft in token::SOFTWARE_PROVIDER_MARKERS {
            for hard in token::HARDWARE_PROVIDER_MARKERS {
                assert!(
                    !soft.contains(hard),
                    "{soft} chứa dấu hiệu phần cứng {hard}"
                );
            }
        }
    }
}
