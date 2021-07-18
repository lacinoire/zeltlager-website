//! User authentication (login/logout)
//! and authorization (rights management).

use std::borrow::Cow;
use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::*;
use anyhow::{bail, format_err, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{error, info, warn};
use serde::Deserialize;
use strum_macros::EnumString;
use t4rust_derive::Template;

use crate::form::Form;
use crate::{auth, db, State};

#[derive(Clone, Copy, EnumString, Debug, Deserialize, PartialEq, Eq)]
pub enum Roles {
	Admin,
	Erwischt,
	ImageDownload2018,
	ImageDownload2019,
	ImageDownload2020,
	ImageDownload2021,
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
	fn get_values(&self) -> Cow<HashMap<String, String>> { Cow::Borrowed(&self.values) }
}

impl Login {
	fn new(values: HashMap<String, String>) -> Login { Login { values } }
}

async fn rate_limit(state: &State, req: &HttpRequest) -> Result<()> {
	let ip = req
		.connection_info()
		.realip_remote_addr()
		.ok_or_else(|| format_err!("no ip detected"))?
		.to_string();
	match state.db_addr.send(db::CheckRateMessage { ip }).await {
		Ok(result) => {
			if result? {
				Ok(())
			} else {
				bail!("Rate limit exceeded");
			}
		}
		Err(msg) => bail!(msg),
	}
}

/// Return the login site with the prefilled `values`.
///
/// The `values` can contain the `username` and an `error`.
async fn render_login(
	state: &State, id: &Identity, req: &HttpRequest, values: HashMap<String, String>,
) -> HttpResponse {
	let roles = match auth::get_roles(state, id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(state);
		}
	};
	if let Ok(site) = state.sites["public"].get_site(state.config.clone(), "login", roles) {
		let mut resp = if values.contains_key("error") {
			HttpResponse::Unauthorized()
		} else {
			HttpResponse::Ok()
		};

		let content = format!("{}", site);
		let content = content.replace("<insert content here>", &format!("{}", Login::new(values)));

		resp.content_type("text/html; charset=utf-8").body(content)
	} else {
		crate::not_found(state, id, req).await
	}
}

#[derive(Deserialize)]
pub struct LoginArgs {
	redirect: Option<String>,
}

#[get("/login")]
pub async fn login(
	state: web::Data<State>, id: Identity, req: HttpRequest, mut args: web::Query<LoginArgs>,
) -> HttpResponse {
	if logged_in_user(&id).is_some() {
		let redir = args.redirect.as_ref().map(|s| s.as_str()).unwrap_or("/");
		HttpResponse::Found().append_header(("location", redir)).finish()
	} else {
		let mut values = HashMap::new();
		if let Some(redirect) = args.redirect.take() {
			values.insert("redirect".to_string(), redirect);
		}
		render_login(&state, &id, &req, values).await
	}
}

fn set_logged_in(id: i32, identity: &Identity) {
	// Logged in: Store "user id|timeout"
	identity.remember(format!(
		"{}|{}",
		id,
		(Utc::now() + crate::cookie_maxtime()).format("%Y-%m-%d %H:%M:%S")
	));
}

#[post("/login")]
pub async fn login_send(
	state: web::Data<State>, req: HttpRequest, identity: Identity,
	mut body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	// Search user in database
	let db_addr = state.db_addr.clone();
	let error_message = state.config.error_message.clone();

	// Check rate limit
	let redirect = body.get("redirect").map(Clone::clone);
	let msg = match db::AuthenticateMessage::from_hashmap(body.clone()) {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get authentication message: {}", e);
			return crate::error_response(&**state);
		}
	};
	body.remove("password");

	if let Err(error) = rate_limit(&**state, &req).await {
		body.insert(
			"error".to_string(),
			"Zu viele Login Anfragen. Probieren Sie es später noch einmal.".to_string(),
		);
		info!("Rate limit exceeded ({:?})", error);
		render_login(&**state, &identity, &req, body.into_inner()).await
	} else {
		match db_addr.send(msg).await.map_err(|e| e.into()) {
			Err(error) | Ok(Err(error)) => {
				// Show error and prefilled form
				body.insert(
					"error".to_string(),
					format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message),
				);
				warn!("Error by auth message: {}", error);
				render_login(&**state, &identity, &req, body.into_inner()).await
			}
			Ok(Ok(Some(id))) => {
				set_logged_in(id, &identity);
				let ip = match req
					.connection_info()
					.realip_remote_addr()
					.ok_or_else(|| format_err!("no ip detected"))
				{
					Ok(r) => r.to_string(),
					Err(e) => {
						error!("Failed to get ip: {}", e);
						return crate::error_response(&**state);
					}
				};
				if let Err(e) = state.db_addr.send(db::DecreaseRateCounterMessage { ip }).await {
					error!("Failed to decrease rate limiting counter: {}", e);
				}
				// Redirect somewhere else if there is a
				// redirect argument.
				if let Some(redirect) = redirect {
					let redirect = redirect.trim_start_matches('/');
					let redirect = format!("/{}", redirect);
					HttpResponse::Found().append_header(("location", redirect.as_str())).finish()
				} else {
					HttpResponse::Found().append_header(("location", "/")).finish()
				}
			}
			Ok(Ok(None)) => {
				// Wrong username or password
				// Show error and prefilled form
				body.insert(
					"error".to_string(),
					"Falsches Passwort oder falscher Benutzername".to_string(),
				);
				render_login(&*state, &identity, &req, body.into_inner()).await
			}
		}
	}
}

#[get("/logout")]
pub fn logout(id: Identity) -> HttpResponse {
	id.forget();
	HttpResponse::Found().append_header(("location", "/")).finish()
}

// Utility methods for other modules

/// Get the id of the logged in user
pub fn logged_in_user(identity: &Identity) -> Option<i32> {
	identity
		.identity()
		.and_then(|s| {
			if let [id, timeout] = *s.splitn(2, '|').collect::<Vec<_>>().as_slice() {
				if let Ok(id) = id.parse::<i32>() {
					if let Ok(timeout) = NaiveDateTime::parse_from_str(timeout, "%Y-%m-%d %H:%M:%S")
					{
						let timeout = DateTime::<Utc>::from_utc(timeout, Utc);
						return Some((id, timeout));
					}
				}
			}
			None
		})
		.and_then(|(id, timeout)| {
			// Check if the token expired
			if timeout < Utc::now() {
				return None;
			}
			// Refresh token
			set_logged_in(id, identity);
			Some(id)
		})
}

pub async fn get_roles(state: &State, id: &Identity) -> Result<Option<Vec<Roles>>> {
	if let Some(user) = logged_in_user(id) {
		Ok(Some(user_get_roles(state, user).await?))
	} else {
		Ok(None)
	}
}

pub async fn user_get_roles(state: &State, user: i32) -> Result<Vec<Roles>> {
	let msg = db::GetRolesMessage { user };
	Ok(state
		.db_addr
		.send(msg)
		.await
		.map_err(|e| format_err!("Failed to get user roles: {}", e))??)
}
