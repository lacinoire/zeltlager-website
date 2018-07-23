use std::borrow::Cow;
use std::collections::HashMap;

pub trait Form {
	fn get_values(&self) -> Cow<HashMap<String, String>>;

	/// If this value exists and is the given value, return ` active`.
	fn bool_active(&self, name: &str, value: &str) -> String {
		if let Some(val) = self.get_values().get(name) {
			if val == value {
				return String::from(" active");
			}
		}
		String::new()
	}

	/// If this value exists and is the given value, return ` checked=""`.
	fn bool_checked(&self, name: &str, value: &str) -> String {
		if let Some(val) = self.get_values().get(name) {
			if val == value {
				return String::from(" checked=\"\"");
			}
		}
		String::new()
	}

	/// If this value exists, return ` value="<value>"` else return an empty
	/// string.
	fn opt_val(&self, name: &str) -> String {
		if let Some(val) = self.get_values().get(name) {
			format!(" value=\"{}\"", ::escape_html_attribute(val))
		} else {
			String::new()
		}
	}
}
