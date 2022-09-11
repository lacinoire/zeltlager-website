//! The basic template.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::auth::Roles;

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescription {
	pub name: String,
	pub title: String,
	pub role: Option<Roles>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescriptions {
	pub sites: Vec<SiteDescription>,
	pub prefix: String,
}

impl SiteDescriptions {
	pub fn parse(path: &str) -> Result<Self> {
		let mut content = String::new();
		File::open(Path::new("dynamic").join(path))?.read_to_string(&mut content)?;
		Ok(toml::from_str(&content)?)
	}
}
