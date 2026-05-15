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
pub trait MailClient: Send + Sync {
    fn send(
        &self,
        msg: OutgoingMail,
    ) -> impl std::future::Future<Output = Result<(), AppError>> + Send;
}
