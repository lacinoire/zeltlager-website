use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::Result;
use log::{error, warn};
use serde::Serialize;

use crate::{db, mail, HttpResponse, State};

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignupState {
	is_full: bool,
}

/// Check if too many members are already registered, then call `signup_mail`.
async fn signup_check_count(
	count: i64, max_members: i64, db_addr: &actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>, member: db::models::Teilnehmer,
	error_message: String, log_file: Option<PathBuf>, log_mutex: Arc<Mutex<()>>, state: &State,
) -> (StatusCode, SignupResult) {
	if state.config.test_mail.as_ref().map(|m| m == &member.eltern_mail).unwrap_or(false) {
		// Don't insert test signup into database
		signup_mail(&mail_addr, member, error_message).await
	} else if count >= max_members {
		// Show error
		warn!("Already too many members registered (from {})", member.eltern_mail);
		(StatusCode::BAD_REQUEST, SignupResult {
			error: Some(
				"Während Ihrer Anmeldung ist das Zeltlager leider schon voll geworden.".to_string(),
			),
		})
	} else {
		match db_addr.send(db::SignupMessage { member: member.clone() }).await {
			Err(error) => {
				warn!("Error inserting into database: {:?}", error);
			}
			Ok(Err(error)) => {
				warn!("Error inserting into database: {:?}", error);
			}
			Ok(Ok(())) => {
				if let Some(log_file) = log_file {
					let res: Result<_, Error> = (|| {
						let _lock = log_mutex.lock().unwrap();
						let mut file =
							std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
						writeln!(file, "Teilnehmer: {}", serde_json::to_string(&member)?)?;

						Ok(())
					})();

					if let Err(error) = res {
						warn!("Failed to log new member: {:?}", error);
						// Ignore logging failure as it was saved in the db
					}
				}

				return signup_mail(&mail_addr, member, error_message).await;
			}
		}

		(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
			error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
		})
	}
}

/// Write an email and show a success site.
async fn signup_mail(
	mail_addr: &actix::Addr<mail::MailExecutor>, member: db::models::Teilnehmer,
	error_message: String,
) -> (StatusCode, SignupResult) {
	// Write an e-mail
	let mail = member.eltern_mail.clone();
	match mail_addr.send(mail::SignupMessage { member }).await {
		Err(error) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
		}
		Ok(Err(error)) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
		}
		Ok(Ok(())) => {
			// Signup successful
			// TODO redirect in nojs
			/*return HttpResponse::Found()
			.append_header((http::header::LOCATION, "anmeldung-erfolgreich"))
			.finish();*/
			return (StatusCode::OK, SignupResult { error: None });
		}
	}

	(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
		error: Some(format!(
			"Ihre Daten wurden erfolgreich gespeichert.\nEs ist leider ein Fehler beim E-Mail \
			 senden aufgetreten.\n{}",
			error_message
		)),
	})
}

#[get("/signup-state")]
pub async fn signup_state(state: web::Data<State>) -> HttpResponse {
	match state.db_addr.send(db::CountMemberMessage).await.map_err(|e| e.into()) {
		Err(error) | Ok(Err(error)) => {
			error!("Failed to get current member count: {:?}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(count)) => {
			return HttpResponse::Ok()
				.json(SignupState { is_full: count >= state.config.max_members });
		}
	}
}

async fn signup_internal(
	state: &State, body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	let db_addr = state.db_addr.clone();
	let mail_addr = state.mail_addr.clone();
	let error_message = state.config.error_message.clone();
	let max_members = state.config.max_members;
	let birthday_date = state.config.birthday_date.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();
	let db_addr2 = db_addr.clone();

	// Get the body of the request
	let mut member = match db::models::Teilnehmer::from_hashmap(body, &birthday_date) {
		Ok(member) => member,
		Err(error) => {
			warn!("Error handling form content: {}", error);
			return (StatusCode::BAD_REQUEST, SignupResult { error: Some(error.to_string()) });
		}
	};

	// Remove spaces
	member.trim();

	match db_addr.send(db::CountMemberMessage).await {
		Err(error) => {
			warn!("Error inserting into database: {}", error);
			return (StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
				error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
			});
		}
		Ok(Err(error)) => {
			warn!("Error inserting into database: {}", error);
			return (StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
				error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
			});
		}
		Ok(Ok(count)) => {
			signup_check_count(
				count,
				max_members,
				&db_addr2,
				mail_addr,
				member,
				error_message,
				log_file,
				log_mutex,
				state,
			)
			.await
		}
	}
}

#[post("/signup")]
pub async fn signup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&**state, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/signup-nojs")]
pub async fn signup_nojs(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&**state, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found().append_header(("location", "/anmeldung-erfolgreich")).finish()
	}
}
