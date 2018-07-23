//! auth code

#[derive(Clone, EnumString, Debug, Deserialize)]
pub enum Roles {
	ImageDownload2018,
	ImageUpload,
}
