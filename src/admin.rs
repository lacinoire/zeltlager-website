use actix_web::http::StatusCode;
use actix_web::*;
use anyhow::bail;
use diesel::prelude::*;
use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::{db, mail, State};

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveMemberData {
	member: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveSupervisorData {
	supervisor: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EditMemberData {
	member: i32,
	bezahlt: bool,
	anwesend: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EditSupervisorData {
	supervisor: i32,
	juleica_nummer: Option<String>,
	fuehrungszeugnis_ausstellung: Option<chrono::NaiveDate>,
	fuehrungszeugnis_eingesehen: Option<chrono::NaiveDate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditMemberResult {
	error: Option<String>,
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
	state: web::Data<State>, data: web::Json<EditMemberData>,
) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::teilnehmer;
			use db::schema::teilnehmer::columns::*;

			let member = teilnehmer::table
				.filter(id.eq(data.member))
				.get_result::<db::models::FullTeilnehmer>(&mut db.connection)?;
			let new_payed = data.bezahlt && !member.bezahlt;

			diesel::update(teilnehmer::table.filter(id.eq(data.member)))
				.set((bezahlt.eq(data.bezahlt), anwesend.eq(data.anwesend)))
				.execute(&mut db.connection)?;
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
	state: web::Data<State>, data: web::Json<EditSupervisorData>,
) -> HttpResponse {
	match state
		.db_addr
		.send(db::RunOnDbMsg(move |db| {
			use db::schema::betreuer;
			use db::schema::betreuer::columns::*;

			diesel::update(betreuer::table.filter(id.eq(data.supervisor)))
				.set((
					juleica_nummer.eq(&data.juleica_nummer),
					fuehrungszeugnis_auststellung.eq(&data.fuehrungszeugnis_ausstellung),
					fuehrungszeugnis_eingesehen.eq(&data.fuehrungszeugnis_eingesehen),
				))
				.execute(&mut db.connection)?;
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
