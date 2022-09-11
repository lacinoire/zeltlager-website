//! User authentication (login/logout)
//! and authorization (rights management).

use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::{bail, format_err, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{error, info, warn};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{db, State};

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

fn set_logged_in(id: i32, identity: &Identity) {
	// Logged in: Store "user id|timeout"
	identity.remember(format!(
		"{}|{}",
		id,
		(Utc::now() + crate::cookie_maxtime()).format("%Y-%m-%d %H:%M:%S")
	));
}

async fn login_internal(
	state: &State, req: &HttpRequest, identity: &Identity, body: HashMap<String, String>,
) -> (StatusCode, LoginResult) {
	// Search user in database
	let db_addr = state.db_addr.clone();
	let error_message = &state.config.error_message;

	// Check rate limit
	let msg = match db::AuthenticateMessage::from_hashmap(body) {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get authentication message: {}", e);
			return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
				error: Some(format!("Es ist ein interner Fehler aufgetreten.\n{}", error_message)),
			});
		}
	};

	if let Err(error) = rate_limit(state, req).await {
		info!("Rate limit exceeded ({:?})", error);
		(StatusCode::FORBIDDEN, LoginResult {
			error: Some("Zu viele Login Anfragen. Probieren Sie es später noch einmal.".into()),
		})
	} else {
		match db_addr.send(msg).await.map_err(|e| e.into()) {
			Err(error) | Ok(Err(error)) => {
				// Show error
				warn!("Error by auth message: {}", error);
				(StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
					error: Some(format!(
						"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
						error_message
					)),
				})
			}
			Ok(Ok(Some(id))) => {
				set_logged_in(id, identity);
				let ip = match req
					.connection_info()
					.realip_remote_addr()
					.ok_or_else(|| format_err!("no ip detected"))
				{
					Ok(r) => r.to_string(),
					Err(e) => {
						error!("Failed to get ip: {}", e);
						return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
							error: Some(format!(
								"Es ist ein interner Fehler aufgetreten.\n{}",
								error_message
							)),
						});
					}
				};
				if let Err(e) = state.db_addr.send(db::DecreaseRateCounterMessage { ip }).await {
					error!("Failed to decrease rate limiting counter: {}", e);
				}
				(StatusCode::OK, LoginResult { error: None })
			}
			Ok(Ok(None)) => {
				// Wrong username or password
				// Show error and prefilled form
				(StatusCode::FORBIDDEN, LoginResult {
					error: Some("Falsches Passwort oder falscher Benutzername".into()),
				})
			}
		}
	}
}

#[post("/login")]
pub async fn login(
	state: web::Data<State>, req: HttpRequest, identity: Identity,
	body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = login_internal(&**state, &req, &identity, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/login-nojs")]
pub async fn login_nojs(
	state: web::Data<State>, req: HttpRequest, identity: Identity,
	body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = login_internal(&**state, &req, &identity, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found().append_header(("location", "/")).finish()
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
