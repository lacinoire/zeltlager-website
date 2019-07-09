#![allow(proc_macro_derive_resolution_fallback)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate t4rust_derive;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{Read, Write};

use actix_web::http::header::DispositionType;
use actix_web::http::Method;
use actix_web::middleware::identity::{CookieIdentityPolicy, IdentityService};
use actix_web::middleware::{self, csrf, Middleware};
use actix_web::*;
use chrono::Duration;
use futures::{future, Future, IntoFuture};
use rand::Rng;
use structopt::clap::AppSettings;
use structopt::StructOpt;

macro_rules! tryf {
	($e:expr) => {
		match $e {
			Ok(e) => e,
			Err(error) => return Box::new(::futures::future::err(error.into())),
			}
	};
}

mod admin;
mod auth;
mod basic;
mod db;
mod discourse;
mod form;
mod images;
mod mail;
mod management;
mod signup;
mod signup_supervisor;
mod thumbs;

type Result<T> = std::result::Result<T, failure::Error>;
type BoxFuture<T> = Box<dyn futures::Future<Item = T, Error = failure::Error>>;

const DEFAULT_PREFIX: &str = "public";
const DEFAULT_NAME: &str = "startseite";
const RATELIMIT_MAX_COUNTER: i32 = 50;
const KEY_FILE: &str = "secret.key";

fn cookie_maxtime() -> Duration {
	Duration::minutes(30)
}
fn ratelimit_duration() -> Duration {
	Duration::days(1)
}

#[derive(StructOpt, Debug)]
#[structopt(
	raw(
		global_settings = "&[AppSettings::ColoredHelp, \
		                   AppSettings::VersionlessSubcommands]"
	)
)]
struct Args {
	#[structopt(subcommand, help = "Default action is to start the server")]
	action: Option<Action>,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "action")]
enum Action {
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
	name: Option<String>,
	address: String,
}

fn submission_port() -> u16 { 587 }

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MailAccount {
	/// Host for smtp.
	host: String,
	/// Username to login to smtp.
	name: Option<String>,
	/// Password to login to smtp.
	password: String,
	#[serde(default = "submission_port")]
	port: u16,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscourseConfig {
	/// The api endpoint, something like `https://discourse.example.com`.
	endpoint: String,
	/// The api token.
	token: String,
	/// The username for the api.
	username: String,
	/// Add new users to this group.
	group: String,
	/// Subscribe new users to this category.
	category: String,
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
	/// If a member signs up with this mail address, the signup mail will only
	/// be sent to this address, but not to additional receivers. The member
	/// will also not be entered into the database.
	test_mail: Option<String>,

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
	#[serde(default = "default_bind_address")]
	bind_address: String,
	/// A message which will be displayed on top of all basic templated sites.
	global_message: Option<String>,
	/// If this site is served over https.
	///
	/// If `true`, the authentication cookie can only be transfered over https.
	#[serde(default = "get_true")]
	secure: bool,
	/// This should be the domain the server.
	///
	/// If set, it restricts the authentication cookie to a domain
	/// and protects against csrf using the referer and origin header.
	domain: Option<String>,

	/// The configuration of the discourse integration.
	discourse: Option<DiscourseConfig>,

	/// The sentry DSN.
	sentry: Option<String>,
}

fn get_true() -> bool {
	true
}
fn default_bind_address() -> String {
	String::from("127.0.0.1:8080")
}

#[derive(Clone)]
pub struct AppState {
	sites: HashMap<String, basic::SiteDescriptions>,
	config: Config,
	db_addr: actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>,
	disc_addr: Option<actix::Addr<discourse::DiscourseExecutor>>,
}

impl Into<lettre_email::Mailbox> for MailAddress {
	fn into(self) -> lettre_email::Mailbox {
		lettre_email::Mailbox {
			name: self.name,
			address: self.address,
		}
	}
}

#[derive(Default)]
struct StaticFilesConfig;

impl actix_web::fs::StaticFileConfig for StaticFilesConfig {
	fn content_disposition_map(typ: mime::Name) -> DispositionType {
		if typ == "application" {
			// For application/pdf in object tags
			DispositionType::Inline
		} else {
			actix_web::fs::DefaultConfig::content_disposition_map(typ)
		}
	}
}

struct HasRolePredicate {
	role: auth::Roles,
}

impl HasRolePredicate {
	fn new(role: auth::Roles) -> Self {
		Self { role }
	}
}

impl Middleware<AppState> for HasRolePredicate {
	fn start(
		&self,
		req: &HttpRequest<AppState>,
	) -> error::Result<middleware::Started> {
		let role = self.role.clone();
		let forbidden_site = forbidden(req);
		let path = req.path().to_string();
		let fut = auth::get_roles(req).and_then(move |r| -> BoxFuture<Option<HttpResponse>> {
			if let Some(roles) = r {
				if roles.contains(&role) {
					Box::new(future::ok(None))
				} else {
					warn!("Forbidden '{}'", path);
					Box::new(forbidden_site.map(Some))
				}
			} else {
				// Not logged in
				// Redirect to login site with redirect to original site
				Box::new(future::ok(Some(HttpResponse::Found()
						.header(
							"location",
							format!(
								"/login?redirect={}",
								url::form_urlencoded::byte_serialize(
									path.as_bytes()
								).collect::<String>()
							).as_str(),
						)
						.finish())))
			}
		});
		Ok(middleware::Started::Future(Box::new(fut.from_err())))
	}
}

impl Config {
	fn validate(&self) -> Result<()> {
		mail::check_parsable(&self.sender_mail.address)?;
		for r in &self.additional_mail_receivers {
			mail::check_parsable(&r.address)?;
		}
		if let Some(addr) = &self.test_mail {
			mail::check_parsable(&addr)?;
		}
		Ok(())
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

fn get_scrypt_params() -> scrypt::ScryptParams {
	scrypt::ScryptParams::new(15, 8, 1).unwrap()
}

fn sites(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let prefix: String;
	let name: String;
	if let Some(n) = req.match_info().get("name").and_then(|s| {
		if s.is_empty() {
			None
		} else {
			Some(s)
		}
	}) {
		if let Some(p) = req.match_info().get("prefix") {
			prefix = p.to_string();
			name = n.to_string();
		} else if req.state().sites.get(n).is_some() {
			// Check if the name is actually a prefix
			prefix = n.to_string();
			name = DEFAULT_NAME.to_string();
		} else {
			prefix = DEFAULT_PREFIX.to_string();
			name = n.to_string();
		}
	} else {
		name = DEFAULT_NAME.to_string();
		prefix = req.match_info().get("prefix").unwrap_or(DEFAULT_PREFIX).to_string();
	}

	let site_res = site(&req, &prefix, &name);
	Box::new(site_res.and_then(move |site_opt| -> BoxFuture<HttpResponse> {
		match site_opt {
			Some(res) => Box::new(future::ok(res)),
			None => not_found(&req),
		}
	}))
}

fn site(
	req: &HttpRequest<AppState>,
	prefix: &str,
	name: &str,
) -> BoxFuture<Option<HttpResponse>> {
	// TODO nicht alles kopieren
	if let Some(site_descriptions) = req.state().sites.get(prefix) {
		let config = req.state().config.clone();
		let site_descriptions = site_descriptions.clone();
		let name = name.to_string();
		Box::new(
			auth::get_roles(req)
				.map(move |res| {
					site_descriptions.get_site(config, &name, res).ok()
						.map(|site| {
							let content = format!("{}", site);

							HttpResponse::Ok()
								.content_type("text/html; charset=utf-8")
								.body(content)
						})
				})
		)
	} else {
		Box::new(Ok(None).into_future())
	}
}

fn not_found(req: &HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	warn!("File not found '{}'", req.path());
	let state = req.state().clone();
	Box::new(auth::get_roles(&req).and_then(move |res| {
		let site = state.sites["public"].get_site(
			state.config.clone(), "notfound", res)?;
		let content = format!("{}", site);
		Ok(HttpResponse::NotFound()
			.content_type("text/html; charset=utf-8")
			.body(content))
	}))
}

fn forbidden(req: &HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	// This gets printed sometimes without a request being forbidden because
	// we need the request.
	warn!("Forbidden '{}'", req.path());
	let state = req.state().clone();
	Box::new(auth::get_roles(&req).and_then(move |res| {
		let site = state.sites["public"].get_site(
			state.config.clone(), "forbidden", res)?;
		let content = format!("{}", site);
		Ok(HttpResponse::NotFound()
			.content_type("text/html; charset=utf-8")
			.body(content))
	}))
}

fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		// Default log level
		env::set_var("RUST_LOG", "actix_web=info,zeltlager_website=info");
	}
	env_logger::init();

	// Command line arguments
	let args = Args::from_args();
	if let Some(action) = args.action {
		management::cmd_action(action)?;
		return Ok(());
	}

	// Get cookie master key
	let mut key = [0; 32];
	if let Ok(mut key_file) = File::open(KEY_FILE) {
		key_file.read_exact(&mut key)?;
	} else {
		rand::thread_rng().fill(&mut key);
		// Save in file
		let mut key_file = File::create(KEY_FILE)?;
		key_file.write_all(&key)?;
	}

	let mut sites = HashMap::new();
	for name in &["public", "intern"] {
		sites.insert(
			name.to_string(),
			basic::SiteDescriptions::parse(&format!("{}.toml", name))
				.unwrap_or_else(|e| panic!("Failed to parse {}.toml ({:?})", name, e)),
		);
	}

	let mut content = String::new();
	File::open("config.toml")
		.unwrap()
		.read_to_string(&mut content)?;
	let config: Config =
		toml::from_str(&content).expect("Failed to parse config.toml");
	config.validate().unwrap();

	// Start sentry
	let _sentry_guard;
	if let Some(s) = &config.sentry {
		_sentry_guard = sentry::init(s.as_str());
		sentry::integrations::panic::register_panic_handler();
	}

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

	// Start some parallel discourse executors
	let disc_addr = if let Some(config) = &config.discourse {
		let config2 = config.clone();
		Some(actix::SyncArbiter::start(4, move || {
			discourse::DiscourseExecutor::new(config2.clone()).unwrap()
		}))
	} else { None };

	let address = config.bind_address.clone();
	let state = AppState {
		sites,
		config,
		db_addr,
		mail_addr,
		disc_addr,
	};

	// Start thumbnail creator
	std::thread::spawn(|| thumbs::watch_thumbs("Bilder2018"));

	server::new(move || {
		let mut identity_policy = CookieIdentityPolicy::new(&key)
			.name("user")
			.max_age(cookie_maxtime())
			.secure(state.config.secure);

		let mut app =
			App::with_state(state.clone()).middleware(middleware::Logger::default());

		if let Some(ref domain) = state.config.domain {
			// TODO Test
			identity_policy = identity_policy.domain(domain.clone());
			app = app.middleware(csrf::CsrfFilter::new().allowed_origin(format!(
				"http{}://{}",
				if state.config.secure { "s" } else { "" },
				domain,
			)))
		}

		app
			.middleware(IdentityService::new(identity_policy))
			// Register static file handler as resource. If it is registered as
			// handler, it will be overwritten by resources.
			.resource("/static/{tail:.*}", |r| {
				r.h(fs::StaticFiles::with_config("static", StaticFilesConfig)
					.unwrap().default_handler(not_found))
			})
			.route("/anmeldung", Method::GET, signup::signup)
			.route(
				"/intern/betreuer-anmeldung",
				Method::GET,
				signup_supervisor::signup,
			)
			.route("/anmeldung-test", Method::GET, signup::signup_test)
			.route("/signup-send", Method::POST, signup::signup_send)
			.route(
				"/intern/signup-supervisor-send",
				Method::POST,
				signup_supervisor::signup_send,
			)
			.route("/login", Method::GET, auth::login)
			.route("/login", Method::POST, auth::login_send)
			.route("/logout", Method::GET, auth::logout)
			.scope("/Bilder2018/", |scope| { scope
				.middleware(HasRolePredicate::new(auth::Roles::ImageDownload2018))
				.resource("/static/{tail:.*}", |r| {
					r.h(fs::StaticFiles::with_config("Bilder2018", StaticFilesConfig)
						.unwrap().default_handler(not_found))
				})
				.route("", Method::GET, images::render_images)
				.default_resource(|r| r.f(not_found))
			})
			.route("/Bilder2018", Method::GET, |_: HttpRequest<AppState>|
				HttpResponse::Found().header("location", "/Bilder2018/").finish())
			.scope("/admin/", |scope| { scope
				.middleware(HasRolePredicate::new(auth::Roles::Admin))
				.route("", Method::GET, admin::render_admin)
				.route("/teilnehmer.csv", Method::GET, admin::download_members_csv)
				.route("/betreuer.csv", Method::GET, admin::download_betreuer_csv)
				.default_resource(|r| r.f(not_found))
			})
			.route("/admin", Method::GET, |_: HttpRequest<AppState>|
				HttpResponse::Found().header("location", "/admin/").finish())
			// Allow an empty name
			.route("/{prefix}/{name:[^/]*}", Method::GET, crate::sites)
			.route("/{name}", Method::GET, crate::sites)
			.route("/", Method::GET, crate::sites)
			.default_resource(|r| r.f(not_found))
	}).bind(address)?
		.start();

	let _ = sys.run();
	Ok(())
}
