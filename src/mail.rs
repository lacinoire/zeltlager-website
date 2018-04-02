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

		let email = EmailBuilder::new()
			.to((
				msg.member.eltern_mail.clone(),
				msg.member.eltern_name.clone(),
			))
			.from((
				self.config.email_username.clone(),
				self.config.email_userdescription.clone(),
			))
			.subject(subject)
			.text(body)
			.build()?;

		let mut mailer = SmtpTransport::simple_builder(
			self.config.email_host.clone(),
		)?.credentials(Credentials::new(
			self.config.email_username.clone(),
			self.config.email_password.clone(),
		))
			.build();
		// Send the email
		mailer.send(&email)?;

		Ok(())
	}
}
