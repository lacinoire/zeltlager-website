//! Display images from a folder.

use actix_web::{HttpRequest, HttpResponse};
use futures::Future;

use crate::auth;
use crate::{AppState, BoxFuture};

#[derive(Template)]
#[TemplatePath = "templates/images.tt"]
#[derive(Debug)]
pub struct Images {
	pub title: String,
	/// Name of the folder
	pub name: String,
}

impl Images {
	fn new(title: String, name: String) -> Self {
		Self { title, name }
	}
}

pub fn render_images(
	req: HttpRequest<AppState>,
	name: &'static str,
) -> BoxFuture<HttpResponse> {
	Box::new(auth::get_roles(&req)
		.and_then(move |res| {
			req.state().sites["public"].get_site(req.state().config.clone(), &format!("{}/", name), res)
		})
		.map(move |site| {
			let content = format!("{}", site);
			let images = format!(
				"{}",
				Images::new("Bilder".to_string(), name.to_string())
			);
			let content = content.replace("<insert content here>", &images);

			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(content)
		}))
}
