//! Gọi API Windows: đưa file vào Thùng rác (Recycle Bin) khi "Xóa file gốc" (ADR 0003 / D3).
//!
//! Chỉ đưa vào Thùng rác trên ổ đĩa cố định của máy (ổ mạng, USB thường không có Thùng rác; khi đó Windows
//! xoá hẳn). Có cờ `FOF_WANTNUKEWARNING` để nếu Windows vẫn không đưa được vào Thùng rác thì hỏi người dùng
//! thay vì xoá hẳn. Không chắc file đã vào Thùng rác thì báo lỗi; nơi gọi giữ file gốc.
#![allow(unsafe_code)]

use std::path::Path;

/// Đưa `path` vào Thùng rác. `Err`: lý do (tiếng Việt) để ghi vào kết quả, file gốc không bị xoá.
#[cfg(windows)]
pub fn move_to_recycle_bin(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
        FOF_WANTNUKEWARNING, SHFILEOPSTRUCTW, SHFileOperationW,
    };
    use windows::core::PCWSTR;

    const DRIVE_FIXED: u32 = 3;

    let full =
        std::path::absolute(path).map_err(|e| format!("không xác định được đường dẫn ({e})"))?;
    let root = full
        .ancestors()
        .last()
        .map(|r| r.as_os_str().to_owned())
        .unwrap_or_default();
    let mut root_w: Vec<u16> = root.encode_wide().collect();
    if !root_w.ends_with(&[u16::from(b'\\')]) {
        root_w.push(u16::from(b'\\'));
    }
    root_w.push(0);
    // SAFETY: `root_w` là chuỗi UTF-16 kết thúc bằng 0, sống tới hết lời gọi.
    let kind = unsafe { GetDriveTypeW(PCWSTR(root_w.as_ptr())) };
    if kind != DRIVE_FIXED {
        return Err("file không nằm trên ổ đĩa cố định của máy nên không có Thùng rác".into());
    }

    // pFrom: danh sách đường dẫn, mỗi đường dẫn kết thúc bằng 0, cả danh sách thêm một số 0 nữa.
    let from: Vec<u16> = full.as_os_str().encode_wide().chain([0, 0]).collect();
    let flags =
        FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT | FOF_WANTNUKEWARNING;
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        // Các cờ FOF_* đều nằm trong 16 bit thấp (trường `fFlags` là u16).
        fFlags: (flags.0 & 0xFFFF) as u16,
        ..Default::default()
    };
    // SAFETY: `op` hợp lệ; `from` kết thúc bằng hai số 0 và sống tới hết lời gọi; `pTo` rỗng với FO_DELETE.
    let rc = unsafe { SHFileOperationW(&mut op) };
    if rc != 0 {
        return Err(format!("Windows không đưa được vào Thùng rác (mã {rc})"));
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err("đã huỷ đưa vào Thùng rác".into());
    }
    if full.exists() {
        return Err("file gốc vẫn còn sau khi đưa vào Thùng rác".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn move_to_recycle_bin(_path: &Path) -> Result<(), String> {
    Err("chỉ hỗ trợ Thùng rác trên Windows".into())
}
