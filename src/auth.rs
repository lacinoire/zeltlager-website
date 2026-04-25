//! User authentication (login/logout)
//! and authorization (rights management).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

use anyhow::{Result, bail, format_err};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{Form, Json, extract};
use axum_oidc::OidcRpInitiatedLogout;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use tower_sessions::Session;
use tracing::{error, info, warn};

use crate::{ExtractState, OidcClaims, State, db};

const USER_KEY: &str = "user";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Roles {
	Admin,
	Erwischt,
	Images(String),
}

#[derive(Debug, Deserialize, Serialize)]
struct Identity {
	id: i32,
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

fn get_ip(headers: &HeaderMap, addr: SocketAddr) -> Result<IpAddr> {
	real_ip::real_ip(headers, addr.ip(), &[
		IpAddr::from([127, 0, 0, 1]).into(),
		IpAddr::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]).into(),
	])
	.ok_or_else(|| format_err!("no ip detected"))
}

async fn rate_limit(state: &State, headers: &HeaderMap, addr: SocketAddr) -> Result<()> {
	let ip = get_ip(headers, addr)?.to_string();
	match state.db.check_rate(&ip).await {
		Ok(true) => Ok(()),
		Ok(false) => bail!("Rate limit exceeded"),
		Err(msg) => bail!(msg),
	}
}

async fn set_logged_in(id: i32, session: &Session) -> Result<()> {
	// Logged in: Store "user id"
	session.insert(USER_KEY, Identity { id }).await?;
	Ok(())
}

async fn login_internal(
	state: &State, headers: &HeaderMap, addr: SocketAddr, body: HashMap<String, String>,
	session: &Session,
) -> (StatusCode, LoginResult) {
	// Check rate limit
	if let Err(error) = rate_limit(state, headers, addr).await {
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
			if let Err(error) = set_logged_in(id, session).await {
				warn!(%error, "Failed to set login identity");
				return (StatusCode::INTERNAL_SERVER_ERROR, LoginResult {
					error: Some(format!(
						"Es ist ein Fehler beim Login aufgetreten.\n{}",
						state.config.error_message
					)),
				});
			}
			let ip = match get_ip(headers, addr) {
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

pub async fn token_login(
	state: &State, headers: &HeaderMap, addr: SocketAddr, token: &str, session: &Session,
) -> Result<()> {
	// Check rate limit
	if let Err(error) = rate_limit(state, headers, addr).await {
		bail!("Rate limit exceeded ({error})");
	}

	// Search token in database
	let user = db::schema::users::dsl::users
		.filter(db::schema::users::dsl::token.eq(Some(token)))
		.select(db::schema::users::dsl::id)
		.first::<i32>(&mut state.db.get().await?)
		.await?;

	if let Err(error) = set_logged_in(user, session).await {
		bail!("Failed to set login identity ({error})");
	}
	let ip = match get_ip(headers, addr) {
		Ok(r) => r.to_string(),
		Err(error) => bail!("Failed to get ip ({error})"),
	};
	if let Err(error) = state.db.decrease_rate_counter(&ip).await {
		error!(%error, "Failed to decrease rate limiting counter");
	}
	Ok(())
}

pub async fn login(
	extract::State(state): ExtractState, headers: HeaderMap, session: Session,
	ConnectInfo(addr): ConnectInfo<SocketAddr>, Form(body): Form<HashMap<String, String>>,
) -> impl IntoResponse {
	let (status, result) = login_internal(&state, &headers, addr, body, &session).await;
	(status, Json(result))
}

pub async fn login_nojs(
	extract::State(state): ExtractState, headers: HeaderMap, session: Session,
	ConnectInfo(addr): ConnectInfo<SocketAddr>, Form(body): Form<HashMap<String, String>>,
) -> Response {
	let (status, result) = login_internal(&state, &headers, addr, body, &session).await;
	if let Some(error) = result.error {
		(status, error).into_response()
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		Response::builder()
			.status(StatusCode::FOUND)
			.header("location", "/")
			.body(Body::empty())
			.unwrap()
	}
}

pub async fn logout(session: Session, logout: Option<OidcRpInitiatedLogout>) -> Response {
	// Non-oidc logout
	if let Err(error) = session.flush().await {
		error!(%error, "Failed to log out");
	}

	// Oidc logout
	if let Some(logout) = logout {
		return logout.with_post_logout_redirect(Uri::from_static("/")).into_response();
	}

	Response::builder()
		.status(StatusCode::FOUND)
		.header("location", "/")
		.body(Body::empty())
		.unwrap()
}

// Utility methods for other modules

/// Get the id of the logged in user
pub async fn logged_in_user(session: &Session) -> Option<i32> {
	match session.get::<Identity>(USER_KEY).await {
		Err(error) => {
			error!(%error, "Failed to get session");
			None
		}
		Ok(r) => r.map(|i| i.id),
	}
}

pub async fn get_roles(
	state: &State, session: &Session, oidc: &Option<OidcClaims>,
) -> Result<Option<Vec<Roles>>> {
	if let Some(user) = logged_in_user(session).await {
		let roles = state
			.db
			.get_roles(user)
			.await
			.map_err(|e| format_err!("Failed to get user roles: {}", e))?;
		Ok(Some(roles))
	} else if let Some(_oidc) = oidc {
		Ok(Some(vec![Roles::Admin]))
	} else {
		Ok(None)
	}
}
