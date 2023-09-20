//! Automatically create thumbnails for pictures in a folder.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::{bail, format_err, Result};
use log::{error, warn};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

pub fn watch_thumbs<P: AsRef<Path>>(path: P) {
	// Create a channel to receive the events.
	let (tx, rx) = channel();

	if let Err(e) = scan_files(&path) {
		error!("Error when scanning files: {:?}", e);
	}

	let mut debouncer = new_debouncer(Duration::from_secs(10), None, tx).unwrap();
	// Add a path to be watched. All files and directories at that path and
	// below will be monitored for changes.
	debouncer
		.watcher()
		.watch(path.as_ref(), RecursiveMode::Recursive)
		.expect("Cannot watch directory");

	loop {
		match rx.recv() {
			Ok(_) => {
				if let Err(e) = scan_files(&path) {
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

fn scan_files<P: AsRef<Path>>(path: P) -> Result<()> {
	let path = path.as_ref();
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
						if lower_name.ends_with(".jpg") || lower_name.ends_with(".png") {
							// Check if there is a thumbnail for it
							if let Err(e) = create_thumb(path, name) {
								warn!("Failed to create thumbnail for {}: {:?}", name, e);
							}
						}
					}
				}
			}
		}
	}

	// TODO Remove outdated thumbnails
	if let Ok(thumbs) = fs::read_dir(&path.join("thumbs")) {
		for file in thumbs {
			let file = file?;
			let name = file.file_name();
			if !files.iter().any(|f| f.file_name() == name) {
				if let Err(e) = fs::remove_file(file.path()) {
					warn!("Failed to remove outdated thumbnail {:?}: {:?}", name, e);
				}
			}
		}
	}

	Ok(())
}

fn create_thumb<P: AsRef<Path>>(path: P, file: &str) -> Result<()> {
	// Check if thumbnails directory exists
	let path = path.as_ref();
	let thumbs_path = path.join("thumbs");
	if !thumbs_path.exists() {
		fs::create_dir(&thumbs_path)?;
	}
	let orig_file = path.join(file);
	let thumb_file = thumbs_path.join(file);
	// Check if thumbnail exists already and it is newer than the source image
	let orig_meta = orig_file.metadata()?;
	if let Ok(thumb_meta) = thumb_file.metadata() {
		let mut is_newer = false;
		if let Ok(orig_t) = orig_meta.modified() {
			if let Ok(thumb_t) = thumb_meta.modified() {
				if thumb_t.duration_since(orig_t).is_ok() {
					// Thumbnail exists and is newer
					is_newer = true
				}
			}
		}

		// This function is not available on linux
		if let Ok(orig_t) = orig_meta.created() {
			// When copying or renaming files, the modification time is
			// preserved so we have to look at the creation time too.
			if let Ok(thumb_t) = thumb_meta.created() {
				if thumb_t.duration_since(orig_t).is_err() {
					is_newer = false;
				}
			}
		}

		#[cfg(unix)]
		{
			let orig_t = orig_meta.ctime();
			let thumb_t = thumb_meta.ctime();
			if orig_t > thumb_t {
				is_newer = false;
			}
		}

		// Thumbnail exists and is newer in modification and
		// creation time.
		if is_newer {
			return Ok(());
		}
	}

	// Check if we can scale it down
	let proc = Command::new("convert")
		.args([
			orig_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
			"-resize",
			"300x300",
			thumb_file.to_str().ok_or_else(|| format_err!("Path is not valid unicode"))?,
		])
		.status()?;

	if !proc.success() {
		bail!("convert exited with exit code {}", proc);
	}

	Ok(())
}
