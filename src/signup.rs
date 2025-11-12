use std::collections::HashMap;
use std::io::Write;

use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::Result;
use serde::Serialize;
use tracing::{error, warn};

use crate::{HttpResponse, State, db};

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<db::FormError>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignupState {
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

#[get("/signup-state")]
pub async fn signup_state(state: web::Data<State>) -> HttpResponse {
	match state.db_addr.send(db::CountMemberMessage).await.map_err(|e| e.into()) {
		Err(error) | Ok(Err(error)) => {
			error!(%error, "Failed to get current member count");
			crate::error_response(&state)
		}
		Ok(Ok(count)) => {
			HttpResponse::Ok().json(SignupState { is_full: count >= state.config.max_members })
		}
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

	let count = match state.db_addr.send(db::CountMemberMessage).await {
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
		Ok(Err(error)) => {
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
		Ok(Ok(count)) => count,
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
		let res: Result<_, Error> = (|| {
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

	match state.db_addr.send(db::SignupMessage { member: member.clone() }).await {
		Err(error) => {
			warn!(%error, "Error inserting into database");
		}
		Ok(Err(error)) => {
			warn!(%error, "Error inserting into database");
		}
		Ok(Ok(())) => {
			return signup_mail(state, member).await;
		}
	}

	(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
		error: Some(
			format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", state.config.error_message)
				.into(),
		),
	})
}

#[post("/signup")]
pub async fn signup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&state, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/signup-nojs")]
pub async fn signup_nojs(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&state, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error.message)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found().append_header(("location", "/anmeldung-erfolgreich")).finish()
	}
}
