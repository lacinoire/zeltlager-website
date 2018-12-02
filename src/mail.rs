use actix::prelude::*;
use lettre::smtp::authentication::Credentials;
use lettre::smtp::client::net::{DEFAULT_TLS_PROTOCOLS, ClientTlsParameters};
use lettre::smtp::ClientSecurity;
use lettre::{Transport, SmtpClient};
use lettre_email::EmailBuilder;
use native_tls::TlsConnector;

use db::models::{Gender, Teilnehmer};
use Result;

pub struct MailExecutor {
	config: ::Config,
}

impl Actor for MailExecutor {
	type Context = SyncContext<Self>;
}

pub struct SignupMessage {
	pub member: Teilnehmer,
}

impl Message for SignupMessage {
	type Result = Result<()>;
}

#[derive(Template)]
#[TemplatePath = "templates/mail-subject.tt"]
#[derive(Debug)]
struct Subject<'a> {
	member: &'a Teilnehmer,
}

#[derive(Template)]
#[TemplatePath = "templates/mail-body.tt"]
#[derive(Debug)]
struct Body<'a> {
	member: &'a Teilnehmer,
}

impl MailExecutor {
	pub fn new(config: ::Config) -> Self {
		Self { config }
	}
}

impl Handler<SignupMessage> for MailExecutor {
	type Result = Result<()>;

	fn handle(
		&mut self,
		msg: SignupMessage,
		_: &mut Self::Context,
	) -> Self::Result {
		let subject = format!(
			"{}",
			Subject {
				member: &msg.member
			}
		).trim()
			.to_string();
		let body = format!(
			"{}",
			Body {
				member: &msg.member
			}
		).trim()
			.to_string();

		let mut email_builder = EmailBuilder::new()
			.to((
				msg.member.eltern_mail.clone(),
				msg.member.eltern_name.clone(),
			))
			.from(self.config.sender_mail.clone())
			.subject(subject)
			.text(body);

		if self.config.test_mail.as_ref().map(|m| m != &msg.member.eltern_mail).unwrap_or(true) {
			// Send to additional receivers in bcc
			for receiver in &self.config.additional_mail_receivers {
				email_builder = email_builder.bcc(receiver.clone());
			}
		}

		let email = email_builder.build()?;

		let mut tls_builder = TlsConnector::builder();
			tls_builder.min_protocol_version(Some(DEFAULT_TLS_PROTOCOLS[0]));
		let tls_parameters = ClientTlsParameters::new(
			self.config.sender_mail_account.host.clone(),
			tls_builder.build().unwrap(),
		);
		let mut mailer = SmtpClient::new(
			(self.config.sender_mail_account.host.as_str(),
				self.config.sender_mail_account.port),
			ClientSecurity::Required(tls_parameters),
		)?.credentials(Credentials::new(
			self.config
				.sender_mail_account
				.name
				.clone()
				.unwrap_or_else(|| self.config.sender_mail.address.clone()),
			self.config.sender_mail_account.password.clone(),
		)).transport();

		// Send the email
		mailer.send(email.into())?;

		Ok(())
	}
}
