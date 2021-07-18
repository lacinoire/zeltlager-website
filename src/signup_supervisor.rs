//! The signup template.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use actix_identity::Identity;
use actix_web::*;
use log::{error, warn};
use t4rust_derive::Template;

use crate::form::Form;
use crate::{db, HttpRequest, HttpResponse, State};

#[derive(Template)]
#[TemplatePath = "templates/signupSupervisor.tt"]
#[derive(Debug)]
pub struct SignupSupervisor {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
}

impl Form for SignupSupervisor {
	fn get_values(&self) -> Cow<HashMap<String, String>> { Cow::Borrowed(&self.values) }
}

impl SignupSupervisor {
	pub fn new(_state: &State, values: HashMap<String, String>) -> Self { Self { values } }
}

#[get("/intern/betreuer-anmeldung")]
pub async fn signup(state: web::Data<State>, id: Identity, req: HttpRequest) -> HttpResponse {
	render_signup(&**state, &id, &req, HashMap::new()).await
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
	if let Ok(site) =
		state.sites["intern"].get_site(state.config.clone(), "betreuer-anmeldung", roles)
	{
		let content = format!("{}", site);
		let new_content = SignupSupervisor::new(state, values);
		let content = content.replace("<insert content here>", &format!("{}", new_content));
		HttpResponse::Ok().content_type("text/html; charset=utf-8").body(content)
	} else {
		crate::not_found(state, id, req).await
	}
}

/// show a success site.
fn signup_success() -> HttpResponse {
	// Redirect to success site
	HttpResponse::Found().header(http::header::LOCATION, "betreuer-anmeldung-erfolgreich").finish()
}

#[post("/intern/signup-supervisor-send")]
pub async fn signup_send(
	state: web::Data<State>, id: Identity, req: HttpRequest,
	mut body: web::Form<HashMap<String, String>>,
) -> HttpResponse {
	let db_addr = state.db_addr.clone();
	let error_message = state.config.error_message.clone();
	let birthday_date = state.config.birthday_date.clone();
	let log_file = state.config.log_file.clone();
	let log_mutex = state.log_mutex.clone();

	// Get the body of the request
	let supervisor = match db::models::Supervisor::from_hashmap(body.clone(), &birthday_date) {
		Ok(supervisor) => supervisor,
		Err(error) => {
			// Show error and prefilled form
			body.insert("error".to_string(), format!("{}", error));
			warn!("Error handling form content: {}", error);
			return render_signup(&**state, &id, &req, body.into_inner()).await;
		}
	};

	match db_addr.send(db::SignupSupervisorMessage { supervisor: supervisor.clone() }).await {
		Ok(Err(error)) => {
			warn!("Error inserting into database: {}", error);
		}
		Err(error) => {
			warn!("Error inserting into database: {}", error);
		}
		Ok(Ok(())) => {
			if let Some(log_file) = log_file {
				let res: Result<_, Error> = (|| {
					let _lock = log_mutex.lock().unwrap();
					let mut file =
						std::fs::OpenOptions::new().create(true).append(true).open(log_file)?;
					writeln!(file, "Betreuer: {}", serde_json::to_string(&supervisor)?)?;

					Ok(())
				})();

				if let Err(error) = res {
					body.insert(
						"error".to_string(),
						format!("Es ist ein Fehler beim Speichern aufgetreten.\n{}", error_message),
					);
					warn!("Failed to log new supervisor: {:?}", error);
					return render_signup(&**state, &id, &req, body.into_inner()).await;
				}
			}

			return signup_success();
		}
	}

	// Show error and prefilled form
	body.insert(
		"error".to_string(),
		format!("Es ist ein Datenbank-Fehler aufgetreten.\n{}", error_message),
	);
	render_signup(&**state, &id, &req, body.into_inner()).await
}
