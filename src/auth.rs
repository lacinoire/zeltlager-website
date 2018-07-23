//! User authentication

use std::collections::HashMap;

use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, HttpMessage, HttpRequest, HttpResponse};
use failure;
use futures::{future, Future};

use {AppState, BoxFuture, Result};

#[derive(Clone, EnumString, Debug, Deserialize)]
pub enum Roles {
	ImageDownload2018,
	ImageUpload,
}

pub fn login(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	// Search user in database
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();

	Box::new(
		req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let msg = tryf!(::db::AuthenticateMessage::
				from_hashmap(body.clone()));
			let username = msg.username.clone();

			Box::new(db_addr.send(msg)
				.from_err::<failure::Error>()
				.then(move |result| -> Result<HttpResponse> { Ok(match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert("error".to_string(), format!("\
							Es ist ein Datenbank-Fehler aufgetreten.\n{}",
							error_message));
						warn!("Error by auth message: {}", error);
						// TODO Actually, do nothing
						HttpResponse::Found().header("location", "/").finish()
					}
					Ok(Ok(true)) => {
						req.remember(username);
						HttpResponse::Found().header("location", "/images").finish()
					}
					_ => {
						// Wrong username or password
						HttpResponse::Found().header("location", "/wrong").finish()
					}
				})})
		)})
		.responder(),
	)
}

pub fn logout(req: HttpRequest<AppState>) -> HttpResponse {
	req.forget();
	HttpResponse::Found().header("location", "/").finish()
}
