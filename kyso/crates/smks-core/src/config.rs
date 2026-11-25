//! Cấu hình người dùng (`%APPDATA%\SaoMai-KySo\config.json`, kế hoạch mục 11, F-11).
//!
//! Module này chỉ có kiểu dữ liệu, giá trị mặc định, làm sạch giá trị và đọc/ghi chuỗi JSON. Đọc/ghi file do
//! ứng dụng làm (ghi `.tmp` rồi đổi tên). Mọi trường đều có mặc định nên file thiếu trường vẫn đọc được; file có
//! `version` mới hơn vẫn đọc phần hiểu được.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::consts::{appearance as ap, output as out, signing as k, stamp_image as si};
use crate::{ErrorCode, Result, SmksError};

/// Phiên bản cấu trúc hiện tại. Đổi cấu trúc thì tăng số này và thêm bước ở [`upgrade`].
/// 2: mẫu chữ ký mặc định theo prototype (dưới phải, trang cuối), xem [`upgrade`].
/// 3: vị trí tự đặt thành danh sách khung (`positions`, giá trị của `filePositions`), thêm lượt ký nháy.
pub const CURRENT_VERSION: u32 = 3;

/// 1 mm theo điểm PDF.
pub const PT_PER_MM: f64 = 72.0 / 25.4;

/// Kiểu hiển thị chữ ký.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SignatureMode {
    /// Chữ ký ẩn.
    Hidden,
    /// Chỉ chữ "Ký bởi / Ngày ký" (mặc định, như bản cũ).
    #[default]
    Text,
    /// Ảnh (dấu hoặc chữ ký tươi) bên trái, chữ bên phải.
    ImageAndText,
    /// Chỉ ảnh, phủ khung (như `EmbedSignatureInImage` của bản cũ).
    Image,
}

/// Vị trí của ảnh so với chữ trong kiểu "Ảnh + thông tin".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImageLayout {
    /// Ảnh bên trái, chữ bên phải (như prototype và các bản trước).
    #[default]
    Left,
    /// Chữ bên trái, ảnh bên phải.
    Right,
    /// Ảnh ở trên, chữ ở dưới.
    Top,
    /// Chữ ở trên, ảnh ở dưới.
    Bottom,
}

impl ImageLayout {
    /// Ảnh và chữ xếp chồng theo chiều dọc.
    pub fn is_stacked(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// Chia khối "Ảnh + thông tin" giữa ảnh và chữ (TD-51). Tính theo % của khối theo chiều xếp (trái / phải: bề ngang,
/// trên / dưới: chiều cao) nên giữ đúng tỉ lệ khi đổi cỡ khung. `None` là tự động như trước.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ImageSplit {
    /// Phần dành cho ảnh, [`IMAGE_SHARE_MIN`]–[`IMAGE_SHARE_MAX`] %.
    pub share: Option<u8>,
    /// Khoảng cách giữa ảnh và chữ, 0–[`IMAGE_GAP_MAX`] %.
    pub gap: Option<u8>,
    /// Kiểu "Tự do" (TD-52): ô của ảnh và ô của chữ do người dùng kéo thả; có cả hai thì bỏ qua `layout`, `share`,
    /// `gap`.
    pub free: Option<(PartBox, PartBox)>,
}

/// Một ô bên trong khối chữ ký, theo tỉ lệ 0–1 của khối, gốc **trên trái** (như giao diện).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct PartBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Cạnh nhỏ nhất của một ô bên trong khối (tỉ lệ khối).
pub const PART_BOX_MIN: f64 = 0.03;

impl PartBox {
    /// Ô nằm gọn trong khối, cạnh không nhỏ hơn [`PART_BOX_MIN`]; số không hợp lệ thì `None`.
    pub fn clean(self) -> Option<Self> {
        if ![self.x, self.y, self.w, self.h]
            .iter()
            .all(|v| v.is_finite())
        {
            return None;
        }
        let w = self.w.clamp(PART_BOX_MIN, 1.0);
        let h = self.h.clamp(PART_BOX_MIN, 1.0);
        Some(Self {
            x: self.x.clamp(0.0, 1.0 - w),
            y: self.y.clamp(0.0, 1.0 - h),
            w,
            h,
        })
    }
}

pub const IMAGE_SHARE_MIN: u8 = 10;
pub const IMAGE_SHARE_MAX: u8 = 90;
pub const IMAGE_GAP_MAX: u8 = 30;

impl ImageSplit {
    /// Phần ảnh (0–1) đã giới hạn trong khoảng cho phép.
    pub fn share_ratio(self) -> Option<f64> {
        self.share
            .map(|s| f64::from(s.clamp(IMAGE_SHARE_MIN, IMAGE_SHARE_MAX)) / 100.0)
    }

    /// Khoảng cách (0–1) đã giới hạn.
    pub fn gap_ratio(self) -> Option<f64> {
        self.gap.map(|g| f64::from(g.min(IMAGE_GAP_MAX)) / 100.0)
    }
}

/// Vị trí theo tỉ lệ 0–1 so với khung hiển thị của trang, gốc ở **góc trên trái** (như giao diện và bản cũ).
/// `page` đánh số từ 1; `0` là trang cuối của từng file.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub page: u32,
    pub x_ratio: f64,
    pub y_ratio: f64,
    pub width_ratio: f64,
    pub height_ratio: f64,
}

impl Placement {
    /// Số trang thật trong file có `page_count` trang: `0` là trang cuối, vượt quá thì trang cuối (như bản cũ).
    pub fn page_in(&self, page_count: u32) -> u32 {
        if self.page == 0 {
            page_count.max(1)
        } else {
            self.page.clamp(1, page_count.max(1))
        }
    }

    fn is_valid(&self) -> bool {
        [
            self.x_ratio,
            self.y_ratio,
            self.width_ratio,
            self.height_ratio,
        ]
        .iter()
        .all(|v| v.is_finite())
            && self.width_ratio > 0.0
            && self.height_ratio > 0.0
    }

    fn clamped(self) -> Self {
        let width_ratio = self.width_ratio.clamp(0.01, 1.0);
        let height_ratio = self.height_ratio.clamp(0.005, 1.0);
        Self {
            page: self.page,
            x_ratio: self.x_ratio.clamp(0.0, 1.0 - width_ratio),
            y_ratio: self.y_ratio.clamp(0.0, 1.0 - height_ratio),
            width_ratio,
            height_ratio,
        }
    }
}

/// Loại chữ ký của một lượt ký. Mỗi lượt ký ra đúng **một** chữ ký (một chứng thư) trên mỗi văn bản, hiện ở một
/// hay nhiều khung; muốn có chữ ký nháy và chữ ký chính của hai người thì ký hai lượt (lượt sau ký chồng).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SignKind {
    /// Chữ ký chính: khung theo mẫu khổ giấy (hoặc tự đặt).
    #[default]
    Main,
    /// Ký nháy: mặc định một khung ở mọi trang, góc dưới đối diện chữ ký chính.
    Initial,
}

/// Số khung tự đặt tối đa của một danh sách.
pub const MAX_BOXES: usize = 500;

/// Làm sạch danh sách khung: bỏ khung không hợp lệ, ép vào trang, bỏ khung trùng, tối đa [`MAX_BOXES`].
fn clean_boxes(list: Vec<Placement>) -> Vec<Placement> {
    let mut out: Vec<Placement> = Vec::new();
    for p in list
        .into_iter()
        .filter(Placement::is_valid)
        .map(Placement::clamped)
    {
        if !out.contains(&p) && out.len() < MAX_BOXES {
            out.push(p);
        }
    }
    out
}

/// Khổ giấy của mẫu chữ ký.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Paper {
    A1,
    A2,
    A3,
    A4,
    A5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Trang đặt chữ ký của mẫu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PagePick {
    /// Trang đầu (như bản cũ).
    #[default]
    First,
    Last,
    /// Mọi trang (ký nháy): một chữ ký, mỗi trang một khối hiển thị.
    All,
}

/// Điểm neo của khối chữ ký: trên/dưới × trái/giữa/phải.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Tl,
    Tc,
    #[default]
    Tr,
    Bl,
    Bc,
    Br,
}

impl Anchor {
    pub fn is_bottom(self) -> bool {
        matches!(self, Self::Bl | Self::Bc | Self::Br)
    }

    /// 0 = trái, 1 = giữa, 2 = phải.
    pub fn column(self) -> u8 {
        match self {
            Self::Tl | Self::Bl => 0,
            Self::Tc | Self::Bc => 1,
            Self::Tr | Self::Br => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MarginMm {
    /// Cách mép trái / phải (theo điểm neo).
    pub x: f64,
    /// Cách mép trên / dưới.
    pub y: f64,
}

impl Default for MarginMm {
    fn default() -> Self {
        // Lề của mẫu A4 trong prototype (`PROFILES_DEF`).
        Self { x: 15.0, y: 20.0 }
    }
}

/// Mẫu chữ ký theo khổ giấy (CH-01…CH-03): khi ký, mỗi file dùng mẫu khớp khổ trang đầu của nó.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SignatureProfile {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub paper: Paper,
    pub orientation: Orientation,
    pub page: PagePick,
    pub anchor: Anchor,
    pub margin_mm: MarginMm,
    /// Cỡ khối so với cỡ chuẩn 170 × 30 pt (%), 40–200.
    pub scale: u32,
}

impl Default for SignatureProfile {
    fn default() -> Self {
        Self {
            id: "a4".into(),
            name: "Khổ A4".into(),
            is_default: true,
            paper: Paper::A4,
            orientation: Orientation::Portrait,
            page: PagePick::Last,
            anchor: Anchor::Br,
            margin_mm: MarginMm::default(),
            scale: 100,
        }
    }
}

pub const SCALE_MIN: u32 = 40;
pub const SCALE_MAX: u32 = 200;
pub const MARGIN_X_MAX_MM: f64 = 100.0;
/// Số vị trí riêng từng file tối đa giữ trong cấu hình.
pub const MAX_FILE_POSITIONS: usize = 5000;
pub const MARGIN_Y_MAX_MM: f64 = 150.0;

/// Mẫu mặc định: mọi khổ đặt chữ ký góc trên phải trang đầu như bản 1.4.0; cỡ tăng theo khổ.
pub fn default_profiles() -> Vec<SignatureProfile> {
    // Theo `PROFILES_DEF` của prototype (L621–626): mọi khổ đặt dưới phải, trang cuối; lề và tỉ lệ theo khổ.
    let make = |id: &str, paper, orientation, (x, y), scale, is_default| SignatureProfile {
        id: id.into(),
        name: format!("Khổ {paper:?}"),
        is_default,
        paper,
        orientation,
        page: PagePick::Last,
        anchor: Anchor::Br,
        margin_mm: MarginMm { x, y },
        scale,
    };
    vec![
        make(
            "a4",
            Paper::A4,
            Orientation::Portrait,
            (15.0, 20.0),
            100,
            true,
        ),
        make(
            "a5",
            Paper::A5,
            Orientation::Portrait,
            (10.0, 12.0),
            70,
            false,
        ),
        make(
            "a3",
            Paper::A3,
            Orientation::Landscape,
            (20.0, 20.0),
            130,
            false,
        ),
        make(
            "a2",
            Paper::A2,
            Orientation::Landscape,
            (25.0, 25.0),
            160,
            false,
        ),
        make(
            "a1",
            Paper::A1,
            Orientation::Landscape,
            (30.0, 30.0),
            200,
            false,
        ),
    ]
}

/// Mẫu mặc định của cấu hình phiên bản 1 (trên phải, trang đầu, lề 3 mm như bản 1.4.0), để [`upgrade`] nhận ra
/// mẫu người dùng chưa sửa.
fn v1_default_profiles() -> Vec<SignatureProfile> {
    default_profiles()
        .into_iter()
        .map(|p| SignatureProfile {
            page: PagePick::First,
            anchor: Anchor::Tr,
            margin_mm: MarginMm { x: 3.0, y: 3.0 },
            ..p
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FontConfig {
    pub family: String,
    pub size: f64,
    pub color: String,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: ap::DEFAULT_FONT_FAMILY.into(),
            size: ap::DEFAULT_FONT_SIZE_PT,
            color: ap::DEFAULT_TEXT_COLOR.into(),
        }
    }
}

/// Ảnh trong khối chữ ký là con dấu hay chữ ký tay (chữ ký tay có xử lý nét và màu mực).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImageKind {
    #[default]
    Seal,
    WetSignature,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageConfig {
    /// Độ đậm 40–250.
    pub darkness: i32,
    /// Độ dày nét chữ ký tay 0–200.
    pub stroke: i32,
    /// Màu mực chữ ký tay `#RRGGBB`; không có thì giữ màu ảnh.
    pub ink_color: Option<String>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            darkness: si::DARKNESS_DEFAULT,
            stroke: si::STROKE_DEFAULT,
            ink_color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SignatureConfig {
    pub display: SignatureMode,
    pub reason: String,
    pub location: String,
    pub image_path: Option<String>,
    pub image_kind: ImageKind,
    /// Ảnh nằm ở đâu so với chữ (kiểu "Ảnh + thông tin").
    pub image_layout: ImageLayout,
    /// Phần khối dành cho ảnh (%), `None` = tự động (TD-51).
    pub image_share: Option<u8>,
    /// Khoảng cách ảnh – chữ (% khối), `None` = tự động (TD-51).
    pub image_gap: Option<u8>,
    /// Kiểu "Tự do" (TD-52): ô ảnh và ô chữ kéo thả trong khối; chỉ dùng khi có cả hai.
    pub image_box: Option<PartBox>,
    pub text_box: Option<PartBox>,
    pub font: FontConfig,
    pub image: ImageConfig,
    pub profiles: Vec<SignatureProfile>,
    /// Loại chữ ký của lượt ký sắp tới (màn Ký số).
    pub kind: SignKind,
    /// Khung chữ ký chính tự đặt (kéo, thêm, xoá trên màn Ký số), áp cho cả lô; rỗng thì theo mẫu khổ giấy
    /// (K-13). Một lượt ký vẫn là một chữ ký, mỗi khung là một chỗ hiện chữ ký đó.
    pub positions: Vec<Placement>,
    /// Khung ký nháy tự đặt, áp cho cả lô; rỗng thì mặc định mọi trang.
    pub initial_positions: Vec<Placement>,
    /// Khung chữ ký chính riêng từng file (đường dẫn tuyệt đối → khung), đặt ở màn Vị trí chữ ký chế độ "Từng
    /// file" (bản cũ: `fileOverrides`) hoặc thêm / xoá khung trên màn Ký số. Chỉ áp khi `per_file` bật.
    pub file_positions: BTreeMap<String, Vec<Placement>>,
    /// Khung ký nháy riêng từng file (thêm / xoá khung trên màn Ký số). Chỉ áp khi `per_file` bật.
    pub file_initial_positions: BTreeMap<String, Vec<Placement>>,
    /// Kiểu hiển thị riêng từng file (đường dẫn tuyệt đối → kiểu), đặt ở màn Vị trí chữ ký chế độ "Từng file".
    /// Chỉ áp khi `per_file` bật; file không có trong danh sách dùng `display`.
    pub file_displays: BTreeMap<String, SignatureMode>,
    /// Chế độ "Từng file" của màn Vị trí chữ ký. Tắt thì vẫn giữ `file_positions`, `file_displays` để bật lại
    /// không mất.
    pub per_file: bool,
    /// Văn bản "chưa gán chữ ký" của chữ ký chính (TD-56): đã xoá hết khung, bỏ qua khi ký số (đường dẫn tuyệt đối).
    pub skip_files: Vec<String>,
    /// Như `skip_files`, cho lượt ký nháy.
    pub initial_skip_files: Vec<String>,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            display: SignatureMode::Text,
            reason: k::DEFAULT_REASON.into(),
            location: k::DEFAULT_LOCATION.into(),
            image_path: None,
            image_kind: ImageKind::Seal,
            image_layout: ImageLayout::Left,
            image_share: None,
            image_gap: None,
            image_box: None,
            text_box: None,
            font: FontConfig::default(),
            image: ImageConfig::default(),
            profiles: default_profiles(),
            kind: SignKind::Main,
            positions: Vec::new(),
            initial_positions: Vec::new(),
            file_positions: BTreeMap::new(),
            file_initial_positions: BTreeMap::new(),
            file_displays: BTreeMap::new(),
            per_file: false,
            skip_files: Vec::new(),
            initial_skip_files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    /// Thư mục con cạnh file gốc (mặc định `DaKySo`).
    #[default]
    Subfolder,
    /// Một thư mục cố định cho mọi file.
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputConfig {
    pub mode: OutputMode,
    pub subfolder_name: String,
    pub fixed_folder: Option<String>,
    pub suffix: String,
    /// Trùng tên thì ghi đè; tắt thì thêm " (1)", " (2)"…
    pub overwrite: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            mode: OutputMode::Subfolder,
            subfolder_name: out::DEFAULT_SUBFOLDER_NAME.into(),
            fixed_folder: None,
            suffix: out::DEFAULT_SIGNED_SUFFIX.into(),
            overwrite: false,
        }
    }
}

/// Tab "USB Token & chứng thư số" (CH-07).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TokenConfig {
    /// Thumbprint (chữ hoa) các chứng thư người dùng đã xoá khỏi danh sách của ứng dụng. Chứng thư vẫn còn trong
    /// kho Windows; chỉ không hiện ở hộp Cấu hình và hộp Xác nhận ký.
    pub hidden_certs: Vec<String>,
}

/// Số chứng thư ẩn tối đa giữ trong cấu hình.
pub const MAX_HIDDEN_CERTS: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub signature: SignatureConfig,
    pub output: OutputConfig,
    pub token: TokenConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            signature: SignatureConfig::default(),
            output: OutputConfig::default(),
            token: TokenConfig::default(),
        }
    }
}

/// Ký tự không dùng được trong tên file / thư mục Windows.
fn has_bad_name_chars(s: &str) -> bool {
    s.chars().any(|c| {
        c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
    })
}

/// `#RRGGBB`.
pub fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl AppConfig {
    /// Đưa mọi giá trị về khoảng hợp lệ. Trả về các thay đổi đã làm (để ghi log / báo người dùng).
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut notes = Vec::new();
        // Thumbprint: 40 ký tự hex (SHA-1), chữ hoa, không trùng.
        let mut seen = std::collections::BTreeSet::new();
        self.token.hidden_certs = std::mem::take(&mut self.token.hidden_certs)
            .into_iter()
            .map(|t| t.trim().to_ascii_uppercase())
            .filter(|t| t.len() == 40 && t.chars().all(|c| c.is_ascii_hexdigit()))
            .filter(|t| seen.insert(t.clone()))
            .take(MAX_HIDDEN_CERTS)
            .collect();
        let s = &mut self.signature;
        if !ap::FONT_FAMILIES.contains(&s.font.family.as_str()) {
            notes.push(format!(
                "Font \"{}\" không hỗ trợ, dùng {}.",
                s.font.family,
                ap::DEFAULT_FONT_FAMILY
            ));
            s.font.family = ap::DEFAULT_FONT_FAMILY.into();
        }
        if !s.font.size.is_finite() {
            s.font.size = ap::DEFAULT_FONT_SIZE_PT;
        }
        s.font.size = s
            .font
            .size
            .clamp(ap::MIN_FONT_SIZE_PT, ap::MAX_FONT_SIZE_PT);
        if !is_hex_color(&s.font.color) {
            notes.push("Màu chữ không hợp lệ, dùng màu đen.".into());
            s.font.color = ap::DEFAULT_TEXT_COLOR.into();
        }
        s.image.darkness = s.image.darkness.clamp(si::DARKNESS_MIN, si::DARKNESS_MAX);
        s.image.stroke = s.image.stroke.clamp(si::STROKE_MIN, si::STROKE_MAX);
        if s.image
            .ink_color
            .as_deref()
            .is_some_and(|c| !is_hex_color(c))
        {
            s.image.ink_color = None;
        }
        s.image_share = s
            .image_share
            .map(|v| v.clamp(IMAGE_SHARE_MIN, IMAGE_SHARE_MAX));
        s.image_gap = s.image_gap.map(|v| v.min(IMAGE_GAP_MAX));
        s.image_box = s.image_box.and_then(PartBox::clean);
        s.text_box = s.text_box.and_then(PartBox::clean);
        if s.image_box.is_none() || s.text_box.is_none() {
            s.image_box = None;
            s.text_box = None;
        }
        s.reason = s.reason.trim().chars().take(200).collect();
        s.location = s.location.trim().chars().take(200).collect();
        if s.image_path.as_deref().is_some_and(|p| p.trim().is_empty()) {
            s.image_path = None;
        }
        // Kiểu có ảnh mà chưa có ảnh thì về "Chỉ thông tin".
        if s.image_path.is_none()
            && matches!(
                s.display,
                SignatureMode::Image | SignatureMode::ImageAndText
            )
        {
            s.display = SignatureMode::Text;
        }
        s.positions = clean_boxes(std::mem::take(&mut s.positions));
        s.initial_positions = clean_boxes(std::mem::take(&mut s.initial_positions));
        let clean_map =
            |map: BTreeMap<String, Vec<Placement>>| -> BTreeMap<String, Vec<Placement>> {
                map.into_iter()
                    .filter(|(path, _)| !path.trim().is_empty())
                    .map(|(path, list)| (path, clean_boxes(list)))
                    .filter(|(_, list)| !list.is_empty())
                    .take(MAX_FILE_POSITIONS)
                    .collect()
            };
        let before = s.file_positions.len() + s.file_initial_positions.len();
        s.file_positions = clean_map(std::mem::take(&mut s.file_positions));
        s.file_initial_positions = clean_map(std::mem::take(&mut s.file_initial_positions));
        let after = s.file_positions.len() + s.file_initial_positions.len();
        if after != before {
            notes.push(format!(
                "Bỏ {} vị trí riêng từng file không hợp lệ.",
                before - after
            ));
        }
        // Văn bản chưa gán chữ ký: bỏ đường dẫn rỗng, trùng; giới hạn như vị trí riêng.
        let clean_skips = |list: Vec<String>| -> Vec<String> {
            let mut seen = std::collections::BTreeSet::new();
            list.into_iter()
                .filter(|p| !p.trim().is_empty() && seen.insert(p.clone()))
                .take(MAX_FILE_POSITIONS)
                .collect()
        };
        s.skip_files = clean_skips(std::mem::take(&mut s.skip_files));
        s.initial_skip_files = clean_skips(std::mem::take(&mut s.initial_skip_files));
        // Kiểu riêng từng file: bỏ khoá rỗng; chưa có ảnh thì kiểu có ảnh về "Chỉ thông tin" như `display`.
        let before = s.file_displays.len();
        let no_image = s.image_path.is_none();
        s.file_displays = std::mem::take(&mut s.file_displays)
            .into_iter()
            .filter(|(path, _)| !path.trim().is_empty())
            .map(|(path, m)| {
                let image = matches!(m, SignatureMode::Image | SignatureMode::ImageAndText);
                (
                    path,
                    if no_image && image {
                        SignatureMode::Text
                    } else {
                        m
                    },
                )
            })
            .take(MAX_FILE_POSITIONS)
            .collect();
        if s.file_displays.len() != before {
            notes.push(format!(
                "Bỏ {} kiểu hiển thị riêng từng file không hợp lệ.",
                before - s.file_displays.len()
            ));
        }
        sanitize_profiles(&mut s.profiles, &mut notes);

        let o = &mut self.output;
        let sub = o.subfolder_name.trim();
        if sub.is_empty() || has_bad_name_chars(sub) || sub == "." || sub == ".." {
            notes.push(format!(
                "Tên thư mục con không hợp lệ, dùng {}.",
                out::DEFAULT_SUBFOLDER_NAME
            ));
            o.subfolder_name = out::DEFAULT_SUBFOLDER_NAME.into();
        } else {
            o.subfolder_name = sub.to_owned();
        }
        if has_bad_name_chars(&o.suffix) {
            notes.push(format!(
                "Hậu tố có ký tự không dùng được trong tên file, dùng {}.",
                out::DEFAULT_SIGNED_SUFFIX
            ));
            o.suffix = out::DEFAULT_SIGNED_SUFFIX.into();
        }
        if o.fixed_folder
            .as_deref()
            .is_some_and(|f| f.trim().is_empty())
        {
            o.fixed_folder = None;
        }
        if o.mode == OutputMode::Fixed && o.fixed_folder.is_none() {
            notes.push("Chưa chọn thư mục lưu cố định, dùng thư mục con cạnh file gốc.".into());
            o.mode = OutputMode::Subfolder;
        }
        self.version = self.version.max(CURRENT_VERSION);
        notes
    }

    /// Mẫu chữ ký cho trang có khổ `paper` (xem [`pick_profile`]).
    pub fn profile_for(&self, paper: Option<Paper>, landscape: bool) -> SignatureProfile {
        pick_profile(&self.signature.profiles, paper, landscape)
    }

    /// Đọc chuỗi JSON: nâng cấp từ bản cũ, rồi làm sạch giá trị. JSON hỏng trả lỗi 1057.
    pub fn from_json(text: &str) -> Result<(Self, Vec<String>)> {
        let value: Value = serde_json::from_str(text).map_err(|e| invalid(&e.to_string()))?;
        if !value.is_object() {
            return Err(invalid("không phải đối tượng JSON"));
        }
        let mut config: Self =
            serde_json::from_value(upgrade(value)).map_err(|e| invalid(&e.to_string()))?;
        let notes = config.sanitize();
        Ok((config, notes))
    }

    /// JSON có thụt lề để người dùng đọc được.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            SmksError::new(
                ErrorCode::InternalError,
                format!("Không ghi được cấu hình: {e}"),
            )
        })
    }
}

/// Mẫu cho trang có khổ `paper` (không nhận diện được thì `None`): khớp khổ và hướng, rồi khớp khổ, cuối cùng là
/// mẫu mặc định. Danh sách rỗng thì mẫu A4 mặc định.
pub fn pick_profile(
    profiles: &[SignatureProfile],
    paper: Option<Paper>,
    landscape: bool,
) -> SignatureProfile {
    let orientation = if landscape {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    };
    paper
        .and_then(|p| {
            profiles
                .iter()
                .find(|x| x.paper == p && x.orientation == orientation)
                .or_else(|| profiles.iter().find(|x| x.paper == p))
        })
        .or_else(|| profiles.iter().find(|x| x.is_default))
        .or_else(|| profiles.first())
        .cloned()
        .unwrap_or_default()
}

fn invalid(detail: &str) -> SmksError {
    SmksError::new(
        ErrorCode::ConfigInvalid,
        format!("{} ({detail}).", ErrorCode::ConfigInvalid.default_message()),
    )
}

fn sanitize_profiles(profiles: &mut Vec<SignatureProfile>, notes: &mut Vec<String>) {
    if profiles.is_empty() {
        *profiles = default_profiles();
        return;
    }
    let mut seen = std::collections::HashSet::new();
    for (i, p) in profiles.iter_mut().enumerate() {
        p.name = p.name.trim().chars().take(60).collect();
        if p.name.is_empty() {
            p.name = format!("Khổ {:?}", p.paper);
        }
        if p.id.trim().is_empty() || !seen.insert(p.id.clone()) {
            p.id = format!("mau-{}", i + 1);
            seen.insert(p.id.clone());
        }
        p.scale = p.scale.clamp(SCALE_MIN, SCALE_MAX);
        let fix = |v: f64, max: f64| {
            if v.is_finite() {
                v.clamp(0.0, max)
            } else {
                3.0
            }
        };
        p.margin_mm.x = fix(p.margin_mm.x, MARGIN_X_MAX_MM);
        p.margin_mm.y = fix(p.margin_mm.y, MARGIN_Y_MAX_MM);
    }
    let defaults = profiles.iter().filter(|p| p.is_default).count();
    if defaults != 1 {
        if defaults > 1 {
            notes.push("Có nhiều mẫu mặc định, giữ mẫu đầu tiên.".into());
        }
        let first = profiles.iter().position(|p| p.is_default).unwrap_or(0);
        for (i, p) in profiles.iter_mut().enumerate() {
            p.is_default = i == first;
        }
    }
}

/// Nâng cấp JSON từ phiên bản cũ lên [`CURRENT_VERSION`].
///
/// - 1 → 2: mẫu chữ ký mặc định đổi theo prototype. Mẫu nào còn đúng y mẫu mặc định cũ (người dùng chưa sửa) thì
///   thay bằng mẫu mặc định mới cùng mã; mẫu đã sửa giữ nguyên.
/// - 2 → 3: vị trí tự đặt một khung `position` thành danh sách `positions`; mỗi giá trị của `filePositions` thành
///   danh sách một khung.
pub fn upgrade(mut value: Value) -> Value {
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version < 3
        && let Some(sig) = value.get_mut("signature").and_then(Value::as_object_mut)
    {
        if let Some(old) = sig.remove("position")
            && !sig.contains_key("positions")
        {
            let list = if old.is_object() {
                vec![old]
            } else {
                Vec::new()
            };
            sig.insert("positions".into(), Value::Array(list));
        }
        if let Some(map) = sig.get_mut("filePositions").and_then(Value::as_object_mut) {
            for v in map.values_mut() {
                if v.is_object() {
                    *v = Value::Array(vec![v.take()]);
                }
            }
        }
    }
    if version < 2
        && let Some(list) = value
            .pointer_mut("/signature/profiles")
            .and_then(Value::as_array_mut)
    {
        let old = v1_default_profiles();
        let new = default_profiles();
        for item in list.iter_mut() {
            let same_as_old = serde_json::from_value::<SignatureProfile>(item.clone())
                .ok()
                .and_then(|p| old.iter().position(|o| *o == p));
            if let Some(i) = same_as_old
                && let Ok(v) = serde_json::to_value(&new[i])
            {
                *item = v;
            }
        }
    }
    if version < u64::from(CURRENT_VERSION)
        && let Some(obj) = value.as_object_mut()
    {
        obj.insert("version".into(), Value::from(CURRENT_VERSION));
    }
    value
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips() {
        let c = AppConfig::default();
        let (back, notes) = AppConfig::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back, c);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(c.signature.profiles.len(), 5);
        assert_eq!(
            c.signature.profiles.iter().filter(|p| p.is_default).count(),
            1
        );
    }

    #[test]
    fn missing_fields_use_defaults_and_version_is_upgraded() {
        let (c, _) = AppConfig::from_json(r#"{"signature":{"reason":"Phê duyệt"}}"#).unwrap();
        assert_eq!(c.version, CURRENT_VERSION);
        assert_eq!(c.signature.reason, "Phê duyệt");
        assert_eq!(c.signature.location, k::DEFAULT_LOCATION);
        assert_eq!(c.output, OutputConfig::default());
    }

    #[test]
    fn defaults_follow_prototype_profiles() {
        let p = default_profiles();
        let a4 = &p[0];
        assert!(a4.is_default && a4.paper == Paper::A4);
        assert_eq!(
            (a4.anchor, a4.page, a4.margin_mm, a4.scale),
            (
                Anchor::Br,
                PagePick::Last,
                MarginMm { x: 15.0, y: 20.0 },
                100
            )
        );
        let rows: Vec<_> = p
            .iter()
            .map(|x| {
                (
                    x.paper,
                    x.orientation,
                    x.margin_mm.x,
                    x.margin_mm.y,
                    x.scale,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (Paper::A4, Orientation::Portrait, 15.0, 20.0, 100),
                (Paper::A5, Orientation::Portrait, 10.0, 12.0, 70),
                (Paper::A3, Orientation::Landscape, 20.0, 20.0, 130),
                (Paper::A2, Orientation::Landscape, 25.0, 25.0, 160),
                (Paper::A1, Orientation::Landscape, 30.0, 30.0, 200),
            ]
        );
        assert!(
            p.iter()
                .all(|x| x.anchor == Anchor::Br && x.page == PagePick::Last)
        );
    }

    #[test]
    fn v1_untouched_profiles_upgrade_to_new_defaults() {
        // Cấu hình phiên bản 1: A4, A3 còn y mẫu cũ; A5 người dùng đã đổi tỉ lệ; thêm một mẫu tự tạo.
        let mut old = v1_default_profiles();
        old[1].scale = 90;
        old.push(SignatureProfile {
            id: "mau-1".into(),
            name: "Mẫu riêng".into(),
            is_default: false,
            ..old[0].clone()
        });
        let text =
            serde_json::json!({ "version": 1, "signature": { "profiles": old } }).to_string();
        let (c, _) = AppConfig::from_json(&text).unwrap();
        let p = &c.signature.profiles;
        let new = default_profiles();
        assert_eq!(c.version, CURRENT_VERSION);
        assert_eq!(p[0], new[0], "A4 chưa sửa → mặc định mới");
        assert_eq!(p[2], new[2], "A3 chưa sửa → mặc định mới");
        assert_eq!(p[1], old[1], "A5 đã sửa → giữ nguyên");
        assert_eq!(p[5].id, "mau-1");
        assert_eq!(p[5].anchor, Anchor::Tr, "mẫu tự tạo giữ nguyên");
        // Đã ở phiên bản 2: không đổi gì nữa.
        let text =
            serde_json::json!({ "version": 2, "signature": { "profiles": v1_default_profiles() } })
                .to_string();
        let (c, _) = AppConfig::from_json(&text).unwrap();
        assert_eq!(c.signature.profiles, v1_default_profiles());
    }

    #[test]
    fn v2_single_positions_become_lists() {
        let text = r##"{
          "version": 2,
          "signature": {
            "position": {"page": 3, "xRatio": 0.1, "yRatio": 0.2, "widthRatio": 0.3, "heightRatio": 0.05},
            "filePositions": {"D:\\a.pdf": {"page": 1, "xRatio": 0.5, "yRatio": 0.5, "widthRatio": 0.2, "heightRatio": 0.05}},
            "perFile": true
          }
        }"##;
        let (c, _) = AppConfig::from_json(text).unwrap();
        let s = &c.signature;
        assert_eq!(c.version, CURRENT_VERSION);
        assert_eq!(s.kind, SignKind::Main);
        assert_eq!(s.positions.len(), 1);
        assert_eq!((s.positions[0].page, s.positions[0].x_ratio), (3, 0.1));
        assert!(s.initial_positions.is_empty());
        assert_eq!(s.file_positions[r"D:\a.pdf"].len(), 1);
        assert_eq!(s.file_positions[r"D:\a.pdf"][0].x_ratio, 0.5);
        // Không có vị trí tự đặt (null): danh sách rỗng, theo mẫu.
        let (c, _) =
            AppConfig::from_json(r#"{"version": 2, "signature": {"position": null}}"#).unwrap();
        assert!(c.signature.positions.is_empty());
        // Phiên bản 3 ghi ra rồi đọc lại giữ nguyên.
        let mut c = AppConfig::default();
        c.signature.kind = SignKind::Initial;
        c.signature.initial_positions = vec![Placement {
            page: 2,
            x_ratio: 0.05,
            y_ratio: 0.9,
            width_ratio: 0.2,
            height_ratio: 0.05,
        }];
        let (back, notes) = AppConfig::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back, c);
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn broken_json_is_error_1057() {
        let e = AppConfig::from_json("{ hỏng").unwrap_err();
        assert_eq!(e.code, ErrorCode::ConfigInvalid);
        assert!(AppConfig::from_json("[1,2]").is_err());
    }

    /// Văn bản chưa gán chữ ký (TD-56): bỏ rỗng, trùng; JSON camelCase; cấu hình cũ không có trường thì rỗng.
    #[test]
    fn sanitize_skip_files() {
        let mut c = AppConfig::default();
        c.signature.skip_files = vec![
            r"D:.pdf".into(),
            " ".into(),
            r"D:.pdf".into(),
            r"D:.pdf".into(),
        ];
        c.signature.initial_skip_files = vec![String::new()];
        c.sanitize();
        assert_eq!(c.signature.skip_files, [r"D:.pdf", r"D:.pdf"]);
        assert!(c.signature.initial_skip_files.is_empty());
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["signature"]["skipFiles"][1], r"D:.pdf");
        assert!(json["signature"]["initialSkipFiles"].is_array());
        let old: SignatureConfig = serde_json::from_str(r#"{"display":"text"}"#).unwrap();
        assert!(old.skip_files.is_empty() && old.initial_skip_files.is_empty());
    }

    /// Kiểu "Tự do" (TD-52): ô ra ngoài khối bị kéo vào; chỉ có một ô thì bỏ cả hai.
    #[test]
    fn sanitize_free_boxes() {
        let mut c = AppConfig::default();
        c.signature.image_box = Some(PartBox {
            x: 0.8,
            y: 0.0,
            w: 0.5,
            h: 1.0,
        });
        c.signature.text_box = Some(PartBox {
            x: 0.0,
            y: 0.0,
            w: 0.4,
            h: 0.5,
        });
        c.sanitize();
        assert_eq!(
            c.signature.image_box,
            Some(PartBox {
                x: 0.5,
                y: 0.0,
                w: 0.5,
                h: 1.0
            })
        );
        c.signature.text_box = None;
        c.sanitize();
        assert_eq!((c.signature.image_box, c.signature.text_box), (None, None));
        let json = serde_json::to_value(&c.signature).unwrap();
        assert!(
            json.get("imageBox").is_some() && json.get("textBox").is_some(),
            "{json}"
        );
    }

    #[test]
    fn sanitize_clamps_and_fixes() {
        let text = r##"{
          "signature": {
            "display": "imageAndText",
            "font": {"family": "Comic Sans", "size": 99, "color": "đỏ"},
            "image": {"darkness": 5, "stroke": 900, "inkColor": "#12"},
            "positions": [
              {"page": 2, "xRatio": 0.95, "yRatio": -1, "widthRatio": 0.3, "heightRatio": 0.05},
              {"page": 2, "xRatio": 0.7, "yRatio": 0, "widthRatio": 0.3, "heightRatio": 0.05},
              {"page": 3, "xRatio": 0.1, "yRatio": 0.1, "widthRatio": -1, "heightRatio": 0.05}
            ],
            "filePositions": {
              "D:\\a.pdf": [{"page": 0, "xRatio": 0.9, "yRatio": 0.1, "widthRatio": 0.3, "heightRatio": 0.05}],
              "": [{"page": 1, "xRatio": 0.1, "yRatio": 0.1, "widthRatio": 0.3, "heightRatio": 0.05}],
              "D:\\b.pdf": [{"page": 1, "xRatio": 0.1, "yRatio": 0.1, "widthRatio": 0, "heightRatio": 0.05}]
            },
            "fileDisplays": {"D:\\a.pdf": "hidden", "D:\\b.pdf": "image", "": "text"},
            "perFile": true,
            "profiles": [
              {"id": "x", "name": "", "isDefault": true, "paper": "A3", "scale": 1000, "marginMm": {"x": -4, "y": 500}},
              {"id": "x", "name": "B", "isDefault": true, "paper": "A5"}
            ]
          },
          "output": {"subfolderName": "a/b", "suffix": "_ký?", "mode": "fixed"},
          "token": {"hiddenCerts": ["ab12cd34ab12cd34ab12cd34ab12cd34ab12cd34", "AB12CD34AB12CD34AB12CD34AB12CD34AB12CD34", "xyz", ""]}
        }"##;
        let (c, notes) = AppConfig::from_json(text).unwrap();
        let s = &c.signature;
        assert_eq!(
            s.display,
            SignatureMode::Text,
            "không có ảnh thì không dùng kiểu có ảnh"
        );
        assert_eq!(s.font.family, "Times New Roman");
        assert_eq!(s.font.size, ap::MAX_FONT_SIZE_PT);
        assert_eq!(s.font.color, "#000000");
        assert_eq!(
            (s.image.darkness, s.image.stroke, s.image.ink_color.clone()),
            (si::DARKNESS_MIN, si::STROKE_MAX, None)
        );
        assert_eq!(
            s.positions.len(),
            1,
            "ép vào trang thì hai khung trùng nhau; bỏ khung hỏng"
        );
        let p = s.positions[0];
        assert!((p.x_ratio - 0.7).abs() < 1e-9 && p.y_ratio == 0.0);
        assert!(s.per_file);
        assert_eq!(
            s.file_positions.keys().collect::<Vec<_>>(),
            [r"D:\a.pdf"],
            "bỏ khoá rỗng và vị trí không hợp lệ"
        );
        assert!((s.file_positions[r"D:\a.pdf"][0].x_ratio - 0.7).abs() < 1e-9);
        assert_eq!(
            s.file_displays
                .iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect::<Vec<_>>(),
            [
                (r"D:\a.pdf", SignatureMode::Hidden),
                (r"D:\b.pdf", SignatureMode::Text)
            ],
            "bỏ khoá rỗng; chưa có ảnh thì kiểu có ảnh về Chỉ thông tin"
        );
        assert_eq!(
            c.token.hidden_certs,
            ["AB12CD34AB12CD34AB12CD34AB12CD34AB12CD34"],
            "chữ hoa, bỏ trùng và giá trị không phải thumbprint"
        );
        assert_eq!(s.profiles[0].name, "Khổ A3");
        assert_eq!(s.profiles[0].scale, SCALE_MAX);
        assert_eq!(
            (s.profiles[0].margin_mm.x, s.profiles[0].margin_mm.y),
            (0.0, MARGIN_Y_MAX_MM)
        );
        assert_ne!(s.profiles[0].id, s.profiles[1].id);
        assert!(s.profiles[0].is_default && !s.profiles[1].is_default);
        assert_eq!(c.output.subfolder_name, "DaKySo");
        assert_eq!(c.output.suffix, "_signed");
        assert_eq!(c.output.mode, OutputMode::Subfolder);
        assert!(notes.len() >= 5, "{notes:?}");
    }

    #[test]
    fn profile_matching() {
        let c = AppConfig::default();
        assert_eq!(c.profile_for(Some(Paper::A3), true).id, "a3");
        assert_eq!(
            c.profile_for(Some(Paper::A3), false).id,
            "a3",
            "khớp khổ khi không có mẫu đúng hướng"
        );
        assert_eq!(c.profile_for(Some(Paper::A5), false).id, "a5");
        assert_eq!(
            c.profile_for(None, false).id,
            "a4",
            "khổ lạ thì dùng mẫu mặc định"
        );
    }

    #[test]
    fn last_page_and_clamp() {
        let p = Placement {
            page: 0,
            ..Placement::default()
        };
        assert_eq!(p.page_in(7), 7);
        let p = Placement { page: 9, ..p };
        assert_eq!(p.page_in(3), 3);
        assert_eq!(Placement { page: 2, ..p }.page_in(3), 2);
    }
}
