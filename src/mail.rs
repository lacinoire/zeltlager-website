use std::convert::TryInto;
use std::str::FromStr;

use actix::prelude::*;
use anyhow::Result;
use lettre::message::header;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use t4rust_derive::Template;

use crate::config::Config;
use crate::db::models::{Gender, Teilnehmer};

pub struct MailExecutor {
	config: Config,
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
	pub fn new(config: Config) -> Self { Self { config } }
}

impl Handler<SignupMessage> for MailExecutor {
	type Result = Result<()>;

	fn handle(&mut self, msg: SignupMessage, _: &mut Self::Context) -> Self::Result {
		let subject = format!("{}", Subject { member: &msg.member }).trim().to_string();
		let body = format!("{}", Body { member: &msg.member }).trim().to_string();

		let mut email_builder = lettre::Message::builder()
			.to((msg.member.eltern_name.clone(), msg.member.eltern_mail.clone()).try_into()?)
			.header(header::ContentType::TEXT_PLAIN)
			.from(self.config.sender_mail.clone().try_into()?)
			.subject(subject);

		if self.config.test_mail.as_ref().map(|m| m != &msg.member.eltern_mail).unwrap_or(true) {
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
