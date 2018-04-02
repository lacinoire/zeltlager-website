//! The signup template.
use std::collections::HashMap;

use futures::Future;

use BoxFuture;

#[derive(Template)]
#[TemplatePath = "templates/signup.tt"]
#[derive(Debug)]
pub struct Signup {
	/// Already entered values, which should be inserted into the form.
	pub values: HashMap<String, String>,
	pub reached_max_members: Option<String>,
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

	/// If this value exists and is the given value, return ` active`.
	fn bool_active(&self, name: &str, value: &str) -> String {
		if let Some(val) = self.values.get(name) {
			if val == value {
				return String::from(" active");
			}
		}
		String::new()
	}

	/// If this value exists and is the given value, return ` checked=""`.
	fn bool_checked(&self, name: &str, value: &str) -> String {
		if let Some(val) = self.values.get(name) {
			if val == value {
				return String::from(" checked=\"\"");
			}
		}
		String::new()
	}

	/// If this value exists, return ` value="<value>"` else return an empty
	/// string.
	fn opt_val(&self, name: &str) -> String {
		if let Some(val) = self.values.get(name) {
			format!(" value=\"{}\"", ::escape_html_attribute(val))
		} else {
			String::new()
		}
	}
}
