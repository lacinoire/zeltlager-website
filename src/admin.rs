//! Admin tools.

use actix_identity::Identity;
use actix_web::*;
use actix_web::http::header::CONTENT_DISPOSITION;
use anyhow::bail;
use diesel::prelude::*;
use log::{error, warn};
use serde::Deserialize;

use crate::auth;
use crate::{db, State};

#[derive(Clone, Debug, Deserialize)]
pub struct RemoveMemberData {
	member: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EditMemberData {
	member: i32,
	bezahlt: bool,
	anwesend: bool,
}

#[get("/")]
pub async fn render_admin(
	state: web::Data<State>,
	id: Identity,
) -> HttpResponse {
	let roles = match auth::get_roles(&**state, &id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(&**state);
		}
	};
	match state.sites["public"].get_site(state.config.clone(), "admin/", roles) {
		Ok(site) => HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(format!("{}", site)),
		Err(e) => {
			error!("Failed to get site: {}", e);
			crate::error_response(&**state)
		}
	}
}

#[get("/teilnehmer")]
pub async fn render_members(
	state: web::Data<State>,
	id: Identity,
) -> HttpResponse {
	let roles = match auth::get_roles(&**state, &id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(&**state);
		}
	};
	match state.sites["public"].get_site(state.config.clone(), "admin/teilnehmer", roles) {
		Ok(site) => HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(format!("{}", site)),
		Err(e) => {
			error!("Failed to get site: {}", e);
			crate::error_response(&**state)
		}
	}
}

#[post("/teilnehmer/remove")]
pub(crate) async fn remove_member(
	state: web::Data<State>,
	data: web::Json<RemoveMemberData>,
) -> HttpResponse {
	match state.db_addr.send(db::RunOnDbMsg(move |db| {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		let r = diesel::delete(teilnehmer::table.filter(id.eq(data.member)))
			.execute(&db.connection)?;
		if r == 0 {
			bail!("Member not found");
		}
		Ok(())
	})).await.map_err(|e| e.into()) {
		Ok(Err(e)) | Err(e) => {
			error!("Failed to remove member: {}", e);
			HttpResponse::InternalServerError()
				.body("Failed to remove member")
		}
		Ok(Ok(())) => {
			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body("Success")
		}
	}
}

#[post("/teilnehmer/edit")]
pub(crate) async fn edit_member(
	state: web::Data<State>,
	data: web::Json<EditMemberData>,
) -> HttpResponse {
	match state.db_addr.send(db::RunOnDbMsg(move |db| {
		use db::schema::teilnehmer;
		use db::schema::teilnehmer::columns::*;

		diesel::update(teilnehmer::table.filter(id.eq(data.member)))
			.set((bezahlt.eq(data.bezahlt), anwesend.eq(data.anwesend)))
			.execute(&db.connection)?;
		Ok(())
	})).await.map_err(|e| e.into()) {
		Ok(Err(e)) | Err(e) => {
			error!("Failed to edit member: {}", e);
			HttpResponse::InternalServerError()
				.body("Failed to edit member")
		}
		Ok(Ok(())) => {
			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body("Success")
		}
	}
}

/// Return all current members as json.
#[get("/teilnehmer.json")]
pub async fn download_members_json(
	state: web::Data<State>,
) -> HttpResponse {
	match state.db_addr.send(db::DownloadFullMembersMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(members)) => {
			HttpResponse::Ok()
				.json(members)
		}
	}
}

/// Return all current members as csv.
#[get("/teilnehmer.csv")]
pub async fn download_members_csv(
	state: web::Data<State>,
) -> HttpResponse {
	match state.db_addr.send(db::DownloadMembersMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(members)) => {
			let mut res = Vec::new();
			{
				let mut writer = csv::WriterBuilder::new()
					.delimiter(b'|')
					.from_writer(&mut res);
				for t in members {
					if let Err(e) = writer.serialize(t) {
						error!("Failed converting member to csv: {}", e);
						return HttpResponse::InternalServerError().body("Failed to create csv");
					}
				}
			}

			HttpResponse::Ok()
				.content_type("text/csv; charset=utf-8")
				.header(CONTENT_DISPOSITION, "attachment;filename=teilnehmer.csv")
				.body(res)
		}
	}
}

/// Return all current members as csv.
#[get("/betreuer.csv")]
pub async fn download_betreuer_csv(
	state: web::Data<State>,
) -> HttpResponse {
	match state.db_addr.send(db::DownloadBetreuerMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(betreuer)) => {
			let mut res = Vec::new();
			{
				let mut writer = csv::WriterBuilder::new()
					.delimiter(b'|')
					.from_writer(&mut res);
				for t in betreuer {
					if let Err(e) = writer.serialize(t) {
						error!("Failed converting supervisor to csv: {}", e);
						return HttpResponse::InternalServerError().body("Failed to create csv");
					}
				}
			}

			HttpResponse::Ok()
				.content_type("text/csv; charset=utf-8")
				.header(CONTENT_DISPOSITION, "attachment;filename=betreuer.csv")
				.body(res)
		}
	}
}
