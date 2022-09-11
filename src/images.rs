//! Display images from a folder.

use actix_identity::Identity;
use actix_web::*;
use log::{error, warn};
use t4rust_derive::Template;

use crate::auth;
use crate::basic::SiteDescription;
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

pub fn split_image_name(s: &str) -> String {
	#[derive(Debug, Eq, PartialEq)]
	enum CharType {
		Letter,
		Number,
		None,
	}

	let mut last_type = CharType::None;
	let mut res = String::new();
	for c in s.chars() {
		let new_type = if c.is_ascii_digit() { CharType::Number } else { CharType::Letter };
		if new_type != last_type && last_type != CharType::None {
			res.push(' ');
		}
		res.push(c);
		last_type = new_type;
	}
	res
}

/*pub async fn render_images(state: web::Data<State>, id: Identity, name: String) -> HttpResponse {
	let roles = match auth::get_roles(&**state, &id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(&**state);
		}
	};

	let images = format!("{}", Images::new("Bilder".to_string(), name.clone()));
	let site = Basic {
		logged_in_roles: roles,
		config: state.config.clone(),
		all_sites: state.sites["public"].clone(),
		current_site: SiteDescription {
			name: format!("Bilder{}/", name),
			file_name: "Empty.txt".into(),
			title: format!("Bilder {}", split_image_name(&name)),
			description: format!("Bilder aus dem Zeltlager {}", name),
			navbar_visible: true,
			role: Some(auth::Roles::Images(name.into())),
		},
		content: images,
	};
	let content = format!("{}", site);

	HttpResponse::Ok().content_type("text/html; charset=utf-8").body(content)
}*/
