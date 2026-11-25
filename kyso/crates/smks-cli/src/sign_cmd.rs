//! Lệnh `certs`, `sign`, `verify`, `inspect`.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use smks_core::consts::{appearance as ap, signing as k};
use smks_crypto::cert::Cert;
use smks_crypto::key::SigningKey;
use smks_crypto::tsa::TsaClient;
use smks_pdf::appearance::{FontData, StampImage, TextStyle, max_pixels_for, parse_color};
use smks_pdf::geometry::{PaperSize, Placement, page_scale, resolve};
use smks_pdf::incremental::{SignRequest, Widget, WidgetContent};
use smks_pdf::reader::PdfFile;
use smks_sign::verify::{VerifyOptions, verify_pdf};

/// Tham số của lệnh `sign`.
#[derive(Debug, clap::Args)]
pub struct SignArgs {
    /// File PDF cần ký
    pub input: PathBuf,
    /// File kết quả (không được trùng file gốc)
    pub output: PathBuf,
    /// Thumbprint (SHA-1) của chứng thư trong kho Windows (USB token)
    #[arg(long)]
    pub thumbprint: Option<String>,
    /// [dev-keys] Chứng thư DER của khoá thử
    #[arg(long)]
    pub dev_cert: Option<PathBuf>,
    /// [dev-keys] Khoá bí mật PKCS#8 DER của khoá thử
    #[arg(long)]
    pub dev_key: Option<PathBuf>,
    /// [dev-keys] Chứng thư CA trung gian DER nhúng kèm
    #[arg(long)]
    pub dev_chain: Vec<PathBuf>,
    /// Không lấy dấu thời gian (chỉ để thử offline)
    #[arg(long)]
    pub no_tsa: bool,
    /// Không nhúng dữ liệu kiểm tra dài hạn (OCSP / CRL) sau khi ký
    #[arg(long)]
    pub no_ltv: bool,
    /// Cho phép ký bằng chứng thư không thuộc CA Ban Cơ yếu (chỉ để thử)
    #[arg(long)]
    pub allow_untrusted: bool,
    /// Hiện khối chữ ký (mặc định: chữ ký ẩn)
    #[arg(long)]
    pub visible: bool,
    /// Trang đặt khối chữ ký (từ 1)
    #[arg(long, default_value_t = 1)]
    pub page: u32,
    /// Vị trí theo tỉ lệ 0–1 tính từ góc trên trái: x,y,rộng,cao (mặc định: góc trên phải)
    #[arg(long, value_delimiter = ',')]
    pub at: Option<Vec<f64>>,
    /// Ảnh (PNG/JPG) đặt bên trái khối chữ ký
    #[arg(long)]
    pub image: Option<PathBuf>,
    #[arg(long, default_value = ap::DEFAULT_FONT_FAMILY)]
    pub font: String,
    #[arg(long, default_value_t = ap::DEFAULT_FONT_SIZE_PT)]
    pub size: f64,
    #[arg(long, default_value = ap::DEFAULT_TEXT_COLOR)]
    pub color: String,
    #[arg(long, default_value = k::DEFAULT_REASON)]
    pub reason: String,
    #[arg(long, default_value = k::DEFAULT_LOCATION)]
    pub location: String,
}

fn open_key(args: &SignArgs) -> anyhow::Result<(Box<dyn SigningKey>, Vec<Cert>)> {
    if let Some(thumbprint) = &args.thumbprint {
        #[cfg(windows)]
        {
            use smks_crypto::chain::{intermediates_for, is_trusted_signer};
            let key = smks_crypto::store::StoreKey::open(thumbprint)?;
            if !args.allow_untrusted && !is_trusted_signer(key.certificate()) {
                bail!(
                    "Chứng thư số không thuộc CA Ban Cơ yếu (thêm --allow-untrusted nếu chỉ để thử)."
                );
            }
            println!("Ký thử để kiểm token (middleware có thể hỏi PIN)…");
            key.preflight()?;
            let chain = intermediates_for(key.certificate());
            return Ok((Box::new(key), chain));
        }
        #[cfg(not(windows))]
        bail!("Ký bằng kho chứng thư chỉ có trên Windows ({thumbprint}).");
    }
    #[cfg(feature = "dev-keys")]
    if let (Some(cert), Some(key)) = (&args.dev_cert, &args.dev_key) {
        let cert = Cert::from_der(&std::fs::read(cert)?)?;
        let key =
            smks_crypto::key::software::SoftwareKey::from_pkcs8_der(cert, &std::fs::read(key)?)?;
        let chain = args
            .dev_chain
            .iter()
            .map(|p| Ok(Cert::from_der(&std::fs::read(p)?)?))
            .collect::<anyhow::Result<Vec<_>>>()?;
        return Ok((Box::new(key), chain));
    }
    bail!("Cần --thumbprint (hoặc --dev-cert và --dev-key khi build với feature dev-keys).")
}

pub fn sign(args: &SignArgs) -> anyhow::Result<()> {
    let input = std::fs::read(&args.input)
        .with_context(|| format!("Không đọc được {}", args.input.display()))?;
    if same_file(&args.input, &args.output) {
        bail!("File kết quả trùng file gốc: không bao giờ ghi đè file gốc.");
    }
    if args.at.as_ref().is_some_and(|v| v.len() != 4) {
        bail!("--at cần đúng 4 số: x,y,rộng,cao.");
    }
    let (key, chain) = open_key(args)?;
    let signer = key.certificate().common_name();
    let (now, now_text) = smks_sign::verify::now_vietnam();
    let mut widgets = Vec::new();
    if args.visible {
        let file = PdfFile::open(&input)?;
        let page = file
            .pages()?
            .into_iter()
            .find(|p| p.number == args.page)
            .with_context(|| format!("File không có trang {}", args.page))?;
        let display = page.display_box();
        let date = now_text.get(..16).unwrap_or(&now_text).to_owned();
        let lines = vec![format!("Ký bởi: {signer}"), format!("Ngày ký: {date}")];
        let (font, switched) = FontData::for_lines(&args.font, &lines)?;
        if switched {
            println!(
                "Font {} thiếu chữ tiếng Việt, dùng {} (TD-33).",
                args.font, font.family
            );
        }
        let size = args.size * page_scale(&display);
        let (text_w, line_h) = font.measure(&lines, size)?;
        let color = parse_color(&args.color).unwrap_or([0.0; 3]);
        let placement = args.at.as_ref().map(|v| Placement {
            page: args.page,
            x_ratio: v[0],
            y_ratio: v[1],
            width_ratio: v[2],
            height_ratio: v[3],
        });
        let style = TextStyle { font, size, color };
        let (rect, content) = match &args.image {
            None => (
                resolve(&display, placement.as_ref(), text_w, line_h * 2.0)?,
                WidgetContent::Text { lines, style },
            ),
            Some(path) => {
                let p = placement.unwrap_or(Placement {
                    page: args.page,
                    x_ratio: 0.55,
                    y_ratio: 0.05,
                    width_ratio: 0.4,
                    height_ratio: 0.08,
                });
                let rect = smks_pdf::geometry::frame(&display, &p)?;
                let image = StampImage::prepare(
                    &std::fs::read(path)?,
                    100,
                    None,
                    max_pixels_for(rect.width, rect.height),
                )?;
                (
                    rect,
                    WidgetContent::ImageAndText {
                        image,
                        stroke: 0,
                        lines,
                        style,
                        layout: smks_core::config::ImageLayout::Left,
                        split: smks_core::config::ImageSplit::default(),
                    },
                )
            }
        };
        widgets.push(Widget {
            page: args.page,
            rect,
            content,
        });
    }
    let request = SignRequest {
        signer_name: signer.clone(),
        reason: args.reason.clone(),
        location: args.location.clone(),
        signed_at: now,
        widgets,
        stamps: Vec::new(),
    };
    let tsa = (!args.no_tsa).then(TsaClient::default);
    let started = std::time::Instant::now();
    let mut signed =
        smks_sign::sign::sign_pdf(&input, &request, key.as_ref(), &chain, tsa.as_ref())?;
    // Ký dài hạn (TD-58): có dấu thời gian thì nhúng dữ liệu kiểm tra thu hồi (DSS), trừ khi tắt.
    let mut ltv = "không";
    if tsa.is_some() && !args.no_ltv {
        let outcome = smks_sign::sign::add_ltv(&signed.bytes, &smks_crypto::ltv::LtvCache::new());
        if outcome.bytes.len() != signed.bytes.len() {
            ltv = if outcome.long_term {
                "có (người ký + TSA)"
            } else {
                "có (TSA)"
            };
        }
        if let Some(note) = &outcome.note {
            println!("Lưu ý: {note}");
        }
        signed.bytes = outcome.bytes;
    }
    write_atomic(&args.output, &signed.bytes)?;
    println!(
        "Đã ký {} → {} ({} byte, {:.2}s). Người ký: {signer} | provider: {} | dấu thời gian: {} | dữ liệu dài hạn: {ltv}",
        args.input.display(),
        args.output.display(),
        signed.bytes.len(),
        started.elapsed().as_secs_f32(),
        key.provider(),
        signed.check.trusted_time.as_deref().unwrap_or("không có"),
    );
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension("pdf.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    Ok(())
}

pub fn verify(path: &Path, revocation: bool, json: bool) -> anyhow::Result<()> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Không đọc được {}", path.display()))?;
    let report = verify_pdf(
        &bytes,
        VerifyOptions {
            check_revocation: revocation,
        },
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!(
        "{}: {} — {}",
        path.display(),
        report.verdict.label(),
        report.reason.as_deref().unwrap_or("")
    );
    for (i, s) in report.signatures.iter().enumerate() {
        println!(
            "  #{} [{}] {} | ký bởi {} (CA {}) | gốc {} | dấu thời gian {} bởi {} | thu hồi {:?} | trang {:?}{} | {}",
            i + 1,
            s.field_name,
            s.verdict.label(),
            s.signed_by.as_deref().unwrap_or("?"),
            s.issuer.as_deref().unwrap_or("?"),
            s.signer_root.as_deref().unwrap_or("-"),
            s.trusted_time.as_deref().unwrap_or("-"),
            s.timestamp_authority.as_deref().unwrap_or("-"),
            s.revocation,
            s.page,
            if s.visible {
                " (hiển thị)"
            } else {
                " (ẩn)"
            },
            s.reason.as_deref().unwrap_or(""),
        );
    }
    Ok(())
}

pub fn inspect(path: &Path) -> anyhow::Result<()> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Không đọc được {}", path.display()))?;
    let file = PdfFile::open(&bytes)?;
    println!(
        "{}: {} byte, xref cuối kiểu {} ở offset {}, {} chữ ký, ký được: {}",
        path.display(),
        bytes.len(),
        if file.xref_stream { "stream" } else { "bảng" },
        file.last_xref,
        smks_pdf::sigscan::count(&bytes),
        match file.check_signable() {
            Ok(()) => "có".to_owned(),
            Err(e) => format!("không ({})", e.message),
        }
    );
    for p in file.pages()? {
        let d = p.display_box();
        println!(
            "  trang {}: {:.1} × {:.1} pt, xoay {}°, khổ {}",
            p.number,
            d.width,
            d.height,
            p.rotation,
            PaperSize::detect(&d).map_or("khác".to_owned(), |s| format!("{s:?}"))
        );
    }
    Ok(())
}

#[cfg(windows)]
pub fn certs() -> anyhow::Result<()> {
    let scan = smks_crypto::store::list_certificates(&smks_crypto::chain::is_trusted_signer);
    for line in &scan.diagnostics {
        println!("{line}");
    }
    if scan.certificates.is_empty() {
        println!(
            "Không có chứng thư nào trong kho (chưa cắm USB token hoặc middleware chưa chạy)."
        );
    }
    for c in &scan.certificates {
        println!(
            "{} {} | {} | cấp bởi {} | {:?} | provider {} | {}",
            if c.can_sign {
                "[ký được]"
            } else {
                "[không ký]"
            },
            c.thumbprint,
            c.common_name,
            c.issuer_common_name,
            c.source,
            c.key_provider.as_deref().unwrap_or("-"),
            c.reason.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn certs() -> anyhow::Result<()> {
    bail!("Kho chứng thư chỉ có trên Windows.")
}
