use anyhow::{Error, bail};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, extract};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::db::models::{FullSupervisor, FullTeilnehmer};
use crate::{ExtractState, WebResult, db, mail};
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

fn err<T>(error: Error, msg: &'static str) -> WebResult<T> {
	error!(%error, "{msg}");
	Err((StatusCode::INTERNAL_SERVER_ERROR, msg).into_response())
}

pub(crate) async fn remove_member(
	extract::State(state): ExtractState, Json(data): Json<RemoveMemberData>,
) -> WebResult<&'static str> {
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
		Err(error) => err(error, "Failed to remove member"),
		Ok(()) => Ok("Success"),
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

pub(crate) async fn edit_member(
	extract::State(state): ExtractState, Json(data): Json<FullTeilnehmer>,
) -> Response {
	match async {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let mut connection = state.db.get().await?;

		let member = teilnehmer::table
			.filter(id.eq(data.id))
			.get_result::<db::models::FullTeilnehmer>(&mut connection)
			.await?;
		let new_payed = data.bezahlt && !member.bezahlt;

		diesel::update(&data).set(&data).execute(&mut connection).await?;
		DbResult::Ok((new_payed, member))
	}
	.await
	{
		Err(error) => {
			error!(%error, "Failed to edit member");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(EditMemberResult {
					error: Some(format!("Teilnehmer konnte nicht gespeichert werden: {error}")),
				}),
			)
				.into_response()
		}
		Ok((new_payed, member)) => {
			if new_payed {
				let (status, result) = payed_mail(&state.mail, member).await;
				(status, Json(result)).into_response()
			} else {
				Json(EditMemberResult { error: None }).into_response()
			}
		}
	}
}

// TODO Use delete("/betreuer/{id}") here and for teilnehmer
pub(crate) async fn remove_supervisor(
	extract::State(state): ExtractState, Json(data): Json<RemoveSupervisorData>,
) -> WebResult<&'static str> {
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
		Err(error) => err(error, "Failed to remove supervisor"),
		Ok(()) => Ok("Success"),
	}
}

pub(crate) async fn edit_supervisor(
	extract::State(state): ExtractState, Json(data): Json<FullSupervisor>,
) -> WebResult<&'static str> {
	match async {
		diesel::update(&data).set(&data).execute(&mut state.db.get().await?).await?;
		DbResult::Ok(())
	}
	.await
	{
		Err(error) => err(error, "Failed to edit supervisor"),
		Ok(()) => Ok("Success"),
	}
}

/// Return all current members as json.
pub async fn download_members(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<FullTeilnehmer>>> {
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
		Ok(members) => Ok(Json(members)),
	}
}

/// Return all supervisors as json.
pub async fn download_supervisors(
	extract::State(state): ExtractState,
) -> WebResult<Json<Vec<FullSupervisor>>> {
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
		Ok(supervisors) => Ok(Json(supervisors)),
	}
}

pub async fn download_mails(
	extract::State(state): ExtractState,
) -> Result<Json<Vec<String>>, Response> {
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
		Ok(mails) => Ok(Json(mails)),
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
pub async fn lager_info(extract::State(state): ExtractState) -> WebResult<Json<LagerInfo>> {
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
		Ok(info) => Ok(Json(info)),
	}
}

/// Remove all data for the current lager.
pub async fn remove_lager(extract::State(state): ExtractState) -> WebResult<&'static str> {
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
		Ok(()) => Ok("Success"),
	}
}
