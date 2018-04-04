use actix::prelude::*;
use lettre::{EmailTransport, SmtpTransport};
use lettre::smtp::authentication::Credentials;
use lettre_email::EmailBuilder;

use Result;
use db::models::{Gender, Teilnehmer};

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

		// Send to additional receivers in bcc
		for receiver in &self.config.additional_mail_receivers {
			email_builder.add_bcc(receiver.clone());
		}

		let email = email_builder.build()?;

		let mut mailer = SmtpTransport::simple_builder(
			self.config.sender_mail_account.host.clone(),
		)?.credentials(Credentials::new(
			self.config.sender_mail_account.name.clone().unwrap_or_else(||
				self.config.sender_mail.address.clone()),
			self.config.sender_mail_account.password.clone(),
		))
			.build();

		// Send the email
		mailer.send(&email)?;

		Ok(())
	}
}
