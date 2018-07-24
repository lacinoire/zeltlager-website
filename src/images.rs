//! Display images from a folder.

use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, HttpMessage, HttpRequest, HttpResponse};
use chrono::{DateTime, NaiveDateTime, Utc};
use failure;
use futures::{Future, IntoFuture};

use {AppState, BoxFuture, Result};

#[derive(Template)]
#[TemplatePath = "templates/images.tt"]
#[derive(Debug)]
pub struct Images {
	pub title: String,
	/// Name of the folder
	pub name: String,
}

fn render_images(
	req: HttpRequest<AppState>,
) -> Result<HttpResponse> {
	if let Ok(site) =
		req.state().sites["public"].get_site(&req.state().config, "images")
	{
		let content = format!("{}", site);
		let login = format!("{}", Images::new(values));
		let content = content.replace("<insert content here>", &login);

		return Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(content));
	}
	::not_found(&req)
}

// TODO Also restrict acces on the raw images
