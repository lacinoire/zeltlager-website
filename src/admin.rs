use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::bail;
use diesel::prelude::*;
use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::{
	State,
	db::{
		self,
		models::{FullSupervisor, FullTeilnehmer},
	},
	mail,
};
use time::OffsetDateTime;

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
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::teilnehmer;
			use db::schema::teilnehmer::columns::*;

			let r = diesel::delete(teilnehmer::table.filter(id.eq(data.member)))
				.execute(&mut db.connection)?;
			if r == 0 {
				bail!("Member not found");
			}
			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to remove member: {}", e);
			HttpResponse::InternalServerError().body("Failed to remove member")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

/// Write mail to confirm payment
async fn payed_mail(
	mail_addr: &actix::Addr<mail::MailExecutor>, member: db::models::FullTeilnehmer,
) -> (StatusCode, EditMemberResult) {
	// Write an e-mail
	let mail = member.eltern_mail.clone();
	let error = match mail_addr.send(mail::PayedMessage { member }).await {
		Err(error) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
			format!(
				"Die Änderung wurde erfolgreich gespeichert.\nEs ist leider ein Fehler beim \
				 E-Mail senden aufgetreten.\n{}",
				error
			)
		}
		Ok(Err(error)) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
			format!(
				"Die Änderung wurde erfolgreich gespeichert.\nEs ist leider ein Fehler beim \
				 E-Mail senden aufgetreten.\n{}",
				error
			)
		}
		Ok(Ok(())) => {
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
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::teilnehmer;
			use db::schema::teilnehmer::columns::*;

			let member = teilnehmer::table
				.filter(id.eq(data.id))
				.get_result::<db::models::FullTeilnehmer>(&mut db.connection)?;
			let new_payed = data.bezahlt && !member.bezahlt;

			diesel::update(&*data).set(&*data).execute(&mut db.connection)?;
			Ok((new_payed, member))
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to edit member: {}", e);
			HttpResponse::InternalServerError().json(EditMemberResult {
				error: Some(format!("Teilnehmer konnte nicht gespeichert werden: {}", e)),
			})
		}
		Ok(Ok((new_payed, member))) => {
			if new_payed {
				let (status, result) = payed_mail(&state.mail_addr, member).await;
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
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::betreuer::columns::*;

			let r = diesel::delete(betreuer::table.filter(id.eq(data.supervisor)))
				.execute(&mut db.connection)?;
			if r == 0 {
				bail!("Supervisor not found");
			}
			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to remove supervisor: {}", e);
			HttpResponse::InternalServerError().body("Failed to remove supervisor")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

#[post("/betreuer/edit")]
pub(crate) async fn edit_supervisor(
	state: web::Data<State>, data: web::Json<FullSupervisor>,
) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			diesel::update(&*data).set(&*data).execute(&mut db.connection)?;
			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(e)) | Err(e) => {
			error!("Failed to edit supervisor: {}", e);
			HttpResponse::InternalServerError().body("Failed to edit supervisor")
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}

/// Return all current members as json.
#[get("/teilnehmer")]
pub async fn download_members(state: web::Data<State>) -> HttpResponse {
	match state.db_addr.send(db::DownloadFullMembersMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&state)
		}
		Ok(Ok(members)) => HttpResponse::Ok().json(members),
	}
}

/// Return all supervisors as json.
#[get("/betreuer")]
pub async fn download_supervisors(state: web::Data<State>) -> HttpResponse {
	match state.db_addr.send(db::DownloadFullSupervisorsMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&state)
		}
		Ok(Ok(supervisors)) => HttpResponse::Ok().json(supervisors),
	}
}

/// Return all unique mail addresses as json.
#[get("/mails")]
pub async fn download_mails(state: web::Data<State>) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use crate::db::schema::teilnehmer;
			use diesel::prelude::*;

			let mut mails = teilnehmer::table
				.select(teilnehmer::eltern_mail)
				.load::<String>(&mut db.connection)?;
			mails.sort();
			mails.dedup();

			Ok(mails)
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&state)
		}
		Ok(Ok(mails)) => HttpResponse::Ok().json(mails),
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
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use crate::db::schema::{betreuer, erwischt_game, teilnehmer};
			use diesel::prelude::*;

			let teilnehmer_count = teilnehmer::table.count().get_result(&mut db.connection)?;
			let old_betreuer_count = betreuer::table
				.filter(betreuer::anmeldedatum.lt(betreuer_signup_date_last_year()))
				.count()
				.get_result(&mut db.connection)?;
			let erwischt_game_count =
				erwischt_game::table.count().get_result(&mut db.connection)?;

			Ok(LagerInfo { teilnehmer_count, old_betreuer_count, erwischt_game_count })
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			warn!("Error getting lager info: {}", error);
			crate::error_response(&state)
		}
		Ok(Ok(info)) => HttpResponse::Ok().json(info),
	}
}

/// Remove all data for the current lager.
#[delete("/lager")]
pub async fn remove_lager(state: web::Data<State>) -> HttpResponse {
	// Remove log file
	if let Some(log_file) = &state.config.log_file {
		if let Err(error) = std::fs::remove_file(log_file) {
			if error.kind() != std::io::ErrorKind::NotFound {
				error!("Failed to remove log file ({}): {error}", log_file.display());
			}
		}
	}

	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use crate::db::schema::{betreuer, erwischt_game, erwischt_member, teilnehmer};
			use diesel::prelude::*;

			diesel::delete(teilnehmer::table).execute(&mut db.connection)?;
			diesel::delete(erwischt_member::table).execute(&mut db.connection)?;
			diesel::delete(erwischt_game::table).execute(&mut db.connection)?;
			diesel::delete(
				betreuer::table.filter(betreuer::anmeldedatum.lt(betreuer_signup_date_last_year())),
			)
			.execute(&mut db.connection)?;

			Ok(())
		}))
		.await
		.map_err(|e| e.into())
	{
		Ok(Err(error)) | Err(error) => {
			warn!("Error deleting lager: {}", error);
			crate::error_response(&state)
		}
		Ok(Ok(())) => HttpResponse::Ok().content_type("text/html; charset=utf-8").body("Success"),
	}
}
