use lettre::{EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;
use std::path::Path;
use mime;
use Result;
use lettre::smtp::authentication::Credentials;
use AppState;

pub struct MailData {
    pub parent_mail: String,
    pub parent_name: String,
    pub child_first_name: String,
    pub child_last_name: String
}

pub fn send_mail(maildata: MailData, state: &AppState) -> Result<()> {
    let email = EmailBuilder::new()
        .to((maildata.parent_mail, maildata.parent_name))
        .from(("test@flakebi.de", "Die Zeltlager Betreuer"))
        .subject("Anmeldung für das Zeltlager 2018 - Zahlungsaufforderung")
        .text("Bitte Zahlen 😄")
        .build()?;
    
    println!("{:?}", email);

    // Open a local connection on port 25
    let mut mailer = SmtpTransport::simple_builder("mail.flakebi.de".to_string())?
        .credentials(Credentials::new(state.config.email_username.clone(), state.config.email_password.clone()))
        .build();
    // Send the email
    mailer.send(&email)?;

    Ok(())
}