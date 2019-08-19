//! The signup template.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use actix_web::*;
use futures::{future, Future, IntoFuture};
use sentry::integrations::failure::capture_error;
use t4rust_derive::Template;

use crate::{db, AppState, BoxFuture, HttpRequest, HttpResponse};
use crate::form::Form;

#[derive(Template)]
#[TemplatePath = "templates/signupSupervisor.tt"]
#[derive(Debug)]
pub struct SignupSupervisor {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
}

impl Form for SignupSupervisor {
	fn get_values(&self) -> Cow<HashMap<String, String>> {
		Cow::Borrowed(&self.values)
	}
}

impl SignupSupervisor {
	pub fn new(_state: &AppState, values: HashMap<String, String>) -> Self {
		Self { values }
	}
}

pub fn signup(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	render_signup(req, HashMap::new())
}

/// Return the signup site with the prefilled `values`.
fn render_signup(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> BoxFuture<HttpResponse> {
	Box::new(crate::auth::get_roles(&req).and_then(move |res| -> BoxFuture<HttpResponse> {
		if let Ok(site) = req.state().sites["intern"].get_site(
			req.state().config.clone(), "betreuer-anmeldung", res) {
			let content = format!("{}", site);
			let new_content = SignupSupervisor::new(req.state(), values);
			let content = content
				.replace("<insert content here>", &format!("{}", new_content));
			Box::new(future::ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(content)))
		} else {
			crate::not_found(&req)
		}
	}))
}

/// show a success site.
fn signup_success() -> BoxFuture<HttpResponse> {
	// Redirect to success site
	Box::new(future::ok(
		HttpResponse::Found()
			.header(http::header::LOCATION, "betreuer-anmeldung-erfolgreich")
			.finish(),
	))
}

pub fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();
	let log_file = req.state().config.log_file.clone();
	let log_mutex = req.state().log_mutex.clone();

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
							capture_error(&format_err!("Error inserting {} {} \
								into database: {:?}", supervisor.vorname,
								supervisor.nachname, error));
							Box::new(render_signup(req, body).into_future())
						}
						Ok(Ok(())) => {
							if let Some(log_file) = log_file {
								let res: Result<_, Error> = (|| {
									let _lock = log_mutex.lock().unwrap();
									let mut file = std::fs::OpenOptions::new()
										.create(true)
										.append(true)
										.open(log_file)?;
									writeln!(file, "Betreuer: {}", serde_json::to_string(&supervisor)?)?;

									Ok(())
								})();

								if let Err(error) = res {
									body.insert(
										"error".to_string(),
										format!(
											"Es ist ein Fehler beim Speichern \
											 aufgetreten.\n{}",
											error_message
										),
									);
									warn!("Failed to log new supervisor: {:?}", error);
									return render_signup(req, body);
								}
							}

							signup_success()
						}
					}
				}),
		)})
		.responder()
}
