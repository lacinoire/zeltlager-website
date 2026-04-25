use std::collections::HashMap;
use std::io::Write;

use anyhow::Result;
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json, extract};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime};
use tracing::{error, warn};

use crate::db::models::{self, Gender, date, opt_date};
use crate::{ExtractState, State, db};

type DbResult<T> = anyhow::Result<T>;

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<db::FormError>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ResignupResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GetDataRequest {
	token: String,
}

#[derive(Clone, Debug, Serialize)]
struct GetDataResult {
	vorname: String,
	nachname: String,
	#[serde(with = "date")]
	geburtsdatum: Date,
	geschlecht: Gender,
	juleica_nummer: Option<String>,
	mail: String,
	handynummer: String,
	strasse: Option<String>,
	hausnummer: Option<String>,
	ort: Option<String>,
	plz: Option<String>,
	kommentar: Option<String>,
	#[serde(with = "opt_date")]
	fuehrungszeugnis_ausstellung: Option<Date>,
	allergien: Option<String>,
	unvertraeglichkeiten: Option<String>,
	medikamente: Option<String>,
	krankenversicherung: Option<String>,
	vegetarier: Option<bool>,
	tetanus_impfung: Option<bool>,
	land: Option<String>,
	krankheiten: Option<String>,
	#[serde(with = "opt_date")]
	juleica_gueltig_bis: Option<Date>,
}

async fn signup_internal(
	state: &State, body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	// Get the body of the request
	let supervisor = match db::models::Supervisor::from_hashmap(body.clone()) {
		Ok(supervisor) => supervisor,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return (StatusCode::BAD_REQUEST, SignupResult { error: Some(error) });
		}
	};
	if let Some(log_file) = &state.config.log_file {
		let res: Result<_> = (|| {
			let _lock = state.log_mutex.lock().unwrap();
			let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
			writeln!(
				file,
				"{}: Betreuer: {}",
				time::OffsetDateTime::now_utc()
					.format(&time::format_description::well_known::Rfc3339)
					.unwrap(),
				serde_json::to_string(&supervisor)?
			)?;

			Ok(())
		})();

		if let Err(error) = res {
			warn!(%error, "Failed to log new supervisor");
		}
	}

	match state.db.signup_supervisor(&supervisor, false).await {
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
		Ok(()) => (StatusCode::OK, SignupResult { error: None }),
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
		Response::builder().status(status).body(Body::new(error.message)).unwrap()
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		Response::builder()
			.status(StatusCode::FOUND)
			.header("location", "/intern/betreuer-anmeldung-erfolgreich")
			.body(Body::empty())
			.unwrap()
	}
}

/// Send mail with token link
pub async fn resignup(
	extract::State(state): ExtractState, Form(body): Form<HashMap<String, String>>,
) -> (StatusCode, Json<ResignupResult>) {
	let err = |msg| (StatusCode::BAD_REQUEST, Json(ResignupResult { error: Some(msg) }));

	let Some(mail) = body.get("mail") else {
		return err("Etwas ist schiefgegangen (E-Mailadresse nicht bekannt)".into());
	};

	// Generate token in URL-safe base64
	let token = {
		let mut rng = rand::rng();
		(0..24).map(|_| rng.sample(rand::distr::Alphanumeric) as char).collect::<String>()
	};
	let mail2 = mail.clone();
	let token2 = token.clone();
	let supervisor = match async {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		let mut connection = state.db.get().await?;

		let supervisor = match betreuer::table
			.filter(mail.eq(mail2))
			.first::<models::FullSupervisor>(&mut connection)
			.await
		{
			Err(diesel::result::Error::NotFound) => return DbResult::Ok(None),
			Err(e) => return Err(e.into()),
			Ok(supervisor) => supervisor,
		};

		// Save token in db
		diesel::update(betreuer::table)
			.filter(id.eq(supervisor.id))
			.set((signup_token.eq(token2.as_str()), signup_token_time.eq(diesel::dsl::now)))
			.execute(&mut connection)
			.await?;

		Ok(Some(supervisor))
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to get supervisor");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(None) => {
			// Not found
			warn!(mail, "Failed to find supervisor by mail");
			return (StatusCode::OK, Json(ResignupResult { error: None }));
		}
		Ok(Some(supervisor)) => supervisor,
	};

	// Send mail with link
	match state.mail.send_supervisor_resignup(&supervisor, &token).await {
		Err(error) => {
			error!(mail, %error, "Error sending e-mail");
			err("Es ist leider ein Fehler beim Versenden der E-Mail aufgetreten".into())
		}
		Ok(()) => {
			// Successful
			(StatusCode::OK, Json(ResignupResult { error: None }))
		}
	}
}

/// Get data for resignup
pub async fn get_data(
	extract::State(state): ExtractState, Json(request): Json<GetDataRequest>,
) -> Response {
	let err =
		|msg| (StatusCode::BAD_REQUEST, Json(ResignupResult { error: Some(msg) })).into_response();

	// Check token
	let token = request.token;
	let supervisor = match async {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		let mut connection = state.db.get().await?;

		let since = OffsetDateTime::now_utc() - Duration::days(1);
		let since_primitive = PrimitiveDateTime::new(since.date(), since.time());
		let supervisor = match betreuer::table
			.filter(signup_token.eq(token).and(signup_token_time.gt(since_primitive)))
			.first::<models::FullSupervisor>(&mut connection)
			.await
		{
			Err(diesel::result::Error::NotFound) => return DbResult::Ok(None),
			Err(e) => return Err(e.into()),
			Ok(supervisor) => supervisor,
		};

		// Remove token from db
		diesel::update(betreuer::table)
			.filter(id.eq(supervisor.id))
			.set((signup_token.eq(None::<String>), signup_token_time.eq(None::<PrimitiveDateTime>)))
			.execute(&mut connection)
			.await?;

		Ok(Some(supervisor))
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to get supervisor by token");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(None) => {
			// Not found
			warn!("Failed to find supervisor by token");
			return err("Daten konnten nicht vorausgefüllt werden".into());
		}
		Ok(Some(supervisor)) => supervisor,
	};

	Json(GetDataResult {
		vorname: supervisor.vorname,
		nachname: supervisor.nachname,
		geburtsdatum: supervisor.geburtsdatum,
		geschlecht: supervisor.geschlecht,
		juleica_nummer: supervisor.juleica_nummer,
		mail: supervisor.mail,
		handynummer: supervisor.handynummer,
		strasse: supervisor.strasse,
		hausnummer: supervisor.hausnummer,
		ort: supervisor.ort,
		plz: supervisor.plz,
		kommentar: supervisor.kommentar,
		fuehrungszeugnis_ausstellung: supervisor.fuehrungszeugnis_ausstellung,
		allergien: supervisor.allergien,
		unvertraeglichkeiten: supervisor.unvertraeglichkeiten,
		medikamente: supervisor.medikamente,
		krankenversicherung: supervisor.krankenversicherung,
		vegetarier: supervisor.vegetarier,
		tetanus_impfung: supervisor.tetanus_impfung,
		land: supervisor.land,
		krankheiten: supervisor.krankheiten,
		juleica_gueltig_bis: supervisor.juleica_gueltig_bis,
	})
	.into_response()
}

async fn presignup_internal(
	state: &State, mut body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	let err = |msg| (StatusCode::BAD_REQUEST, SignupResult { error: Some(msg) });
	let internal_err =
		|msg: String| (StatusCode::INTERNAL_SERVER_ERROR, SignupResult { error: Some(msg.into()) });

	// Get the body of the request
	let grund = match db::get_freetext_str!(body, "grund") {
		Ok(res) => res,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return err(error);
		}
	};
	let kommentar = match db::get_freetext_str!(body, "kommentar") {
		Ok(res) => res,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return err(error);
		}
	};
	let supervisor = match db::models::Supervisor::from_pre_hashmap(body) {
		Ok(supervisor) => supervisor,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return err(error);
		}
	};
	if let Some(log_file) = &state.config.log_file {
		let res: Result<_> = (|| {
			let _lock = state.log_mutex.lock().unwrap();
			let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
			writeln!(
				file,
				"{}: Pre-Betreuer: {}",
				time::OffsetDateTime::now_utc()
					.format(&time::format_description::well_known::Rfc3339)
					.unwrap(),
				serde_json::to_string(&supervisor)?
			)?;

			Ok(())
		})();

		if let Err(error) = res {
			warn!(%error, "Failed to log new supervisor");
		}
	}

	// If a mail address is already signed up, do not store in database but send a mail
	let supervisor_mail = supervisor.mail.clone();
	match async {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		// Check if the e-mail already exists
		match betreuer::table
			.filter(mail.eq(&supervisor_mail))
			.select(id)
			.first::<i32>(&mut state.db.get().await?)
			.await
		{
			Err(diesel::result::Error::NotFound) => DbResult::Ok(false),
			Err(e) => Err(e.into()),
			Ok(_) => Ok(true),
		}
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to get supervisor by mail");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(true) => {
			// Already exists, send mail
			warn!(mail = supervisor.mail, "Supervisor tried to pre-signup but already exists");
			match state.mail.send_supervisor_presignup_failed(&supervisor).await {
				Err(error) => {
					error!(%error, "Error sending presignup failed e-mail");
				}
				Ok(()) => {}
			}
			return (StatusCode::OK, SignupResult { error: None });
		}
		Ok(false) => {}
	};

	match state.db.signup_supervisor(&supervisor, true).await {
		Err(error) => {
			warn!(%error, "Error inserting into database");
			return internal_err(format!(
				"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
				state.config.error_message
			));
		}
		Ok(()) => {}
	}

	// Send mail to specified users
	match state.mail.send_supervisor_presignup(&supervisor, &grund, &kommentar).await {
		Err(error) => {
			warn!(%error, "Error sending presignup e-mail");
			internal_err(format!("Es ist ein Fehler aufgetreten.\n{}", state.config.error_message))
		}
		Ok(()) => {
			// Successful
			(StatusCode::OK, SignupResult { error: None })
		}
	}
}

pub async fn presignup(
	extract::State(state): ExtractState, Form(body): Form<HashMap<String, String>>,
) -> impl IntoResponse {
	let (status, result) = presignup_internal(&state, body).await;
	(status, Json(result))
}

pub async fn presignup_nojs(
	extract::State(state): ExtractState, Form(body): Form<HashMap<String, String>>,
) -> Response {
	let (status, result) = presignup_internal(&state, body).await;
	if let Some(error) = result.error {
		Response::builder().status(status).body(Body::new(error.message)).unwrap()
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		Response::builder()
			.status(StatusCode::FOUND)
			.header("location", "/betreuer-anmeldung-erfolgreich")
			.body(Body::empty())
			.unwrap()
	}
}
