//! Display images from a folder.

use actix_web::{HttpRequest, HttpResponse};
use auth;
use futures::Future;

use {AppState, BoxFuture};

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
) -> BoxFuture<HttpResponse> {
	Box::new(auth::get_roles(&req)
		.and_then(move |res| {
			req.state().sites["public"].get_site(req.state().config.clone(), "Bilder2018/", res)
		})
		.map(|site| {
			let content = format!("{}", site);
		let images = format!(
			"{}",
			Images::new("Bilder 2018".to_string(), "Bilder2018".to_string())
		);
			let content = content.replace("<insert content here>", &images);

			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(content)
		}))
}
