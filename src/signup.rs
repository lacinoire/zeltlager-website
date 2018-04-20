//! The signup template.
use std::collections::HashMap;

use actix;
use actix_web::*;
use AppState;
use BoxFuture;
use db;
use failure;
use form::Form;
use futures::{BoxFuture, future, Future, IntoFuture};
use HttpResponse;
use HttpRequest;
use mail;

#[derive(Template)]
#[TemplatePath = "templates/signup.tt"]
#[derive(Debug)]
pub struct Signup {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
	pub reached_max_members: Option<String>,
}

impl Form for Signup {
	fn get_values(&self) -> &HashMap<String, String> {
		&self.values
	}
}

impl Signup {
	pub fn new(
		state: &::AppState,
		values: HashMap<String, String>,
	) -> BoxFuture<Self> {
		let max_members = state.config.max_members;
		let reached_max_members = state.config.max_members_reached.clone();
		Box::new(
			state
				.db_addr
				.send(::db::CountMemberMessage)
				.from_err::<::failure::Error>()
				.then(move |result| match result {
					Err(error) | Ok(Err(error)) => {
						error!(
							"Failed to get current member count: {:?}",
							error
						);
						Err(error)
					}
					Ok(Ok(count)) => Ok(Self {
						values,
						reached_max_members: if count >= max_members {
							Some(reached_max_members)
						} else {
							None
						},
					}),
				}),
		)
	}
}

pub fn signup(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	render_signup(req, HashMap::new())
}

pub fn signup_test(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let map = vec![
		("vorname", "a"),
		("nachname", "b"),
		("geburtsdatum", "1.1.2010"),
		("geschlecht", "w"),
		("schwimmer", "true"),
		("vegetarier", "false"),
		("tetanus_impfung", "true"),
		("eltern_name", "d"),
		("eltern_mail", "@"),
		("eltern_handynummer", "f"),
		("strasse", "g"),
		("hausnummer", "h"),
		("ort", "i"),
		("plz", "80000"),
	];

	let map = map.iter()
		.map(|&(a, b)| (a.to_string(), b.to_string()));

	render_signup(req, map.collect())
}

/// Return the signup site with the prefilled `values`.
fn render_signup(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> BoxFuture<HttpResponse> {
	if let Ok(site) = req.state()
		.sites["public"]
		.get_site(&req.state().config, "anmeldung")
	{
		let content = format!("{}", site);
		return Box::new(
			Signup::new(req.state(), values).and_then(
				move |new_content| {
					let content = content.replace(
						"<insert content here>",
						&format!("{}", new_content),
					);

					Ok(HttpResponse::Ok()
						.content_type("text/html; charset=utf-8")
						.body(content))
				},
			),
		);
	}
	Box::new(::not_found(req).into_future().from_err())
}

/// Check if too many members are already registered, then call `signup_insert`.
fn signup_check_count(
	count: i64,
	max_members: i64,
	db_addr: &actix::Addr<actix::Syn, db::DbExecutor>,
	mail_addr: actix::Addr<actix::Syn, mail::MailExecutor>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	if count >= max_members {
		// Show error
		body.insert(
			"error".to_string(),
			"Während Ihrer Anmeldung ist das Zeltlager leider schon voll \
			 geworden."
				.to_string(),
		);
		warn!(
			"Already too many members registered (from {})",
			member.eltern_mail
		);
		render_signup(req, body)
	} else {
		Box::new(
			db_addr
				.send(db::SignupMessage {
					member: member.clone(),
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
							render_signup(req, body)
						}
						Ok(Ok(())) => signup_insert(
							&mail_addr,
							member,
							body,
							error_message,
							req,
						),
					}
				}),
		)
	}
}

/// Insert the member into the database, write an email and show a success site.
fn signup_insert(
	mail_addr: &actix::Addr<actix::Syn, mail::MailExecutor>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	// Write an e-mail
	Box::new(
		mail_addr
			.send(mail::SignupMessage { member })
			.from_err::<failure::Error>()
			.then(move |result| -> BoxFuture<HttpResponse> {
				match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert(
							"error".to_string(),
							format!(
								"Ihre Daten wurden erfolgreich \
								 gespeichert.\nEs ist leider ein Fehler beim \
								 E-Mail senden aufgetreten.\n{}",
								error_message
							),
						);
						warn!("Error sending e-mail: {}", error);
						render_signup(req, body)
					}
					Ok(Ok(())) => {
						// Redirect to success site
						Box::new(future::ok(
							HttpResponse::Found()
								.header(
									http::header::LOCATION,
									"anmeldungErfolgreich",
								)
								.finish(),
						))
					}
				}
			}),
	)
}

pub fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let mail_addr = req.state().mail_addr.clone();
	let error_message = req.state().config.error_message.clone();
	let max_members = req.state().config.max_members;
	let db_addr2 = db_addr.clone();

	// Get the body of the request
	req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let member = match db::models::Teilnehmer::
				from_hashmap(body.clone()) {
				Ok(member) => member,
				Err(error) => {
					// Show error and prefilled form
					body.insert("error".to_string(), format!("{}", error));
					warn!("Error handling form content: {}", error);
					return Box::new(render_signup(req, body).into_future());
				}
			};

			Box::new(db_addr.send(db::CountMemberMessage)
				.from_err::<failure::Error>()
				.then(move |result| -> BoxFuture<HttpResponse> { match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert("error".to_string(), format!("\
							Es ist ein Datenbank-Fehler aufgetreten.\n{}",
							error_message));
						warn!("Error inserting into database: {}", error);
						render_signup(req, body)
					}
					Ok(Ok(count)) => signup_check_count(count, max_members,
						&db_addr2, mail_addr, member, body, error_message, req),
				}})
		)})
		.responder()
}
