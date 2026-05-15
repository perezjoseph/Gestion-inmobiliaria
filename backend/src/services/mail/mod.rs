pub mod client;
pub mod message;

pub use client::{MailClient, OutgoingMail};
pub use message::signature_link_mail;
