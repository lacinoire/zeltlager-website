use std::fs::File;
use std::io::Read;
use std::path::Path;

use {Result, toml};

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescription {
    pub name: String,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SiteDescriptions {
    pub sites: Vec<SiteDescription>,
}

#[derive(Template)]
#[TemplatePath = "templates/basic.tt"]
#[derive(Debug)]
pub struct Basic<'a> {
    pub all_sites: &'a SiteDescriptions,
    pub description: &'a SiteDescription,
    pub content: String,
}

impl SiteDescriptions {
    pub fn parse() -> Result<Self> {
        let mut content = String::new();
        File::open(Path::new("dynamic").join("basic.toml"))?.read_to_string(&mut content)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn get_site(&self, name: &str) -> Result<Basic> {
        // Check if this site exists
        for site in &self.sites {
            if site.name == name {
                // Read file
                let mut content = String::new();
                File::open(Path::new("dynamic").join(&site.name))?.read_to_string(&mut content)?;

                return Ok(Basic {
                    all_sites: self,
                    description: site,
                    content,
                });
            }
        }
        bail!("Cannot find site {}", name)
    }
}