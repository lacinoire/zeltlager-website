use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json, extract};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use tracing::{error, warn};

use crate::{ExtractState, State, WebResult, db};

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<db::FormError>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SignupState {
	is_full: bool,
}

/// Write an email and show a success site.
async fn signup_mail(state: &State, member: db::models::Teilnehmer) -> (StatusCode, SignupResult) {
	match state.mail.send_member_signup(&member).await {
		Err(error) => {
			error!(mail = member.eltern_mail, %error, "Error sending e-mail");
		}
		Ok(()) => {
			// Signup successful
			return (StatusCode::OK, SignupResult { error: None });
		}
	}

	(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
		error: Some(
			format!(
				"Ihre Daten wurden erfolgreich gespeichert.\nEs ist leider ein Fehler beim E-Mail \
				 senden aufgetreten.\n{}",
				state.config.error_message
			)
			.into(),
		),
	})
}

pub async fn signup_state(extract::State(state): ExtractState) -> WebResult<Json<SignupState>> {
	match state.db.count_members().await {
		Err(error) => {
			error!(%error, "Failed to get current member count");
			crate::error_response(&state)
		}
		Ok(count) => Ok(Json(SignupState { is_full: count >= state.config.max_members })),
	}
}

async fn signup_internal(
	state: &State, body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	// Get the body of the request
	let mut member = match db::models::Teilnehmer::from_hashmap(body) {
		Ok(member) => member,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return (StatusCode::BAD_REQUEST, SignupResult { error: Some(error) });
		}
	};

	// Remove spaces
	member.trim();

	let count = match state.db.count_members().await {
		Err(error) => {
			warn!(%error, "Error inserting into database");
			return (StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
				error: Some(
					format!(
						"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
						state.config.error_message
					)
					.into(),
				),
			});
		}
		Ok(count) => count,
	};

	if state.config.test_mail.as_ref().map(|m| m == &member.eltern_mail).unwrap_or(false) {
		// Don't insert test signup into database
		return signup_mail(state, member).await;
	}
	// Check if too many members are already registered, then call `signup_mail`.
	if count >= state.config.max_members {
		// Show error
		warn!(mail = member.eltern_mail, "Already too many members registered");
		return (StatusCode::BAD_REQUEST, SignupResult {
			error: Some(
				"Während Ihrer Anmeldung ist das Zeltlager leider schon voll geworden.".into(),
			),
		});
	}

	if let Some(log_file) = &state.config.log_file {
		let res: Result<_> = (|| {
			let _lock = state.log_mutex.lock().unwrap();
			let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
			writeln!(
				file,
				"{}: Teilnehmer: {}",
				time::OffsetDateTime::now_utc()
					.format(&time::format_description::well_known::Rfc3339)
					.unwrap(),
				serde_json::to_string(&member)?
			)?;

			Ok(())
		})();

		if let Err(error) = res {
			warn!(%error, "Failed to log new member");
		}
	}

	let mut connection = match state.db.get().await {
		Ok(c) => c,
		Err(error) => {
			warn!(%error, "Error getting database connection");
			return (StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
				error: Some(
					format!(
						"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
						state.config.error_message
					)
					.into(),
				),
			});
		}
	};

	match diesel::insert_into(db::schema::teilnehmer::table)
		.values(&member)
		.execute(&mut connection)
		.await
	{
		Err(error) => {
			warn!(%error, "Error inserting into database");
			(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
				error: Some(
					format!(
						"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
						state.config.error_message
					)
					.into(),
				),
			})
		}
		Ok(_) => signup_mail(state, member).await,
	}
}

pub async fn signup(
	extract::State(state): ExtractState, Form(body): Form<HashMap<String, String>>,
) -> impl IntoResponse {
	let (status, result) = signup_internal(&state, body).await;
	(status, Json(result))
}

pub async fn signup_nojs(
	extract::State(state): ExtractState, Form(body): Form<HashMap<String, String>>,
) -> Response {
	let (status, result) = signup_internal(&state, body).await;
	if let Some(error) = result.error {
		(status, error.message).into_response()
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		Response::builder()
			.status(StatusCode::FOUND)
			.header("location", "/anmeldung-erfolgreich")
			.body(Body::empty())
			.unwrap()
	}
}
