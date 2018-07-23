//! rate limiting for requests
use actix_web::HttpRequest;
use AppState;

pub fn check_rate(req: HttpRequest<AppState>) -> bool {
	let request_ip = req.connection_info().remote();
	false
}

