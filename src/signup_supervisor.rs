use std::collections::HashMap;
use std::io::Write;

use actix_web::{http::StatusCode, *};
use diesel::prelude::*;
use scrypt::password_hash;
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime};
use tracing::{error, warn};

use crate::db::models::{self, Gender};
use crate::db::models::{date, opt_date};
use crate::{HttpResponse, State, db};

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<db::FormError>,
}

#[derive(Clone, Debug, Serialize)]
struct ResignupResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GetDataRequest {
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
	let db_addr = state.db_addr.clone();
	let error_message = state.config.error_message.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();

	// Get the body of the request
	let supervisor = match db::models::Supervisor::from_hashmap(body.clone()) {
		Ok(supervisor) => supervisor,
		Err(error) => {
			warn!(?error, "Error handling form content");
			return (StatusCode::BAD_REQUEST, SignupResult { error: Some(error) });
		}
	};
	if let Some(log_file) = log_file {
		let res: Result<_, Error> = (|| {
			let _lock = log_mutex.lock().unwrap();
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

	match db_addr
		.send(db::SignupSupervisorMessage { supervisor: supervisor.clone(), is_pre_signup: false })
		.await
	{
		Ok(Err(error)) => {
			warn!(%error, "Error inserting into database");
		}
		Err(error) => {
			warn!(%error, "Error inserting into database");
		}
		Ok(Ok(())) => {
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

/// Send mail with token link
#[post("/resignup-supervisor")]
pub async fn resignup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let err = |msg| {
		HttpResponse::build(StatusCode::BAD_REQUEST).json(ResignupResult { error: Some(msg) })
	};

	let Some(mail) = body.get("mail") else {
		return err("Etwas ist schiefgegangen (E-Mailadresse nicht bekannt)".into());
	};

	// Generate token in URL-safe base64
	let token = password_hash::SaltString::generate(&mut password_hash::rand_core::OsRng)
		.as_str()
		.replace('+', "-")
		.replace('/', "_")
		.replace('=', "");
	let mail2 = mail.clone();
	let token2 = token.clone();
	let supervisor = match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::betreuer::columns::*;

			let supervisor = match betreuer::table
				.filter(mail.eq(mail2))
				.first::<models::FullSupervisor>(&mut db.connection)
			{
				Err(diesel::result::Error::NotFound) => return Ok(None),
				Err(e) => return Err(e.into()),
				Ok(supervisor) => supervisor,
			};

			// Save token in db
			diesel::update(betreuer::table)
				.filter(id.eq(supervisor.id))
				.set((signup_token.eq(token2.as_str()), signup_token_time.eq(diesel::dsl::now)))
				.execute(&mut db.connection)?;

			Ok(Some(supervisor))
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			error!(%error, "Failed to get supervisor");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(Ok(None)) => {
			// Not found
			warn!(mail, "Failed to find supervisor by mail");
			return HttpResponse::build(StatusCode::OK).json(ResignupResult { error: None });
		}
		Ok(Ok(Some(supervisor))) => supervisor,
	};

	// Send mail with link
	match state.mail.send_supervisor_resignup(&supervisor, &token).await {
		Err(error) => {
			error!(mail, %error, "Error sending e-mail");
			err("Es ist leider ein Fehler beim Versenden der E-Mail aufgetreten".into())
		}
		Ok(()) => {
			// Successful
			HttpResponse::build(StatusCode::OK).json(ResignupResult { error: None })
		}
	}
}

/// Get data for resignup
#[post("/get-supervisor-data")]
pub async fn get_data(state: web::Data<State>, request: web::Json<GetDataRequest>) -> HttpResponse {
	let err = |msg| {
		HttpResponse::build(StatusCode::BAD_REQUEST).json(ResignupResult { error: Some(msg) })
	};

	// Check token
	let token = request.0.token;
	let supervisor = match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::betreuer::columns::*;

			let now = OffsetDateTime::now_utc() - Duration::days(1);
			let now_primitive = PrimitiveDateTime::new(now.date(), now.time());
			let supervisor = match betreuer::table
				.filter(signup_token.eq(token).and(signup_token_time.gt(now_primitive)))
				.first::<models::FullSupervisor>(&mut db.connection)
			{
				Err(diesel::result::Error::NotFound) => return Ok(None),
				Err(e) => return Err(e.into()),
				Ok(supervisor) => supervisor,
			};

			// Remove token from db
			diesel::update(betreuer::table)
				.filter(id.eq(supervisor.id))
				.set((
					signup_token.eq(None::<String>),
					signup_token_time.eq(None::<PrimitiveDateTime>),
				))
				.execute(&mut db.connection)?;

			Ok(Some(supervisor))
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			error!(%error, "Failed to get supervisor by token");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(Ok(None)) => {
			// Not found
			warn!("Failed to find supervisor by token");
			return err("Daten konnten nicht vorausgefüllt werden".into());
		}
		Ok(Ok(Some(supervisor))) => supervisor,
	};

	HttpResponse::build(StatusCode::OK).json(GetDataResult {
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
}

async fn presignup_internal(
	state: &State, mut body: HashMap<String, String>,
) -> (StatusCode, SignupResult) {
	let db_addr = state.db_addr.clone();
	let error_message = state.config.error_message.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();

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
	if let Some(log_file) = log_file {
		let res: Result<_, Error> = (|| {
			let _lock = log_mutex.lock().unwrap();
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
	match db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::betreuer::columns::*;

			// Check if the e-mail already exists
			match betreuer::table
				.filter(mail.eq(&supervisor_mail))
				.select(id)
				.first::<i32>(&mut db.connection)
			{
				Err(diesel::result::Error::NotFound) => Ok(false),
				Err(e) => Err(e.into()),
				Ok(_) => Ok(true),
			}
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			error!(%error, "Failed to get supervisor by mail");
			return err("Es ist leider ein Fehler suchen der E-Mailadresse aufgetreten".into());
		}
		Ok(Ok(true)) => {
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
		Ok(Ok(false)) => {}
	};

	match db_addr
		.send(db::SignupSupervisorMessage { supervisor: supervisor.clone(), is_pre_signup: true })
		.await
	{
		Ok(Err(error)) => {
			warn!(%error, "Error inserting into database");
			return internal_err(format!(
				"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
				error_message
			));
		}
		Err(error) => {
			warn!(%error, "Error inserting into database");
			return internal_err(format!(
				"Es ist ein Datenbank-Fehler aufgetreten.\n{}",
				error_message
			));
		}
		Ok(Ok(())) => {}
	}

	// Send mail to specified users
	match state.mail.send_supervisor_presignup(&supervisor, &grund, &kommentar).await {
		Err(error) => {
			warn!(%error, "Error sending presignup e-mail");
			internal_err(format!("Es ist ein Fehler aufgetreten.\n{}", error_message))
		}
		Ok(()) => {
			// Successful
			(StatusCode::OK, SignupResult { error: None })
		}
	}
}

#[post("/presignup-supervisor")]
pub async fn presignup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = presignup_internal(&state, body.into_inner()).await;
	HttpResponse::build(status).json(result)
}

#[post("/presignup-supervisor-nojs")]
pub async fn presignup_nojs(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let (status, result) = presignup_internal(&state, body.into_inner()).await;
	if let Some(error) = result.error {
		HttpResponse::build(status).body(error.message)
	} else {
		debug_assert_eq!(status, StatusCode::OK);
		HttpResponse::Found()
			.append_header(("location", "/betreuer-anmeldung-erfolgreich"))
			.finish()
	}
}
