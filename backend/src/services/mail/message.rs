use uuid::Uuid;

use super::client::OutgoingMail;

pub fn signature_link_mail(contrato_id: Uuid, link: &str) -> OutgoingMail {
    let subject = format!("Firma electrónica de su contrato #{contrato_id}");

    let body_text = format!(
        "Estimado/a inquilino/a,\n\n\
         Su contrato #{contrato_id} está listo para firma electrónica.\n\n\
         Por favor, acceda al siguiente enlace para firmar:\n\
         {link}\n\n\
         Este enlace es personal e intransferible.\n\n\
         Atentamente,\n\
         Administración de Propiedades"
    );

    let body_html = format!(
        "<p>Estimado/a inquilino/a,</p>\
         <p>Su contrato <strong>#{contrato_id}</strong> está listo para firma electrónica.</p>\
         <p>Por favor, acceda al siguiente enlace para firmar:</p>\
         <p><a href=\"{link}\">{link}</a></p>\
         <p>Este enlace es personal e intransferible.</p>\
         <p>Atentamente,<br/>Administración de Propiedades</p>"
    );

    OutgoingMail {
        to: String::new(),
        subject,
        body_html,
        body_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_link_mail_contains_contract_id_and_link() {
        let id = Uuid::new_v4();
        let link = "https://app.myhomeva.us/firmar/abc123";
        let mail = signature_link_mail(id, link);

        assert!(mail.subject.contains(&id.to_string()));
        assert!(mail.body_text.contains(link));
        assert!(mail.body_html.contains(link));
        assert!(mail.body_text.contains(&id.to_string()));
        assert!(mail.body_html.contains(&id.to_string()));
    }
}
