use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
	/// Default action is to start the server
	#[command(subcommand)]
	pub action: Option<Action>,
}

#[derive(Subcommand, Debug)]
pub enum Action {
	/// Add a user to the database.
	/// It will ask for the password on the command line.
	#[command(name = "adduser")]
	AddUser {
		/// Name of the added user
		username: Option<String>,
		/// Overwrite password of user without asking
		#[arg(long, short)]
		force: bool,
	},
	/// Remove a user from the database
	#[command(name = "deluser")]
	DelUser {
		/// Name of the user to delete
		username: Option<String>,
	},
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAddress {
	pub name: Option<String>,
	pub address: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAccount {
	/// Host for smtp.
	pub host: String,
	/// Username to login to smtp.
	pub name: Option<String>,
	/// Password to login to smtp.
	pub password: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
	/// The sender of emails
	pub sender_mail: MailAddress,
	pub sender_mail_account: MailAccount,

	/// E-Mail addresses that receive mails for supervisor pre-signups.
	#[serde(default)]
	pub supervisor_mail_receivers: Vec<MailAddress>,
	/// E-Mail addresses which should also receive all signup-confirmation
	/// mails.
	#[serde(default)]
	pub additional_mail_receivers: Vec<MailAddress>,
	/// If a member signs up with this mail address, the signup mail will only
	/// be sent to this address, but not to additional receivers. The member
	/// will also not be entered into the database.
	pub test_mail: Option<String>,

	/// Postgresql connection url
	pub database: String,

	/// The maximum allowed amount of members.
	pub max_members: i64,
	/// An error message, which will be displayed on generic errors.
	///
	/// Put here something like: Please write us an e-mail.
	pub error_message: String,
	/// Address to bind to.
	#[serde(default = "default_bind_address")]
	pub bind_address: String,
	/// A message which will be displayed on top of all basic templated sites.
	pub global_message: Option<String>,
	/// If this site is served over https.
	///
	/// If `true`, the authentication cookie can only be transfered over https.
	#[serde(default = "crate::get_true")]
	pub secure: bool,
	/// This should be the domain the server.
	///
	/// If set, it restricts the authentication cookie to a domain
	/// and protects against csrf using the referer and origin header.
	pub domain: Option<String>,

	/// Path to a log file to log signups.
	pub log_file: Option<PathBuf>,
}

fn default_bind_address() -> String { String::from("127.0.0.1:8080") }
