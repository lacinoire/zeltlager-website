extern crate actix_web;
#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate t4rust_derive;
extern crate toml;

use std::fs::File;
use std::io::Read;

use actix_web::*;

mod basic;

type Result<T> = std::result::Result<T, failure::Error>;

struct AppState {
    basics: basic::SiteDescriptions,
}

fn index(req: HttpRequest<AppState>) -> Result<HttpResponse> {
    let name: String = req.match_info().query("name")?;
    let site = req.state().basics.get_site(&name)?;
    let content = format!("{}", site);

    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn not_found(_: HttpRequest<AppState>) -> Result<HttpResponse> {
    let mut content = String::new();
    File::open("static/404.html")?.read_to_string(&mut content)?;
    Ok(httpcodes::HttpNotFound.build()
       .content_type("text/html; charset=utf-8")
       .body(content)?)
}

fn main() {
    HttpServer::new(|| {
        let basics = basic::SiteDescriptions::parse().expect("Failed to parse basic.toml");

        Application::with_state(AppState { basics })
            .middleware(middleware::Logger::default())
            .handler("/static", fs::StaticFiles::new("static", false)
                .default_handler(not_found))
            .resource("/{name}", |r| r.f(index))
            .default_resource(|r| r.f(not_found))
    }).bind("127.0.0.1:8080").unwrap()
        .run();
}
