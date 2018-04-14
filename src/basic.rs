//! The basic template.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use pulldown_cmark::{html, Parser};

use {toml, Result};

fn get_true() -> bool {
	true
}

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescription {
	pub name: String,
	pub file_name: String,
	pub title: String,
	pub description: String,
	#[serde(default = "get_true")]
	pub navbar_visible: bool,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescriptions {
	pub sites: Vec<SiteDescription>,
	pub prefix: String,
}

#[derive(Template)]
#[TemplatePath = "templates/basic.tt"]
#[derive(Debug)]
pub struct Basic<'a> {
	pub config: &'a ::Config,
	pub all_sites: &'a HashMap<String, SiteDescriptions>,
	pub current_site: &'a SiteDescription,
	pub content: String,
	pub prefix: String,
}

impl SiteDescriptions {
	pub fn parse(path: &str) -> Result<Self> {
		let mut content = String::new();
		File::open(Path::new("dynamic").join(path))?
			.read_to_string(&mut content)?;
		Ok(toml::from_str(&content)?)
	}

	pub fn get_site<'a>(
		&'a self,
		config: &'a ::Config,
		name: &str,
	) -> Result<Basic<'a>> {
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
					config,
					all_sites: self,
					current_site: site,
					content,
					prefix: self.prefix,
				});
			}
		}
		bail!("Cannot find site {}", name)
	}
}
