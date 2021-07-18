//! Display images from a folder.

use actix_identity::Identity;
use actix_web::*;
use log::{error, warn};
use t4rust_derive::Template;

use crate::auth;
use crate::State;

#[derive(Template)]
#[TemplatePath = "templates/images.tt"]
#[derive(Debug)]
pub struct Images {
	pub title: String,
	/// Name of the folder
	pub name: String,
}

impl Images {
	fn new(title: String, name: String) -> Self { Self { title, name } }
}

pub async fn render_images(
	state: web::Data<State>, id: Identity, name: &'static str,
) -> HttpResponse {
	let roles = match auth::get_roles(&**state, &id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(&**state);
		}
	};
	let site =
		match state.sites["public"].get_site(state.config.clone(), &format!("{}/", name), roles) {
			Ok(r) => r,
			Err(e) => {
				error!("Failed to get site: {}", e);
				return crate::error_response(&**state);
			}
		};
	let content = format!("{}", site);
	let images = format!("{}", Images::new("Bilder".to_string(), name.to_string()));
	let content = content.replace("<insert content here>", &images);

	HttpResponse::Ok().content_type("text/html; charset=utf-8").body(content)
}
