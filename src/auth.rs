//! User authentication (login/logout)
//! and authorization (rights management).

use std::borrow::Cow;
use std::collections::HashMap;

use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, HttpMessage, HttpRequest, HttpResponse};
use failure;
use futures::{future, Future};

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
	render_login(req, HashMap::new())
}

pub fn login_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	// Search user in database
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();

	Box::new(
		req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let msg = tryf!(::db::AuthenticateMessage::
				from_hashmap(body.clone()));
			let username = msg.username.clone();

			Box::new(db_addr.send(msg)
				.from_err::<failure::Error>()
				.then(move |result| -> Result<HttpResponse> { match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert("error".to_string(), format!("\
							Es ist ein Datenbank-Fehler aufgetreten.\n{}",
							error_message));
						warn!("Error by auth message: {}", error);
						// TODO Actually, do nothing
						render_login(req, HashMap::new())
					}
					Ok(Ok(true)) => {
						req.remember(username);
						Ok(HttpResponse::Found().header("location", "/images").finish())
					}
					_ => {
						// Wrong username or password
						Ok(HttpResponse::Found().header("location", "/wrong").finish())
					}
				}})
		)})
		.responder(),
	)
}

pub fn logout(req: HttpRequest<AppState>) -> HttpResponse {
	req.forget();
	HttpResponse::Found().header("location", "/").finish()
}
