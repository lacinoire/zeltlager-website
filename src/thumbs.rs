//! Automatically create thumbnails for pictures in a folder.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::{Result, bail, format_err};
use log::{debug, error, trace, warn};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use rayon::prelude::*;

use crate::State;

pub fn watch_thumbs(state: State, path: PathBuf) {
	// Create a channel to receive the events.
	let (tx, rx) = channel();

	if let Err(e) = scan_files(&state, &path, true) {
		error!("Error when scanning files: {:?}", e);
	}

	let mut debouncer = new_debouncer(Duration::from_secs(10), tx).unwrap();
	// Add a path to be watched. All files and directories at that path and
	// below will be monitored for changes.
	debouncer
		.watcher()
		.watch(path.as_ref(), RecursiveMode::Recursive)
		.expect("Cannot watch directory");

	loop {
		match rx.recv() {
			Ok(_) => {
				if let Err(e) = scan_files(&state, &path, false) {
					error!("Error when scanning files: {:?}", e);
				}
			}
			Err(e) => {
				error!("Watch error: {:?}", e);
				break;
			}
		}
	}
}

fn scan_files(state: &State, path: &Path, first_run: bool) -> Result<()> {
	let mut thumbs_for_size = Vec::new();
	let mut thumbs_to_generate = Vec::new();
	// Search for files where we need a thumbnail
	let files = fs::read_dir(path)?.map(|e| e.map_err(|e| e.into())).collect::<Result<Vec<_>>>()?;
	for file in &files {
		let file_path = file.path();
		if !file_path.is_file() {
			continue;
		}

		match file_path.file_name() {
			None => warn!("Cannot get filename of {:?}", file_path),
			Some(name) => {
				match name.to_str() {
					None => warn!("Filename {:?} is not valid unicode", file_path),
					Some(name) => {
						let lower_name = name.to_lowercase();
						if lower_name.ends_with(".jpg")
							|| lower_name.ends_with(".jpeg")
							|| lower_name.ends_with(".png")
						{
							// Check if there is a thumbnail for it
							match thumb_up_to_date(path, name) {
								Err(e) => {
									warn!("Failed to check thumbnail for {name}: {e:?}");
								}
								Ok(None) => {
									// up to date
									if first_run {
										// On the first run, get thumbnail size
										if let Some(orig) = path.join(name).to_str() {
											let thumb = path.join("thumbs").join(name);
											thumbs_for_size.push((orig.to_string(), thumb))
										}
									}
								}
								Ok(Some(p)) => thumbs_to_generate.push(p),
							}
						}
					}
				}
			}
		}
	}

	// Remove outdated thumbnails
	if let Ok(thumbs) = fs::read_dir(path.join("thumbs")) {
		for file in thumbs {
			let file = file?;
			let name = file.file_name();
			if !files.iter().any(|f| f.file_name() == name) {
				debug!("Remove outdated thumbnail {:?}", name);
				if let Err(e) = fs::remove_file(file.path()) {
					warn!("Failed to remove outdated thumbnail {:?}: {:?}", name, e);
				}
			}
		}
	}

	// Get thumbnail sizes
	thumbs_for_size.into_par_iter().for_each(|(orig, thumb)| match get_thumb_size(&thumb) {
		Ok(size) => {
			trace!("Got thumbnail size {orig:?} {size:?}");
			let mut thumb_sizes = state.thumb_sizes.write().unwrap();
			thumb_sizes.insert(orig, size);
		}
		Err(e) => {
			warn!("Failed to get thumbnail size for {thumb:?}: {e:?}",);
		}
	});

	// Generate new thumbnails
	thumbs_to_generate.into_par_iter().for_each(|(orig, thumb)| {
		if let Err(e) = create_thumb(&orig, &thumb) {
			warn!("Failed to create thumbnail for {:?}: {:?}", orig.file_name().unwrap(), e);
		} else {
			// Add size
			let Some(name) = orig.to_str() else {
				return;
			};
			let size = match get_thumb_size(&thumb) {
				Ok(r) => r,
				Err(e) => {
					warn!(
						"Failed to get thumbnail size for {:?}: {:?}",
						orig.file_name().unwrap(),
						e
					);
					return;
				}
			};
			trace!("Got thumbnail size {name:?} {size:?}");
			let mut thumb_sizes = state.thumb_sizes.write().unwrap();
			thumb_sizes.insert(name.to_string(), size);
		}
	});

	Ok(())
}

/// Checks if the thumbnail is up-to-date.
///
/// If a thumbnail needs to be generated, returns source path and thumbnail path.
fn thumb_up_to_date(path: &Path, file: &str) -> Result<Option<(PathBuf, PathBuf)>> {
	// Check if thumbnails directory exists
	let thumbs_path = path.join("thumbs");
	if !thumbs_path.exists() {
		fs::create_dir(&thumbs_path)?;
	}
	let orig_file = path.join(file);
	let thumb_file = thumbs_path.join(file);
	// Check if thumbnail exists already and it is newer than the source image
	let orig_meta = orig_file.metadata()?;
	if let Ok(thumb_meta) = thumb_file.metadata() {
		let orig_modified = orig_meta.modified()?;
		let thumb_modified = thumb_meta.modified()?;

		let mut thumb_is_new = thumb_modified.duration_since(orig_modified).is_ok();
		if !thumb_is_new {
			debug!(
				"Thumbnail is outdated (modified time): {thumb_modified:?} earlier than \
				 {orig_modified:?}"
			);
		}

		#[cfg(unix)]
		{
			let orig_t = orig_meta.ctime();
			let thumb_t = thumb_meta.ctime();
			thumb_is_new &= orig_t <= thumb_t;
			if !thumb_is_new {
				debug!("Thumbnail is outdated (ctime): {thumb_t:?} earlier than {orig_t:?}");
			}
		}

		// Thumbnail exists and is newer in modification and
		// creation time.
		if thumb_is_new {
			return Ok(None);
		}
	}

	// Needs to be generated
	Ok(Some((orig_file, thumb_file)))
}

fn create_thumb(orig_file: &Path, thumb_file: &Path) -> Result<()> {
	// Scale it down
	debug!("Create thumbnail for {:?}", orig_file.file_name().unwrap());
	let proc = Command::new("magick")
		.args([
			orig_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
			"-auto-orient", // Rotate, so we can strip metadata with jpegoptim
			"-resize",
			"300x300",
			thumb_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
		])
		.status()?;

	if !proc.success() {
		bail!("magick exited with exit code {}", proc);
	}

	if let Some(ext) = orig_file.extension().and_then(|s| s.to_str()) {
		let ext = ext.to_ascii_lowercase();
		if ["jpg", "jpeg"].contains(&ext.as_str()) {
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
