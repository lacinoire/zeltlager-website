//! The signup template.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use actix_identity::Identity;
use actix_web::*;
use anyhow::Result;
use log::{error, warn};
use serde::Serialize;
use t4rust_derive::Template;

use crate::form::Form;
use crate::{db, mail, HttpRequest, HttpResponse, State};

#[derive(Template)]
#[TemplatePath = "templates/signup.tt"]
#[derive(Debug)]
pub struct Signup {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
	pub reached_max_members: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SignupResult {
	error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignupState {
	is_full: bool,
}

impl Form for Signup {
	fn get_values(&self) -> Cow<HashMap<String, String>> { Cow::Borrowed(&self.values) }
}

impl Signup {
	pub async fn new(state: &State, values: HashMap<String, String>) -> Result<Self> {
		let max_members = state.config.max_members;
		let reached_max_members = state.config.max_members_reached.clone();
		match state.db_addr.send(db::CountMemberMessage).await.map_err(|e| e.into()) {
			Err(error) | Ok(Err(error)) => {
				error!("Failed to get current member count: {:?}", error);
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
		}
	}
}

#[get("/anmeldung-test")]
pub async fn signup_test(state: web::Data<State>, id: Identity, req: HttpRequest) -> HttpResponse {
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
		("krankenversicherung", "gesetzlich"),
	];

	let map = map.iter().map(|(a, b)| (a.to_string(), b.to_string()));

	render_signup(&**state, &id, &req, map.collect()).await
}

/// Return the signup site with the prefilled `values`.
async fn render_signup(
	state: &State, id: &Identity, req: &HttpRequest, values: HashMap<String, String>,
) -> HttpResponse {
	let roles = match crate::auth::get_roles(state, id).await {
		Ok(r) => r,
		Err(e) => {
			error!("Failed to get roles: {}", e);
			return crate::error_response(state);
		}
	};
	if let Ok(site) = state.sites["public"].get_site(state.config.clone(), "anmeldung", roles) {
		let content = format!("{}", site);
		let new_content = match Signup::new(state, values).await {
			Ok(r) => r,
			Err(e) => {
				error!("Failed to create signup: {}", e);
				return crate::error_response(state);
			}
		};
		let content = content.replace("<insert content here>", &format!("{}", new_content));

		HttpResponse::Ok().content_type("text/html; charset=utf-8").body(content)
	} else {
		crate::not_found(state, id, req).await
	}
}

/// Check if too many members are already registered, then call `signup_mail`.
async fn signup_check_count(
	count: i64, max_members: i64, db_addr: &actix::Addr<db::DbExecutor>,
	mail_addr: actix::Addr<mail::MailExecutor>, member: db::models::Teilnehmer,
	error_message: String, log_file: Option<PathBuf>, log_mutex: Arc<Mutex<()>>, state: &State,
) -> HttpResponse {
	if state.config.test_mail.as_ref().map(|m| m == &member.eltern_mail).unwrap_or(false) {
		// Don't insert test signup into database and discourse
		signup_mail(&mail_addr, member, error_message).await
	} else if count >= max_members {
		// Show error
		warn!("Already too many members registered (from {})", member.eltern_mail);
		HttpResponse::BadRequest().json(SignupResult {
			error: Some(
				"Während Ihrer Anmeldung ist das Zeltlager leider schon voll geworden.".to_string(),
			),
		})
	} else {
		match db_addr.send(db::SignupMessage { member: member.clone() }).await {
			Err(error) => {
				warn!("Error inserting into database: {:?}", error);
			}
			Ok(Err(error)) => {
				warn!("Error inserting into database: {:?}", error);
			}
			Ok(Ok(())) => {
				if let Some(log_file) = log_file {
					let res: Result<_, Error> = (|| {
						let _lock = log_mutex.lock().unwrap();
						let mut file =
							std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
						writeln!(file, "Teilnehmer: {}", serde_json::to_string(&member)?)?;

						Ok(())
					})();

					if let Err(error) = res {
						warn!("Failed to log new member: {:?}", error);
						// Ignore as it was saved in the db
					}
				}

				return signup_mail(&mail_addr, member, error_message).await;
			}
		}

		HttpResponse::InternalServerError().json(SignupResult {
			error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
		})
	}
}

/// Write an email and show a success site.
async fn signup_mail(
	mail_addr: &actix::Addr<mail::MailExecutor>, member: db::models::Teilnehmer,
	error_message: String,
) -> HttpResponse {
	// Write an e-mail
	let mail = member.eltern_mail.clone();
	match mail_addr.send(mail::SignupMessage { member }).await {
		Err(error) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
		}
		Ok(Err(error)) => {
			error!("Error sending e-mail to {:?}: {:?}", mail, error);
		}
		Ok(Ok(())) => {
			// Signup successful
			// TODO redirect in nojs
			/*return HttpResponse::Found()
			.append_header((http::header::LOCATION, "anmeldung-erfolgreich"))
			.finish();*/
			return HttpResponse::Ok().json(SignupResult { error: None });
		}
	}

	HttpResponse::InternalServerError().json(SignupResult {
		error: Some(format!(
			"Ihre Daten wurden erfolgreich gespeichert.\nEs ist leider ein Fehler beim E-Mail \
			 senden aufgetreten.\n{}",
			error_message
		)),
	})
}

#[get("/signup-state")]
pub async fn signup_state(state: web::Data<State>) -> HttpResponse {
	match state.db_addr.send(db::CountMemberMessage).await.map_err(|e| e.into()) {
		Err(error) | Ok(Err(error)) => {
			error!("Failed to get current member count: {:?}", error);
			crate::error_response(&**state)
		}
		Ok(Ok(count)) => {
			return HttpResponse::Ok()
				.json(SignupState { is_full: count >= state.config.max_members });
		}
	}
}

#[post("/signup")]
pub async fn signup(
	state: web::Data<State>, body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let db_addr = state.db_addr.clone();
	let mail_addr = state.mail_addr.clone();
	let error_message = state.config.error_message.clone();
	let max_members = state.config.max_members;
	let birthday_date = state.config.birthday_date.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();
	let db_addr2 = db_addr.clone();

	// Get the body of the request
	let mut member = match db::models::Teilnehmer::from_hashmap(body.clone(), &birthday_date) {
		Ok(member) => member,
		Err(error) => {
			warn!("Error handling form content: {}", error);
			return HttpResponse::BadRequest()
				.json(SignupResult { error: Some(error.to_string()) });
		}
	};

	// Remove spaces
	member.trim();

	match db_addr.send(db::CountMemberMessage).await {
		Err(error) => {
			warn!("Error inserting into database: {}", error);
			return HttpResponse::InternalServerError().json(SignupResult {
				error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
			});
		}
		Ok(Err(error)) => {
			warn!("Error inserting into database: {}", error);
			return HttpResponse::InternalServerError().json(SignupResult {
				error: Some(format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message)),
			});
		}
		Ok(Ok(count)) => {
			signup_check_count(
				count,
				max_members,
				&db_addr2,
				mail_addr,
				member,
				error_message,
				log_file,
				log_mutex,
				&**state,
			)
			.await
		}
	}
}

// TODO signup-nojs
