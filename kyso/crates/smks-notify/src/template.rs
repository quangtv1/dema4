//! Nội dung tin nhắn Telegram (`parse_mode=HTML`) của từng loại sự kiện.
//!
//! Máy trạm (chế độ gửi trực tiếp) và gateway dùng chung hàm [`render`], nên tin nhắn giống nhau ở cả hai chế độ, và
//! ở chế độ gateway máy trạm không tự soạn được nội dung tuỳ ý.

use crate::event::{Event, EventType, Metadata};
use crate::text::html_escape;
use crate::time;

/// Giới hạn của Telegram là 4096 ký tự; chừa chỗ cho thẻ HTML.
const MAX_CHARS: usize = 3800;

struct Message(String);

impl Message {
    fn new(event: &Event, intro: &str) -> Self {
        Self(format!(
            "<b>[SAO MAI SIGN - {}]</b>\n\n{}\n\n",
            event.event_type.tag(),
            html_escape(intro)
        ))
    }

    /// Dòng `Nhãn: giá trị`; bỏ qua khi giá trị rỗng.
    fn line(&mut self, label: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.0
                .push_str(&format!("{label}: {}\n", html_escape(value)));
        }
        self
    }

    /// Như [`Self::line`] nhưng giá trị ở dạng chữ đều (mã định danh, dễ chép).
    fn code(&mut self, label: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.0
                .push_str(&format!("{label}: <code>{}</code>\n", html_escape(value)));
        }
        self
    }

    fn bullet(&mut self, text: &str) -> &mut Self {
        self.0.push_str(&format!(" • {}\n", html_escape(text)));
        self
    }

    fn time(&mut self, event: &Event) -> &mut Self {
        let shown = format!("{} (GMT+7)", time::display(event.timestamp));
        self.line("Thời gian", &shown)
    }

    /// Một văn bản: `Tên văn bản: a.pdf`; nhiều văn bản: số lượng và danh sách tên.
    fn documents(&mut self, m: &Metadata) -> &mut Self {
        let total = m.document_count.max(m.documents.len() as u32);
        match (total, m.documents.first()) {
            (0, _) | (_, None) => {}
            (1, Some(name)) => {
                self.line("Tên văn bản", name);
            }
            (_, Some(_)) => {
                self.line("Số văn bản", &total.to_string());
                for name in &m.documents {
                    self.bullet(name);
                }
                let rest = total as usize - m.documents.len().min(total as usize);
                if rest > 0 {
                    self.bullet(&format!("… và {rest} văn bản khác"));
                }
            }
        }
        self
    }

    /// Danh sách văn bản lỗi kèm lý do.
    fn failures(&mut self, m: &Metadata) -> &mut Self {
        match m.failures.as_slice() {
            [] => {}
            [one] if m.document_count <= 1 => {
                self.line("Tên văn bản", &one.name);
            }
            many => {
                let total = (m.document_count as usize).max(many.len());
                self.line("Số văn bản lỗi", &total.to_string());
                for f in many {
                    self.bullet(&format!("{} — {}", f.name, f.message));
                }
                if total > many.len() {
                    self.bullet(&format!("… và {} văn bản khác", total - many.len()));
                }
            }
        }
        self
    }

    fn finish(mut self, status: &str) -> String {
        self.0.push_str(&format!("\nTrạng thái: {status}"));
        if self.0.chars().count() > MAX_CHARS {
            let mut cut: String = self.0.chars().take(MAX_CHARS).collect();
            // Không cắt giữa một thẻ hay một thực thể HTML.
            if let Some(i) = cut.rfind('\n') {
                cut.truncate(i);
            }
            cut.push_str("\n…\n\nTrạng thái: ");
            cut.push_str(status);
            return cut;
        }
        self.0
    }
}

/// Soạn tin nhắn của sự kiện.
pub fn render(event: &Event) -> String {
    let m = &event.metadata;
    let user = if m.user_name.is_empty() {
        &event.user_id
    } else {
        &m.user_name
    };
    match event.event_type {
        EventType::DeviceRegistered => {
            let mut t = Message::new(event, "Thiết bị mới được đăng ký.");
            t.code("Device ID", &event.device_id)
                .line("Tên máy", &m.hostname)
                .line("Hệ điều hành", &m.os)
                .line("Phiên bản OS", &m.os_version)
                .line("Kiến trúc", &m.architecture)
                .line("IP nội bộ", &m.local_ip)
                .line("Địa chỉ MAC", &m.mac_address)
                .line("Phiên bản ứng dụng", &m.app_version)
                .time(event)
                .line("Ghi chú", &m.note);
            t.finish("Đăng ký thành công.")
        }
        EventType::DeviceUnregistered => {
            let mut t = Message::new(event, "Thiết bị đã hủy đăng ký.");
            t.code("Device ID", &event.device_id)
                .line("Tên máy", &m.hostname)
                .line("Người thực hiện", user)
                .line("Phiên bản ứng dụng", &m.app_version)
                .time(event);
            t.finish("Đã hủy đăng ký. Thiết bị không gửi thông báo nữa.")
        }
        EventType::AppUpdated => {
            let mut t = Message::new(event, "Ứng dụng đã được cập nhật.");
            t.line("Thiết bị", &m.hostname)
                .code("Device ID", &event.device_id)
                .line("Phiên bản cũ", &m.previous_version)
                .line("Phiên bản mới", &m.app_version)
                .time(event);
            t.finish("Thành công.")
        }
        EventType::SignSuccess => {
            let mut t = Message::new(event, "Nghiệp vụ ký số hoàn tất.");
            t.line("Thiết bị", &m.hostname)
                .code("Device ID", &event.device_id)
                .line("Người thực hiện", user)
                .documents(m)
                .line("Loại chữ ký", &m.signature_type)
                .line("Chứng thư", &m.signer)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id)
                .line("Ghi chú", &m.note);
            t.finish("Thành công.")
        }
        EventType::SignFailed => {
            let mut t = Message::new(event, "Nghiệp vụ ký số không thành công.");
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .documents(m)
                .failures(m)
                .line("Chứng thư", &m.signer)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id)
                .line("Mã lỗi", &m.error_code)
                .line("Thông báo lỗi", &m.error_message);
            if m.pending_count > 0 {
                t.line("Chưa ký", &format!("{} văn bản", m.pending_count));
            }
            t.finish("Thất bại.")
        }
        EventType::SignCancelled => {
            let mut t = Message::new(event, "Người dùng đã hủy / dừng nghiệp vụ ký số.");
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .documents(m)
                .line("Chứng thư", &m.signer)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id);
            if m.total_count > 0 {
                t.line("Tổng số văn bản", &m.total_count.to_string());
            }
            if m.pending_count > 0 {
                t.line("Chưa ký", &format!("{} văn bản", m.pending_count));
            }
            t.line("Ghi chú", &m.note);
            t.finish("Đã hủy.")
        }
        EventType::DocumentCreated | EventType::DocumentExported => {
            let intro = if event.event_type == EventType::DocumentCreated {
                "Văn bản mới được tạo."
            } else {
                "Văn bản đã được xuất."
            };
            let mut t = Message::new(event, intro);
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .documents(m)
                .line("Loại văn bản", &m.document_type)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id)
                .line("Ghi chú", &m.note);
            t.finish("Thành công.")
        }
        EventType::DocumentFailed => {
            let mut t = Message::new(event, "Không thể tạo văn bản.");
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .documents(m)
                .failures(m)
                .line("Loại văn bản", &m.document_type)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id)
                .line("Mã lỗi", &m.error_code)
                .line("Thông báo", &m.error_message);
            t.finish("Thất bại.")
        }
        EventType::DocumentCancelled => {
            let mut t = Message::new(event, "Người dùng đã dừng tạo văn bản.");
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .line("Loại văn bản", &m.document_type)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id);
            if m.total_count > 0 {
                t.line("Tổng số văn bản", &m.total_count.to_string());
            }
            if m.pending_count > 0 {
                t.line("Chưa tạo", &format!("{} văn bản", m.pending_count));
            }
            t.finish("Đã hủy.")
        }
        EventType::DocumentDeleted => {
            let mut t = Message::new(event, "Văn bản đã bị xóa.");
            t.line("Thiết bị", &m.hostname)
                .line("Người thực hiện", user)
                .documents(m)
                .time(event)
                .code("Mã giao dịch", &event.transaction_id)
                .line("Ghi chú", &m.note);
            t.finish("Thành công.")
        }
        EventType::Test => {
            let mut t = Message::new(event, "Tin nhắn thử nghiệm từ Sao Mai Ký số.");
            t.line("Thiết bị", &m.hostname)
                .code("Device ID", &event.device_id)
                .line("Người thực hiện", user)
                .line("Phiên bản ứng dụng", &m.app_version)
                .time(event);
            t.finish("Kết nối hoạt động.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Failure, Status};

    fn event(event_type: EventType, metadata: Metadata) -> Event {
        Event {
            event_id: "e-1".into(),
            event_type,
            device_id: "SMKS-0001".into(),
            user_id: "an.nv".into(),
            timestamp: 1_794_986_277,
            transaction_id: "KS-20261118-141757-a1b2c3".into(),
            status: Status::Success,
            metadata,
        }
    }

    #[test]
    fn new_device_follows_the_agreed_layout() {
        let text = render(&event(
            EventType::DeviceRegistered,
            Metadata {
                hostname: "PC-KETOAN".into(),
                os: "Windows 10 Pro".into(),
                os_version: "22H2 (build 19045.5131)".into(),
                architecture: "AMD64".into(),
                local_ip: "192.168.1.20".into(),
                app_version: "1.5.0".into(),
                ..Metadata::default()
            },
        ));
        assert_eq!(
            text,
            "<b>[SAO MAI SIGN - NEW DEVICE]</b>\n\nThiết bị mới được đăng ký.\n\n\
             Device ID: <code>SMKS-0001</code>\nTên máy: PC-KETOAN\nHệ điều hành: Windows 10 Pro\n\
             Phiên bản OS: 22H2 (build 19045.5131)\nKiến trúc: AMD64\nIP nội bộ: 192.168.1.20\n\
             Phiên bản ứng dụng: 1.5.0\nThời gian: 18/11/2026 14:17:57 (GMT+7)\n\n\
             Trạng thái: Đăng ký thành công."
        );
        assert!(
            !text.contains("MAC"),
            "không có MAC khi người dùng không cho"
        );
    }

    #[test]
    fn sign_success_single_and_batch() {
        let mut m = Metadata {
            hostname: "PC-1".into(),
            signature_type: "PAdES B-T · USB Token".into(),
            signer: "Nguyễn Văn A".into(),
            ..Metadata::default()
        };
        m.set_documents(["QD <01>.pdf"]);
        let one = render(&event(EventType::SignSuccess, m.clone()));
        assert!(one.contains("Tên văn bản: QD &lt;01&gt;.pdf\n"));
        assert!(one.contains("Người thực hiện: an.nv\n"));
        assert!(one.contains("Mã giao dịch: <code>KS-20261118-141757-a1b2c3</code>\n"));
        assert!(one.ends_with("Trạng thái: Thành công."));

        m.set_documents((1..=13).map(|i| format!("vb {i}.pdf")));
        let many = render(&event(EventType::SignSuccess, m));
        assert!(many.contains("Số văn bản: 13\n • vb 1.pdf\n"));
        assert!(many.contains(" • … và 3 văn bản khác\n"));
        assert!(!many.contains("vb 11.pdf"));
    }

    #[test]
    fn failures_and_cancel() {
        let mut m = Metadata {
            hostname: "PC-1".into(),
            error_code: "1032".into(),
            error_message: "File PDF được mã hóa".into(),
            ..Metadata::default()
        };
        m.set_failures([Failure {
            name: "a.pdf".into(),
            code: String::new(),
            message: "File PDF được mã hóa".into(),
        }]);
        let one = render(&event(EventType::SignFailed, m.clone()));
        assert!(one.contains("Tên văn bản: a.pdf\n"));
        assert!(one.contains("Mã lỗi: 1032\nThông báo lỗi: File PDF được mã hóa\n"));
        assert!(one.ends_with("Trạng thái: Thất bại."));

        m.set_failures((1..=12).map(|i| Failure {
            name: format!("f{i}.pdf"),
            code: String::new(),
            message: "lỗi".into(),
        }));
        let many = render(&event(EventType::SignFailed, m));
        assert!(many.contains("Số văn bản lỗi: 12\n • f1.pdf — lỗi\n"));
        assert!(many.contains("… và 2 văn bản khác"));

        let cancel = render(&event(
            EventType::SignCancelled,
            Metadata {
                total_count: 60,
                pending_count: 10,
                note: "Người dùng bấm Tạm dừng".into(),
                ..Metadata::default()
            },
        ));
        assert!(cancel.contains("Tổng số văn bản: 60\nChưa ký: 10 văn bản\n"));
        assert!(cancel.ends_with("Trạng thái: Đã hủy."));
    }

    #[test]
    fn every_type_renders_within_the_telegram_limit() {
        let mut m = Metadata {
            hostname: "H".repeat(64),
            note: "n".repeat(200),
            error_message: "e".repeat(300),
            ..Metadata::default()
        };
        m.set_failures((0..10).map(|i| Failure {
            name: format!("{i}{}", "x".repeat(118)),
            code: String::new(),
            message: "m".repeat(300),
        }));
        m.documents = (0..10).map(|_| "d".repeat(120)).collect();
        for t in EventType::ALL {
            let text = render(&event(t, m.clone()));
            assert!(text.starts_with(&format!("<b>[SAO MAI SIGN - {}]</b>", t.tag())));
            assert!(text.chars().count() <= 4096, "{t:?}");
            assert!(text.contains("Trạng thái: "));
        }
    }
}
