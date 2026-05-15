use std::pin::Pin;

use lettre::message::{Mailbox, MultiPart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::SmtpConfig;
use crate::errors::AppError;

use super::client::{MailClient, OutgoingMail};

/// Production SMTP mail client backed by `lettre`.
///
/// Connects to Mailcow SMTP via STARTTLS.
pub struct SmtpMailClient {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailClient {
    /// Build from environment-sourced SMTP configuration.
    pub fn from_config(cfg: &SmtpConfig) -> Result<Self, AppError> {
        let creds = Credentials::new(cfg.user.clone(), cfg.pass.clone());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .map_err(|e| {
                tracing::error!(error = %e, "Error configurando transporte SMTP");
                AppError::Internal(anyhow::anyhow!("Error configurando SMTP: {e}"))
            })?
            .port(cfg.port)
            .credentials(creds)
            .build();

        let from: Mailbox = cfg.from.parse().map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Dirección SMTP_FROM inválida: {e}"))
        })?;

        Ok(Self { transport, from })
    }

    async fn send_impl(&self, msg: OutgoingMail) -> Result<(), AppError> {
        let to_mailbox: Mailbox = msg
            .to
            .parse()
            .map_err(|e| AppError::Validation(format!("Dirección de correo inválida: {e}")))?;

        let email = Message::builder()
            .from(self.from.clone())
            .to(to_mailbox)
            .subject(&msg.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(msg.body_text),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(msg.body_html),
                    ),
            )
            .map_err(|e| {
                tracing::error!(error = %e, "Error construyendo mensaje de correo");
                AppError::Internal(anyhow::anyhow!("Error construyendo correo: {e}"))
            })?;

        self.transport.send(email).await.map_err(|e| {
            tracing::error!(error = %e, "Fallo al enviar correo SMTP");
            AppError::BadGateway("No se pudo enviar el correo".into())
        })?;

        Ok(())
    }
}

impl MailClient for SmtpMailClient {
    fn send(
        &self,
        msg: OutgoingMail,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + Send + '_>> {
        Box::pin(self.send_impl(msg))
    }
}
