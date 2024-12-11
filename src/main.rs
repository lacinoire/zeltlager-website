#[macro_use]
extern crate diesel;

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex};

use actix_files::Files;
use actix_identity::{Identity, IdentityExt, IdentityMiddleware};
use actix_session::config::{PersistentSession, TtlExtensionPolicy};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::DispositionType;
use actix_web::web::Data;
use actix_web::*;
use anyhow::{Result, format_err};
use clap::Parser;
use futures::future::LocalBoxFuture;
use futures::prelude::*;
use lettre::message::Mailbox;
use log::{error, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::{Date, Duration};

mod admin;
mod auth;
mod basic;
mod config;
mod db;
mod erwischt;
mod images;
mod mail;
mod management;
mod signup;
mod signup_supervisor;
mod thumbs;

use config::{Config, MailAddress};

const DEFAULT_PREFIX: &str = "public";
const RATELIMIT_MAX_COUNTER: i32 = 50;
const KEY_FILE: &str = "secret.key";
const ISO_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] =
	format_description!("[year]-[month]-[day]");
const GERMAN_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] =
	format_description!("[day].[month].[year]");
const PRIMITIVE_DATE_TIME_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] =
	format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
const LAGER_START_STR: &str = include_str!("../frontend/lager-start.txt");
static LAGER_START: LazyLock<Date> =
	LazyLock::new(|| Date::parse(LAGER_START_STR.trim(), ISO_DATE_FORMAT).unwrap());

fn cookie_maxtime() -> Duration { Duration::days(2) }
fn ratelimit_duration() -> Duration { Duration::days(1) }
fn get_true() -> bool { true }

#[derive(Clone)]
pub struct State {
	sites: HashMap<String, basic::SiteDescriptions>,
	config: Config,
	db_addr: actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>,
	/// Used to lock access to the log file.
	log_mutex: Arc<Mutex<()>>,
}

#[derive(Clone, Debug, Serialize)]
struct MenuItem {
	title: String,
	link: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuData {
	is_logged_in: bool,
	global_message: Option<String>,
	items: Vec<MenuItem>,
}

#[derive(Clone, Debug, Deserialize)]
struct MenuRequestData {
	prefix: Option<String>,
}

#[derive(Debug, Clone)]
struct ImagesPathRewriter {
	image_dirs: Vec<String>,
}

pub struct ImagesPathRewriterTransform<S> {
	service: S,
	image_dirs: Vec<String>,
}

impl<S, B> Transform<S, ServiceRequest> for ImagesPathRewriter
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
{
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Transform = ImagesPathRewriterTransform<S>;
	type InitError = ();
	type Future = future::Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		future::ok(ImagesPathRewriterTransform { service, image_dirs: self.image_dirs.clone() })
	}
}

impl<S, B> Service<ServiceRequest> for ImagesPathRewriterTransform<S>
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
{
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Future = S::Future;

	actix_service::forward_ready!(service);

	fn call(&self, mut req: ServiceRequest) -> Self::Future {
		let head = req.head_mut();

		let original_path = head.uri.path();
		let mut path = original_path;
		if original_path.starts_with("/Bilder") {
			for name in &self.image_dirs {
				if original_path.trim_end_matches('/') == format!("/Bilder{}", name) {
					path = "/images/";
				}
			}
		}
		if path != original_path {
			let mut parts = head.uri.clone().into_parts();
			let query = parts.path_and_query.as_ref().and_then(|pq| pq.query());

			let path = match query {
				Some(q) => web::Bytes::from(format!("{}?{}", path, q)),
				None => web::Bytes::copy_from_slice(path.as_bytes()),
			};
			parts.path_and_query = Some(http::uri::PathAndQuery::from_maybe_shared(path).unwrap());

			let uri = http::Uri::from_parts(parts).unwrap();
			req.match_info_mut().get_mut().update(&uri);
			req.head_mut().uri = uri;
		}
		self.service.call(req)
	}
}

impl TryInto<Mailbox> for MailAddress {
	type Error = anyhow::Error;
	fn try_into(self) -> Result<Mailbox> {
		Ok(Mailbox { name: self.name, email: self.address.parse()? })
	}
}

fn content_disposition_map(typ: &mime::Name) -> DispositionType {
	match *typ {
		// For images and application/pdf in object tags
		mime::IMAGE | mime::TEXT | mime::VIDEO | mime::APPLICATION => DispositionType::Inline,
		_ => DispositionType::Attachment,
	}
}

fn uri_origin(uri: &http::uri::Uri) -> Option<String> {
	match (uri.scheme_str(), uri.host(), uri.port()) {
		(Some(scheme), Some(host), Some(port)) => Some(format!("{}://{}:{}", scheme, host, port)),
		(Some(scheme), Some(host), None) => Some(format!("{}://{}", scheme, host)),
		_ => None,
	}
}

fn get_origin(headers: &http::header::HeaderMap) -> Option<Result<String>> {
	headers
		.get(http::header::ORIGIN)
		.map(|origin| origin.to_str().map_err(|_| format_err!("Bad origin")).map(|o| o.into()))
		.or_else(|| {
			headers.get(http::header::REFERER).map(|referer| {
				http::uri::Uri::try_from(referer.as_bytes())
					.ok()
					.as_ref()
					.and_then(uri_origin)
					.ok_or_else(|| format_err!("Bad origin"))
			})
		})
}

fn check_csrf(req: &ServiceRequest, domain: Option<&str>) -> bool {
	if !req.method().is_safe() {
		if let Some(domain) = domain {
			if let Some(header) = get_origin(req.headers()) {
				match header {
					Ok(ref origin) if origin.ends_with(domain) => {}
					Ok(ref origin) => {
						warn!("Origin does not match: {:?} does not end with {:?}", origin, domain);
						return false;
					}
					Err(e) => {
						warn!("Origin not found: {}", e);
						return false;
					}
				}
			}
		}
	}
	true
}

struct HasRolePredicate {
	/// The role to check for
	role: auth::Roles,
	/// If this is an API endpoint or a user-facing endpoint
	is_api: bool,
}

struct HasRolePredicateMiddleware<S> {
	service: Rc<S>,
	role: auth::Roles,
	is_api: bool,
}

impl HasRolePredicate {
	fn new(role: auth::Roles, is_api: bool) -> Self { Self { role, is_api } }
}

impl<S, B> Transform<S, ServiceRequest> for HasRolePredicate
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
	S::Future: 'static,
	B: MessageBody + 'static,
{
	type Response = ServiceResponse;
	type Error = Error;
	type InitError = ();
	type Transform = HasRolePredicateMiddleware<S>;
	type Future = future::Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		future::ok(HasRolePredicateMiddleware {
			service: Rc::new(service),
			role: self.role.clone(),
			is_api: self.is_api,
		})
	}
}

impl<S, B> Service<ServiceRequest> for HasRolePredicateMiddleware<S>
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
	S::Future: 'static,
	B: MessageBody + 'static,
{
	type Response = ServiceResponse;
	type Error = Error;
	type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

	actix_service::forward_ready!(service);

	fn call(&self, req: ServiceRequest) -> Self::Future {
		let service = Rc::clone(&self.service);
		let state = req.app_data::<Data<State>>().unwrap().clone();
		let identity = req.get_identity();
		let role = self.role.clone();
		let is_api = self.is_api;

		async move {
			let identity = identity.ok();
			let roles = match auth::get_roles(&state, &identity).await {
				Ok(r) => r,
				Err(e) => {
					error!("Failed to get roles: {}", e);
					return Ok(req.into_response(crate::error_response(&state)));
				}
			};
			if let Some(roles) = roles {
				if roles.contains(&role) {
					Ok(service.call(req).await?.map_into_boxed_body())
				} else {
					let res = forbidden().await;
					warn!("Forbidden '{}'", req.path());
					Ok(req.into_response(res))
				}
			} else {
				// Not logged in
				if is_api {
					// Return an error that can be displayed
					Ok(req.into_response(
						HttpResponse::Unauthorized().body("Bitte anmelden, Sie sind ausgeloggt."),
					))
				} else {
					let location = format!(
						"/login?redirect={}",
						url::form_urlencoded::byte_serialize(req.path().as_bytes())
							.collect::<String>()
					);
					// Redirect to login site with redirect to original site
					Ok(req.into_response(
						HttpResponse::Found().append_header(("location", location)).finish(),
					))
				}
			}
		}
		.boxed_local()
	}
}

impl Config {
	fn validate(&self) -> Result<()> {
		mail::check_parsable(&self.sender_mail.address)?;
		for r in &self.additional_mail_receivers {
			mail::check_parsable(&r.address)?;
		}
		if let Some(addr) = &self.test_mail {
			mail::check_parsable(addr)?;
		}
		Ok(())
	}
}

fn error_response(state: &State) -> HttpResponse {
	HttpResponse::InternalServerError()
		.body(format!("Es ist ein interner Fehler aufgetreten.\n{}", state.config.error_message))
}

async fn not_found(req: HttpRequest) -> HttpResponse {
	warn!("File not found '{}'", req.path());
	HttpResponse::NotFound().body("Page not found")
}

async fn forbidden() -> HttpResponse {
	// This gets printed sometimes without a request being forbidden because
	// we need the request.
	HttpResponse::NotFound().body("Page not found")
}

#[get("/menu")]
async fn menu(
	state: web::Data<State>, data: web::Query<MenuRequestData>, id: Option<Identity>,
) -> HttpResponse {
	if let Some(site_descriptions) = data
		.prefix
		.as_deref()
		.and_then(|p| state.sites.get(p))
		.or_else(|| state.sites.get(DEFAULT_PREFIX))
	{
		let roles = match auth::get_roles(&state, &id).await {
			Ok(r) => r,
			Err(e) => {
				error!("Failed to get roles: {}", e);
				return crate::error_response(&state);
			}
		};
		let mut menu_items = Vec::new();

		// Links to images
		for role in roles.iter().flatten() {
			if let auth::Roles::Images(name) = role {
				let site_name = format!("Bilder{}/", name);
				menu_items.push(MenuItem {
					title: format!("Bilder {}", images::split_image_name(name)),
					link: format!(
						"/{}{}{}",
						site_descriptions.prefix,
						if site_descriptions.prefix.is_empty() { "" } else { "/" },
						site_name,
					),
				});
			}
		}

		// Links to other sites
		for site in &site_descriptions.sites {
			if let Some(role) = &site.role {
				if !roles.as_deref().unwrap_or(&[]).contains(role) {
					continue;
				}
			}
			menu_items.push(MenuItem {
				title: site.title.clone(),
				link: format!(
					"/{}{}{}",
					site_descriptions.prefix,
					if site_descriptions.prefix.is_empty() { "" } else { "/" },
					site.name,
				),
			});
		}

		HttpResponse::Ok().json(MenuData {
			is_logged_in: roles.is_some(),
			global_message: state.config.global_message.clone(),
			items: menu_items,
		})
	} else {
		error!("Did not find site prefix {:?}", data.prefix);
		crate::error_response(&state)
	}
}

#[actix_rt::main]
async fn main() -> Result<()> {
	if env::var("RUST_LOG").is_err() {
		// Default log level
		env::set_var("RUST_LOG", "actix_web=info,zeltlager_website=info");
	}
	env_logger::init();

	// Command line arguments
	let args = config::Args::parse();
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
	File::open("config.toml").unwrap().read_to_string(&mut content)?;
	let config: Config = toml::from_str(&content).expect("Failed to parse config.toml");
	config.validate().unwrap();

	// Start some parallel db executors
	let db_addr = actix::SyncArbiter::start(4, move || {
		db::DbExecutor::new().expect("Failed to create db executor")
	});

	// Run database migrations
	db_addr.send(db::RunMigrationsMessage).await??;

	// Start some parallel mail executors
	let config2 = config.clone();
	let mail_addr = actix::SyncArbiter::start(4, move || mail::MailExecutor::new(config2.clone()));

	let address = config.bind_address.clone();
	let state = State { sites, config, db_addr, mail_addr, log_mutex: Arc::new(Mutex::new(())) };

	// Start thumbnail creator
	let mut image_dirs = Vec::new();
	for d in fs::read_dir(".")? {
		let d = d?;
		if let Some(path) =
			d.path().file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_prefix("Bilder"))
		{
			image_dirs.push(path.to_string());
			std::thread::spawn(move || thumbs::watch_thumbs(d.path()));
		}
	}

	HttpServer::new(move || {
		let domain = state.config.domain.clone();
		let app = App::new()
			.app_data(Data::new(state.clone()))
			.wrap(middleware::Logger::default())
			.wrap_fn(move |req, srv| {
				// Test for CSRF, check origin header
				if !check_csrf(&req, domain.as_deref()) {
					future::err(
						io::Error::new(io::ErrorKind::Other, "Cross origin request denied").into(),
					)
					.right_future()
				} else {
					srv.call(req).left_future()
				}
			});

		let cookie_session_store = CookieSessionStore::default();
		let session_middleware = SessionMiddleware::builder(
			cookie_session_store,
			actix_web::cookie::Key::derive_from(&key),
		)
		.cookie_name("user".into())
		.cookie_secure(state.config.secure)
		.cookie_same_site(actix_web::cookie::SameSite::Strict)
		.cookie_domain(state.config.domain.clone())
		.session_lifecycle(
			PersistentSession::default()
				.session_ttl(cookie_maxtime())
				.session_ttl_extension_policy(TtlExtensionPolicy::OnEveryRequest),
		);

		let mut app = app
			.wrap(
				IdentityMiddleware::builder()
					.visit_deadline(Some(cookie_maxtime().unsigned_abs()))
					.build(),
			)
			.wrap(session_middleware.build())
			.service(
				web::scope("/api")
					.service(auth::login)
					.service(auth::login_nojs)
					.service(auth::logout)
					.service(menu)
					.service(signup::signup_state)
					.service(signup::signup)
					.service(signup::signup_nojs)
					.service(signup_supervisor::signup)
					.service(signup_supervisor::signup_nojs)
					.service(
						web::scope("/admin")
							.wrap(HasRolePredicate::new(auth::Roles::Admin, true))
							.service(admin::download_mails)
							.service(admin::download_members)
							.service(admin::download_supervisors)
							.service(admin::lager_info)
							.service(admin::remove_lager)
							.service(admin::remove_member)
							.service(admin::edit_member)
							.service(admin::remove_supervisor)
							.service(admin::edit_supervisor),
					)
					.service(
						web::scope("/erwischt")
							.wrap(HasRolePredicate::new(auth::Roles::Erwischt, true))
							.service(erwischt::get_games)
							.service(erwischt::get_game)
							.service(erwischt::create_game)
							.service(erwischt::delete_game)
							.service(erwischt::catch)
							.service(erwischt::insert),
					),
			);

		// Do not use last-modified, because it goes wrong when building with nix and the timestamp is 0
		for name in &image_dirs {
			let name2 = name.clone();
			app = app
				.service(
					web::scope(&format!("/Bilder{}/list", name))
						.wrap(HasRolePredicate::new(auth::Roles::Images(name.clone()), true))
						.route("", web::get().to(move || images::list_images(name2.clone()))),
				)
				.service(
					web::scope(&format!("/Bilder{}/static", name))
						.wrap(HasRolePredicate::new(auth::Roles::Images(name.clone()), false))
						.service(
							Files::new("", format!("Bilder{}", name))
								.use_last_modified(false)
								.mime_override(content_disposition_map)
								.default_handler(web::to(not_found)),
						),
				);
		}

		// Serve frontend files
		app.wrap(ImagesPathRewriter { image_dirs: image_dirs.clone() })
			.service(
				Files::new("", "frontend/build")
					.use_last_modified(false)
					.mime_override(content_disposition_map)
					.index_file("index.html")
					.default_handler(web::to(not_found)),
			)
			.default_service(web::to(not_found))
	})
	.bind(address)?
	.run()
	.await?;

	Ok(())
}
