use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct OutgoingMail {
    pub to: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

pub trait MailClient: Send + Sync {
    fn send(
        &self,
        msg: OutgoingMail,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + Send + '_>>;
}
