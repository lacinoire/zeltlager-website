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

use actix_web::*;
use futures::{Future, IntoFuture};

mod basic;
mod db;
mod mail;
mod signup;

type Result<T> = std::result::Result<T, failure::Error>;
type BoxFuture<T> = Box<futures::Future<Item = T, Error = failure::Error>>;

/*macro_rules! tryf {
	($e:expr) => {
		match $e {
			Ok(e) => e,
			Err(error) => return Box::new(future::err(error.into())),
		}
	};
}*/

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
	email_username: String,
	email_userdescription: String,
	email_password: String,
	email_host: String,

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
	basics: basic::SiteDescriptions,
	config: Config,
	db_addr: actix::Addr<actix::Syn, db::DbExecutor>,
	mail_addr: actix::Addr<actix::Syn, mail::MailExecutor>,
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

fn basic_sites(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	let name: String = req.match_info().query("name")?;
	if let Ok(site) = req.state()
		.basics
		.get_site(&req.state().config, &name)
	{
		let content = format!("{}", site);

		return Ok(httpcodes::HttpOk
			.build()
			.content_type("text/html; charset=utf-8")
			.body(content)?);
	}
	not_found(req)
}

fn index(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	let site = req.state()
		.basics
		.get_site(&req.state().config, "startseite")?;
	let content = format!("{}", site);

	Ok(httpcodes::HttpOk
		.build()
		.content_type("text/html; charset=utf-8")
		.body(content)?)
}

fn signup(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	render_signup(req, HashMap::new())
}

fn signup_test(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let map = vec![
		("vorname", "a"),
		("nachname", "b"),
		("geburtsdatum", "1.1.2000"),
		("geschlecht", "w"),
		("schwimmer", "true"),
		("vegetarier", "false"),
		("tetanus_impfung", "true"),
		("eltern_name", "d"),
		("eltern_mail", "@"),
		("eltern_handynummer", "f"),
		("strasse", "g"),
		("hausnummer", "h"),
		("ort", "i"),
		("plz", "80000"),
	];

	let map = map.iter()
		.map(|&(a, b)| (a.to_string(), b.to_string()));

	render_signup(req, map.collect())
}

/// Return the signup site with the prefilled `values`.
fn render_signup(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> BoxFuture<HttpResponse> {
	if let Ok(site) = req.state()
		.basics
		.get_site(&req.state().config, "anmeldung")
	{
		let content = format!("{}", site);
		return Box::new(
			signup::Signup::new(req.state(), values).and_then(
				move |new_content| {
					let content = content.replace(
						"<insert content here>",
						&format!("{}", new_content),
					);

					Ok(httpcodes::HttpOk
						.build()
						.content_type("text/html; charset=utf-8")
						.body(content)?)
				},
			),
		);
	}
	Box::new(not_found(req).into_future().from_err())
}

/// Check if too many members are already registered, then call `signup_insert`.
fn signup_check_count(
	count: i64,
	max_members: i64,
	db_addr: actix::Addr<actix::Syn, db::DbExecutor>,
	mail_addr: actix::Addr<actix::Syn, mail::MailExecutor>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	if count >= max_members {
		// Show error
		body.insert(
			"error".to_string(),
			"Während Ihrer Anmeldung ist das Zeltlager leider schon voll \
			 geworden."
				.to_string(),
		);
		warn!(
			"Already too many members registered (from {})",
			member.eltern_mail
		);
		render_signup(req, body)
	} else {
		Box::new(
			db_addr
				.send(db::SignupMessage {
					member: member.clone(),
				})
				.from_err::<failure::Error>()
				.then(move |result| -> BoxFuture<HttpResponse> {
					match result {
						Err(error) | Ok(Err(error)) => {
							// Show error and prefilled form
							body.insert(
								"error".to_string(),
								format!(
									"Es ist ein Datenbank-Fehler \
									 aufgetreten.\n{}",
									error_message
								),
							);
							warn!("Error inserting into database: {}", error);
							render_signup(req, body)
						}
						Ok(Ok(())) => signup_insert(
							&mail_addr,
							member,
							body,
							error_message,
							req,
						),
					}
				}),
		)
	}
}

/// Insert the member into the database, write an email and show a success site.
fn signup_insert(
	mail_addr: &actix::Addr<actix::Syn, mail::MailExecutor>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	// Write an e-mail
	Box::new(
		mail_addr
			.send(mail::SignupMessage { member })
			.from_err::<failure::Error>()
			.then(move |result| -> BoxFuture<HttpResponse> {
				match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert(
							"error".to_string(),
							format!(
								"Ihre Daten wurden erfolgreich \
								 gespeichert.\nEs ist leider ein Fehler beim \
								 E-Mail senden aufgetreten.\n{}",
								error_message
							),
						);
						warn!("Error inserting into database: {}", error);
						render_signup(req, body)
					}
					Ok(Ok(())) => {
						// Redirect to success site
						Box::new(
							httpcodes::HttpFound
								.build()
								.header(
									header::http::LOCATION,
									"anmeldungErfolgreich",
								)
								.finish()
								.into_future()
								.from_err(),
						)
					}
				}
			}),
	)
}

fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let mail_addr = req.state().mail_addr.clone();
	let error_message = req.state().config.error_message.clone();
	let max_members = req.state().config.max_members;
	let db_addr2 = db_addr.clone();

	// Get the body of the request
	req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body| -> BoxFuture<_> {
			let member = match db::models::Teilnehmer::
				from_hashmap(body.clone()) {
				Ok(member) => member,
				Err(error) => {
					// Show error and prefilled form
					body.insert("error".to_string(), format!("{}", error));
					warn!("Error handling form content: {}", error);
					return Box::new(render_signup(req, body).into_future());
				}
			};

			Box::new(db_addr.send(db::CountMemberMessage)
				.from_err::<failure::Error>()
				.then(move |result| -> BoxFuture<HttpResponse> { match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert("error".to_string(), format!("\
							Es ist ein Datenbank-Fehler aufgetreten.\n{}",
							error_message));
						warn!("Error inserting into database: {}", error);
						render_signup(req, body)
					}
					Ok(Ok(count)) => signup_check_count(count, max_members,
						db_addr2, mail_addr, member, body, error_message, req),
				}})
		)})
		.responder()
}

fn not_found(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	warn!("File not found '{}'", req.path());
	let site = req.state()
		.basics
		.get_site(&req.state().config, "404")?;
	let content = format!("{}", site);
	Ok(httpcodes::HttpNotFound
		.build()
		.content_type("text/html; charset=utf-8")
		.body(content)?)
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

	let basics =
		basic::SiteDescriptions::parse().expect("Failed to parse basic.toml");
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
		basics,
		config,
		db_addr,
		mail_addr,
	};

	HttpServer::new(move || {
		Application::with_state(state.clone())
			.middleware(middleware::Logger::default())
			.handler(
				"/static",
				fs::StaticFiles::new("static", false)
					.default_handler(not_found),
			)
			.resource("/anmeldung", |r| r.f(signup))
			.resource("/anmeldung-test", |r| r.f(signup_test))
			.resource("/signup-send", |r| {
				r.method(Method::POST).a(signup_send)
			})
			.resource("/{name}", |r| r.f(basic_sites))
			.resource("", |r| r.f(index))
			.default_resource(|r| r.f(not_found))
	}).bind(address)
		.unwrap()
		.start();

	let _ = sys.run();
}
