//! Admin tools.

use actix_web::{HttpRequest, HttpResponse};
use actix_web::http::header::CONTENT_DISPOSITION;
use futures::Future;

use crate::auth;
use crate::{db, AppState, BoxFuture};

pub fn render_admin(
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	Box::new(auth::get_roles(&req)
		.and_then(move |res| {
			req.state().sites["public"].get_site(req.state().config.clone(), "admin/", res)
		})
		.map(|site| {
			HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(format!("{}", site))
		}))
}

/// Return all current members as csv.
pub fn download_members_csv(
	req: HttpRequest<AppState>,
) -> BoxFuture<HttpResponse> {
	let db_addr = req.state().db_addr.clone();
	let error_message = req.state().config.error_message.clone();

	Box::new(db_addr
		.send(db::DownloadMembersMessage)
		.from_err::<failure::Error>()
		.then(move |result| {
			match result {
				Err(error) | Ok(Err(error)) => {
					warn!("Error fetching from database: {}", error);
					Ok(HttpResponse::InternalServerError()
						.content_type("text/html; charset=utf-8")
						.body(format!(
							"Es ist ein Datenbank-Fehler \
							 aufgetreten.\n{}",
							error_message
						)))
				}
				Ok(Ok(members)) => {
					let mut res = Vec::new();
					{
						let mut writer = csv::Writer::from_writer(&mut res);
						for t in members {
							writer.serialize(t)?;
						}
					}

					Ok(HttpResponse::Ok()
						.content_type("text/csv; charset=utf-8")
						.header(CONTENT_DISPOSITION, "attachment;filename=teilnehmer.csv")
						.body(res))
				}
			}
		}))
}
