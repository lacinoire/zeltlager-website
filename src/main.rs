extern crate actix;
extern crate actix_web;
extern crate bytes;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lettre;
extern crate lettre_email;
#[macro_use]
extern crate log;
extern crate mime;
extern crate pulldown_cmark;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate t4rust_derive;
extern crate toml;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;

use actix_web::http::Method;
use actix_web::*;

mod basic;
mod db;
mod form;
mod mail;
mod signup;
mod signup_supervisor;

type Result<T> = std::result::Result<T, failure::Error>;
type BoxFuture<T> = Box<futures::Future<Item = T, Error = failure::Error>>;

const DEFAULT_PREFIX: &str = "public";
const DEFAULT_NAME: &str = "startseite";

/*macro_rules! tryf {
	($e:expr) => {
		match $e {
			Ok(e) => e,
			Err(error) => return Box::new(future::err(error.into())),
		}
	};
}*/

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAddress {
	name: Option<String>,
	address: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAccount {
	/// Host for smtp.
	host: String,
	/// Username to login to smtp.
	name: Option<String>,
	/// Password to login to smtp.
	password: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
	/// The sender of emails
	sender_mail: MailAddress,
	sender_mail_account: MailAccount,

	/// E-Mail addresses which should also receive all signup-confirmation
	/// mails.
	additional_mail_receivers: Vec<MailAddress>,

	/// The maximum allowed amount of members.
	max_members: i64,
	/// The message which will be shown when the maximum number of members is
	/// reached.
	max_members_reached: String,
	/// An error message, which will be displayed on generic errors.
	///
	/// Put here something like: Please write us an e-mail.
	error_message: String,
	/// Address to bind to.
	///
	/// # Default
	///
	/// 127.0.0.0:8080
	bind_address: Option<String>,
	/// A message which will be displayed on top of all basic templated sites.
	global_message: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
	sites: HashMap<String, basic::SiteDescriptions>,
	config: Config,
	db_addr: actix::Addr<actix::Syn, db::DbExecutor>,
	mail_addr: actix::Addr<actix::Syn, mail::MailExecutor>,
}
impl lettre_email::IntoMailbox for MailAddress {
	fn into_mailbox(self) -> lettre_email::Mailbox {
		lettre_email::Mailbox {
			name: self.name,
			address: self.address,
		}
	}
}
/// Escapes a string so it can be put into html (between tags).
///
/// # Escapes
///
/// - & to &amp;
/// - < to &lt;
/// - > to &gt;
/// - " to &quot;
/// - ' to &#x27;
/// - / to &#x2F;
///
/// Reference: https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet#RULE_.231_-_HTML_Escape_Before_Inserting_Untrusted_Data_into_HTML_Element_Content
fn escape_html_body(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
		.replace('/', "&#x2F;")
}

/// Escape a string so it can be put into a html attribute.
///
/// # Example
///
/// Put a string into `<inupt value="*your string goes here*"/>`. You need to
/// use double quotes then.
///
/// # Escapes
///
/// - & to &amp;
/// - < to &lt;
/// - " to &quot;
///
/// Reference: https://stackoverflow.com/a/9189067
fn escape_html_attribute(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('"', "&quot;")
}

fn sites(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	{
		let prefix;
		let name;
		if let Some(n) = req.match_info().get("name").and_then(|s| {
			if s.is_empty() {
				None
			} else {
				Some(s)
			}
		}) {
			if let Some(p) = req.match_info().get("prefix") {
				prefix = p;
				name = n;
			} else {
				// Check if the name is actually a prefix
				if req.state().sites.get(n).is_some() {
					prefix = n;
					name = DEFAULT_NAME;
				} else {
					prefix = DEFAULT_PREFIX;
					name = n;
				}
			}
		} else {
			name = DEFAULT_NAME;
			prefix = req.match_info()
				.get("prefix")
				.unwrap_or(DEFAULT_PREFIX);
		}

		if let Some(res) = site(&req, prefix, name) {
			return Ok(res);
		}
	}
	not_found(req)
}

fn site(
	req: &HttpRequest<AppState>,
	prefix: &str,
	name: &str,
) -> Option<HttpResponse> {
	if let Some(site) = req.state().sites.get(prefix).and_then(
		|site_descriptions| {
			site_descriptions
				.get_site(&req.state().config, &name)
				.ok()
		},
	) {
		let content = format!("{}", site);

		return Some(
			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(content),
		);
	}
	None
}

fn not_found(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	warn!("File not found '{}'", req.path());
	let site =
		req.state().sites["public"].get_site(&req.state().config, "404")?;
	let content = format!("{}", site);
	Ok(HttpResponse::NotFound()
		.content_type("text/html; charset=utf-8")
		.body(content))
}

fn main() {
	if env::var("RUST_LOG").is_err() {
		// Default log level
		env::set_var(
			"RUST_LOG",
			"actix_web=info,zeltlager_website=info",
		);
	}
	env_logger::init();

	let mut sites = HashMap::new();
	for name in ["public", "intern"].iter() {
		sites.insert(
			name.to_string(),
			basic::SiteDescriptions::parse(&format!("{}.toml", name))
				.expect(&format!("Failed to parse {}.toml", name)),
		);
	}

	let mut content = String::new();
	File::open("config.toml")
		.unwrap()
		.read_to_string(&mut content)
		.unwrap();
	let config: Config =
		toml::from_str(&content).expect("Failed to parse config.toml");

	let sys = actix::System::new(env!("CARGO_PKG_NAME"));

	// Start some parallel db executors
	let db_addr = actix::SyncArbiter::start(4, move || {
		db::DbExecutor::new().expect("Failed to create db executor")
	});

	// Start some parallel mail executors
	let config2 = config.clone();
	let mail_addr = actix::SyncArbiter::start(4, move || {
		mail::MailExecutor::new(config2.clone())
	});

	let address = config
		.bind_address
		.as_ref()
		.map(|s| s.as_str())
		.unwrap_or("127.0.0.1:8080")
		.to_string();
	let state = AppState {
		sites,
		config,
		db_addr,
		mail_addr,
	};

	server::new(move || {
		App::with_state(state.clone())
			.middleware(middleware::Logger::default())
			// Register static file handler as resource. If it is registered as
			// handler, it will be overwritten by resources.
			.resource("/static/{tail:.*}", |r| {
				r.h(fs::StaticFiles::new("static").default_handler(not_found))
			})
			.route("/anmeldung", Method::GET, signup::signup)
			.route(
				"/intern/betreuer-anmeldung",
				Method::GET,
				signup_supervisor::signup,
			)
			.route(
				"/anmeldung-test",
				Method::GET,
				signup::signup_test,
			)
			.route(
				"/signup-send",
				Method::POST,
				signup::signup_send,
			)
			.route(
				"/intern/signup-supervisor-send",
				Method::POST,
				signup_supervisor::signup_send,
			)
			// Allow an empty name
			.route("/{prefix}/{name:[^/]*}", Method::GET, ::sites)
			.route("/{name}", Method::GET, ::sites)
			.route("", Method::GET, ::sites)
			.default_resource(|r| r.f(not_found))
	}).bind(address)
		.unwrap()
		.start();

	let _ = sys.run();
}
