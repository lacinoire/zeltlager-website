//! Display images from a folder.

use std::fs;

use actix_web::*;
use log::{error, warn};

use crate::{State, Thumb};

pub fn split_image_name(s: &str) -> String {
	#[derive(Debug, Eq, PartialEq)]
	enum CharType {
		Letter,
		Number,
		None,
	}

	let mut last_type = CharType::None;
	let mut res = String::new();
	for c in s.chars() {
		let new_type = if c.is_ascii_digit() { CharType::Number } else { CharType::Letter };
		if new_type != last_type && last_type != CharType::None {
			res.push(' ');
		}
		res.push(c);
		last_type = new_type;
	}
	res
}

pub async fn list_images(state: State, name: String) -> HttpResponse {
	// List images
	let files = match fs::read_dir(format!("Bilder{name}")) {
		Ok(files) => files,
		Err(e) => {
			error!("Failed to list images in {}: {}", name, e);
			return HttpResponse::InternalServerError()
				.body("Fehler: Bilder konnten nicht gefunden werden.");
		}
	};
	let mut list = Vec::new();
	let thumbs = state.thumbs.read().unwrap();
	for file in files {
		let file = match file {
			Ok(file) => file,
			Err(error) => {
				error!("Cannot read picture from {} ({:?})", name, error);
				continue;
			}
		};
		let path = file.path();
		if !path.is_file() {
			continue;
		}
		match path.file_name() {
			None => warn!("Cannot get filename of {:?} in {}", path, name),
			Some(file_name) => match file_name.to_str() {
				None => warn!("Filename {:?} in {} is not valid unicode", path, name),
				Some(file_name) => {
					if file_name != ".gitignore" {
						let created = match file.metadata().and_then(|m| m.created()) {
							Ok(time) => time,
							Err(error) => {
								error!(
									"Cannot read created time of picture {} / {} ({:?})",
									name, file_name, error
								);
								std::time::SystemTime::now()
							}
						};
						if let Some(name) = path.to_str() {
							let file = thumbs.get(name).cloned().unwrap_or_else(|| {
								log::debug!("Failed to get thumb size for {name:?}");
								Thumb { name: file_name.to_string(), ..Thumb::default() }
							});
							list.push((file, created));
						}
					}
				}
			},
		}
	}

	// Sort the newest file first
	list.sort_unstable_by(|a, b| a.1.cmp(&b.1).reverse());
	let list = list.into_iter().map(|i| i.0).collect::<Vec<_>>();
	HttpResponse::Ok().json(list)
}
