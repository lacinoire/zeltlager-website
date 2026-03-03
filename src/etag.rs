//! ETag implementation

use std::hash::Hasher;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use axum::extract::{self, Request};
use axum::http::header::{CONTENT_LENGTH, ETAG, IF_NONE_MATCH, LAST_MODIFIED};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use clru::CLruCache;
use http_body_util::BodyExt;
use time::UtcDateTime;
use time::format_description::well_known;
use tracing::error;

#[derive(Clone)]
pub(crate) struct EtagLayer {
	/// ETag cache, mapping path to data
	cache: Arc<Mutex<CLruCache<String, EtagCacheEntry>>>,
}

#[derive(Clone, Debug)]
struct EtagCacheEntry {
	etag: u64,
	last_modified: Option<UtcDateTime>,
}

impl EtagLayer {
	pub(crate) fn new() -> Self {
		Self { cache: Arc::new(Mutex::new(CLruCache::new(NonZeroUsize::new(1000).unwrap()))) }
	}
}

pub(crate) async fn compute_etag(
	extract::State(this): extract::State<EtagLayer>, req: Request, next: axum::middleware::Next,
) -> Response {
	let path = req.uri().path().to_string();

	let if_none_match = req.headers().get_all(IF_NONE_MATCH).iter().cloned().collect::<Vec<_>>();

	let tag_matches = |if_none_match: &[HeaderValue], etag: u64| {
		let etag = etag.to_string();
		for h in if_none_match {
			if h.to_str().map(|h| h == etag).unwrap_or_default() {
				return true;
			}
		}
		false
	};

	// 1. Check if file is in cache
	let entry = this.cache.lock().unwrap().get(&path).cloned();
	if let Some(entry) = entry {
		// 1.1 If tag matches, check last-modified to see if modified
		let head_resp = next.clone().run(Request::head(req.uri()).body(().into()).unwrap()).await;
		let last_modified = head_resp
			.headers()
			.get(LAST_MODIFIED)
			.and_then(|s| s.to_str().ok())
			.and_then(|s| UtcDateTime::parse(s, &well_known::Rfc2822).ok());

		if last_modified == entry.last_modified {
			// 1.2 If not modified, check if tag matches
			if tag_matches(&if_none_match, entry.etag) {
				// 1.3 If yes, return cache hit
				return StatusCode::NOT_MODIFIED.into_response();
			} else {
				let mut resp = next.run(req).await;
				resp.headers_mut()
					.append(ETAG, HeaderValue::from_str(&entry.etag.to_string()).unwrap());
				return resp;
			}
		}
	}

	// 2. Get real response, check status code is 2xx (excluding 204 No Content) and content-length is unset or non-zero
	let resp = next.run(req).await;
	if matches!(resp.status().as_u16(), 200..=203 | 205..=299)
		&& resp.headers().get(CONTENT_LENGTH).map(|l| l != "0").unwrap_or(true)
	{
		let (parts, body) = resp.into_parts();
		let body = match body.collect().await {
			Err(error) => {
				error!(%error, "Failed to collect body");
				return (StatusCode::INTERNAL_SERVER_ERROR, "Fehler beim senden der Antwort")
					.into_response();
			}
			Ok(r) => r.to_bytes(),
		};

		// 2.1 Compute ETag, store it in the cache
		let mut hasher = fnv::FnvHasher::default();
		hasher.write(body.as_ref());
		let etag = hasher.finish();
		let last_modified = parts
			.headers
			.get(LAST_MODIFIED)
			.and_then(|s| s.to_str().ok())
			.and_then(|s| UtcDateTime::parse(s, &well_known::Rfc2822).ok());
		this.cache.lock().unwrap().put(path, EtagCacheEntry { etag, last_modified });

		// 2.2 If tag matches, return cache hit
		if tag_matches(&if_none_match, etag) {
			return StatusCode::NOT_MODIFIED.into_response();
		}

		let mut resp = Response::from_parts(parts, body.into());
		resp.headers_mut().append(ETAG, HeaderValue::from_str(&etag.to_string()).unwrap());
		return resp;
	}

	// 3. Return response
	resp
}
