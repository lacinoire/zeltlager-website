//! Admin tools.

use actix_identity::Identity;
use actix_web::*;
use actix_web::http::header::CONTENT_DISPOSITION;
use log::{error, warn};

use crate::auth;
use crate::{db, State};

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

/// Return all current members as csv.
#[get("/teilnehmer.csv")]
pub async fn download_members_csv(
	state: web::Data<State>,
) -> HttpResponse {
	let db_addr = state.db_addr.clone();

	match db_addr.send(db::DownloadMembersMessage).await {
		Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Err(error)) => {
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
	let db_addr = state.db_addr.clone();

	match db_addr.send(db::DownloadBetreuerMessage).await {
		Err(error) => {
			warn!("Error fetching from database: {}", error);
			crate::error_response(&**state)
		}
		Ok(Err(error)) => {
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
