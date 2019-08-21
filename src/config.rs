use std::path::PathBuf;

use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
	raw(
		global_settings = "&[AppSettings::ColoredHelp, \
		                   AppSettings::VersionlessSubcommands]"
	)
)]
pub struct Args {
	#[structopt(subcommand, help = "Default action is to start the server")]
	pub action: Option<Action>,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "action")]
pub enum Action {
	#[structopt(
		name = "adduser",
		help = "Add a user to the database.\nIt will ask for the password on the \
		        command line"
	)]
	AddUser {
		#[structopt(name = "username", help = "Name of the added user")]
		username: Option<String>,
		#[structopt(
			name = "force",
			long = "force",
			short = "f",
			help = "Overwrite password of user without asking"
		)]
		force: bool,
	},
	#[structopt(name = "deluser", help = "Remove a user from the database")]
	DelUser {
		#[structopt(name = "username", help = "Name of the user to delete")]
		username: Option<String>,
	},
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAddress {
	pub name: Option<String>,
	pub address: String,
}

fn submission_port() -> u16 { 587 }

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAccount {
	/// Host for smtp.
	pub host: String,
	/// Username to login to smtp.
	pub name: Option<String>,
	/// Password to login to smtp.
	pub password: String,
	#[serde(default = "submission_port")]
	pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscourseConfig {
	/// The api endpoint, something like `https://discourse.example.com`.
	pub endpoint: String,
	/// The api token.
	pub token: String,
	/// The username for the api.
	pub username: String,
	/// Add new users to this group.
	pub group: String,
	/// Subscribe new users to this category.
	pub category: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
	/// The sender of emails
	pub sender_mail: MailAddress,
	pub sender_mail_account: MailAccount,

	/// E-Mail addresses which should also receive all signup-confirmation
	/// mails.
	pub additional_mail_receivers: Vec<MailAddress>,
	/// If a member signs up with this mail address, the signup mail will only
	/// be sent to this address, but not to additional receivers. The member
	/// will also not be entered into the database.
	pub test_mail: Option<String>,

	/// The maximum allowed amount of members.
	pub max_members: i64,
	/// The message which will be shown when the maximum number of members is
	/// reached.
	pub max_members_reached: String,
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

	/// The configuration of the discourse integration.
	pub discourse: Option<DiscourseConfig>,

	/// The sentry DSN.
	pub sentry: Option<String>,

	/// Path to a log file to log signups.
	pub log_file: Option<PathBuf>,
}

fn default_bind_address() -> String {
	String::from("127.0.0.1:8080")
}
