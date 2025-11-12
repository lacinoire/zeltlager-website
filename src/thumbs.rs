//! Automatically create thumbnails for pictures in a folder.

use std::collections::HashSet;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::{Result, bail, format_err};
use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::{EventKind, RecursiveMode};
use rayon::prelude::*;
use tracing::{debug, error, info, trace, warn};

use crate::{State, Thumb};

#[derive(Clone, Debug)]
struct CreateThumb {
	thumb: Thumb,
	orig: String,
	need_thumb: bool,
}

pub fn watch_thumbs(state: State, path: PathBuf) {
	// Create a channel to receive the events.
	let (tx, rx) = channel();

	if let Err(error) = scan_files(&state, &path, true) {
		error!(%error, "Error when scanning files");
	}

	let mut debouncer =
		new_debouncer(Duration::from_secs(10), Some(Duration::from_secs(10)), tx).unwrap();
	// Add a path to be watched. All files and directories at that path and
	// below will be monitored for changes.
	debouncer.watch(&path, RecursiveMode::NonRecursive).expect("Cannot watch directory");

	loop {
		match rx.recv() {
			Ok(Ok(v)) => {
				// Check for any write events
				if v.iter().any(|e| {
					matches!(
						e.kind,
						EventKind::Any
							| EventKind::Create(_)
							| EventKind::Modify(_)
							| EventKind::Remove(_)
					)
				}) {
					debug!(?v, "Got notify events");
					if let Err(error) = scan_files(&state, &path, false) {
						error!(%error, "Error when scanning files");
					}
				}
			}
			Ok(Err(error)) => {
				error!(?error, "Watch error");
				break;
			}
			Err(error) => {
				error!(%error, "Watch error");
				break;
			}
		}
	}
}

fn scan_files(state: &State, path: &Path, first_run: bool) -> Result<()> {
	info!(path = %path.display(), "Scanning for thumbnails");
	// Search for files where we need a thumbnail
	let mut create_thumbs = Vec::new();
	let files = fs::read_dir(path)?.map(|e| e.map_err(|e| e.into())).collect::<Result<Vec<_>>>()?;
	for file in &files {
		let file_path = file.path();
		if !file_path.is_file() {
			continue;
		}

		match file_path.file_name() {
			None => warn!(?file_path, "Cannot get filename"),
			Some(name) => {
				match name.to_str() {
					None => warn!(?file_path, "Filename is not valid unicode"),
					Some(name) => {
						let lower_name = name.to_lowercase();
						if lower_name.ends_with(".jpg")
							|| lower_name.ends_with(".jpeg")
							|| lower_name.ends_with(".png")
							|| lower_name.ends_with(".mp4")
						{
							// Check if there is a thumbnail for it
							match thumb_up_to_date(path, name) {
								Err(error) => {
									warn!(name, %error, "Failed to check thumbnail");
								}
								Ok(thumb) => create_thumbs.push(thumb),
							}
						}
					}
				}
			}
		}
	}

	// Remove outdated thumbnails
	if let Ok(thumbs) = fs::read_dir(path.join("thumbs")) {
		let thumb_names: HashSet<_> =
			create_thumbs.iter().filter_map(|t| t.thumb.thumb.as_deref()).collect();
		for file in thumbs {
			let file = file?;
			let name = file.file_name();
			if let Some(name) = name.to_str() {
				if !thumb_names.contains(&name) {
					debug!(name, "Remove outdated thumbnail");
					if let Err(error) = fs::remove_file(file.path()) {
						warn!(name, %error, "Failed to remove outdated thumbnail");
					}
				}
			}
		}
	}

	if first_run {
		// On the first run, get all thumbnail sizes
		create_thumbs.par_iter().for_each(|t| {
			if !t.need_thumb {
				if let Some(thumb) = &t.thumb.thumb {
					match get_thumb_size(&path.join("thumbs").join(thumb)) {
						Ok(size) => {
							let mut thumb = t.thumb.clone();
							thumb.width = Some(size.0);
							thumb.height = Some(size.1);
							trace!(name = t.orig, ?size, "Got thumbnail size");
							let mut thumbs = state.thumbs.write().unwrap();
							thumbs.insert(t.orig.clone(), thumb);
						}
						Err(error) => {
							warn!(thumb, %error, "Failed to get thumbnail size for");
						}
					}
				}
			}
		})
	}

	// Generate new thumbnails
	create_thumbs.into_par_iter().for_each(|t| {
		if !t.need_thumb {
			return;
		}
		let Some(thumb_name) = &t.thumb.thumb else {
			return;
		};
		let orig = path.join(&t.thumb.name);
		let thumb = path.join("thumbs").join(thumb_name);
		if let Err(error) = create_thumb(&orig, &thumb) {
			warn!(path = %orig.display(), %error, "Failed to create thumbnail");
		} else {
			// Add size
			let size = match get_thumb_size(&thumb) {
				Ok(r) => r,
				Err(error) => {
					warn!(file = ?orig.file_name().unwrap(), %error,
						"Failed to get thumbnail size"
					);
					return;
				}
			};
			let mut thumb = t.thumb.clone();
			thumb.width = Some(size.0);
			thumb.height = Some(size.1);
			trace!(name = t.orig, ?size, "Got thumbnail size");
			let mut thumbs = state.thumbs.write().unwrap();
			thumbs.insert(t.orig.clone(), thumb);
		}
	});

	Ok(())
}

/// Checks if the thumbnail is up-to-date.
///
/// If a thumbnail needs to be generated, returns source path and thumbnail path.
fn thumb_up_to_date(path: &Path, file: &str) -> Result<CreateThumb> {
	// Check if thumbnails directory exists
	let thumbs_path = path.join("thumbs");
	if !thumbs_path.exists() {
		fs::create_dir(&thumbs_path)?;
	}
	let orig_file = path.join(file);
	let ext = orig_file
		.extension()
		.and_then(|s| s.to_str())
		.map(|s| s.to_ascii_lowercase())
		.unwrap_or_default();
	let thumb_file = if ["mp4"].contains(&ext.as_str()) {
		thumbs_path.join(file).with_extension("jpg")
	} else {
		thumbs_path.join(file)
	};
	let mut create_thumb = CreateThumb {
		thumb: Thumb {
			name: file.to_string(),
			thumb: Some(
				thumb_file
					.file_name()
					.and_then(|s| s.to_str())
					.ok_or_else(|| format_err!("Failed to get thumb file name"))?
					.to_string(),
			),
			width: None,
			height: None,
		},
		orig: orig_file
			.to_str()
			.ok_or_else(|| format_err!("Path is not valid unicode"))?
			.to_string(),
		need_thumb: true,
	};

	// Check if thumbnail exists already and it is newer than the source image
	let orig_meta = orig_file.metadata()?;
	if let Ok(thumb_meta) = thumb_file.metadata() {
		let orig_modified = orig_meta.modified()?;
		let thumb_modified = thumb_meta.modified()?;

		let mut thumb_is_new = thumb_modified.duration_since(orig_modified).is_ok();
		if !thumb_is_new {
			debug!(
				?thumb_modified,
				?orig_modified,
				"Thumbnail is outdated: modified time earlier than file time"
			);
		}

		#[cfg(unix)]
		{
			let orig_t = orig_meta.ctime();
			let thumb_t = thumb_meta.ctime();
			thumb_is_new &= orig_t <= thumb_t;
			if !thumb_is_new {
				debug!(?thumb_t, ?orig_t, "Thumbnail is outdated: ctime earlier than file time");
			}
		}

		// Thumbnail exists and is newer in modification and
		// creation time.
		if thumb_is_new {
			create_thumb.need_thumb = false;
		}
	}

	Ok(create_thumb)
}

fn create_thumb(orig_file: &Path, thumb_file: &Path) -> Result<()> {
	// Scale it down
	let ext = orig_file
		.extension()
		.and_then(|s| s.to_str())
		.map(|s| s.to_ascii_lowercase())
		.unwrap_or_default();
	let thumb_ext = thumb_file
		.extension()
		.and_then(|s| s.to_str())
		.map(|s| s.to_ascii_lowercase())
		.unwrap_or_default();
	let orig_path_buf;
	let orig_path = orig_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?;
	let orig_path = if ["mp4"].contains(&ext.as_str()) {
		// Extract frame 0
		orig_path_buf = format!("{}[0]", orig_path);
		&orig_path_buf
	} else {
		orig_path
	};
	debug!(path = orig_path, "Create thumbnail");
	let proc = Command::new("magick")
		.args([
			orig_path,
			"-auto-orient", // Rotate, so we can strip metadata with jpegoptim
			"-resize",
			"300x300",
			thumb_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
		])
		.status()?;

	if !proc.success() {
		bail!("magick exited with exit code {}", proc);
	}

	if ["jpg", "jpeg"].contains(&thumb_ext.as_str()) {
		// Compress with jpegoptim
		let proc = Command::new("jpegoptim")
			.args([
				"-s",
				"--size=20",
				thumb_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
			])
			.status()?;

		if !proc.success() {
			bail!("jpegoptim exited with exit code {}", proc);
		}
	}

	Ok(())
}

fn get_thumb_size(thumb_file: &Path) -> Result<(u32, u32)> {
	let proc = Command::new("identify")
		.args([
			"-ping",
			"-format",
			"%w %h",
			thumb_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
		])
		.output()?;

	if !proc.status.success() {
		bail!("identify exited with exit code {:?}", proc);
	}

	match std::str::from_utf8(&proc.stdout)?
		.split(' ')
		.map(|s| s.parse())
		.collect::<Result<Vec<_>, _>>()?[..]
	{
		[w, h] => Ok((w, h)),
		_ => bail!("identify did not return width and height {:?}", proc),
	}
}
