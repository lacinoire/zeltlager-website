//! The signup template.

use std::collections::HashMap;

#[derive(Template)]
#[TemplatePath = "templates/signup.tt"]
#[derive(Debug)]
pub struct Signup {
    /// Already entered values, which should be inserted into the form.
    pub values: HashMap<String, String>,
}

impl Signup {
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
