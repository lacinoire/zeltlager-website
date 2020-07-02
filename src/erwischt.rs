//! Erwischt game.

use actix_identity::Identity;
use actix_web::*;
use log::error;

use crate::auth;
use crate::{db, State};

#[get("/")]
pub async fn render_erwischt(
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
	match state.sites["public"].get_site(state.config.clone(), "erwischt/", roles) {
		Ok(site) => HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(format!("{}", site)),
		Err(e) => {
			error!("Failed to get site: {}", e);
			crate::error_response(&**state)
		}
	}
}
