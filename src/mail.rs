use std::convert::TryInto;
use std::str::FromStr;

use anyhow::Result;
use lettre::message::header;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport};
use t4rust_derive::Template;

use crate::config::{Config, MailAddress};
use crate::db::models::{
	FullSupervisor, FullTeilnehmer, Gender, Supervisor, Teilnehmer, years_old,
};
use crate::{GERMAN_DATE_FORMAT, LAGER_START};

#[derive(Clone, Debug)]
pub struct Mail {
	config: Config,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-subject.tt"]
struct Subject<'a> {
	member: &'a Teilnehmer,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-body.tt"]
struct Body<'a> {
	member: &'a Teilnehmer,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-payed-subject.tt"]
struct PayedSubject<'a> {
	member: &'a FullTeilnehmer,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-payed-body.tt"]
struct PayedBody<'a> {
	member: &'a FullTeilnehmer,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/resignup-mail.tt"]
struct ResignupBody<'a> {
	pub supervisor: &'a FullSupervisor,
	pub token: &'a str,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-presignup-body.tt"]
pub struct PresignupBody<'a> {
	receiver: &'a MailAddress,
	supervisor: &'a Supervisor,
	grund: &'a str,
	kommentar: &'a str,
}

#[derive(Debug, Template)]
#[TemplatePath = "templates/mail-presignup-failed-body.tt"]
pub struct PresignupFailedBody<'a> {
	supervisor: &'a Supervisor,
}

impl Mail {
	pub fn new(config: Config) -> Self { Self { config } }

	fn mailer(&self) -> Result<AsyncSmtpTransport<lettre::Tokio1Executor>> {
		Ok(AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(
			self.config.sender_mail_account.host.as_str(),
		)?
		.credentials(Credentials::new(
			self.config
				.sender_mail_account
				.name
				.clone()
				.unwrap_or_else(|| self.config.sender_mail.address.clone()),
			self.config.sender_mail_account.password.clone(),
		))
		.build())
	}

	async fn send_eltern(
		&self, eltern_name: &str, eltern_mail: &str, subject: String, body: String,
	) -> Result<()> {
		let mut email_builder = lettre::Message::builder()
			.to((eltern_name, eltern_mail).try_into()?)
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

		// Send the email
		self.mailer()?.send(email).await?;

		Ok(())
	}

	pub async fn send_member_signup(&self, member: &Teilnehmer) -> Result<()> {
		let subject = format!("{}", Subject { member }).trim().to_string();
		let body = format!("{}", Body { member }).trim().to_string();

		self.send_eltern(&member.eltern_name, &member.eltern_mail, subject, body).await
	}

	pub async fn send_member_payed(&self, member: &FullTeilnehmer) -> Result<()> {
		let subject = format!("{}", PayedSubject { member }).trim().to_string();
		let body = format!("{}", PayedBody { member }).trim().to_string();

		self.send_eltern(&member.eltern_name, &member.eltern_mail, subject, body).await
	}

	pub async fn send_supervisor_resignup(
		&self, supervisor: &FullSupervisor, token: &str,
	) -> Result<()> {
		let full_name = format!("{} {}", supervisor.vorname, supervisor.nachname);

		let subject = format!("Zeltlager {} Betreueranmeldung", LAGER_START.year());
		let body = format!("{}", ResignupBody { supervisor, token }).trim().to_string();

		let email = lettre::Message::builder()
			.to((full_name, &supervisor.mail).try_into()?)
			.header(header::ContentType::TEXT_PLAIN)
			.from(self.config.sender_mail.clone().try_into()?)
			.subject(subject)
			.body(body)?;

		// Send the email
		self.mailer()?.send(email).await?;
		Ok(())
	}

	pub async fn send_supervisor_presignup(
		&self, supervisor: &Supervisor, grund: &str, kommentar: &str,
	) -> Result<()> {
		let subject =
			format!("Zeltlager Betreueranmeldung {} {}", supervisor.vorname, supervisor.nachname);
		let mailer = self.mailer()?;

		for receiver in &self.config.supervisor_mail_receivers {
			let body = format!("{}", PresignupBody { receiver, supervisor, grund, kommentar })
				.trim()
				.to_string();

			let email = lettre::Message::builder()
				.to(receiver.clone().try_into()?)
				.header(header::ContentType::TEXT_PLAIN)
				.from(self.config.sender_mail.clone().try_into()?)
				.subject(&subject)
				.body(body)?;

			// Send the email
			mailer.send(email).await?;
		}

		Ok(())
	}

	pub async fn send_supervisor_presignup_failed(&self, supervisor: &Supervisor) -> Result<()> {
		let subject = "Zeltlager Betreueranmeldung fehlgeschlagen";

		let body = format!("{}", PresignupFailedBody { supervisor }).trim().to_string();

		let name = format!("{} {}", supervisor.vorname, supervisor.nachname);
		let email = lettre::Message::builder()
			.to((name, &supervisor.mail).try_into()?)
			.header(header::ContentType::TEXT_PLAIN)
			.from(self.config.sender_mail.clone().try_into()?)
			.subject(subject)
			.body(body)?;

		// Send the email
		self.mailer()?.send(email).await?;

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
