pub mod client;
pub mod message;
pub mod smtp;

pub use client::{MailClient, OutgoingMail};
pub use message::signature_link_mail;
pub use smtp::SmtpMailClient;
