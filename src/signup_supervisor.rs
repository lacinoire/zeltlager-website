use std::collections::HashMap;
use std::io::Write;

use actix_web::{http::StatusCode, *};
use log::warn;
use serde::Serialize;

use crate::{db, HttpResponse, State};

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<db::FormError>,
}

async fn signup_internal(
	state: &State, body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	let db_addr = state.db_addr.clone();
	let error_message = state.config.error_message.clone();
	let birthday_date = state.config.birthday_date.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();

	// Get the body of the request
	let supervisor = match db::models::Supervisor::from_hashmap(body.clone(), &birthday_date) {
		Ok(supervisor) => supervisor,
		Err(error) => {
			warn!("Error handling form content: {:?}", error);
			return (StatusCode::BAD_REQUEST, SignupResult { error: Some(error) });
		}
	};

	match db_addr.send(db::SignupSupervisorMessage { supervisor: supervisor.clone() }).await {
		Ok(Err(error)) => {
			warn!("Error inserting into database: {}", error);
		}
		Err(error) => {
			warn!("Error inserting into database: {}", error);
		}
		Ok(Ok(())) => {
			if let Some(log_file) = log_file {
				let res: Result<_, Error> = (|| {
					let _lock = log_mutex.lock().unwrap();
					let mut file =
						std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
					writeln!(file, "Betreuer: {}", serde_json::to_string(&supervisor)?)?;

					Ok(())
				})();

				if let Err(error) = res {
					warn!("Failed to log new supervisor: {:?}", error);
					// Ignore logging failure as it was saved in the db
				}
			}

			return (StatusCode::OK, SignupResult { error: None });
		}
	}

	(StatusCode::INTERNAL_SERVER_ERROR, SignupResult {
		error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message).into()),
	})
}

#[post("/signup-supervisor")]
pub async fn signup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&state, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/signup-supervisor-nojs")]
pub async fn signup_nojs(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = signup_internal(&state, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error.message)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found()
			.append_header(("location", "/intern/betreuer-anmeldung-erfolgreich"))
			.finish()
	}
}
