//! The signup template.
use std::collections::HashMap;

use form::Form;

#[derive(Template)]
#[TemplatePath = "templates/signupBetreuer.tt"]
#[derive(Debug)]
pub struct SignupBetreuer {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
}

impl Form for SignupBetreuer {
	fn get_values(&self) -> &HashMap<String, String> {
		&self.values
	}
}

impl SignupBetreuer {
	pub fn new(
		state: &::AppState,
		values: HashMap<String, String>,
	) -> Self {
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
) -> HttpResponse {
	if let Ok(site) = req.state()
		.sites["intern"]
		.get_site(&req.state().config, "betreuerAnmeldung")
	{
		let content = format!("{}", site);
		let new_content = SignupBetreuer::new(req.state(), values);
		let content = content.replace(
			"<insert content here>",
			&format!("{}", new_content),
		);

		return HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(content);
	}
	::not_found(req)
}

/// Insert the betreueree into the database, write an email and show a success site.
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
