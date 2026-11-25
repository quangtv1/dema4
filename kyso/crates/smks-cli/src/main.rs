//! Công cụ dòng lệnh của Sao Mai Ký số, dùng để kiểm thử lõi độc lập với giao diện.
//!
//! Lệnh: `ca` (chứng thư CA nhúng), `errors` (bảng mã lỗi), `merge` (tạo văn bản), `certs` (kho chứng thư),
//! `sign` (ký PDF), `verify` (xác minh chữ ký), `inspect` (cấu trúc PDF).

mod sign_cmd;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use smks_core::ErrorCode;
use smks_crypto::trust::PINNED_CAS;
use smks_merge::batch::BatchEvent;
use smks_merge::plan::{
    DataSource, FieldSource, MergeRequest, OutputFormat, auto_map, prepare, vietnam_stamp,
};
use smks_merge::table::read_table;

#[derive(Debug, Parser)]
#[command(
    name = "smks-cli",
    version,
    about = "Sao Mai Ký số - công cụ dòng lệnh kiểm thử lõi"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Pdf,
    Docx,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Liệt kê chứng thư trong kho Windows (USB token), cho biết chứng thư nào ký được
    Certs,
    /// Ký một file PDF (PAdES B-T, dấu thời gian của Ban Cơ yếu)
    Sign(sign_cmd::SignArgs),
    /// Xác minh mọi chữ ký trong file PDF
    Verify {
        file: PathBuf,
        /// Kiểm thu hồi qua CRL/OCSP (cần mạng)
        #[arg(long)]
        revocation: bool,
        /// In kết quả dạng JSON
        #[arg(long)]
        json: bool,
    },
    /// In cấu trúc file PDF: kiểu xref, số trang, khổ, xoay, số chữ ký
    Inspect { file: PathBuf },
    /// Kiểm tra các chứng thư CA nhúng trong ứng dụng khớp giá trị SHA-256 đã ghim
    Ca,
    /// In bảng mã lỗi và thông báo mặc định
    Errors,
    /// Tạo văn bản hàng loạt từ mẫu .docx và dữ liệu Excel/CSV (biến được ghép tự động theo tên cột)
    Merge {
        /// File mẫu .docx
        template: PathBuf,
        /// File dữ liệu .xlsx, .xls hoặc .csv
        data: PathBuf,
        /// Sheet cần đọc (mặc định sheet đầu)
        #[arg(long)]
        sheet: Option<String>,
        /// Dòng tiêu đề (mặc định dòng đầu có dữ liệu)
        #[arg(long)]
        header_row: Option<u32>,
        /// Mẫu tên file, ví dụ "{so}_{hoten}" (mặc định: biến đầu tiên của mẫu)
        #[arg(long)]
        pattern: Option<String>,
        #[arg(long, value_enum, default_value = "pdf")]
        format: Format,
        /// Thư mục lưu (mặc định thư mục chứa file dữ liệu)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Chỉ in kế hoạch (tên file, cảnh báo), không tạo file
        #[arg(long)]
        plan: bool,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Ca => print_pinned_cas(),
        Command::Certs => sign_cmd::certs(),
        Command::Sign(args) => sign_cmd::sign(&args),
        Command::Verify {
            file,
            revocation,
            json,
        } => sign_cmd::verify(&file, revocation, json),
        Command::Inspect { file } => sign_cmd::inspect(&file),
        Command::Errors => {
            print_error_codes();
            Ok(())
        }
        Command::Merge {
            template,
            data,
            sheet,
            header_row,
            pattern,
            format,
            out,
            plan,
        } => merge(
            &template, &data, sheet, header_row, pattern, format, out, plan,
        ),
    }
}

fn print_pinned_cas() -> anyhow::Result<()> {
    for ca in &PINNED_CAS {
        let info = ca
            .info()
            .with_context(|| format!("Chứng thư CA {} không hợp lệ", ca.file_name()))?;
        println!("{} ({})", info.file_name, info.role.label());
        println!("  Chủ thể   : {}", info.subject);
        println!("  Cấp bởi   : {}", info.issuer);
        println!("  Serial    : {}", info.serial_hex);
        println!(
            "  Hiệu lực  : {} đến {} (UTC)",
            info.not_before, info.not_after
        );
        println!("  SHA-256   : {} (khớp giá trị ghim)", info.sha256_hex);
    }
    println!(
        "Đã kiểm tra {} chứng thư CA: tất cả khớp giá trị ghim.",
        PINNED_CAS.len()
    );
    Ok(())
}

fn print_error_codes() {
    for code in ErrorCode::ALL {
        println!(
            "{:>5}  {:<28} {}",
            code.as_u32(),
            code.name(),
            code.default_message()
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn merge(
    template: &Path,
    data: &Path,
    sheet: Option<String>,
    header_row: Option<u32>,
    pattern: Option<String>,
    format: Format,
    out: Option<PathBuf>,
    plan_only: bool,
) -> anyhow::Result<()> {
    let locale = smks_merge::word::user_number_locale();
    let table = read_table(data, sheet.as_deref(), header_row, locale)?;
    let fields = smks_merge::docx::read_fields(&std::fs::read(template)?)?;
    let mappings: HashMap<String, FieldSource> = auto_map(&fields, &table.columns)
        .into_iter()
        .map(|(field, column)| {
            (
                field,
                column.map_or(FieldSource::None, |column| FieldSource::Column { column }),
            )
        })
        .collect();
    let pattern = pattern.unwrap_or_else(|| {
        fields
            .first()
            .map(|f| format!("{{{f}}}"))
            .unwrap_or_default()
    });
    let request = MergeRequest {
        template_path: template.display().to_string(),
        data: DataSource::Table {
            path: data.display().to_string(),
            sheet: table.sheet.clone(),
            header_row: table.header_row,
        },
        mappings,
        file_name_pattern: pattern,
        format: match format {
            Format::Pdf => OutputFormat::Pdf,
            Format::Docx => OutputFormat::Docx,
        },
        output_folder: out.map(|p| p.display().to_string()),
    };
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let prepared = prepare(&request, locale, &vietnam_stamp(i64::try_from(seconds)?))?;

    println!(
        "Mẫu {} biến, dữ liệu {} dòng (sheet {:?}, tiêu đề dòng {}), số {}",
        fields.len(),
        table.rows.len(),
        table.sheet,
        table.header_row,
        if locale.decimal == ',' {
            "kiểu 1.234,5"
        } else {
            "kiểu 1,234.5"
        }
    );
    for warning in &prepared.warnings {
        println!("CẢNH BÁO: {warning}");
    }
    for record in &prepared.records {
        println!("  dòng {:>3} -> {}", record.row_number, record.file_name);
        for warning in &record.warnings {
            println!("             ! {warning}");
        }
    }
    if plan_only {
        return Ok(());
    }

    let mut word = None;
    let mut failed = 0;
    smks_merge::batch::run(
        "cli",
        &prepared,
        &AtomicBool::new(false),
        &mut word,
        &mut |event| match event {
            BatchEvent::Manifest {
                output_folder,
                total,
                ..
            } => println!("Tạo {total} file vào {output_folder}"),
            BatchEvent::File {
                current,
                total,
                file_name,
                status,
                reason,
                ..
            } => {
                println!(
                    "[{current}/{total}] {status:<5} {file_name}{}",
                    reason.map(|r| format!(" | {r}")).unwrap_or_default()
                );
            }
            BatchEvent::Summary {
                success,
                failed: f,
                elapsed_ms,
                ..
            } => {
                failed = f;
                println!(
                    "Xong: {success} thành công, {f} lỗi, {:.1}s",
                    elapsed_ms as f64 / 1000.0
                );
            }
        },
    );
    drop(word);
    if failed > 0 {
        bail!("{failed} file lỗi");
    }
    Ok(())
}
