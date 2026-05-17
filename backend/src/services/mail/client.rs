use crate::errors::AppError;

/// Represents an outgoing email message.
#[derive(Debug, Clone)]
pub struct OutgoingMail {
    pub to: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

/// Abstraction over outbound email delivery.
///
/// Implementations may use SMTP (production), file transport (tests),
/// or an in-memory sink (unit tests).
///
/// Uses `Box<dyn Future>` for dyn-compatibility so the trait can be
/// stored as `Arc<dyn MailClient>` in `AppState`.
pub trait MailClient: Send + Sync {
    fn send(
        &self,
        msg: OutgoingMail,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + Send + '_>>;
}
