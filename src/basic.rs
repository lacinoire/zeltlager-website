//! The basic template.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use pulldown_cmark::{html, Parser};

use crate::Result;
use crate::auth::Roles;
use crate::config::Config;

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescription {
	pub name: String,
	pub file_name: String,
	pub title: String,
	pub description: String,
	#[serde(default = "crate::get_true")]
	pub navbar_visible: bool,
	pub role: Option<Roles>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescriptions {
	pub sites: Vec<SiteDescription>,
	pub prefix: String,
}

#[derive(Template)]
#[TemplatePath = "templates/basic.tt"]
#[derive(Debug)]
pub struct Basic {
	pub logged_in_roles: Option<Vec<Roles>>,
	pub config: Config,
	pub all_sites: SiteDescriptions,
	pub current_site: SiteDescription,
	pub content: String,
}

impl SiteDescriptions {
	pub fn parse(path: &str) -> Result<Self> {
		let mut content = String::new();
		File::open(Path::new("dynamic").join(path))?
			.read_to_string(&mut content)?;
		Ok(toml::from_str(&content)?)
	}

	/// `logged_in_roles` should be `None` if the user is not logged in.
	pub fn get_site(
		&self,
		config: Config,
		name: &str,
		logged_in_roles: Option<Vec<Roles>>,
	) -> Result<Basic> {
		// Check if this site exists
		for site in &self.sites {
			if site.name == name {
				// Read file
				let mut content = String::new();
				File::open(Path::new("dynamic").join(&site.file_name))?
					.read_to_string(&mut content)?;
				if site.file_name.ends_with(".md") {
					let old_content = content.clone();
					let parser = Parser::new(&old_content);
					content.clear();
					html::push_html(&mut content, parser);
				}

				return Ok(Basic {
					logged_in_roles,
					config,
					all_sites: self.clone(),
					current_site: site.clone(),
					content,
				});
			}
		}
		bail!("Cannot find site {}", name)
	}
}
