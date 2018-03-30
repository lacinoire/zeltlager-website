use lettre::{EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;
use std::path::Path;
use mime;

pub struct MailData {
    pub parent_mail: String,
    pub parent_name: String,
    pub child_first_name: String,
    pub child_last_name: String
}

pub fn send_mail(maildata: MailData) -> Result<()> {
    let email = EmailBuilder::new()
        .to((maildata.parent_mail, maildata.parent_name))
        .from(("c.betreuer-zeltlager@flakebi.de", "Die Zeltlager Betreuer"))
        .subject("Anmeldung für das Zeltlager 2018 - Zahlungsaufforderung")
        .text("Bitte Zahlen 😄")
        .attachment(Path::new("Cargo.toml"), None, mime::TEXT_PLAIN)?
        .build()?;
    
    println!("{:?}", email);

    // Open a local connection on port 25
    let mut mailer = SmtpTransport::builder_unencrypted_localhost().unwrap().build();
    // Send the email
    mailer.send(&email)?;

    Ok(())
}