//! Display images from a folder.

use actix_web::{HttpRequest, HttpResponse};

use {AppState, Result};

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

pub fn render_images(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	if let Ok(site) =
		req.state().sites["public"].get_site(&req.state().config, "Bilder2018/")
	{
		let content = format!("{}", site);
		let images = format!(
			"{}",
			Images::new("Bilder 2018".to_string(), "Bilder2018".to_string())
		);
		let content = content.replace("<insert content here>", &images);

		return Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(content));
	}
	::not_found(&req)
}
