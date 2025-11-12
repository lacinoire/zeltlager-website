use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::bail;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::{
	State,
	db::{
		self,
		models::{FullSupervisor, FullTeilnehmer},
	},
	mail,
};
use time::OffsetDateTime;

type DbResult<T> = anyhow::Result<T>;

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveMemberData {
	member: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveSupervisorData {
	supervisor: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditMemberResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LagerInfo {
	teilnehmer_count: i64,
	old_betreuer_count: i64,
	erwischt_game_count: i64,
}

#[post("/teilnehmer/remove")]
pub(crate) async fn remove_member(
	state: web::Data<State>, data: web::Json<RemoveMemberData>,
) -> HttpResponse {
	match async {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let r = diesel::delete(teilnehmer::table.filter(id.eq(data.member)))
			.execute(&mut state.db.get().await?)
			.await?;
		if r == 0 {
			bail!("Member not found");
		}
		Ok(())
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to remove member");
			HttpResponse::InternalServerError().body("Failed to remove member")
		}
		Ok(()) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

/// Write mail to confirm payment
async fn payed_mail(
	mail: &mail::Mail, member: db::models::FullTeilnehmer,
) -> (StatusCode, EditMemberResult) {
	let error = match mail.send_member_payed(&member).await {
		Err(error) => {
			error!(mail = member.eltern_mail, %error, "Error sending e-mail");
			format!(
				"Die Änderung wurde erfolgreich gespeichert.\nEs ist leider ein Fehler beim \
				 E-Mail senden aufgetreten.\n{}",
				error
			)
		}
		Ok(()) => {
			// Signup successful
			return (StatusCode::OK, EditMemberResult { error: None });
		}
	};

	(StatusCode::INTERNAL_SERVER_ERROR, EditMemberResult { error: Some(error) })
}

#[post("/teilnehmer/edit")]
pub(crate) async fn edit_member(
	state: web::Data<State>, data: web::Json<FullTeilnehmer>,
) -> HttpResponse {
	match async {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let mut connection = state.db.get().await?;

		let member = teilnehmer::table
			.filter(id.eq(data.id))
			.get_result::<db::models::FullTeilnehmer>(&mut connection)
			.await?;
		let new_payed = data.bezahlt && !member.bezahlt;

		diesel::update(&*data).set(&*data).execute(&mut connection).await?;
		DbResult::Ok((new_payed, member))
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to edit member");
			HttpResponse::InternalServerError().json(EditMemberResult {
				error: Some(format!("Teilnehmer konnte nicht gespeichert werden: {error}")),
			})
		}
		Ok((new_payed, member)) => {
			if new_payed {
				let (status, result) = payed_mail(&state.mail, member).await;
				HttpResponse::build(status).json(result)
			} else {
				HttpResponse::Ok().json(EditMemberResult { error: None })
			}
		}
	}
}

// TODO Use delete("/betreuer/{id}") here and for teilnehmer
#[post("/betreuer/remove")]
pub(crate) async fn remove_supervisor(
	state: web::Data<State>, data: web::Json<RemoveSupervisorData>,
) -> HttpResponse {
	match async {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		let r = diesel::delete(betreuer::table.filter(id.eq(data.supervisor)))
			.execute(&mut state.db.get().await?)
			.await?;
		if r == 0 {
			bail!("Supervisor not found");
		}
		Ok(())
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to remove supervisor");
			HttpResponse::InternalServerError().body("Failed to remove supervisor")
		}
		Ok(()) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

#[post("/betreuer/edit")]
pub(crate) async fn edit_supervisor(
	state: web::Data<State>, data: web::Json<FullSupervisor>,
) -> HttpResponse {
	match async {
		diesel::update(&*data).set(&*data).execute(&mut state.db.get().await?).await?;
		DbResult::Ok(())
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to remove supervisor");
			HttpResponse::InternalServerError().body("Failed to edit supervisor")
		}
		Ok(()) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

/// Return all current members as json.
#[get("/teilnehmer")]
pub async fn download_members(state: web::Data<State>) -> HttpResponse {
	let mut connection = match state.db.get().await {
		Ok(c) => c,
		Err(error) => {
			warn!(%error, "Error getting database connection");
			return crate::error_response(&state);
		}
	};

	match db::schema::teilnehmer::table.load::<FullTeilnehmer>(&mut connection).await {
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(members) => HttpResponse::Ok().json(members),
	}
}

/// Return all supervisors as json.
#[get("/betreuer")]
pub async fn download_supervisors(state: web::Data<State>) -> HttpResponse {
	let mut connection = match state.db.get().await {
		Ok(c) => c,
		Err(error) => {
			warn!(%error, "Error getting database connection");
			return crate::error_response(&state);
		}
	};

	match db::schema::betreuer::table.load::<FullSupervisor>(&mut connection).await {
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(supervisors) => HttpResponse::Ok().json(supervisors),
	}
}

/// Return all unique mail addresses as json.
#[get("/mails")]
pub async fn download_mails(state: web::Data<State>) -> HttpResponse {
	match async {
		use crate::db::schema::teilnehmer;

		let mut mails = teilnehmer::table
			.select(teilnehmer::eltern_mail)
			.load::<String>(&mut state.db.get().await?)
			.await?;
		mails.sort();
		mails.dedup();

		DbResult::Ok(mails)
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error fetching from database");
			crate::error_response(&state)
		}
		Ok(mails) => HttpResponse::Ok().json(mails),
	}
}

/// The date at which betreuer need to be signed up to count towards the current or last year.
fn betreuer_signup_date_last_year() -> OffsetDateTime {
	let mut date = crate::LAGER_START.midnight().assume_utc();
	let now = OffsetDateTime::now_utc();
	if date > now {
		// Start from next time, subtract a year to allow signups from last time.
		date -= time::Duration::days(365);
	}

	// Add two weeks to get the end of the lager, subtract one year to get the date of last year.
	date - time::Duration::days(365) + time::Duration::weeks(2)
}

/// Get overview info of the current lager.
#[get("/lager")]
pub async fn lager_info(state: web::Data<State>) -> HttpResponse {
	match async {
		use crate::db::schema::{betreuer, erwischt_game, teilnehmer};
		use diesel::dsl;

		let mut connection = state.db.get().await?;

		let teilnehmer_count = teilnehmer::table.count().get_result(&mut connection).await?;
		let old_betreuer_count = betreuer::table
			.filter(
				betreuer::anmeldedatum
					.lt(betreuer_signup_date_last_year())
					.or(dsl::not(betreuer::selbsterklaerung)),
			)
			.count()
			.get_result(&mut connection)
			.await?;
		let erwischt_game_count = erwischt_game::table.count().get_result(&mut connection).await?;

		DbResult::Ok(LagerInfo { teilnehmer_count, old_betreuer_count, erwischt_game_count })
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error getting lager info");
			crate::error_response(&state)
		}
		Ok(info) => HttpResponse::Ok().json(info),
	}
}

/// Remove all data for the current lager.
#[delete("/lager")]
pub async fn remove_lager(state: web::Data<State>) -> HttpResponse {
	// Remove log file
	if let Some(log_file) = &state.config.log_file {
		if let Err(error) = std::fs::remove_file(log_file) {
			if error.kind() != std::io::ErrorKind::NotFound {
				error!(file = %log_file.display(), %error, "Failed to remove log file");
			}
		}
	}

	match async {
		use crate::db::schema::{betreuer, erwischt_game, erwischt_member, teilnehmer};
		use diesel::dsl;

		let mut connection = state.db.get().await?;

		diesel::delete(teilnehmer::table).execute(&mut connection).await?;
		diesel::delete(erwischt_member::table).execute(&mut connection).await?;
		diesel::delete(erwischt_game::table).execute(&mut connection).await?;
		diesel::delete(
			betreuer::table.filter(
				betreuer::anmeldedatum
					.lt(betreuer_signup_date_last_year())
					.or(dsl::not(betreuer::selbsterklaerung)),
			),
		)
		.execute(&mut connection)
		.await?;

		DbResult::Ok(())
	}
	.await
	{
		Err(error) => {
			warn!(%error, "Error deleting lager");
			crate::error_response(&state)
		}
		Ok(()) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}
