//! The signup template.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use actix_web::*;
use futures::{future, Future, IntoFuture};
use sentry::integrations::failure::capture_error;
use t4rust_derive::Template;

use crate::{db, discourse, mail, AppState, BoxFuture, HttpRequest,
	HttpResponse};
use crate::form::Form;

#[derive(Template)]
#[TemplatePath = "templates/signup.tt"]
#[derive(Debug)]
pub struct Signup {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
	pub reached_max_members: Option<String>,
}

impl Form for Signup {
	fn get_values(&self) -> Cow<HashMap<String, String>> {
		Cow::Borrowed(&self.values)
	}
}

impl Signup {
	pub fn new(
		state: &AppState,
		values: HashMap<String, String>,
	) -> BoxFuture<Self> {
		let max_members = state.config.max_members;
		let reached_max_members = state.config.max_members_reached.clone();
		Box::new(
			state
				.db_addr
				.send(db::CountMemberMessage)
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
		("hausnummer", "1"),
		("ort", "i"),
		("plz", "80000"),
	];

	let map = map.iter().map(|&(a, b)| (a.to_string(), b.to_string()));

	render_signup(req, map.collect())
}

/// Return the signup site with the prefilled `values`.
fn render_signup(
	req: HttpRequest<AppState>,
	values: HashMap<String, String>,
) -> BoxFuture<HttpResponse> {
	Box::new(crate::auth::get_roles(&req).and_then(move |res| -> BoxFuture<HttpResponse> {
		if let Ok(site) = req.state().sites["public"].get_site(
			req.state().config.clone(), "anmeldung", res) {
			let content = format!("{}", site);
			return Box::new(Signup::new(req.state(), values).and_then(
				move |new_content| {
					let content = content.replace(
						"<insert content here>",
						&format!("{}", new_content),
					);

					Ok(HttpResponse::Ok()
						.content_type("text/html; charset=utf-8")
						.body(content))
				},
			));
		}
		Box::new(crate::not_found(&req).into_future().from_err())
	}))
}

/// Check if too many members are already registered, then call `signup_mail`.
fn signup_check_count(
	count: i64,
	max_members: i64,
	db_addr: &actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>,
	disc_addr: Option<actix::Addr<discourse::DiscourseExecutor>>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	log_file: Option<PathBuf>,
	log_mutex: Arc<Mutex<()>>,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	if req.state().config.test_mail.as_ref().map(|m| m == &member.eltern_mail).unwrap_or(false) {
		// Don't insert test signup into database and discourse
		Box::new(signup_mail(&mail_addr, None, member, body, error_message, req))
	} else if count >= max_members {
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
							warn!("Error inserting into database: {:?}", error);
							render_signup(req, body)
						}
						Ok(Ok(())) => {
							if let Some(log_file) = log_file {
								let res: Result<_, Error> = (|| {
									let _lock = log_mutex.lock().unwrap();
									let mut file = std::fs::OpenOptions::new()
										.create(true)
										.append(true)
										.open(log_file)?;
									writeln!(file, "Teilnehmer: {}", serde_json::to_string(&member)?)?;

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
									warn!("Failed to log new member: {:?}", error);
									return render_signup(req, body);
								}
							}

							signup_mail(
								&mail_addr,
								disc_addr,
								member,
								body,
								error_message,
								req,
							)
						}
					}
				}),
		)
	}
}

/// Write an email and show a success site.
fn signup_mail(
	mail_addr: &actix::Addr<mail::MailExecutor>,
	disc_addr: Option<actix::Addr<discourse::DiscourseExecutor>>,
	member: db::models::Teilnehmer,
	mut body: HashMap<String, String>,
	error_message: String,
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	// Signup to discourse
	let fut = if let Some(addr) = disc_addr {
		signup_discourse(&addr, member.clone())
	} else {
		Box::new(future::ok(()))
	};

	// Write an e-mail
	let mail = member.eltern_mail.clone();
	Box::new(mail_addr
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
					error!("Error sending e-mail to {:?}: {:?}", mail, error);
					capture_error(&format_err!("Error sending e-mail to {:?}: {:?}", mail, error));
					render_signup(req, body)
				}
				Ok(Ok(())) => {
					// Redirect to success site
					Box::new(future::ok(
						HttpResponse::Found()
							.header(
								http::header::LOCATION,
								"anmeldung-erfolgreich",
							)
							.finish(),
					))
				}
			}
		})
		.then(|r| fut.then(move |dr| {
			if let Err(e) = dr {
				error!("Failed to signup to discourse: {:?}", e);
				capture_error(&format_err!("Failed to signup to discourse: {:?}", e));
			}
			r
		}))
	)
}

pub fn signup_send(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let mail_addr = req.state().mail_addr.clone();
	let disc_addr = req.state().disc_addr.clone();
	let error_message = req.state().config.error_message.clone();
	let max_members = req.state().config.max_members;
	let birthday_date = req.state().config.birthday_date.clone();
	let log_file = req.state().config.log_file.clone();
	let log_mutex = req.state().log_mutex.clone();
	let db_addr2 = db_addr.clone();

	// Get the body of the request
	req.clone().urlencoded()
		.limit(1024 * 5) // 5 kiB
		.from_err()
		.and_then(move |mut body: HashMap<_, _>| -> BoxFuture<_> {
			let mut member = match db::models::Teilnehmer::
				from_hashmap(body.clone(), &birthday_date) {
				Ok(member) => member,
				Err(error) => {
					// Show error and prefilled form
					body.insert("error".to_string(), format!("{}", error));
					warn!("Error handling form content: {}", error);
					return Box::new(render_signup(req, body).into_future());
				}
			};

			// Remove spaces
			member.trim();

			Box::new(db_addr.send(db::CountMemberMessage)
				.from_err::<failure::Error>()
				.then(move |result| -> BoxFuture<HttpResponse> { match result {
					Err(error) | Ok(Err(error)) => {
						// Show error and prefilled form
						body.insert("error".to_string(), format!("\
							Es ist ein Datenbank-Fehler aufgetreten.\n{}",
							error_message));
						warn!("Error inserting into database: {}", error);
						capture_error(&format_err!("Error inserting {:?} into database: {:?}", member.eltern_mail, error));
						render_signup(req, body)
					}
					Ok(Ok(count)) => signup_check_count(count, max_members,
						&db_addr2, mail_addr, disc_addr, member, body,
						error_message, log_file, log_mutex, req),
				}})
		)})
		.responder()
}

/// Add to discourse group
fn signup_discourse(
	disc_addr: &actix::Addr<discourse::DiscourseExecutor>,
	member: db::models::Teilnehmer,
) -> BoxFuture<()> {
	// Write an e-mail
	Box::new(
		disc_addr
			.send(discourse::SignupMessage { member })
			.from_err::<failure::Error>()
			.and_then(|r| r)
	)
}
