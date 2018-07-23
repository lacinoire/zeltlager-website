//! User authentication (login/logout)
//! and authorization (rights management).

use std::borrow::Cow;
use std::collections::HashMap;

use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, HttpMessage, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use failure;
use futures::{Future, IntoFuture};

use form::Form;
use {AppState, BoxFuture, Result};

#[derive(Clone, EnumString, Debug, Deserialize)]
pub enum Roles {
	ImageDownload2018,
	ImageUpload,
}

#[derive(Template)]
#[TemplatePath = "templates/login.tt"]
#[derive(Debug)]
pub struct Login {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
}

impl Form for Login {
	fn get_values(&self) -> Cow<HashMap<String, String>> {
		Cow::Borrowed(&self.values)
	}
}

impl Login {
	fn new(values: HashMap<String, String>) -> Login {
		Login { values }
	}
}

fn rate_limit(req: &HttpRequest<AppState>) -> BoxFuture<()> {
	let ip = tryf!(
		req.connection_info()
			.remote()
			.ok_or_else(|| format_err!("no ip detected"))
	).to_string();
	let res = req.state().db_addr.send(::db::CheckRateMessage { ip });
	Box::new(res.from_err().and_then(|db_result| match db_result {
		Ok(result) => {
			if result {
				Ok(())
			} else {
				bail!("Rate limit exceeded");
			}
		}
		Err(msg) => bail!(msg),
	}))
}

/// Return the login site with the prefilled `values`.
///
/// The `values` can contain the `username` and an `error`.
fn render_login(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> Result<HttpResponse> {
	if let Ok(site) =
		req.state().sites["public"].get_site(&req.state().config, "login")
	{
		let content = format!("{}", site);
		let login = format!("{}", Login::new(values));
		let content = content.replace("<insert content here>", &login);

		return Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(content));
	}
	::not_found(&req)
}

pub fn login(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	if logged_in_user(&req).is_some() {
		Ok(HttpResponse::Found().header("location", "/").finish())
	} else {
		render_login(req, HashMap::new())
	}
}

fn set_logged_in(id: i32, req: &HttpRequest<AppState>) {
	// Logged in: Store "user id|timeout"
	req.remember(format!("{}|{}", id, Utc::now() + ::cookie_maxtime()));
}

pub fn login_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	// Search user in database
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();

	Box::new(
		// Check rate limit
		req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let msg = tryf!(::db::AuthenticateMessage::
				from_hashmap(body.clone()));
			body.remove("password");

			Box::new(rate_limit(&req).then(move |limit| -> BoxFuture<_> {
				if limit.is_err() {
					body.insert("error".to_string(), format!(
						"Zu viele Login Anfragen. \
						Probieren Sie es später noch einmal.",
					));
					info!("Rate limit exceeded");
					Box::new(render_login(req, body).into_future())
				} else {
					Box::new(db_addr.send(msg)
						.from_err::<failure::Error>()
						.then(move |result| -> Result<HttpResponse> { match result {
							Err(error) | Ok(Err(error)) => {
								// Show error and prefilled form
								body.insert("error".to_string(), format!(
									"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
									error_message));
								warn!("Error by auth message: {}", error);
								render_login(req, body)
							}
							Ok(Ok(Some(id))) => {
								set_logged_in(id, &req);
								Ok(HttpResponse::Found().header("location", "/").finish())
							}
							Ok(Ok(None)) => {
								// Wrong username or password
								// Show error and prefilled form
								body.insert("error".to_string(),
									"Falsches Passwort oder falscher Benutzername"
									.to_string());
								render_login(req, body)
							}
						}}))
			}
		}))})
		.responder(),
	)
}

pub fn logout(req: HttpRequest<AppState>) -> HttpResponse {
	req.forget();
	HttpResponse::Found().header("location", "/").finish()
}

// Utility methods for other modules

/// Get the id of the logged in user
pub fn logged_in_user(req: &HttpRequest<AppState>) -> Option<i32> {
	req.identity()
		.and_then(|s| {
			if let &[id, timeout] =
				s.splitn(2, '|').collect::<Vec<_>>().as_slice()
			{
				if let Ok(id) = id.parse::<i32>() {
					if let Ok(timeout) = timeout.parse::<DateTime<Utc>>() {
						return Some((id, timeout));
					}
					println!("time {}", timeout);
				}
			}
			None
		})
		.and_then(|(id, timeout)| {
			println!("Token expires at {}", timeout);
			// Check if the token expired
			if timeout < Utc::now() {
				return None;
			}
			// Refresh token
			set_logged_in(id, req);
			Some(id)
		})
}
