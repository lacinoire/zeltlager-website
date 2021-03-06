//! Admin tools.

use actix_identity::Identity;
use actix_web::*;
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

#[derive(Clone, Debug, Deserialize)]
pub struct EditSupervisorData {
	supervisor: i32,
	juleica_nummer: Option<String>,
	fuehrungszeugnis_ausstellung: Option<chrono::NaiveDate>,
	fuehrungszeugnis_eingesehen: Option<chrono::NaiveDate>,
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

#[get("/betreuer")]
pub async fn render_supervisors(
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
	match state.sites["public"].get_site(state.config.clone(), "admin/betreuer", roles) {
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

#[post("/betreuer/edit")]
pub(crate) async fn edit_supervisor(
	state: web::Data<State>,
	data: web::Json<EditSupervisorData>,
) -> HttpResponse {
	match state.db_addr.send(db::RunOnDbMsg(move |db| {
		use db::schema::betreuer;
		use db::schema::betreuer::columns::*;

		diesel::update(betreuer::table.filter(id.eq(data.supervisor)))
			.set((juleica_nummer.eq(&data.juleica_nummer),
				fuehrungszeugnis_auststellung.eq(&data.fuehrungszeugnis_ausstellung),
				fuehrungszeugnis_eingesehen.eq(&data.fuehrungszeugnis_eingesehen)))
			.execute(&db.connection)?;
		Ok(())
	})).await.map_err(|e| e.into()) {
		Ok(Err(e)) | Err(e) => {
			error!("Failed to edit supervisor: {}", e);
			HttpResponse::InternalServerError()
				.body("Failed to edit supervisor")
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

/// Return all current members as json.
#[get("/betreuer.json")]
pub async fn download_supervisors_json(
	state: web::Data<State>,
) -> HttpResponse {
	match state.db_addr.send(db::DownloadFullSupervisorsMessage).await.map_err(|e| e.into()) {
		Ok(Err(error)) | Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(supervisors)) => {
			HttpResponse::Ok()
				.json(supervisors)
		}
	}
}
