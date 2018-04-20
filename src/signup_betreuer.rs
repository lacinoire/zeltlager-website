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

fn signup(req: HttpRequest<AppState>) -> BoxFuture<HttpResponse> {
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
	not_found(req)
}
