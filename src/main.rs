#[macro_use]
extern crate diesel;

use std::collections::HashMap;
use std::convert::{Infallible, TryFrom, TryInto};
use std::fs;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use anyhow::{Result, format_err};
use axum::body::{Body, Bytes};
use axum::error_handling::HandleErrorLayer;
use axum::extract::{Query, Request};
use axum::handler::HandlerWithoutStateExt;
use axum::http::header::LAST_MODIFIED;
use axum::http::{self, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router, extract};
use axum_oidc::error::MiddlewareError;
use axum_oidc::openidconnect::{ClientId, ClientSecret, IssuerUrl};
use axum_oidc::{
	EmptyAdditionalClaims, OidcAuthLayer, OidcClient, OidcLoginLayer, handle_oidc_redirect,
};
use clap::Parser;
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::{Date, Duration};
use tower::{Layer, ServiceBuilder, ServiceExt};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::{ExpiredDeletion, Expiry, Session, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tower_sessions_sqlx_store::sqlx::Pool;
use tracing::{error, info, warn};

mod admin;
mod auth;
mod basic;
mod config;
mod db;
mod erwischt;
mod etag;
mod images;
mod mail;
mod management;
mod signup;
mod signup_supervisor;
mod thumbs;

use crate::config::{Config, MailAddress};

const DEFAULT_PREFIX: &str = "public";
const RATELIMIT_MAX_COUNTER: i32 = 50;
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

type WebResult<T, E = Response> = Result<T, E>;
type ExtractState = extract::State<Arc<State>>;
type OidcClaims = axum_oidc::OidcClaims<EmptyAdditionalClaims>;

pub struct State {
	sites: HashMap<String, basic::SiteDescriptions>,
	config: Config,
	db: db::Database,
	mail: mail::Mail,
	/// Sizes of thumbnails.
	/// Map path to width, height.
	thumbs: RwLock<HashMap<String, Thumb>>,
	/// Used to lock access to the log file.
	log_mutex: Mutex<()>,
}

#[derive(Clone, Debug, Serialize)]
struct MenuItem {
	title: String,
	link: String,
}

#[derive(Clone, Debug, Default, Serialize)]
struct Thumb {
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	thumb: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	width: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	height: Option<u32>,
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

#[derive(Clone, Debug, Deserialize)]
struct OauthLoginData {
	redirect: Option<String>,
}

fn rewrite_images_path<B>(image_dirs: &[String], mut req: Request<B>) -> Request<B> {
	let uri = req.uri_mut();
	let mut path = uri.path();
	if path.starts_with("/Bilder") {
		for name in image_dirs {
			if path.trim_end_matches('/') == format!("/Bilder{}", name) {
				path = "/images/";
				break;
			}
		}
	}
	if path != uri.path() {
		let mut parts = uri.clone().into_parts();

		let path = match uri.query() {
			Some(q) => Bytes::from(format!("{}?{}", path, q)),
			None => Bytes::copy_from_slice(path.as_bytes()),
		};
		parts.path_and_query = Some(http::uri::PathAndQuery::from_maybe_shared(path).unwrap());

		*uri = http::Uri::from_parts(parts).unwrap();
	}
	req
}

impl TryInto<Mailbox> for MailAddress {
	type Error = anyhow::Error;
	fn try_into(self) -> Result<Mailbox> {
		Ok(Mailbox { name: self.name, email: self.address.parse()? })
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

async fn check_csrf(
	extract::State(config): extract::State<Config>, req: Request, next: axum::middleware::Next,
) -> Response {
	if !req.method().is_safe() {
		if let Some(domain) = config.domain {
			if let Some(header) = get_origin(req.headers()) {
				match header {
					Ok(ref origin) if origin.ends_with(&domain) => {}
					Ok(ref origin) => {
						warn!(%origin, %domain, "Origin does not end with domain");
						return (StatusCode::BAD_REQUEST, "Cross origin request denied")
							.into_response();
					}
					Err(error) => {
						warn!(%error, "Origin not found");
						return (StatusCode::BAD_REQUEST, "Cross origin request denied")
							.into_response();
					}
				}
			}
		}
	}
	next.run(req).await
}

fn remove_last_modified(mut resp: Response) -> Response {
	resp.headers_mut().remove(LAST_MODIFIED);
	resp
}

#[derive(Clone)]
struct HasRolePredicate {
	state: Arc<State>,
	/// The role to check for
	role: auth::Roles,
	/// If this is an API endpoint or a user-facing endpoint
	is_api: bool,
}

impl HasRolePredicate {
	fn new(state: Arc<State>, role: auth::Roles, is_api: bool) -> Self {
		Self { state, role, is_api }
	}
}

async fn has_role(
	extract::State(this): extract::State<HasRolePredicate>, req: Request,
	next: axum::middleware::Next,
) -> Response {
	let Some(session) = req.extensions().get::<Session>() else {
		error!("Failed to get session");
		return error_response::<()>(&this.state).unwrap_err();
	};
	let oidc = req.extensions().get::<OidcClaims>().cloned();
	let roles = match auth::get_roles(&this.state, &session, &oidc).await {
		Ok(r) => r,
		Err(error) => {
			error!(%error, "Failed to get roles");
			return error_response::<()>(&this.state).unwrap_err();
		}
	};
	if let Some(roles) = roles {
		if roles.contains(&this.role) { next.run(req).await } else { forbidden(req) }
	} else {
		// Not logged in
		if this.is_api {
			// Return an error that can be displayed
			(StatusCode::UNAUTHORIZED, "Bitte anmelden, Sie sind ausgeloggt.").into_response()
		} else {
			let location = format!(
				"/login?redirect={}",
				url::form_urlencoded::byte_serialize(req.uri().path().as_bytes())
					.collect::<String>()
			);
			Response::builder()
				.status(StatusCode::FOUND)
				.header("location", location)
				.body(Body::empty())
				.unwrap()
		}
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

impl State {
	fn get_kanidm_base_url(&self) -> Result<&str> {
		if let Some(config) = &self.config.oidc {
			let prefix = "https://".len();
			let i = config.server_url[prefix..]
				.find('/')
				.map(|i| i + prefix)
				.unwrap_or(config.server_url.len());
			Ok(&config.server_url[..i])
		} else {
			Err(format_err!("Oidc not configured"))
		}
	}

	fn get_kanidm_token(&self) -> Result<&str> {
		if let Some(config) = &self.config.kanidm {
			Ok(&config.token)
		} else {
			Err(format_err!("Kanidm user access token not configured"))
		}
	}
}

fn error_response<T>(state: &State) -> WebResult<T, Response> {
	Err((
		StatusCode::INTERNAL_SERVER_ERROR,
		format!("Es ist ein interner Fehler aufgetreten.\n{}", state.config.error_message),
	)
		.into_response())
}

async fn not_found(req: Request) -> Response {
	warn!(path = %req.uri(), "File not found");
	(StatusCode::NOT_FOUND, "Page not found").into_response()
}

fn forbidden(req: Request) -> Response {
	warn!(path = %req.uri(), "Forbidden");
	(StatusCode::NOT_FOUND, "Page not found").into_response()
}

async fn redirect_start(Query(data): Query<OauthLoginData>) -> Response {
	Response::builder()
		.status(StatusCode::FOUND)
		.header("location", data.redirect.as_deref().unwrap_or("/"))
		.body(Body::empty())
		.unwrap()
}

async fn menu(
	extract::State(state): ExtractState, Query(data): Query<MenuRequestData>, session: Session,
	oidc: Option<OidcClaims>,
) -> Result<Json<MenuData>, Response> {
	if let Some(site_descriptions) = data
		.prefix
		.as_deref()
		.and_then(|p| state.sites.get(p))
		.or_else(|| state.sites.get(DEFAULT_PREFIX))
	{
		let roles = match auth::get_roles(&state, &session, &oidc).await {
			Ok(r) => r,
			Err(error) => {
				error!(%error, "Failed to get roles");
				return error_response(&state);
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

		Ok(Json(MenuData {
			is_logged_in: roles.is_some(),
			global_message: state.config.global_message.clone(),
			items: menu_items,
		}))
	} else {
		error!(prefix = data.prefix, "Did not find site prefix");
		crate::error_response(&state)
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or(
			tracing_subscriber::EnvFilter::new("tower_http=debug,zeltlager_website=info,info"),
		))
		.init();

	// Command line arguments
	let args = config::Args::parse();

	let content = fs::read_to_string("config.toml")?;
	let config: Config = toml::from_str(&content)?;
	config.validate().unwrap();

	if let Some(action) = args.action {
		management::cmd_action(&config, action).await?;
		return Ok(());
	}

	let mut sites = HashMap::new();
	for name in &["public", "intern"] {
		sites.insert(
			name.to_string(),
			basic::SiteDescriptions::parse(&format!("{}.toml", name))
				.unwrap_or_else(|e| panic!("Failed to parse {}.toml ({:?})", name, e)),
		);
	}

	let database = db::Database::new(&config)?;
	database.run_migrations().await?;

	let mail = mail::Mail::new(config.clone());

	let address = config.bind_address.clone();
	let state = Arc::new(State {
		sites,
		config,
		db: database,
		mail,
		thumbs: Default::default(),
		log_mutex: Mutex::new(()),
	});

	// Start thumbnail creator
	let mut image_dirs = Vec::new();
	for d in fs::read_dir(".")? {
		let d = d?;
		if let Some(path) =
			d.path().file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_prefix("Bilder"))
		{
			image_dirs.push(path.to_string());
			let state2 = state.clone();
			std::thread::spawn(move || thumbs::watch_thumbs(&state2, d.file_name().into()));
		}
	}

	let api_admin_routes = Router::new()
		.route("/mails", get(admin::download_mails))
		.route("/teilnehmer", get(admin::download_members))
		.route("/betreuer", get(admin::download_supervisors))
		.route("/lager", get(admin::lager_info).delete(admin::remove_lager))
		.route("/teilnehmer/remove", post(admin::remove_member))
		.route("/teilnehmer/edit", post(admin::edit_member))
		.route("/betreuer/remove", post(admin::remove_supervisor))
		.route("/betreuer/edit", post(admin::edit_supervisor))
		.route("/user/list", get(admin::list_users))
		.route("/user/reset_password", post(admin::reset_password))
		.route("/user/create", post(admin::create_user))
		.layer(axum::middleware::from_fn_with_state(
			HasRolePredicate::new(state.clone(), auth::Roles::Admin, true),
			has_role,
		));

	let api_erwischt_routes = Router::new()
		.route("/games", get(erwischt::get_games))
		.route("/game/{id}", get(erwischt::get_game).delete(erwischt::delete_game))
		.route("/game", post(erwischt::create_game))
		.route("/game/setCatch", post(erwischt::catch))
		.route("/game/insert", post(erwischt::insert))
		.layer(axum::middleware::from_fn_with_state(
			HasRolePredicate::new(state.clone(), auth::Roles::Erwischt, true),
			has_role,
		));

	let mut api_routes = Router::new()
		.route("/login", post(auth::login))
		.route("/login-nojs", post(auth::login_nojs))
		.route("/logout", get(auth::logout))
		.route("/menu", get(menu))
		.route("/signup-state", get(signup::signup_state))
		.route("/signup", post(signup::signup))
		.route("/signup-nojs", post(signup::signup_nojs))
		.route("/signup-supervisor", post(signup_supervisor::signup))
		.route("/signup-supervisor-nojs", post(signup_supervisor::signup_nojs))
		.route("/resignup-supervisor", post(signup_supervisor::resignup))
		.route("/get-supervisor-data", post(signup_supervisor::get_data))
		.route("/presignup-supervisor", post(signup_supervisor::presignup))
		.route("/presignup-supervisor-nojs", post(signup_supervisor::presignup_nojs))
		.nest("/admin", api_admin_routes)
		.nest("/erwischt", api_erwischt_routes);

	let etag_layer =
		axum::middleware::from_fn_with_state(etag::EtagLayer::new(), etag::compute_etag);

	// Do not use last-modified, because it goes wrong when building with nix and the timestamp is 0
	let mut app = Router::new();
	for name in &image_dirs {
		let name2 = name.clone();
		app = app.merge(
			Router::new()
				.route(
					&format!("/Bilder{}/list", name),
					get(async move |extract::State(state): ExtractState| {
						images::list_images(&state, &name2).await
					}),
				)
				.nest(
					&format!("/Bilder{}/static", name),
					Router::new().fallback_service(
						etag_layer
							.layer(
								ServeDir::new(format!("Bilder{}", name)).fallback(any(not_found)),
							)
							.map_response(remove_last_modified),
					),
				)
				.layer(axum::middleware::from_fn_with_state(
					HasRolePredicate::new(state.clone(), auth::Roles::Images(name.clone()), true),
					has_role,
				)),
		);
	}

	let rewrite_img_path_middleware =
		tower::util::MapRequestLayer::new(move |req| rewrite_images_path(&image_dirs, req));

	let session_store = PostgresStore::new(Pool::connect(&state.config.database).await?);
	session_store.migrate().await?;
	tokio::task::spawn(
		session_store.clone().continuously_delete_expired(tokio::time::Duration::from_mins(10)),
	);

	let mut session_layer = SessionManagerLayer::new(session_store)
		.with_name("user")
		.with_secure(state.config.secure)
		.with_same_site(tower_sessions::cookie::SameSite::Strict)
		.with_expiry(Expiry::OnInactivity(cookie_maxtime()))
		.with_always_save(true);

	if let Some(domain) = &state.config.domain() {
		session_layer = session_layer.with_domain(domain.to_string());
	}

	if let Some(cfg) = &state.config.oidc {
		let redirect_url = format!(
			"http{}://{}/api/oauth2/callback",
			if state.config.secure { "s" } else { "" },
			if let Some(origin) = &state.config.domain {
				origin
			} else {
				&state.config.bind_address
			}
		);

		let oidc_client = OidcClient::<EmptyAdditionalClaims>::builder()
			.with_client_id(ClientId::new(cfg.client_id.clone()))
			.with_client_secret(ClientSecret::new(cfg.client_secret.clone()))
			.with_redirect_url(Uri::from_maybe_shared(redirect_url).unwrap())
			.with_default_http_client()
			.discover(IssuerUrl::new(cfg.server_url.clone())?)
			.await?
			.build();

		let oidc_auth_service = ServiceBuilder::new()
			.layer(HandleErrorLayer::new(|error: MiddlewareError| async {
				warn!(%error, "oidc error");
				error.into_response()
			}))
			.layer(OidcAuthLayer::new(oidc_client));

		let oidc_login_service = ServiceBuilder::new()
			.layer(HandleErrorLayer::new(|error: MiddlewareError| async {
				warn!(%error, "oidc login error");
				error.into_response()
			}))
			.layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

		api_routes = api_routes
			.route("/oauth2/login", get(redirect_start).layer(oidc_login_service))
			.route("/oauth2/callback", any(handle_oidc_redirect::<EmptyAdditionalClaims>))
			.layer(oidc_auth_service);
	}

	let serve_frontend: tower::util::BoxCloneSyncService<Request, Response, Infallible> =
		if args.dev {
			async fn forward(uri: Uri) -> Response {
				match reqwest::get(format!(
					"http://localhost:5173{}",
					uri.path_and_query().unwrap().as_str()
				))
				.await
				{
					Ok(resp) => {
						let mut ret = Response::builder();
						*ret.headers_mut().unwrap() = resp.headers().clone();
						ret.body(Body::from_stream(resp.bytes_stream())).unwrap()
					}
					Err(error) => {
						warn!(%error, "Failed to forward request");
						"Internal error".into_response()
					}
				}
			}
			tower::util::BoxCloneSyncService::new(HandlerWithoutStateExt::into_service(forward))
		} else {
			tower::util::BoxCloneSyncService::new(
				etag_layer.layer(ServeDir::new("frontend/build").fallback(any(not_found))),
			)
		};

	let app = app
		.nest("/api", api_routes)
		.fallback_service(
			rewrite_img_path_middleware.layer(serve_frontend).map_response(remove_last_modified),
		)
		.layer(axum::middleware::from_fn_with_state(state.config.clone(), check_csrf))
		.layer(TraceLayer::new_for_http())
		.layer(session_layer)
		.with_state(state)
		.into_make_service_with_connect_info::<SocketAddr>();

	info!(%address, "Starting server");
	let listener = tokio::net::TcpListener::bind(address).await?;
	axum::serve(listener, app).await?;

	Ok(())
}
