//! User authentication (login/logout)
//! and authorization (rights management).

use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::{Result, bail, format_err};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{error, info, warn};

use crate::{State, db};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Roles {
	Admin,
	Erwischt,
	Images(String),
}

#[derive(Clone, Debug, Serialize)]
struct LoginResult {
	error: Option<String>,
}

impl std::str::FromStr for Roles {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"Admin" => Roles::Admin,
			"Erwischt" => Roles::Erwischt,
			_ => {
				if let Some(val) = s.strip_prefix("Images") {
					Roles::Images(val.to_string())
				} else {
					return Err(format_err!("Unknown role '{}'", s));
				}
			}
		})
	}
}

impl<'de> Deserialize<'de> for Roles {
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		let s = String::deserialize(d)?;
		match s.parse() {
			Ok(r) => Ok(r),
			Err(e) => Err(D::Error::custom(e.to_string())),
		}
	}
}

async fn rate_limit(state: &State, req: &HttpRequest) -> Result<()> {
	let ip = req
		.connection_info()
		.realip_remote_addr()
		.ok_or_else(|| format_err!("no ip detected"))?
		.to_string();
	match state.db.check_rate(&ip).await {
		Ok(true) => Ok(()),
		Ok(false) => bail!("Rate limit exceeded"),
		Err(msg) => bail!(msg),
	}
}

fn set_logged_in(id: i32, request: &HttpRequest) -> Result<()> {
	// Logged in: Store "user id"
	Identity::login(&request.extensions(), id.to_string())?;
	Ok(())
}

async fn login_internal(
	state: &State, req: &HttpRequest, body: HashMap<String, String>,
) -> (StatusCode, LoginResult) {
	// Check rate limit
	if let Err(error) = rate_limit(state, req).await {
		info!(%error, "Rate limit exceeded");
		return (StatusCode::FORBIDDEN, LoginResult {
			error: Some("Zu viele Login Anfragen. Probieren Sie es später noch einmal.".into()),
		});
	}

	// Search user in database
	let msg = match db::Authentication::from_hashmap(body) {
		Ok(r) => r,
		Err(error) => {
			error!(?error, "Failed to get authentication message");
			return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
				error: Some(format!(
					"Es ist ein interner Fehler aufgetreten.\n{}",
					state.config.error_message
				)),
			});
		}
	};

	match state.db.authenticate(&msg).await {
		Err(error) => {
			// Show error
			warn!(%error, "Error by auth message");
			(StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
				error: Some(format!(
					"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
					state.config.error_message
				)),
			})
		}
		Ok(Some(id)) => {
			if let Err(error) = set_logged_in(id, req) {
				warn!(%error, "Failed to set login identity");
				return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
					error: Some(format!(
						"Es ist ein Fehler beim Login aufgetreten.\n{}",
						state.config.error_message
					)),
				});
			}
			let ip = match req
				.connection_info()
				.realip_remote_addr()
				.ok_or_else(|| format_err!("no ip detected"))
			{
				Ok(r) => r.to_string(),
				Err(error) => {
					error!(%error, "Failed to get ip");
					return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
						error: Some(format!(
							"Es ist ein interner Fehler aufgetreten.\n{}",
							state.config.error_message
						)),
					});
				}
			};
			if let Err(error) = state.db.decrease_rate_counter(&ip).await {
				error!(%error, "Failed to decrease rate limiting counter");
			}
			(StatusCode::OK, LoginResult { error: None })
		}
		Ok(None) => {
			// Wrong username or password
			// Show error and prefilled form
			(StatusCode::FORBIDDEN, LoginResult {
				error: Some("Falsches Passwort oder falscher Benutzername".into()),
			})
		}
	}
}

#[post("/login")]
pub async fn login(
	state: web::Data<State>, req: HttpRequest, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = login_internal(&state, &req, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/login-nojs")]
pub async fn login_nojs(
	state: web::Data<State>, req: HttpRequest, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = login_internal(&state, &req, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found().append_header(("location", "/")).finish()
	}
}

#[get("/logout")]
pub async fn logout(id: Identity) -> HttpResponse {
	id.logout();
	HttpResponse::Found().append_header(("location", "/")).finish()
}

// Utility methods for other modules

/// Get the id of the logged in user
pub fn logged_in_user(identity: &Option<Identity>) -> Option<i32> {
	identity.as_ref().and_then(|i| i.id().ok()).and_then(|id| id.parse::<i32>().ok())
}

pub async fn get_roles(state: &State, id: &Option<Identity>) -> Result<Option<Vec<Roles>>> {
	if let Some(user) = logged_in_user(id) {
		let roles = state
			.db
			.get_roles(user)
			.await
			.map_err(|e| format_err!("Failed to get user roles: {}", e))?;
		Ok(Some(roles))
	} else {
		Ok(None)
	}
}
