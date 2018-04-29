//! The signup template.
use std::collections::HashMap;

use actix_web::*;
use db;
use failure;
use form::Form;
use futures::{future, Future, IntoFuture};
use AppState;
use BoxFuture;
use HttpRequest;
use HttpResponse;

use Result;

#[derive(Template)]
#[TemplatePath = "templates/signupSupervisor.tt"]
#[derive(Debug)]
pub struct SignupSupervisor {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
}

impl Form for SignupSupervisor {
	fn get_values(&self) -> &HashMap<String, String> {
		&self.values
	}
}

impl SignupSupervisor {
	pub fn new(_state: &::AppState, values: HashMap<String, String>) -> Self {
		Self { values }
	}
}

pub fn signup(req: HttpRequest<AppState>) -> Result<HttpResponse> {
	render_signup(req, HashMap::new())
}

/// Return the signup site with the prefilled `values`.
fn render_signup(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> Result<HttpResponse> {
	if let Ok(site) = req.state().sites["intern"]
		.get_site(&req.state().config, "betreuer-anmeldung")
	{
		let content = format!("{}", site);
		let new_content = SignupSupervisor::new(req.state(), values);
		let content = content.replace(
			"<insert content here>",
			&format!("{}", new_content),
		);
		return Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(content));
	}
	::not_found(req)
}

/// show a success site.
fn signup_success() -> BoxFuture<HttpResponse> {
	// Redirect to success site
	Box::new(future::ok(
		HttpResponse::Found()
			.header(
				http::header::LOCATION,
				"betreuer-anmeldung-erfolgreich",
			)
			.finish(),
	))
}

pub fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();

	// Get the body of the request
	req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let supervisor = match db::models::Supervisor::
				from_hashmap(body.clone()) {
				Ok(supervisor) => supervisor,
				Err(error) => {
					// Show error and prefilled form
					body.insert("error".to_string(), format!("{}", error));
					warn!("Error handling form content: {}", error);
					return Box::new(render_signup(req, body).into_future());
				}
			};

			Box::new(
			db_addr
				.send(db::SignupSupervisorMessage {
					supervisor: supervisor.clone(),
				})
				.from_err::<failure::Error>()
				.then(move |result| -> BoxFuture<HttpResponse> {
					match result {
						Err(error) | Ok(Err(error)) => {
							// Show error and prefilled form
							body.insert(
								"error".to_string(),
								format!(
									"Es ist ein Datenbank-Fehler \
									 aufgetreten.\n{}",
									error_message
								),
							);
							warn!("Error inserting into database: {}", error);
							Box::new(render_signup(req, body).into_future())
						}
						Ok(Ok(())) => signup_success(),
					}
				}),
		)})
		.responder()
}
