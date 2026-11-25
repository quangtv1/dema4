import { Injectable } from '@angular/core';
import { Channel, invoke, isTauri } from '@tauri-apps/api/core';

/*
 * HỢP ĐỒNG IPC giữa giao diện và lõi Rust (crate smks-app).
 * Mọi tên trường là camelCase, khớp với `#[serde(rename_all = "camelCase")]` bên Rust.
 * Khi một lời gọi thất bại, Promise bị reject với một `IpcError`.
 * Sửa file này thì phải sửa phía Rust tương ứng (app/src-tauri/src/commands/*.rs).
 */

/** Lỗi nghiệp vụ do lõi trả về: mã lỗi (Phụ lục B của kế hoạch) và thông báo tiếng Việt hiển thị thẳng. */
export interface IpcError {
  code: number;
  message: string;
}

/** Đổi lỗi bất kỳ của `invoke` thành câu hiển thị được cho người dùng. */
export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as IpcError).message);
  }
  return String(error);
}

// ---------------------------------------------------------------------------------------------
// Ứng dụng
// ---------------------------------------------------------------------------------------------

export interface AppInfo {
  name: string;
  version: string;
  /** Số chứng thư CA nhúng đã qua kiểm tra ghim lúc khởi động. */
  pinnedCaCount: number;
  /** Tên người dùng Windows, tên máy (hiện ở tooltip chân Trang chủ). */
  userName: string;
  computerName: string;
  /** Máy có Microsoft Word không (cần để xuất PDF từ mẫu DOCX). */
  wordAvailable: boolean;
}

// ---------------------------------------------------------------------------------------------
// Tạo văn bản
// ---------------------------------------------------------------------------------------------

/** Thông tin mẫu văn bản. Hiện chỉ hỗ trợ .docx. */
export interface TemplateInfo {
  path: string;
  fileName: string;
  kind: 'docx';
  /** Tên các biến `{ten_bien}` theo thứ tự xuất hiện, không trùng. */
  fields: string[];
}

export interface TableColumn {
  /** Số cột thật trên sheet, đánh từ 1 (A = 1). Dùng làm khoá khi ghép biến. */
  column: number;
  /** Tên cột (ô ở dòng tiêu đề). */
  name: string;
  /** Số giá trị khác nhau (không tính ô trống) trong các dòng dữ liệu. */
  distinct: number;
}

export interface TableRow {
  /** Số dòng thật trên sheet (đánh từ 1), hoặc số dòng trong file CSV. */
  rowNumber: number;
  /** Giá trị đã định dạng như Excel hiển thị, cùng thứ tự với `TableInfo.columns`. */
  values: string[];
}

export interface TableInfo {
  path: string;
  fileName: string;
  kind: 'xlsx' | 'xls' | 'csv';
  /** Danh sách sheet (rỗng với CSV). */
  sheets: string[];
  /** Sheet đang đọc (null với CSV). */
  sheet: string | null;
  /** Dòng tiêu đề đang dùng (đánh từ 1). */
  headerRow: number;
  columns: TableColumn[];
  /** Các dòng dữ liệu sau dòng tiêu đề, đã bỏ dòng trống hoàn toàn. */
  rows: TableRow[];
}

/** Nguồn giá trị của một biến. */
export type FieldSource =
  | { kind: 'column'; column: number }
  | { kind: 'fixed'; value: string }
  | { kind: 'none' };

export type DataSource =
  | { kind: 'table'; path: string; sheet: string | null; headerRow: number }
  /** Nhập trực tiếp một bản ghi: khoá là tên biến. */
  | { kind: 'manual'; values: Record<string, string> };

export interface MergeRequest {
  templatePath: string;
  data: DataSource;
  /** Khoá là tên biến. Biến không có trong đây coi như `none` (để trống khi tạo). Bỏ qua khi nhập trực tiếp. */
  mappings: Record<string, FieldSource>;
  /** Mẫu tên file, ví dụ `{so}_{hoten}`; tên trong ngoặc là tên biến của mẫu văn bản. */
  fileNamePattern: string;
  format: 'pdf' | 'docx';
  /** Thư mục lưu; null = thư mục chứa file dữ liệu (hoặc file mẫu khi nhập trực tiếp). */
  outputFolder: string | null;
}

export interface PlannedFile {
  rowNumber: number;
  /** Tên file sẽ tạo, đã có đuôi .pdf/.docx. */
  fileName: string;
  /** Cảnh báo riêng dòng này (ô trống, tên trùng…). */
  warnings: string[];
}

export interface MergePlan {
  files: PlannedFile[];
  /** Biến chưa ghép nguồn (sẽ để trống khi tạo). */
  unmappedFields: string[];
  /** Cảnh báo chung. */
  warnings: string[];
  /** Thư mục kết quả sẽ được tạo: `<thư mục lưu>\<tên mẫu>_<yyyyMMdd_HHmmss>`. */
  outputFolder: string;
  /** Máy có Word không; false thì chỉ xuất được DOCX. */
  wordAvailable: boolean;
}

export interface PreviewResult {
  /** File PDF xem trước trong thư mục tạm; giao diện đọc bằng `convertFileSrc(pdfPath)`. */
  pdfPath: string;
  pageCount: number;
  warnings: string[];
}

export type MergeEvent =
  | { type: 'manifest'; jobId: string; total: number; outputFolder: string; files: string[] }
  | {
      type: 'file';
      jobId: string;
      /** Thứ tự trong lô, đánh từ 0. */
      index: number;
      rowNumber: number;
      fileName: string;
      status: 'ok' | 'error';
      outputPath: string | null;
      reason: string | null;
      warnings: string[];
      /** Số file đã xử lý xong (kể cả lỗi). */
      current: number;
      total: number;
    }
  | {
      type: 'summary';
      jobId: string;
      total: number;
      success: number;
      failed: number;
      elapsedMs: number;
      outputFolder: string;
      /** true nếu người dùng bấm Dừng. File chưa xử lý không có sự kiện `file`. */
      stopped: boolean;
    };

// ---------------------------------------------------------------------------------------------
// Ký số
// ---------------------------------------------------------------------------------------------

export type CertSource = 'usbToken' | 'local' | 'server';

/** Một chứng thư trong kho Windows (CurrentUser\My). */
export interface CertEntry {
  thumbprint: string;
  commonName: string;
  subject: string;
  issuer: string;
  issuerCommonName: string;
  serialNumber: string;
  /** Giây unix. */
  validFrom: number;
  validTo: number;
  source: CertSource;
  keyProvider: string | null;
  hasPrivateKey: boolean;
  isExpired: boolean;
  allowsSigning: boolean;
  isTrusted: boolean;
  /** Dùng để ký được không; false thì `reason` ghi lý do. */
  canSign: boolean;
  reason: string | null;
}

export interface CertScan {
  certificates: CertEntry[];
  diagnostics: string[];
}

/** Kết quả mở token và ký thử. */
export interface TokenInfo {
  thumbprint: string;
  commonName: string;
  issuerCommonName: string;
  provider: string;
  hardware: boolean;
  validTo: number;
  dev: boolean;
  /** PIN dùng lần mở này: vừa gõ / đã lưu; null: middleware tự hỏi. */
  pinSource: 'typed' | 'saved' | null;
  /** Middleware có nhận PIN do ứng dụng đưa vào không (spike S6). */
  pinHandoff: 'accepted' | 'notSupported' | null;
  /** Vừa lưu PIN trên máy. */
  pinSaved: boolean;
  /** Lý do không lưu PIN dù đã chọn. */
  pinNote: string | null;
}

export interface DriverItem {
  id: string;
  name: string;
  /** Loại token dùng driver này. */
  tokens: string;
  installed: boolean;
  version: string | null;
  /** Dấu hiệu tìm thấy (khoá registry / file). */
  evidence: string | null;
}

export interface DriverStatus {
  drivers: DriverItem[];
  anyInstalled: boolean;
  /** Dịch vụ Smart Card của Windows bị tắt. */
  smartCardDisabled: boolean;
  downloadUrl: string;
}

/** Mã lỗi PIN sai / token bị khoá (`ErrorCode::PinIncorrect`, `PinBlocked`). */
export const PIN_ERROR_CODES = [1060, 1061];

export interface SignFileItem {
  name: string;
  /** Đường dẫn tuyệt đối; dùng làm khoá và gửi nguyên cho `sign_run`. */
  path: string;
  size: number;
  /** Số chữ ký đã có trong file (file đã ký vẫn được liệt kê). */
  signatureCount: number;
  unreadable: boolean;
  /** Thư mục người dùng chọn chứa file này (kể cả thư mục con); `null` khi chọn từng file. `name` khi đó là đường dẫn tương đối. */
  root: string | null;
}

/** Vị trí khối chữ ký theo tỉ lệ trang (0–1), gốc trên trái. `page` từ 1; 0 là trang cuối. */
export interface SignPlacement {
  page: number;
  xRatio: number;
  yRatio: number;
  widthRatio: number;
  heightRatio: number;
}

export type SignatureMode = 'hidden' | 'text' | 'imageAndText' | 'image';
/** Vị trí của ảnh so với chữ trong kiểu "Ảnh + thông tin". */
export type ImageLayout = 'left' | 'right' | 'top' | 'bottom';
/** Lượt ký chính hay ký nháy. Mỗi lượt ký ra đúng một chữ ký trên mỗi văn bản. */
export type SignKind = 'main' | 'initial';

export interface SignAppearance {
  mode: SignatureMode;
  kind: SignKind;
  /** Khung tự đặt áp cho cả lô; rỗng thì theo mẫu của lượt ký (chính: mẫu khổ giấy; nháy: mọi trang). */
  placements: SignPlacement[];
  /** Mẫu chữ ký theo khổ giấy; rỗng thì góc trên phải trang 1 như bản cũ. */
  profiles: SignatureProfile[];
  fontFamily: string;
  fontSize: number;
  textColor: string;
  reason: string;
  location: string;
  sealImage: string | null;
  wetImage: string | null;
  signatureImage: 'seal' | 'wetSignature' | null;
  imageLayout: ImageLayout;
  /** Phần khối dành cho ảnh (%), `null` = tự động (TD-51). */
  imageShare: number | null;
  /** Khoảng cách ảnh – chữ (% khối), `null` = tự động (TD-51). */
  imageGap: number | null;
  /** Kiểu "Tự do" (TD-52): ô ảnh và ô chữ trong khối; chỉ dùng khi có cả hai. */
  imageBox: PartBox | null;
  textBox: PartBox | null;
  sealDarkness: number;
  wetDarkness: number;
  wetStroke: number;
  wetColor: string | null;
  stamps: unknown[];
}

// ---- Cấu hình (config.json, smks_core::config) ----------------------------------------------------

export type Paper = 'A1' | 'A2' | 'A3' | 'A4' | 'A5';
export type Orientation = 'portrait' | 'landscape';
export type PagePick = 'first' | 'last' | 'all';
export type Anchor = 'tl' | 'tc' | 'tr' | 'bl' | 'bc' | 'br';
export type ImageKind = 'seal' | 'wetSignature';

export interface SignatureProfile {
  id: string;
  name: string;
  isDefault: boolean;
  paper: Paper;
  orientation: Orientation;
  page: PagePick;
  anchor: Anchor;
  marginMm: { x: number; y: number };
  /** % so với khối chuẩn 170 × 30 pt, 40–200. */
  scale: number;
}

export interface AppConfig {
  version: number;
  signature: {
    display: SignatureMode;
    reason: string;
    location: string;
    imagePath: string | null;
    imageKind: ImageKind;
    /** Ảnh nằm ở đâu so với chữ (kiểu "Ảnh + thông tin"). */
    imageLayout: ImageLayout;
    /** Phần khối dành cho ảnh (%, 10–90), `null` = tự động (TD-51). */
    imageShare?: number | null;
    /** Khoảng cách ảnh – chữ (%, 0–30), `null` = tự động (TD-51). */
    imageGap?: number | null;
    /** Kiểu "Tự do" (TD-52): ô ảnh và ô chữ kéo thả trong khối. */
    imageBox?: PartBox | null;
    textBox?: PartBox | null;
    font: { family: string; size: number; color: string };
    image: { darkness: number; stroke: number; inkColor: string | null };
    profiles: SignatureProfile[];
    /** Lượt ký sắp tới: chữ ký chính hay ký nháy. */
    kind: SignKind;
    /** Khung chữ ký chính tự đặt dùng chung cho mọi file; rỗng thì theo mẫu khổ giấy. */
    positions: SignPlacement[];
    /** Khung ký nháy tự đặt dùng chung; rỗng thì mọi trang. */
    initialPositions: SignPlacement[];
    /** Khung chữ ký chính riêng từng file (đường dẫn → khung), chế độ "Từng file" hoặc thêm / xoá trên màn Ký số. */
    filePositions: Record<string, SignPlacement[]>;
    /** Khung ký nháy riêng từng file (thêm / xoá trên màn Ký số). */
    fileInitialPositions: Record<string, SignPlacement[]>;
    /** Kiểu hiển thị riêng từng file (đường dẫn → kiểu), cùng chế độ "Từng file"; không có thì theo `display`. */
    fileDisplays: Record<string, SignatureMode>;
    /** Đang dùng chế độ "Từng file" (vị trí, kiểu riêng chỉ áp khi bật). */
    perFile: boolean;
    /** Văn bản "chưa gán chữ ký" của chữ ký chính (TD-56): đã xoá hết khung, bỏ qua khi ký số. */
    skipFiles?: string[];
    /** Như `skipFiles`, cho lượt ký nháy. */
    initialSkipFiles?: string[];
  };
  /** Tab USB Token: chứng thư đã xoá khỏi danh sách của ứng dụng (thumbprint chữ hoa); vẫn còn trong kho Windows. */
  token: { hiddenCerts: string[] };
  output: {
    mode: 'subfolder' | 'fixed';
    subfolderName: string;
    fixedFolder: string | null;
    suffix: string;
    overwrite: boolean;
  };
}

/** Bố cục chữ ký của một file (màn Vị trí chữ ký). */
/** Một ô bên trong khối chữ ký, theo tỉ lệ 0–1 của khối, gốc trên trái (kiểu "Tự do", TD-52). */
export interface PartBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface FileLayout {
  pageCount: number;
  /** Khổ (pt) của trang có khối chữ ký đầu tiên. */
  pageWidth: number;
  pageHeight: number;
  /** Chỗ các khối với vị trí đang đặt. */
  boxes: SignPlacement[];
  /** Chỗ các khối theo mẫu khổ giấy. */
  template: SignPlacement[];
}

export interface FileLayoutItem {
  path: string;
  layout: FileLayout | null;
  error: string | null;
}

export interface ConfigLoad {
  config: AppConfig;
  path: string;
  /** Chưa có file (lần đầu chạy). */
  created: boolean;
  warning: string | null;
}

export interface ConfigSaved {
  config: AppConfig;
  notes: string[];
}

export interface SignOutputOptions {
  subfolder: string;
  suffix: string;
  fixedFolder: string | null;
  overwrite: boolean;
  /** "Xóa file gốc" (ADR 0003): file đã ký thay chỗ file gốc, file gốc vào Thùng rác. */
  replaceOriginal: boolean;
  /** Thư mục đã chọn để ký cả thư mục: kết quả vào `<tên>_<subfolder>` cùng cấp, giữ nguyên thư mục con (TD-50). */
  sourceRoots: string[];
}

/** Nơi lưu dự kiến của một file; `exists`: đã có file trùng tên (hộp "File đã ký / trùng tên"). */
export interface PlannedOutput {
  source: string;
  output: string;
  exists: boolean;
}

export interface SignJobRequest {
  files: string[];
  appearance: SignAppearance;
  output: SignOutputOptions;
  /** Khung riêng từng file của lượt ký này, ghi đè `appearance.placements`. */
  placements: Record<string, SignPlacement[]>;
  /** Kiểu hiển thị riêng từng file, ghi đè `appearance.mode`. */
  modes: Record<string, SignatureMode>;
  /** Lõi tự đặt lại theo chứng thư; giao diện gửi false. */
  requireTrusted: boolean;
}

/** Kết luận một chữ ký, từ nhẹ đến nặng (cùng thứ tự với lõi, `smks-sign::verify::Verdict`). */
export type Verdict =
  | 'notApplicable'
  | 'unsigned'
  | 'validTrusted'
  | 'revocationUnknown'
  | 'timeMismatch'
  | 'noTimestamp'
  | 'untrustedTsa'
  | 'unsupported'
  | 'untrustedSigner'
  | 'modifiedAfterSigning'
  | 'invalidSignature';

export type CheckState = 'ok' | 'warn' | 'fail' | 'unknown';
export type ChangeLevel = 'none' | 'signatures' | 'formFill' | 'annotations' | 'other';
export type RevocationStatus = 'notChecked' | 'good' | 'revokedAfterSigning' | 'revoked' | 'unknown';

/** Một điều kiện của Nghị định 68/2024 Điều 30 cho một chữ ký. */
export interface VerifyCheck {
  id: 'integrity' | 'signer' | 'time' | 'certificate' | 'chain';
  label: string;
  state: CheckState;
  detail: string;
}

export interface VerifyChainItem {
  name: string;
  serial: string;
  validFrom: string;
  validTo: string;
}

/** Kết quả một chữ ký (`smks-sign::verify::SignatureReport`). */
export interface SignatureVerify {
  fieldName: string;
  kind: 'signature' | 'docTimestamp';
  verdict: Verdict;
  reason: string | null;
  signedBy: string | null;
  organization: string | null;
  signerSerial: string | null;
  issuer: string | null;
  certValidFrom: string | null;
  certValidTo: string | null;
  signerRoot: string | null;
  chain: VerifyChainItem[];
  trustedTime: string | null;
  signingTime: string | null;
  pdfTime: string | null;
  displayTime: string | null;
  timestampAuthority: string | null;
  hasTimestamp: boolean;
  revocation: RevocationStatus;
  /** Tình trạng thu hồi lấy từ dữ liệu nhúng trong file (DSS), không hỏi qua mạng. */
  revocationEmbedded: boolean;
  revokedAt: string | null;
  /** File có dữ liệu kiểm tra dài hạn cho chứng thư người ký (PAdES B-LT, TD-58). */
  longTerm: boolean;
  page: number | null;
  visible: boolean;
  placement: SignPlacement | null;
  coversWholeFile: boolean;
  subFilter: string;
  digestAlgorithm: string | null;
  signReason: string | null;
  location: string | null;
  docmdp: number | null;
  changes: ChangeLevel;
  changeNotes: { level: ChangeLevel; text: string }[];
  checks: VerifyCheck[];
  warnings: string[];
}

/** Kết quả kiểm cả file (`smks-sign::verify::PdfReport`). */
export interface VerifyReport {
  verdict: Verdict;
  reason: string | null;
  signatures: SignatureVerify[];
  validCount: number;
  revocationChecked: boolean;
  /** Dữ liệu kiểm tra dài hạn trong file (`/DSS`). */
  dss: { certs: number; ocsps: number; crls: number } | null;
}

export type SignEvent =
  | {
      type: 'manifest';
      jobId: string;
      total: number;
      files: string[];
      outputFolder: string;
      /** Nguồn giờ ký (TD-54): giờ máy chủ dấu thời gian Ban Cơ yếu, hoặc giờ máy tính (không có dấu thời gian). */
      clock: 'tsa' | 'computer';
      /** Có khi ký theo giờ máy tính vì không lấy được giờ máy chủ: lý do. */
      clockNote: string | null;
    }
  | {
      type: 'file';
      /** Chỉ số trong `request.files`. Sự kiện về theo thứ tự xong, không theo thứ tự gửi. */
      index: number;
      name: string;
      success: boolean;
      outputPath: string | null;
      reason: string | null;
      verdict: Verdict | null;
      trustedTime: string | null;
      /** File chưa ký vì lô tạm dừng (bấm Tạm dừng hoặc rút token); không phải lỗi, "Ký tiếp" sẽ ký. */
      paused: boolean;
      done: number;
      total: number;
    }
  | {
      type: 'summary';
      jobId: string;
      total: number;
      success: number;
      /** File lỗi thật (không tính file tạm dừng). */
      failed: number;
      /** File chưa ký vì lô tạm dừng. */
      paused: number;
      stopped: boolean;
      tokenRemoved: boolean;
      elapsedMs: number;
      outputFolder: string;
      /** Như `manifest.clockNote`. */
      clockNote: string | null;
    };

// ---------------------------------------------------------------------------------------------
// Thông báo Telegram và đăng ký thiết bị (crate smks-notify, commands/notify.rs)
// ---------------------------------------------------------------------------------------------

/** Bộ công tắc bật / tắt thông báo. */
export interface NotifyFlags {
  /** Công tắc chung. */
  enabled: boolean;
  /** Đăng ký / hủy đăng ký thiết bị, cập nhật ứng dụng. */
  device: boolean;
  sign: boolean;
  document: boolean;
  /** Sự kiện thất bại hoặc bị hủy. */
  failure: boolean;
}

/** Thông tin thiết bị (xem trước ở hộp đăng ký; IP và MAC chỉ gửi khi người dùng chọn). */
export interface NotifyDevice {
  hostname: string;
  os: string;
  osVersion: string;
  architecture: string;
  appVersion: string;
  localIp: string;
  macAddress: string;
}

/** Trạng thái thông báo. Không bao giờ chứa Bot Token hay Chat ID đầy đủ. */
export interface NotifyStatus {
  /** Máy có cấu hình quản trị (`notify.json`) dùng được. */
  configured: boolean;
  configSource: string | null;
  configProblem: string | null;
  mode: 'gateway' | 'direct' | null;
  /** Nơi nhận, đã che bí mật. */
  target: string;
  /** Quản trị viên khoá các công tắc. */
  locked: boolean;
  admin: NotifyFlags;
  prefs: NotifyFlags;
  effective: NotifyFlags;
  consent: 'pending' | 'accepted' | 'declined';
  /** Cần hỏi người dùng (có cấu hình, chưa trả lời). */
  needsConsent: boolean;
  deviceId: string;
  registration: 'none' | 'pending' | 'registered';
  registeredAt: string | null;
  includeIp: boolean;
  includeMac: boolean;
  device: NotifyDevice;
  connection: { state: 'unknown' | 'ok' | 'error'; message: string; checkedAt: string | null };
  /** Số sự kiện đang chờ gửi. */
  pending: number;
  nextAttemptAt: string | null;
  lastError: string | null;
  lastSentAt: string | null;
  auditDir: string;
}

/**
 * Cửa duy nhất để giao diện gọi lõi Rust. Component không gọi `invoke` trực tiếp.
 */
@Injectable({ providedIn: 'root' })
export class IpcService {
  /** `false` khi chạy bằng `ng serve` trong trình duyệt thường, không có lõi. */
  readonly available = isTauri();

  appInfo(): Promise<AppInfo> {
    return invoke<AppInfo>('app_info');
  }

  /** Mở thư mục hoặc file kết quả (.pdf/.docx) bằng Explorer. */
  shellOpen(path: string): Promise<void> {
    return invoke('shell_open', { path });
  }

  mergeInspectTemplate(path: string): Promise<TemplateInfo> {
    return invoke<TemplateInfo>('merge_inspect_template', { path });
  }

  /** `sheet` null = sheet đầu tiên; `headerRow` null = dòng đầu tiên có dữ liệu. */
  mergeReadTable(path: string, sheet: string | null, headerRow: number | null): Promise<TableInfo> {
    return invoke<TableInfo>('merge_read_table', { path, sheet, headerRow });
  }

  /** Tự ghép biến với cột theo tên (bỏ dấu, không phân biệt hoa thường). Trả về số cột hoặc null. */
  mergeAutoMap(fields: string[], columns: TableColumn[]): Promise<Record<string, number | null>> {
    return invoke('merge_auto_map', { fields, columns });
  }

  /** Kiểm tra và lập danh sách file sẽ tạo (tên file, cảnh báo). Không ghi gì xuống đĩa. */
  mergePlan(request: MergeRequest): Promise<MergePlan> {
    return invoke<MergePlan>('merge_plan', { request });
  }

  /** Tạo PDF xem trước cho một dòng (null = dòng đầu). Cần Word. */
  mergePreview(request: MergeRequest, rowNumber: number | null, highlight = false): Promise<PreviewResult> {
    return invoke<PreviewResult>('merge_preview', { request, rowNumber, highlight });
  }

  /** PDF của mẫu gốc (chưa điền), để xem ở tab Mẫu; `highlight` thì tô màu các biến. Cần Word. */
  mergePreviewTemplate(templatePath: string, highlight = false): Promise<PreviewResult> {
    return invoke<PreviewResult>('merge_preview_template', { templatePath, highlight });
  }

  /** Chạy lô tạo văn bản; tiến độ đổ về `onEvent`. Trả về mã job để Dừng. */
  mergeRun(request: MergeRequest, onEvent: (event: MergeEvent) => void): Promise<string> {
    const channel = new Channel<MergeEvent>();
    channel.onmessage = onEvent;
    return invoke<string>('merge_run', { request, onEvent: channel });
  }

  jobCancel(jobId: string): Promise<void> {
    return invoke('job_cancel', { jobId });
  }

  // ---- Ký số --------------------------------------------------------------------------------

  /** Quét chứng thư trong kho Windows, kèm lý do không ký được của từng cái. */
  certList(): Promise<CertScan> {
    return invoke<CertScan>('cert_list');
  }

  /** Mở token và ký thử. Middleware của token có thể tự hiện hộp nhập PIN; có thể mất một lúc. */
  /**
   * Mở token và ký thử. `pin`: PIN người dùng gõ (trống: dùng PIN đã lưu nếu có, không thì middleware tự hỏi);
   * `savePin`: lưu PIN vừa gõ trên máy (DPAPI) khi mở được token.
   */
  tokenCheck(thumbprint: string, pin: string | null = null, savePin = false): Promise<TokenInfo> {
    return invoke<TokenInfo>('token_check', { thumbprint, pin, savePin });
  }

  /** Trạng thái driver USB Token trên máy (Hỗ trợ › Cài đặt driver USB Token). */
  driverStatus(): Promise<DriverStatus> {
    return invoke<DriverStatus>('driver_status');
  }

  /** Mở trang web cố định (`driver`: trang tải phần mềm của Ban Cơ yếu; `home`: saomaiso.com). */
  openLink(id: 'driver' | 'home'): Promise<void> {
    return invoke('open_link', { id });
  }

  /** Thumbprint các chứng thư có PIN lưu trên máy. */
  pinSaved(): Promise<string[]> {
    return invoke<string[]>('pin_saved');
  }

  /** Xoá PIN đã lưu của một chứng thư. */
  pinForget(thumbprint: string): Promise<boolean> {
    return invoke<boolean>('pin_forget', { thumbprint });
  }

  tokenForget(): Promise<void> {
    return invoke('token_forget');
  }

  /** Liệt kê PDF từ các file / thư mục (thư mục: chỉ cấp đầu, sắp xếp tự nhiên). */
  signListFiles(sources: string[]): Promise<SignFileItem[]> {
    return invoke<SignFileItem[]>('sign_list_files', { sources });
  }

  /** Cấp quyền đọc qua asset protocol cho một PDF đã liệt kê hoặc file đã ký; trả lại đúng đường dẫn đó. */
  signPreview(path: string): Promise<string> {
    return invoke<string>('sign_preview', { path });
  }

  /** Kiểm mọi chữ ký của một PDF (màn Kiểm tra). `revocation`: kiểm thu hồi qua CRL/OCSP (cần mạng). */
  verifyFile(path: string, revocation: boolean): Promise<VerifyReport> {
    return invoke<VerifyReport>('verify_file', { path, revocation });
  }

  /** In khối báo cáo đang hiện (chỉ hiện khi in) ra file PDF `path`. */
  reportExportPdf(path: string): Promise<void> {
    return invoke('report_export_pdf', { path });
  }

  /** Chỗ các khối chữ ký sẽ đặt trong file (để vẽ khung xem trước đúng như khi ký). */
  signPositions(path: string, appearance: SignAppearance, signer: string): Promise<SignPlacement[]> {
    return invoke<SignPlacement[]>('sign_positions', { path, appearance, signer });
  }

  signPlanOutputs(files: string[], output: SignOutputOptions): Promise<PlannedOutput[]> {
    return invoke<PlannedOutput[]>('sign_plan_outputs', { files, output });
  }

  /**
   * Bố cục chữ ký của nhiều file: khung riêng trong `placements`, không có thì `appearance.placements`, rồi mẫu;
   * kiểu hiển thị riêng trong `modes`, không có thì `appearance.mode`.
   */
  signLayouts(
    paths: string[],
    appearance: SignAppearance,
    placements: Record<string, SignPlacement[]>,
    modes: Record<string, SignatureMode>,
    signer: string,
  ): Promise<FileLayoutItem[]> {
    return invoke<FileLayoutItem[]>('sign_layouts', { paths, appearance, placements, modes, signer });
  }

  configDefaults(): Promise<AppConfig> {
    return invoke<AppConfig>('config_defaults');
  }

  configGet(): Promise<ConfigLoad> {
    return invoke<ConfigLoad>('config_get');
  }

  /** Lõi làm sạch giá trị rồi lưu; trả lại cấu hình đã lưu. */
  configSave(config: AppConfig): Promise<ConfigSaved> {
    return invoke<ConfigSaved>('config_save', { config });
  }

  /** Cấp quyền đọc ảnh chữ ký / con dấu đã chọn (vẽ khung chữ ký trên bản xem trước); trả lại đường dẫn. */
  signImagePreview(path: string): Promise<string> {
    return invoke<string>('sign_image_preview', { path });
  }

  // ---- Thông báo Telegram ------------------------------------------------------------------

  notifyStatus(): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_status');
  }

  /** Trả lời hộp đăng ký thiết bị. Đồng ý thì gửi thông tin thiết bị một lần; từ chối thì không gửi gì. */
  notifyConsent(accept: boolean, includeIp: boolean, includeMac: boolean): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_consent', { accept, includeIp, includeMac });
  }

  notifyUnregister(): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_unregister');
  }

  /** Đổi các công tắc của người dùng; lõi từ chối khi quản trị viên đã khoá. */
  notifySetPrefs(prefs: NotifyFlags): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_set_prefs', { prefs });
  }

  /** Kiểm tra kết nối; kết quả nằm trong `connection`. */
  notifyCheck(): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_check');
  }

  /** Gửi một tin nhắn thử nghiệm (cần đã đồng ý đăng ký thiết bị). */
  notifyTest(): Promise<NotifyStatus> {
    return invoke<NotifyStatus>('notify_test');
  }

  /** Chạy lô ký; tiến độ đổ về `onEvent`. Trả về mã job để Dừng. */
  signRun(thumbprint: string, request: SignJobRequest, onEvent: (event: SignEvent) => void): Promise<string> {
    const channel = new Channel<SignEvent>();
    channel.onmessage = onEvent;
    return invoke<string>('sign_run', { thumbprint, request, onEvent: channel });
  }
}
