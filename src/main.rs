#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use actix_http::cookie::SameSite;
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_files::Files;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::DispositionType;
use actix_web::*;
use anyhow::{format_err, Result};
use chrono::Duration;
use futures::prelude::*;
use log::{error, warn};
use rand::Rng;
use structopt::StructOpt;

mod admin;
mod auth;
mod basic;
mod config;
mod db;
mod erwischt;
mod form;
mod images;
mod mail;
mod management;
mod signup;
mod signup_supervisor;
mod thumbs;

use config::{Config, MailAddress};

const DEFAULT_PREFIX: &str = "public";
const DEFAULT_NAME: &str = "startseite";
const RATELIMIT_MAX_COUNTER: i32 = 50;
const KEY_FILE: &str = "secret.key";

static IMAGE_YEARS: &[(&str, auth::Roles)] = &[
	("Bilder2018", auth::Roles::ImageDownload2018),
	("Bilder2019", auth::Roles::ImageDownload2019),
	("Bilder2020", auth::Roles::ImageDownload2020),
];

fn cookie_maxtime() -> Duration {
	Duration::minutes(120)
}
fn ratelimit_duration() -> Duration {
	Duration::days(1)
}
fn get_true() -> bool {
	true
}

#[derive(Clone)]
pub struct State {
	sites: HashMap<String, basic::SiteDescriptions>,
	config: Config,
	db_addr: actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>,
	/// Used to lock access to the log file.
	log_mutex: Arc<Mutex<()>>,
}

impl Into<lettre_email::Mailbox> for MailAddress {
	fn into(self) -> lettre_email::Mailbox {
		lettre_email::Mailbox {
			name: self.name,
			address: self.address,
		}
	}
}

fn content_disposition_map(typ: &mime::Name) -> DispositionType {
	match *typ {
		// For images and application/pdf in object tags
		mime::IMAGE | mime::TEXT | mime::VIDEO | mime::APPLICATION => DispositionType::Inline,
		_ => DispositionType::Attachment,
	}
}

struct CsrfFilter {
	domain: Option<String>,
}

struct CsrfFilterMiddleware<S> {
	service: S,
	domain: Option<String>,
}

impl CsrfFilter {
	fn new(domain: Option<String>) -> Self {
		Self { domain }
	}
}

impl<S: 'static, B> Transform<S> for CsrfFilter
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type InitError = ();
	type Transform = CsrfFilterMiddleware<S>;
	type Future = future::Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		future::ok(CsrfFilterMiddleware {
			service,
			domain: self.domain.clone(),
		})
	}
}

fn uri_origin(uri: &http::uri::Uri) -> Option<String> {
	match (uri.scheme_str(), uri.host(), uri.port()) {
		(Some(scheme), Some(host), Some(port)) => {
			Some(format!("{}://{}:{}", scheme, host, port))
		}
		(Some(scheme), Some(host), None) => {
			Some(format!("{}://{}", scheme, host))
		}
		_ => None
	}
}

fn get_origin(headers: &http::header::HeaderMap) -> Option<Result<String>> {
	headers.get(http::header::ORIGIN)
		.map(|origin| {
			origin
				.to_str()
				.map_err(|_| format_err!("Bad origin"))
				.map(|o| o.into())
		})
		.or_else(|| {
			headers.get(http::header::REFERER)
				.map(|referer| {
					http::uri::Uri::try_from(referer.as_bytes())
						.ok()
						.as_ref()
						.and_then(uri_origin)
						.ok_or_else(|| format_err!("Bad origin"))
						.map(|o| o.into())
				})
		})
}

impl<S: 'static, B> Service for CsrfFilterMiddleware<S>
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&mut self, req: ServiceRequest) -> Self::Future {
		if !req.method().is_safe() {
			if let Some(domain) = &self.domain {
				if let Some(header) = get_origin(req.headers()) {
					match header {
						Ok(ref origin) if origin.ends_with(domain) => {}
						Ok(ref origin) => {
							warn!("Origin does not match: {:?} does not end with {:?}", origin, domain);
							return Box::pin(future::err(io::Error::new(io::ErrorKind::Other,
								"Cross origin request denied").into()));
						}
						Err(e) => {
							warn!("Origin not found: {}", e);
							return Box::pin(future::err(io::Error::new(io::ErrorKind::Other,
								"Cross origin request failure").into()));
						}
					}
				}
			}
		}
		Box::pin(self.service.call(req))
	}
}

struct HasRolePredicate {
	role: auth::Roles,
}

struct HasRolePredicateMiddleware<S> {
	service: Rc<RefCell<S>>,
	role: auth::Roles,
}

impl HasRolePredicate {
	fn new(role: auth::Roles) -> Self {
		Self { role }
	}
}

impl<S: 'static, B> Transform<S> for HasRolePredicate
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type InitError = ();
	type Transform = HasRolePredicateMiddleware<S>;
	type Future = future::Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		future::ok(HasRolePredicateMiddleware {
			service: Rc::new(RefCell::new(service)),
			role: self.role,
		})
	}
}

impl<S: 'static, B> Service for HasRolePredicateMiddleware<S>
where
	S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Request = ServiceRequest;
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&mut self, req: ServiceRequest) -> Self::Future {
		let mut service = self.service.clone();
		let state = req.app_data::<State>().unwrap();
		let (req, mut pay) = req.into_parts();
		let identity = Identity::from_request(&req, &mut pay);
		let role = self.role;

		Box::pin(async move {
			let identity = identity.await?;
			let roles = match auth::get_roles(&**state, &identity).await {
				Ok(r) => r,
				Err(e) => {
					error!("Failed to get roles: {}", e);
					drop(identity);
					let req = match ServiceRequest::from_parts(req, pay) {
						Ok(r) => r,
						Err(_) => {
							return Err(io::Error::new(io::ErrorKind::Other,
								"Failed to reassemble request").into());
						}
					};
					return Ok(req.into_response(crate::error_response(&state).into_body()));
				}
			};
			if let Some(roles) = roles {
				if roles.contains(&role) {
					drop(identity);
					let req = match ServiceRequest::from_parts(req, pay) {
						Ok(r) => r,
						Err(_) => {
							return Err(io::Error::new(io::ErrorKind::Other,
								"Failed to reassemble request").into());
						}
					};
					service.call(req).await
				} else {
					let res = forbidden(&state, &identity).await.into_body();
					drop(identity);
					let req = match ServiceRequest::from_parts(req, pay) {
						Ok(r) => r,
						Err(_) => {
							return Err(io::Error::new(io::ErrorKind::Other,
								"Failed to reassemble request").into());
						}
					};
					warn!("Forbidden '{}'", req.path());
					Ok(req.into_response(res))
				}
			} else {
				drop(identity);
				let req = match ServiceRequest::from_parts(req, pay) {
					Ok(r) => r,
					Err(_) => {
						return Err(io::Error::new(io::ErrorKind::Other,
							"Failed to reassemble request").into());
					}
				};
				let location = format!(
					"/login?redirect={}",
					url::form_urlencoded::byte_serialize(
						req.path().as_bytes()
					).collect::<String>()
				);
				// Not logged in
				// Redirect to login site with redirect to original site
				Ok(req.into_response(HttpResponse::Found()
					.header("location", location)
					.finish()
					.into_body()))
			}
		})
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
		db::models::get_birthday_date(&self.birthday_date);
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

fn error_response(state: &State) -> HttpResponse {
	HttpResponse::InternalServerError()
		.content_type("text/html; charset=utf-8")
		.body(format!(
			"Es ist ein interner Fehler aufgetreten.\n{}",
			state.config.error_message
		))
}

async fn sites(state: web::Data<State>, id: Identity, req: HttpRequest) -> HttpResponse {
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
		} else if state.sites.get(n).is_some() {
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

	match site(&**state, &id, &prefix, &name).await {
		Some(res) => res,
		None => not_found(&*state, &id, &req).await,
	}
}

async fn site(
	state: &State,
	id: &Identity,
	prefix: &str,
	name: &str,
) -> Option<HttpResponse> {
	if let Some(site_descriptions) = state.sites.get(prefix) {
		let name = name.to_string();
		let roles = match auth::get_roles(state, id).await {
			Ok(r) => r,
			Err(e) => {
				error!("Failed to get roles: {}", e);
				return Some(crate::error_response(state));
			}
		};
		site_descriptions.get_site(state.config.clone(), &name, roles).ok()
			.map(|site| {
				let content = format!("{}", site);

				HttpResponse::Ok()
					.content_type("text/html; charset=utf-8")
					.body(content)
			})
	} else {
		None
	}
}

async fn not_found_handler(state: web::Data<State>, id: Identity, req: HttpRequest) -> HttpResponse {
	not_found(&**state, &id, &req).await
}

async fn not_found(state: &State, id: &Identity, req: &HttpRequest) -> HttpResponse {
	warn!("File not found '{}'", req.path());
	let roles = match auth::get_roles(state, id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(state);
		}
	};
	let site = match state.sites["public"].get_site(
		state.config.clone(), "notfound", roles) {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get site: {}", e);
			return error_response(state);
		}
	};
	let content = format!("{}", site);
	HttpResponse::NotFound()
		.content_type("text/html; charset=utf-8")
		.body(content)
}

async fn forbidden(state: &State, id: &Identity) -> HttpResponse {
	// This gets printed sometimes without a request being forbidden because
	// we need the request.
	let roles = match auth::get_roles(state, id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(state);
		}
	};
	let site = match state.sites["public"].get_site(
		state.config.clone(), "forbidden", roles) {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get site: {}", e);
			return error_response(state);
		}
	};
	let content = format!("{}", site);
	HttpResponse::NotFound()
		.content_type("text/html; charset=utf-8")
		.body(content)
}

#[actix_rt::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		// Default log level
		env::set_var("RUST_LOG", "actix_web=info,zeltlager_website=info");
	}
	env_logger::init();

	// Command line arguments
	let args = config::Args::from_args();
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

	// Start some parallel db executors
	let db_addr = actix::SyncArbiter::start(4, move || {
		db::DbExecutor::new().expect("Failed to create db executor")
	});

	// Run database migrations
	db_addr.send(db::RunMigrationsMessage).await??;

	// Start some parallel mail executors
	let config2 = config.clone();
	let mail_addr = actix::SyncArbiter::start(4, move || {
		mail::MailExecutor::new(config2.clone())
	});

	let address = config.bind_address.clone();
	let state = State {
		sites,
		config,
		db_addr,
		mail_addr,
		log_mutex: Arc::new(Mutex::new(())),
	};

	// Start thumbnail creator
	for (name, _) in IMAGE_YEARS {
		let name = *name;
		std::thread::spawn(move || thumbs::watch_thumbs(name));
	}

	HttpServer::new(move || {
		let mut identity_policy = CookieIdentityPolicy::new(&key)
			.name("user")
			.max_age_time(cookie_maxtime())
			.secure(state.config.secure)
			.same_site(SameSite::Strict);

		let app = App::new()
			.data(state.clone())
			.wrap(middleware::Logger::default())
			.wrap(CsrfFilter::new(state.config.domain.clone()));

		if let Some(ref domain) = state.config.domain {
			// TODO Test for CSRF, check origin header
			identity_policy = identity_policy.domain(domain.clone());
		}

		let mut app = app
			.wrap(IdentityService::new(identity_policy))
			.service(Files::new("/static", "static")
				.mime_override(content_disposition_map)
				.default_handler(web::to(not_found_handler)))
			.service(signup::signup)
			.service(signup::signup_test)
			.service(signup::signup_send)
			.service(signup_supervisor::signup)
			.service(signup_supervisor::signup_send)
			.service(auth::login)
			.service(auth::login_send)
			.service(auth::logout);

		for (name, role) in IMAGE_YEARS {
			let name = *name;
			app = app
				.service(web::scope(&format!("/{}", name))
					.wrap(HasRolePredicate::new(*role))
					.service(web::resource("/").route(web::get().to(move |s, i| images::render_images(s, i, name))))
					.service(Files::new("/static", name)
						.mime_override(content_disposition_map)
						.default_handler(web::to(not_found_handler)))
					.default_service(web::to(not_found_handler))
				)
				.service(web::resource(&format!("/{}", name)).route(web::get().to(move ||
					HttpResponse::Found().header("location", format!("/{}/", name)).finish())));
		}

		app
			// admin
			.service(web::scope("/admin")
				.wrap(HasRolePredicate::new(auth::Roles::Admin))
				.service(admin::render_members)
				.service(admin::render_supervisors)
				.service(admin::download_members_json)
				.service(admin::download_supervisors_json)
				.service(admin::remove_member)
				.service(admin::edit_member)
				.service(admin::edit_supervisor)
				.default_service(web::to(not_found_handler))
			)
			.service(web::resource("/admin").route(web::get().to(||
				HttpResponse::Found().header("location", "/admin/").finish())
			))
			// erwischt
			.service(web::scope("/erwischt")
				.wrap(HasRolePredicate::new(auth::Roles::Erwischt))
				.service(erwischt::render_erwischt)
				.service(erwischt::get_games)
				.service(erwischt::get_game)
				.service(erwischt::create_game)
				.service(erwischt::delete_game)
				.service(erwischt::catch)
				.service(erwischt::insert)
				.service(erwischt::create_game_pdf)
				.service(erwischt::create_members_pdf)
				.default_service(web::to(not_found_handler))
			)
			.service(web::resource("/erwischt").route(web::get().to(||
				HttpResponse::Found().header("location", "/erwischt/").finish())
			))
			// Allow an empty name
			.service(web::resource("/{prefix}/{name:[^/]*}").route(web::get().to(crate::sites)))
			.service(web::resource("/{name}").route(web::get().to(crate::sites)))
			.service(web::resource("/").route(web::get().to(crate::sites)))
			.default_service(web::to(not_found_handler))
	}).bind(address)?
		.run()
		.await?;

	Ok(())
}
