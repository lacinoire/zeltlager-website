use std::convert::TryInto;
use std::str::FromStr;

use actix::prelude::*;
use anyhow::Result;
use lettre::message::header;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use t4rust_derive::Template;

use crate::config::Config;
use crate::db::models::{FullSupervisor, FullTeilnehmer, Gender, Teilnehmer};
use crate::{GERMAN_DATE_FORMAT, LAGER_START};

pub struct MailExecutor {
	config: Config,
}

impl Actor for MailExecutor {
	type Context = SyncContext<Self>;
}

pub struct SignupMessage {
	pub member: Teilnehmer,
}

#[derive(Template)]
#[TemplatePath = "templates/resignup-mail.tt"]
#[derive(Debug)]
pub struct ResignupMessage {
	pub supervisor: FullSupervisor,
	pub token: String,
}

pub struct PayedMessage {
	pub member: FullTeilnehmer,
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

#[derive(Template)]
#[TemplatePath = "templates/mail-payed-subject.tt"]
#[derive(Debug)]
struct PayedSubject<'a> {
	member: &'a FullTeilnehmer,
}

#[derive(Template)]
#[TemplatePath = "templates/mail-payed-body.tt"]
#[derive(Debug)]
struct PayedBody<'a> {
	member: &'a FullTeilnehmer,
}

impl Message for SignupMessage {
	type Result = Result<()>;
}

impl Message for ResignupMessage {
	type Result = Result<()>;
}

impl Message for PayedMessage {
	type Result = Result<()>;
}

impl MailExecutor {
	pub fn new(config: Config) -> Self { Self { config } }

	fn send_eltern_mail(
		&self, eltern_name: String, eltern_mail: String, subject: String, body: String,
	) -> Result<()> {
		let mut email_builder = lettre::Message::builder()
			.to((eltern_name, eltern_mail.clone()).try_into()?)
			.header(header::ContentType::TEXT_PLAIN)
			.from(self.config.sender_mail.clone().try_into()?)
			.subject(subject);

		if self.config.test_mail.as_ref().map(|m| m != &eltern_mail).unwrap_or(true) {
			// Send to additional receivers in bcc
			for receiver in &self.config.additional_mail_receivers {
				email_builder = email_builder.bcc(receiver.clone().try_into()?);
			}
		}

		let email = email_builder.body(body)?;

		let mailer = SmtpTransport::starttls_relay(self.config.sender_mail_account.host.as_str())?
			.credentials(Credentials::new(
				self.config
					.sender_mail_account
					.name
					.clone()
					.unwrap_or_else(|| self.config.sender_mail.address.clone()),
				self.config.sender_mail_account.password.clone(),
			))
			.build();

		// Send the email
		mailer.send(&email)?;

		Ok(())
	}
}

impl Handler<SignupMessage> for MailExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: SignupMessage, _: &mut Self::Context) -> Self::Result {
		let subject = format!("{}", Subject { member: &msg.member }).trim().to_string();
		let body = format!("{}", Body { member: &msg.member }).trim().to_string();

		self.send_eltern_mail(msg.member.eltern_name.clone(), msg.member.eltern_mail, subject, body)
	}
}

impl Handler<ResignupMessage> for MailExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: ResignupMessage, _: &mut Self::Context) -> Self::Result {
		let full_name = format!("{} {}", msg.supervisor.vorname, msg.supervisor.nachname);

		let subject = format!("Zeltlager {} Betreueranmeldung", LAGER_START.year());
		let body = format!("{}", msg).trim().to_string();

		let email = lettre::Message::builder()
			.to((full_name, msg.supervisor.mail.clone()).try_into()?)
			.header(header::ContentType::TEXT_PLAIN)
			.from(self.config.sender_mail.clone().try_into()?)
			.subject(subject)
			.body(body)?;

		let mailer = SmtpTransport::starttls_relay(self.config.sender_mail_account.host.as_str())?
			.credentials(Credentials::new(
				self.config
					.sender_mail_account
					.name
					.clone()
					.unwrap_or_else(|| self.config.sender_mail.address.clone()),
				self.config.sender_mail_account.password.clone(),
			))
			.build();

		// Send the email
		mailer.send(&email)?;
		Ok(())
	}
}

impl Handler<PayedMessage> for MailExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: PayedMessage, _: &mut Self::Context) -> Self::Result {
		let subject = format!("{}", PayedSubject { member: &msg.member }).trim().to_string();
		let body = format!("{}", PayedBody { member: &msg.member }).trim().to_string();

		self.send_eltern_mail(msg.member.eltern_name.clone(), msg.member.eltern_mail, subject, body)
	}
}

pub fn check_parsable(mail_addr: &str) -> Result<()> {
	lettre::Address::from_str(mail_addr)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::check_parsable;

	#[test]
	fn test_parse_mails() {
		let mails = &["x@abc.de", "a.b@d.e", "my.long-mail_address@even-longer-domain.ending"];
		for &m in mails {
			check_parsable(m).unwrap();
		}
	}
}
