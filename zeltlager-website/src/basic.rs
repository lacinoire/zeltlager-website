#[derive(Deserialize, Debug)]
pub struct SiteDescription {
    pub name: String,
    pub title: String,
    pub description: String,
}

#[derive(Deserialize, Debug)]
pub struct SiteDescriptions {
    pub sites: Vec<SiteDescription>,
}

#[derive(Template)]
#[TemplatePath = "templates/basic.tt"]
#[derive(Debug)]
pub struct Basic<'a> {
    pub all_sites: &'a [SiteDescription],
    pub description: &'a SiteDescription,
    pub content: String,
}
